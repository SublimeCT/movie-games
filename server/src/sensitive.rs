use sensitive_rs::Filter;
use serde_json::Value;
use std::path::PathBuf;

pub(crate) struct SensitiveFilter {
    filter: Filter,
}

impl SensitiveFilter {
    pub(crate) fn from_env() -> Self {
        let mut filter = create_filter_with_default_dict();

        let mut words: Vec<String> = Vec::new();

        if let Ok(raw) = std::env::var("SENSITIVE_WORDS") {
            for part in raw.split([',', '\n', '\r', '\t']) {
                let w = part.trim();
                if !w.is_empty() {
                    words.push(w.to_string());
                }
            }
        }

        let path = std::env::var("SENSITIVE_WORDS_PATH")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "./sensitive_words.txt".to_string());

        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let w = line.trim();
                if w.is_empty() {
                    continue;
                }
                if w.starts_with('#') {
                    continue;
                }
                words.push(w.to_string());
            }
        }

        let refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
        filter.add_words(&refs);
        Self { filter }
    }

    #[cfg(test)]
    pub(crate) fn from_words(words: &[String]) -> Self {
        let mut filter = Filter::new();
        let refs: Vec<&str> = words.iter().map(|s| s.as_str()).collect();
        filter.add_words(&refs);
        Self { filter }
    }

    pub(crate) fn sanitize_json(&self, value: &mut Value) -> usize {
        self.sanitize_json_inner(value, None)
    }

    fn sanitize_json_inner(&self, value: &mut Value, key: Option<&str>) -> usize {
        match value {
            Value::String(s) => {
                if let Some(k) = key {
                    if should_skip_key(k) {
                        return 0;
                    }
                }

                let (cleaned, count) = self.sanitize_str(s);
                if count > 0 {
                    *s = cleaned;
                }
                count
            }
            Value::Array(arr) => arr
                .iter_mut()
                .map(|v| self.sanitize_json_inner(v, None))
                .sum(),
            Value::Object(obj) => obj
                .iter_mut()
                .map(|(k, v)| self.sanitize_json_inner(v, Some(k.as_str())))
                .sum(),
            _ => 0,
        }
    }

    pub(crate) fn sanitize_str(&self, text: &str) -> (String, usize) {
        let found = self.filter.find_all(text);
        let count = found.len();
        if count == 0 {
            return (text.to_string(), 0);
        }
        
        let mut cleaned = text.to_string();
        for word in found {
            let mask: String = std::iter::repeat('*').take(word.chars().count()).collect();
            cleaned = cleaned.replace(&word, &mask);
        }
        (cleaned, count)
    }
}

fn should_skip_key(key: &str) -> bool {
    matches!(
        key,
        "apiKey"
            | "baseUrl"
            | "model"
            | "size"
            | "backgroundImageBase64"
            | "avatarPath"
            | "avatar"
            | "image"
    )
}

fn create_filter_with_default_dict() -> Filter {
    if let Ok(path) = std::env::var("SENSITIVE_DEFAULT_DICT_PATH") {
        let p = path.trim();
        if !p.is_empty() {
            let mut filter = Filter::new();
            filter.load_word_dict(p).unwrap_or_else(|e| {
                panic!("无法加载 SENSITIVE_DEFAULT_DICT_PATH 指定的词库: {}", e)
            });
            return filter;
        }
    }

    if let Ok(filter) = Filter::with_default_dict() {
        return filter;
    }

    if let Some(p) = find_sensitive_rs_default_dict_in_cargo_registry() {
        let mut filter = Filter::new();
        filter
            .load_word_dict(&p)
            .unwrap_or_else(|e| panic!("无法加载 sensitive-rs 默认词库文件 {:?}: {}", p, e));
        return filter;
    }

    panic!(
        "无法加载 sensitive-rs 默认词库。请提供 SENSITIVE_DEFAULT_DICT_PATH 或确保运行目录存在 dict/dict.txt"
    );
}

fn find_sensitive_rs_default_dict_in_cargo_registry() -> Option<PathBuf> {
    let cargo_home = std::env::var("CARGO_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|home| PathBuf::from(home).join(".cargo"))
        })?;

    let registry_src = cargo_home.join("registry").join("src");
    let mut candidates: Vec<PathBuf> = Vec::new();

    let Ok(entries) = std::fs::read_dir(registry_src) else {
        return None;
    };

    for entry in entries.flatten() {
        let root = entry.path();
        let Ok(pkgs) = std::fs::read_dir(root) else {
            continue;
        };

        for pkg in pkgs.flatten() {
            let name = pkg.file_name().to_string_lossy().to_string();
            if !name.starts_with("sensitive-rs-") {
                continue;
            }
            let dict = pkg.path().join("dict").join("dict.txt");
            if dict.is_file() {
                candidates.push(dict);
            }
        }
    }

    candidates.sort();
    candidates.pop()
}

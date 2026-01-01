use sensitive_rs::Filter;
use serde_json::Value;

pub(crate) struct SensitiveFilter {
    filter: Filter,
}

impl SensitiveFilter {
    pub(crate) fn from_env() -> Self {
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

        Self::from_words(&words)
    }

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
        let cleaned = self.filter.replace(text, '*');
        (cleaned, count)
    }
}

fn should_skip_key(key: &str) -> bool {
    matches!(key, "apiKey" | "baseUrl" | "model" | "size")
}

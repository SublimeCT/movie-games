#[cfg(test)]
mod tests {
    use crate::sensitive::SensitiveFilter;
    use serde_json::json;

    #[test]
    fn test_sensitive_filter_sanitize_str_replaces_matches() {
        let filter = SensitiveFilter::from_words(&["abc".to_string()]);
        let (out, count) = filter.sanitize_str("xxabcxx");
        assert!(count > 0);
        assert_ne!(out, "xxabcxx");
        assert!(out.contains('*'));
    }

    #[test]
    fn test_sensitive_filter_sanitize_json_replaces_nested_strings() {
        let filter = SensitiveFilter::from_words(&["abc".to_string()]);
        let mut v = json!({
            "a": "xxabcxx",
            "b": ["no", "abc"],
            "c": { "d": "abc" }
        });

        let found = filter.sanitize_json(&mut v);
        assert!(found > 0);
        assert!(v.to_string().contains('*'));
    }

    #[test]
    fn test_sensitive_replacement_chinese() {
        let filter = SensitiveFilter::from_words(&["坏蛋".to_string()]);
        let (cleaned, count) = filter.sanitize_str("你是个坏蛋吗");
        assert_eq!(count, 1);
        assert!(cleaned.contains('*'));
        assert!(!cleaned.contains("坏蛋"));
        println!("Cleaned: {}", cleaned);
    }
}

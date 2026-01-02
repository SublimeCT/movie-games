#[cfg(test)]
mod tests {
    use crate::sensitive::SensitiveFilter;

    #[test]
    fn test_sensitive_replacement() {
        let filter = SensitiveFilter::from_words(&["bad".to_string()]);
        let (cleaned, count) = filter.sanitize_str("this is a bad word");
        assert_eq!(count, 1);
        assert_eq!(cleaned, "this is a *** word");
    }
}

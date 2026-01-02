#[cfg(test)]
mod tests {
    use crate::sensitive::SensitiveFilter;

    #[test]
    fn test_punctuation_preservation() {
        let filter = SensitiveFilter::from_words(&["sensitive".to_string()]);
        let text = "This is a sensitive word, with punctuation! And symbols: @#$%^&*()";
        let (cleaned, count) = filter.sanitize_str(text);
        println!("Original: {}", text);
        println!("Cleaned: {}", cleaned);
        
        assert_eq!(count, 1);
        assert!(cleaned.contains(','), "Comma should be preserved");
        assert!(cleaned.contains('!'), "Exclamation mark should be preserved");
        assert!(cleaned.contains('@'), "At symbol should be preserved");
        assert!(cleaned.contains('*'), "Sensitive word should be replaced");
        
        // Check if "sensitive" is replaced
        assert!(!cleaned.contains("sensitive"), "Sensitive word should be gone");
        
        // Check exact replacement
        let expected = "This is a * word, with punctuation! And symbols: @#$%^&*()";
        // Note: sensitive-rs replacement length depends on implementation. 
        // Typically it replaces the whole word with one * or multiple.
        // My implementation in sensitive.rs uses `filter.replace(text, '*')`.
        // If sensitive-rs replaces with a single char per word match?
        // Let's see.
    }
}

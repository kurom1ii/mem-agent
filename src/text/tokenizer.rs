use unicode_segmentation::UnicodeSegmentation;

const STOPWORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "can", "shall", "to", "of", "in", "for",
    "on", "with", "at", "by", "from", "as", "into", "through", "during",
    "before", "after", "above", "below", "between", "out", "off", "over",
    "under", "again", "further", "then", "once", "here", "there", "when",
    "where", "why", "how", "all", "each", "every", "both", "few", "more",
    "most", "other", "some", "such", "no", "not", "only", "own", "same",
    "so", "than", "too", "very", "just", "about", "up", "it", "its",
    "and", "but", "or", "nor", "because", "if", "that", "this", "which",
    "what", "who", "whom", "whose", "me", "my", "we", "our", "you",
    "your", "he", "his", "she", "her", "they", "their", "them",
];

pub fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word)
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_word_bounds()
        .filter(|w| {
            let trimmed = w.trim();
            !trimmed.is_empty()
                && trimmed.len() > 1
                && trimmed.chars().any(|c| c.is_alphabetic())
                && !is_stopword(trimmed)
        })
        .map(|w| w.trim().to_string())
        .collect()
}

pub fn tokenize_keep_stopwords(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_word_bounds()
        .filter(|w| {
            let trimmed = w.trim();
            !trimmed.is_empty()
                && trimmed.len() > 1
                && trimmed.chars().any(|c| c.is_alphabetic())
        })
        .map(|w| w.trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_stopword() {
        assert!(is_stopword("the"));
        assert!(is_stopword("and"));
        assert!(is_stopword("is"));
        assert!(!is_stopword("rust"));
        assert!(!is_stopword("memory"));
        assert!(!is_stopword("search"));
    }

    #[test]
    fn test_tokenize_english() {
        let text = "The quick brown fox jumps over the lazy dog";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert!(tokens.contains(&"jumps".to_string()));
        assert!(tokens.contains(&"lazy".to_string()));
        assert!(tokens.contains(&"dog".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"over".to_string()));
    }

    #[test]
    fn test_stopword_filter() {
        let text = "this is a test of the stopword filter and it works well";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"test".to_string()));
        assert!(tokens.contains(&"stopword".to_string()));
        assert!(tokens.contains(&"filter".to_string()));
        assert!(tokens.contains(&"works".to_string()));
        assert!(tokens.contains(&"well".to_string()));
        assert!(!tokens.contains(&"this".to_string()));
        assert!(!tokens.contains(&"is".to_string()));
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"of".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"and".to_string()));
        assert!(!tokens.contains(&"it".to_string()));
    }

    #[test]
    fn test_tokenize_keep_stopwords() {
        let text = "The quick brown fox";
        let tokens = tokenize_keep_stopwords(text);
        assert!(tokens.contains(&"the".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
    }

    #[test]
    fn test_tokenize_empty() {
        assert!(tokenize("").is_empty());
        assert!(tokenize("   ").is_empty());
    }

    #[test]
    fn test_tokenize_single_char() {
        let text = "a b c x y z";
        let tokens = tokenize(text);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_punctuation() {
        let text = "Hello, world! How are you? I'm fine.";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"fine".to_string()));
        assert!(!tokens.contains(&"you".to_string()));
    }
}

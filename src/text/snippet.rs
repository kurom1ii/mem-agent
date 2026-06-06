use crate::core::config::SNIPPET_WINDOW;

/// Generate a snippet from content around the first match of query tokens.
/// Highlights matching keywords by wrapping them in `**...**`.
pub fn generate_snippet(content: &str, query: &str, window: usize) -> String {
    let window = if window == 0 { SNIPPET_WINDOW } else { window };

    let query_tokens: Vec<String> = crate::text::tokenizer::tokenize(query)
        .into_iter()
        .filter(|t| t.len() > 1)
        .collect();

    if query_tokens.is_empty() {
        return content
            .chars()
            .take(window * 2)
            .collect::<String>()
            .trim()
            .to_string()
            + if content.chars().count() > window * 2 { "..." } else { "" };
    }

    let content_lower = content.to_lowercase();

    let best_offset = query_tokens
        .iter()
        .filter_map(|token| content_lower.find(token.as_str()))
        .min();

    match best_offset {
        Some(pos) => {
            let snippet = extract_window(content, pos, window);
            highlight(&snippet, &query_tokens, window)
        }
        None => {
            let preview: String = content.chars().take(window * 2).collect();
            if content.chars().count() > window * 2 {
                format!("{}...", preview.trim())
            } else {
                preview.trim().to_string()
            }
        }
    }
}

fn extract_window(content: &str, match_pos: usize, window: usize) -> String {
    let chars: Vec<char> = content.chars().collect();
    let total = chars.len();

    let half = window / 2;
    let start = match_pos.saturating_sub(half);
    let end = (match_pos + half).min(total);

    let mut result = String::with_capacity(window * 2);

    if start > 0 {
        result.push_str("...");
    }

    for ch in &chars[start..end] {
        result.push(*ch);
    }

    if end < total {
        result.push_str("...");
    }

    result
}

fn highlight(snippet: &str, query_tokens: &[String], _window: usize) -> String {
    let mut result = String::new();
    let lower = snippet.to_lowercase();
    let chars: Vec<char> = snippet.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let mut matched = false;
        for token in query_tokens {
            let rem = &lower[i..];
            if rem.starts_with(token.as_str()) {
                // Check token boundary
                let after_end = i + token.len();
                let is_start_boundary = i == 0 || !chars[i - 1].is_alphanumeric();
                let is_end_boundary =
                    after_end >= chars.len() || !chars[after_end].is_alphanumeric();

                if is_start_boundary && is_end_boundary {
                    result.push_str("**");
                    for ch in &chars[i..after_end] {
                        result.push(*ch);
                    }
                    result.push_str("**");
                    i = after_end;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_match_in_middle() {
        let content = "This is a very long document that contains many words and sentences. \
                       The important keyword is right here in the middle. \
                       And then there is more text after that which is not relevant.";
        let query = "important keyword";
        let snippet = generate_snippet(content, query, 80);
        assert!(snippet.contains("**important**"));
        assert!(snippet.contains("**keyword**"));
        assert!(snippet.contains("..."));
    }

    #[test]
    fn test_snippet_short_content() {
        let content = "Short text here";
        let snippet = generate_snippet(content, "text", 100);
        assert!(snippet.contains("**text**"));
    }

    #[test]
    fn test_snippet_no_match() {
        let content = "this is some random text without the query term";
        let snippet = generate_snippet(content, "xyznonexistent", 50);
        assert_eq!(snippet, "this is some random text without the query term");
    }

    #[test]
    fn test_snippet_empty_query() {
        let content = "Some content that should be previewed";
        let snippet = generate_snippet(content, "", 30);
        assert!(snippet.starts_with("Some content that"));
    }

    #[test]
    fn test_snippet_default_window() {
        let content = "x".repeat(50)
            + " target "
            + &"y".repeat(50);
        let snippet = generate_snippet(&content, "target", 0);
        assert!(snippet.contains("**target**"));
    }

    #[test]
    fn test_snippet_match_at_start() {
        let content = "target at the beginning of a much longer document with lots of extra words";
        let snippet = generate_snippet(content, "target", 40);
        assert!(snippet.contains("**target**"));
        assert!(!snippet.starts_with("..."));
    }

    #[test]
    fn test_snippet_match_at_end() {
        let content = "A long document with many words at the start and the target at the very end";
        let snippet = generate_snippet(content, "target", 40);
        assert!(snippet.contains("**target**"));
    }
}

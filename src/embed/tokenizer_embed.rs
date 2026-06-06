use std::path::Path;
use tokenizers::Tokenizer;

#[derive(Clone)]
pub struct TokenizerWrapper {
    inner: Tokenizer,
    max_length: usize,
}

impl TokenizerWrapper {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let tokenizer = Tokenizer::from_file(path)
            .map_err(|e| format!("Failed to load tokenizer: {e}"))?;
        Ok(Self {
            inner: tokenizer,
            max_length: 128,
        })
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u32>, String> {
        let encoding = self
            .inner
            .encode(text, false)
            .map_err(|e| format!("Tokenize failed: {e}"))?;
        let ids: Vec<u32> = encoding.get_ids().to_vec();
        if ids.len() > self.max_length {
            Ok(ids[..self.max_length].to_vec())
        } else {
            Ok(ids)
        }
    }

    pub fn encode_batch(&self, texts: &[&str]) -> Result<Vec<Vec<u32>>, String> {
        let encodings = self
            .inner
            .encode_batch(texts.to_vec(), false)
            .map_err(|e| format!("Batch tokenize failed: {e}"))?;
        let result: Vec<Vec<u32>> = encodings
            .iter()
            .map(|enc| {
                let ids = enc.get_ids().to_vec();
                if ids.len() > self.max_length {
                    ids[..self.max_length].to_vec()
                } else {
                    ids
                }
            })
            .collect();
        Ok(result)
    }

    pub fn vocab_size(&self) -> usize {
        self.inner.get_vocab_size(true)
    }

    pub fn pad_token_id(&self) -> Option<u32> {
        self.inner.get_padding().map(|p| {
            p.pad_id
                .try_into()
                .unwrap_or(0)
        })
    }
}

pub fn make_test_tokenizer() -> TokenizerWrapper {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("test_tokenizer_{id}.json"));
    let config = r#"{
  "version": "1.0",
  "truncation": null,
  "padding": null,
  "added_tokens": [
    {"id": 0, "content": "[PAD]", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true},
    {"id": 1, "content": "[UNK]", "single_word": false, "lstrip": false, "rstrip": false, "normalized": false, "special": true}
  ],
  "normalizer": {"type": "Lowercase"},
  "pre_tokenizer": {"type": "Whitespace"},
  "post_processor": null,
  "decoder": null,
  "model": {"type": "WordLevel", "vocab": {"[PAD]": 0, "[UNK]": 1, "hello": 2, "world": 3, "the": 4, "rust": 5, "is": 6, "fast": 7}, "unk_token": "[UNK]"}
}"#;
    std::fs::write(&tmp, config).unwrap();
    TokenizerWrapper::from_file(&tmp).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_encode() {
        let tok = make_test_tokenizer();
        let ids = tok.encode("hello world").unwrap();
        assert!(!ids.is_empty());
    }

    #[test]
    fn test_tokenizer_encode_batch() {
        let tok = make_test_tokenizer();
        let results = tok.encode_batch(&["hello world", "rust is fast"]).unwrap();
        assert_eq!(results.len(), 2);
    }
}

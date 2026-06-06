use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;

use crate::embed::tokenizer_embed::TokenizerWrapper;

const QUERY_PREFIX: &str = "task: search result | query: ";
const DOC_PREFIX: &str = "title: none | text: ";
const DEFAULT_MAX_LEN: usize = 256;

pub struct EmbeddingEngine {
    model_path: String,
    tokenizer: TokenizerWrapper,
    dim: usize,
}

impl EmbeddingEngine {
    pub fn new(model_path: &str, tokenizer: TokenizerWrapper) -> Self {
        Self {
            model_path: model_path.to_string(),
            tokenizer,
            dim: 768,
        }
    }

    pub fn init(mut self) -> Result<Self, String> {
        self.dim = Self::probe_dim(&self.model_path)?;
        Ok(self)
    }

    pub fn from_pretrained() -> Result<Self, String> {
        let (model_path, tokenizer_path) = crate::download::ensure_model_downloaded()?;
        let tokenizer = TokenizerWrapper::from_file(&tokenizer_path)?;
        Self::new(&model_path, tokenizer).init()
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    fn probe_dim(model_path: &str) -> Result<usize, String> {
        let mut session = Session::builder()
            .map_err(|e| format!("Builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Opt: {e}"))?
            .with_intra_threads(1)
            .map_err(|e| format!("Threads: {e}"))?
            .commit_from_file(model_path)
            .map_err(|e| format!("Load: {e}"))?;

        let ids_tensor = Tensor::from_array(([1usize, 3], vec![1i64, 2, 3]))
            .map_err(|e| format!("Probe ids: {e}"))?;
        let mask_tensor = Tensor::from_array(([1usize, 3], vec![1i64, 1, 1]))
            .map_err(|e| format!("Probe mask: {e}"))?;

        let mut outputs = session
            .run(ort::inputs![
                "input_ids" => ids_tensor.view(),
                "attention_mask" => mask_tensor.view(),
            ])
            .map_err(|e| format!("Probe run: {e}"))?;

        let embedding_value = outputs
            .remove("sentence_embedding")
            .or_else(|| {
                let key = outputs.keys().next().map(|k| k.to_string());
                key.and_then(|k| outputs.remove(k.as_str()))
            })
            .ok_or_else(|| "No output".to_string())?;

        let tensor: Tensor<f32> = embedding_value
            .downcast()
            .map_err(|e| format!("Downcast: {e}"))?;
        let arr = tensor.extract_array();
        let dim = *arr.shape().last().unwrap_or(&768);
        Ok(dim)
    }

    fn session(&self) -> Result<Session, String> {
        Session::builder()
            .map_err(|e| format!("Builder: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Opt: {e}"))?
            .with_intra_threads(4)
            .map_err(|e| format!("Threads: {e}"))?
            .commit_from_file(&self.model_path)
            .map_err(|e| format!("Load: {e}"))
    }

    pub fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        let prefixed = format!("{QUERY_PREFIX}{text}");
        self.embed(&prefixed)
    }

    pub fn embed_document(&self, text: &str) -> Result<Vec<f32>, String> {
        let prefixed = format!("{DOC_PREFIX}{text}");
        self.embed(&prefixed)
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let mut results = self.embed_batch(&[text])?;
        Ok(results.pop().unwrap_or_default())
    }

    pub fn embed_query_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        let prefixed: Vec<String> = texts
            .iter()
            .map(|t| format!("{QUERY_PREFIX}{t}"))
            .collect();
        let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&refs)
    }

    pub fn embed_document_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        let prefixed: Vec<String> = texts
            .iter()
            .map(|t| format!("{DOC_PREFIX}{t}"))
            .collect();
        let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&refs)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut session = self.session()?;

        let encodings: Vec<Vec<u32>> = texts
            .iter()
            .map(|s| self.tokenizer.encode(s))
            .collect::<Result<Vec<_>, _>>()?;

        let max_len = encodings
            .iter()
            .map(|e| e.len())
            .max()
            .unwrap_or(1)
            .min(DEFAULT_MAX_LEN);

        let batch_size = encodings.len();
        let mut input_ids = vec![0i64; batch_size * max_len];
        let mut attention_mask = vec![0i64; batch_size * max_len];

        for (i, ids) in encodings.iter().enumerate() {
            let len = ids.len().min(max_len);
            for (j, &id) in ids.iter().take(len).enumerate() {
                input_ids[i * max_len + j] = id as i64;
                attention_mask[i * max_len + j] = 1;
            }
        }

        let ids_tensor = Tensor::from_array(([batch_size, max_len], input_ids))
            .map_err(|e| format!("ids tensor: {e}"))?;
        let mask_tensor = Tensor::from_array(([batch_size, max_len], attention_mask))
            .map_err(|e| format!("mask tensor: {e}"))?;

        let mut outputs = session
            .run(ort::inputs![
                "input_ids" => ids_tensor.view(),
                "attention_mask" => mask_tensor.view(),
            ])
            .map_err(|e| format!("Inference: {e}"))?;

        let embedding_value = outputs
            .remove("sentence_embedding")
            .or_else(|| {
                let key = outputs.keys().next().map(|k| k.to_string());
                key.and_then(|k| outputs.remove(k.as_str()))
            })
            .ok_or_else(|| "No output".to_string())?;

        let tensor: Tensor<f32> = embedding_value
            .downcast()
            .map_err(|e| format!("Downcast: {e}"))?;
        let view = tensor.extract_array();

        let shape = view.shape();
        let out_dim = *shape.last().unwrap();

        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let mut sum = vec![0f32; out_dim];
            let valid_len = encodings[i].len().min(max_len);

            if shape.len() == 3 {
                let seq_len = shape[1];
                let mut count = 0f32;
                for j in 0..valid_len.min(seq_len) {
                    for k in 0..out_dim {
                        sum[k] += view[[i, j, k]];
                    }
                    count += 1.0;
                }
                if count > 0.0 {
                    for x in &mut sum {
                        *x /= count;
                    }
                }
            } else {
                for k in 0..out_dim {
                    sum[k] = view[[i, k]];
                }
            }

            let norm: f32 = sum.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
            for x in &mut sum {
                *x /= norm;
            }
            results.push(sum);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_stub() {
        assert!(true);
    }
}

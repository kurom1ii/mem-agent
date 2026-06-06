use std::collections::HashMap;
use candle_core::{Device, Tensor};
use candle_onnx::simple_eval;

use crate::embed::tokenizer_embed::TokenizerWrapper;

const QUERY_PREFIX: &str = "task: search result | query: ";
const DOC_PREFIX: &str = "title: none | text: ";

pub struct EmbeddingEngine {
    model_path: String,
    tokenizer: TokenizerWrapper,
}

struct EmbeddingSession {
    model: candle_onnx::onnx::ModelProto,
    tokenizer: TokenizerWrapper,
    device: Device,
    input_names: Vec<String>,
    output_name: String,
}

impl EmbeddingEngine {
    pub fn new(model_path: &str, tokenizer: TokenizerWrapper) -> Result<Self, String> {
        Ok(Self {
            model_path: model_path.to_string(),
            tokenizer,
        })
    }

    fn session(&self) -> Result<EmbeddingSession, String> {
        let model = candle_onnx::read_file(&self.model_path)
            .map_err(|e| format!("Failed to load ONNX model: {e}"))?;

        let input_names: Vec<String> = model
            .graph
            .as_ref()
            .map(|g| g.input.iter().map(|i| i.name.clone()).collect())
            .unwrap_or_default();

        let output_name = model
            .graph
            .as_ref()
            .and_then(|g| g.output.first())
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "sentence_embedding".into());

        Ok(EmbeddingSession {
            model,
            tokenizer: self.tokenizer.clone(),
            device: Device::Cpu,
            input_names,
            output_name,
        })
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
        let sess = self.session()?;
        sess.embed_text(text)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        let sess = self.session()?;
        sess.embed_batch(texts)
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
}

impl EmbeddingSession {
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        let token_ids = self.tokenizer.encode(text)?;
        let seq_len = token_ids.len();

        let ids_tensor = Tensor::from_slice(&token_ids, (1, seq_len), &self.device)
            .map_err(|e| format!("Tensor error: {e}"))?;

        let mask_data: Vec<f32> = vec![1.0f32; seq_len];
        let mask_tensor = Tensor::from_slice(&mask_data, (1, seq_len), &self.device)
            .map_err(|e| format!("Tensor error: {e}"))?;

        let mut inputs = HashMap::new();
        if self.input_names.len() >= 2 {
            inputs.insert(self.input_names[0].clone(), ids_tensor);
            inputs.insert(self.input_names[1].clone(), mask_tensor.clone());
        } else {
            inputs.insert("input_ids".into(), ids_tensor);
            inputs.insert("attention_mask".into(), mask_tensor.clone());
        }

        let outputs = simple_eval(&self.model, inputs)
            .map_err(|e| format!("Inference error: {e}"))?;

        let hidden = outputs
            .get(&self.output_name)
            .or_else(|| outputs.values().next())
            .ok_or_else(|| "No output tensor".to_string())?;

        if hidden.rank() == 2 && hidden.dims()[0] == 1 {
            let vec: Vec<f32> = hidden
                .get(0)
                .map_err(|e| format!("Slice error: {e}"))?
                .to_vec1()
                .map_err(|e| format!("To vec error: {e}"))?;
            self.l2_normalize_vec(vec)
        } else {
            let pooled = self.mean_pool_single(hidden, &mask_tensor)?;
            self.l2_normalize(&pooled)
        }
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        let all_ids: Vec<Vec<u32>> = self.tokenizer.encode_batch(texts)?;
        let n = all_ids.len();
        let max_len = all_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);

        let mut padded = vec![0u32; n * max_len];
        let mut masks = vec![0.0f32; n * max_len];

        for (i, ids) in all_ids.iter().enumerate() {
            let offset = i * max_len;
            for (j, &id) in ids.iter().enumerate() {
                padded[offset + j] = id;
                masks[offset + j] = 1.0;
            }
        }

        let ids_tensor = Tensor::from_slice(&padded, (n, max_len), &self.device)
            .map_err(|e| format!("Tensor error: {e}"))?;
        let mask_tensor = Tensor::from_slice(&masks, (n, max_len), &self.device)
            .map_err(|e| format!("Tensor error: {e}"))?;

        let mut inputs = HashMap::new();
        if self.input_names.len() >= 2 {
            inputs.insert(self.input_names[0].clone(), ids_tensor);
            inputs.insert(self.input_names[1].clone(), mask_tensor.clone());
        } else {
            inputs.insert("input_ids".into(), ids_tensor);
            inputs.insert("attention_mask".into(), mask_tensor.clone());
        }

        let outputs = simple_eval(&self.model, inputs)
            .map_err(|e| format!("Batch inference error: {e}"))?;

        let hidden = outputs
            .get(&self.output_name)
            .or_else(|| outputs.values().next())
            .ok_or_else(|| "No output tensor".to_string())?;

        self.mean_pool_batch(hidden, &mask_tensor, n)
    }

    fn mean_pool_single(&self, hidden: &Tensor, mask: &Tensor) -> Result<Tensor, String> {
        let mask_expanded = mask
            .unsqueeze(2)
            .map_err(|e| format!("Reshape error: {e}"))?;

        let weighted = hidden
            .broadcast_mul(&mask_expanded)
            .map_err(|e| format!("Mul error: {e}"))?;
        let summed = weighted
            .sum(1)
            .map_err(|e| format!("Sum error: {e}"))?;

        let mask_sum = mask
            .sum_all()
            .map_err(|e| format!("Sum error: {e}"))?;

        let mask_sum_reshaped = mask_sum
            .unsqueeze(0)
            .map_err(|e| format!("Reshape error: {e}"))?;

        summed
            .broadcast_div(&mask_sum_reshaped)
            .map_err(|e| format!("Div error: {e}").into())
    }

    fn mean_pool_batch(
        &self,
        hidden: &Tensor,
        mask: &Tensor,
        n: usize,
    ) -> Result<Vec<Vec<f32>>, String> {
        let mask_expanded = mask
            .unsqueeze(2)
            .map_err(|e| format!("Reshape error: {e}"))?;

        let weighted = hidden
            .broadcast_mul(&mask_expanded)
            .map_err(|e| format!("Mul error: {e}"))?;
        let summed = weighted
            .sum(1)
            .map_err(|e| format!("Sum error: {e}"))?;

        let mask_sum = mask
            .sum(1)
            .map_err(|e| format!("Sum error: {e}"))?;

        let summed_div = summed
            .broadcast_div(&mask_sum.unsqueeze(1).map_err(|e| format!("Reshape error: {e}"))?)
            .map_err(|e| format!("Div error: {e}"))?;

        (0..n)
            .map(|i| {
                let row = summed_div
                    .get(i)
                    .map_err(|e| format!("Slice error: {e}"))?;
                let vec: Vec<f32> = row
                    .to_vec1()
                    .map_err(|e| format!("To vec error: {e}"))?;
                self.l2_normalize_vec(vec)
            })
            .collect()
    }

    fn l2_normalize(&self, tensor: &Tensor) -> Result<Vec<f32>, String> {
        let vec: Vec<f32> = tensor
            .to_vec1()
            .map_err(|e| format!("To vec error: {e}"))?;
        self.l2_normalize_vec(vec)
    }

    fn l2_normalize_vec(&self, vec: Vec<f32>) -> Result<Vec<f32>, String> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-10);
        Ok(vec.into_iter().map(|x| x / norm).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_normalize() {
        let device = Device::Cpu;
        let t = Tensor::from_slice(&[3.0f32, 4.0], (1, 2), &device).unwrap();
        let t_flat = t.get(0).unwrap();
        let sess = EmbeddingSession {
            model: Default::default(),
            tokenizer: crate::embed::tokenizer_embed::make_test_tokenizer(),
            device,
            input_names: vec![],
            output_name: "sentence_embedding".into(),
        };
        let result = sess.l2_normalize(&t_flat).unwrap();
        assert!((result[0] - 0.6).abs() < 0.01);
        assert!((result[1] - 0.8).abs() < 0.01);
    }
}

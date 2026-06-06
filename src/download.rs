use std::path::PathBuf;

use crate::core::config::{MODEL_DIR, MODEL_ONNX, TOKENIZER_PATH};

const MODEL_ID: &str = "onnx-community/embeddinggemma-300m-ONNX";
const ONNX_FILE: &str = "onnx/model.onnx";
const ONNX_DATA_FILE: &str = "onnx/model.onnx_data";
const TOKENIZER_FILE: &str = "tokenizer.json";

pub fn ensure_model_downloaded() -> Result<(String, String), String> {
    let model_dir = PathBuf::from(MODEL_DIR);
    let model_onnx = PathBuf::from(MODEL_ONNX);
    let tokenizer_path = PathBuf::from(TOKENIZER_PATH);

    if model_onnx.exists() && tokenizer_path.exists() {
        return Ok((MODEL_ONNX.to_string(), TOKENIZER_PATH.to_string()));
    }

    std::fs::create_dir_all(&model_dir)
        .map_err(|e| format!("Cannot create model dir {model_dir:?}: {e}"))?;

    let api = hf_hub::api::sync::Api::new()
        .map_err(|e| format!("HF Hub error: {e}"))?;

    let repo = api.model(MODEL_ID.to_string());

    println!("⬇ Downloading embeddinggemma-300m-ONNX from HuggingFace...");

    let onnx_path = download_file(&repo, ONNX_FILE, MODEL_ONNX)?;
    let _ = download_file(&repo, ONNX_DATA_FILE, &format!("{MODEL_ONNX}_data"));

    let tok_path = download_file(&repo, TOKENIZER_FILE, TOKENIZER_PATH)?;

    println!("  Model ready at: {MODEL_ONNX}");

    Ok((onnx_path, tok_path))
}

fn download_file(repo: &hf_hub::api::sync::ApiRepo, remote: &str, local: &str) -> Result<String, String> {
    if std::path::Path::new(local).exists() {
        return Ok(local.to_string());
    }

    if let Some(parent) = std::path::Path::new(local).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create dir {parent:?}: {e}"))?;
    }

    println!("  Downloading {remote}...");
    let cached = repo
        .get(remote)
        .map_err(|e| format!("Download {remote} failed: {e}"))?;

    if cached != std::path::Path::new(local) {
        std::fs::copy(&cached, local)
            .map_err(|e| format!("Copy {remote} to {local}: {e}"))?;
    }

    Ok(local.to_string())
}

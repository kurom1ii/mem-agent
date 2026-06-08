use std::path::PathBuf;

use crate::core::config::{MODEL_DIR, TOKENIZER_PATH};

const MODEL_REPO: &str = "onnx-community/embeddinggemma-300m-ONNX";
const HF_BASE: &str = "https://huggingface.co";
const MODEL_ONNX_RELATIVE: &str =
    "plugins/opencode/models/embeddinggemma-300m-ONNX/onnx/model.onnx";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn model_dir() -> PathBuf {
    repo_root().join(MODEL_DIR)
}

fn model_onnx_path() -> PathBuf {
    repo_root().join(MODEL_ONNX_RELATIVE)
}

fn tokenizer_path() -> PathBuf {
    repo_root().join(TOKENIZER_PATH)
}

pub fn ensure_model_downloaded() -> Result<(String, String), String> {
    let model_file = model_onnx_path();
    let tokenizer_path = tokenizer_path();

    if model_file.exists() && tokenizer_path.exists() {
        return Ok((
            model_file.to_string_lossy().into_owned(),
            tokenizer_path.to_string_lossy().into_owned(),
        ));
    }

    let model_dir = model_dir();
    std::fs::create_dir_all(model_dir.join("onnx"))
        .map_err(|e| format!("Create model dir: {e}"))?;

    eprintln!("⬇ Downloading embeddinggemma-300m-ONNX from HuggingFace...");

    download_hf("onnx/model.onnx", &model_file)?;
    download_hf(
        "onnx/model.onnx_data",
        &PathBuf::from(format!("{}_data", model_file.to_string_lossy())),
    )?;
    download_hf("tokenizer.json", &tokenizer_path)?;

    eprintln!("  Model ready at: {}", model_file.to_string_lossy());

    Ok((
        model_file.to_string_lossy().into_owned(),
        tokenizer_path.to_string_lossy().into_owned(),
    ))
}

fn download_hf(remote: &str, dest: &PathBuf) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Create dir {parent:?}: {e}"))?;
    }

    let url = format!("{HF_BASE}/{MODEL_REPO}/resolve/main/{remote}");

    eprintln!("  Downloading {remote}...");
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "mem-agent/0.1")
        .send()
        .map_err(|e| format!("Download {remote}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("HF error {remote}: {e}"))?;

    let bytes = response
        .bytes()
        .map_err(|e| format!("Read body {remote}: {e}"))?;

    std::fs::write(dest, bytes).map_err(|e| format!("Write {remote}: {e}"))?;

    Ok(())
}

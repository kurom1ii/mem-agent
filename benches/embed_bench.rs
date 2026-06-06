fn main() {
    println!("Embed benchmarks - requires embeddinggemma-300m-ONNX model.");
    println!();
    println!("Download the model:");
    println!("  git clone https://huggingface.co/onnx-community/embeddinggemma-300m-ONNX models/embeddinggemma-300m-ONNX");
    println!();
    println!("Then run:");
    println!("  cargo bench --bench embed_bench");
}

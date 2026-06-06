use crate::embed::engine::EmbeddingEngine;

pub const DEFAULT_BATCH_SIZE: usize = 32;

pub fn embed_corpus(
    engine: &EmbeddingEngine,
    texts: &[String],
    batch_size: usize,
) -> Result<Vec<Vec<f32>>, String> {
    let batch_size = if batch_size == 0 { DEFAULT_BATCH_SIZE } else { batch_size };
    let n = texts.len();
    let mut results = Vec::with_capacity(n);

    for chunk in texts.chunks(batch_size) {
        let text_refs: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();
        let batch_results = engine.embed_batch(&text_refs)?;
        results.extend(batch_results);
    }

    Ok(results)
}

pub fn embed_corpus_parallel(
    engine: &EmbeddingEngine,
    texts: &[String],
    batch_size: usize,
) -> Result<Vec<Vec<f32>>, String> {
    let batch_size = if batch_size == 0 { DEFAULT_BATCH_SIZE } else { batch_size };

    let batches: Vec<Vec<&str>> = texts
        .chunks(batch_size)
        .map(|chunk| chunk.iter().map(|s| s.as_str()).collect())
        .collect();

    let results: Vec<Vec<Vec<f32>>> = batches
        .iter()
        .map(|batch| engine.embed_batch(batch))
        .collect::<Result<_, _>>()?;

    let total = results.iter().map(|r| r.len()).sum();
    let mut flat = Vec::with_capacity(total);
    for batch in results {
        flat.extend(batch);
    }
    Ok(flat)
}

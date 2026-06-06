use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mem_agent::core::config::DEFAULT_DIM;
use mem_agent::core::types::VectorMeta;
use mem_agent::vector::distance::{cosine, cosine_scalar, cosine_simd};
use mem_agent::vector::exact::exact_knn;
use mem_agent::vector::store::VectorStore;
use rand::Rng;

fn random_vector(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn normalized_random_vector(dim: usize) -> Vec<f32> {
    let v = random_vector(dim);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.into_iter().map(|x| x / norm).collect()
    } else {
        v
    }
}

fn bench_cosine_simd(c: &mut Criterion) {
    let dim = DEFAULT_DIM;
    let a = normalized_random_vector(dim);
    let b = normalized_random_vector(dim);

    c.bench_function("cosine_simd_384", |bench| {
        bench.iter(|| cosine_simd(black_box(&a), black_box(&b)));
    });
}

fn bench_cosine_scalar(c: &mut Criterion) {
    let dim = DEFAULT_DIM;
    let a = normalized_random_vector(dim);
    let b = normalized_random_vector(dim);

    c.bench_function("cosine_scalar_384", |bench| {
        bench.iter(|| cosine_scalar(black_box(&a), black_box(&b)));
    });
}

fn bench_cosine_auto(c: &mut Criterion) {
    let dim = DEFAULT_DIM;
    let a = normalized_random_vector(dim);
    let b = normalized_random_vector(dim);

    c.bench_function("cosine_auto_384", |bench| {
        bench.iter(|| cosine(black_box(&a), black_box(&b)));
    });
}

fn bench_exact_knn_1k(c: &mut Criterion) {
    let dim = DEFAULT_DIM;
    let n = 1000;
    let mut store = VectorStore::with_capacity(dim, n);
    let meta = VectorMeta {
        memory_id: 0,
        created_at: String::new(),
    };
    for i in 0..n {
        let v = normalized_random_vector(dim);
        let mut m = meta.clone();
        m.memory_id = i as i64;
        store.insert(i as i64, v, m).unwrap();
    }

    let query = normalized_random_vector(dim);

    c.bench_function("exact_knn_1k_k10", |bench| {
        bench.iter(|| exact_knn(black_box(&query), black_box(&store), black_box(10)));
    });
}

fn bench_exact_knn_10k(c: &mut Criterion) {
    let dim = DEFAULT_DIM;
    let n = 10_000;
    let mut store = VectorStore::with_capacity(dim, n);
    let meta = VectorMeta {
        memory_id: 0,
        created_at: String::new(),
    };
    for i in 0..n {
        let v = normalized_random_vector(dim);
        let mut m = meta.clone();
        m.memory_id = i as i64;
        store.insert(i as i64, v, m).unwrap();
    }

    let query = normalized_random_vector(dim);

    c.bench_function("exact_knn_10k_k10", |bench| {
        bench.iter(|| exact_knn(black_box(&query), black_box(&store), black_box(10)));
    });
}

fn bench_exact_knn_100k(c: &mut Criterion) {
    let dim = DEFAULT_DIM;
    let n = 100_000;
    let mut store = VectorStore::with_capacity(dim, n);
    let meta = VectorMeta {
        memory_id: 0,
        created_at: String::new(),
    };
    for i in 0..n {
        let v = normalized_random_vector(dim);
        let mut m = meta.clone();
        m.memory_id = i as i64;
        store.insert(i as i64, v, m).unwrap();
    }

    let query = normalized_random_vector(dim);

    c.bench_function("exact_knn_100k_k10", |bench| {
        bench.iter(|| exact_knn(black_box(&query), black_box(&store), black_box(10)));
    });
}

criterion_group!(
    benches,
    bench_cosine_simd,
    bench_cosine_scalar,
    bench_cosine_auto,
    bench_exact_knn_1k,
    bench_exact_knn_10k,
    bench_exact_knn_100k,
);
criterion_main!(benches);

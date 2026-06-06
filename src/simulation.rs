use crate::core::error::Result;
use crate::db::ops;
use rand::Rng;
use rusqlite::Connection;

const SAMPLE_TITLES: &[&str] = &[
    "Rust async programming guide",
    "SQLite optimization tips",
    "Machine learning pipeline design",
    "WebAssembly deep dive",
    "Distributed systems patterns",
    "TypeScript migration strategy",
    "Kubernetes deployment checklist",
    "PostgreSQL indexing guide",
    "React performance optimization",
    "Docker multi-stage builds",
];

const SAMPLE_CONTENTS: &[&str] = &[
    "Tokio provides async runtime for Rust. Use async/await for non-blocking I/O operations. The runtime handles task scheduling and execution efficiently across multiple threads.",
    "SQLite with WAL mode provides better concurrent read performance. Use prepared statements to avoid query parsing overhead. Index columns used in WHERE clauses for faster lookups.",
    "A typical ML pipeline includes data ingestion, preprocessing, feature engineering, model training, evaluation, and deployment. Use Apache Airflow or Prefect for orchestration.",
    "WebAssembly enables running code at near-native speed in browsers. Rust has first-class Wasm support through wasm-pack. Memory is managed through linear memory model.",
    "CAP theorem states you can only have two of Consistency, Availability, and Partition Tolerance. Eventual consistency models trade off strong consistency for availability.",
    "Converting large TypeScript codebases requires incremental migration. Use strict mode gradually starting with new files. Leverage type inference to reduce annotation burden.",
    "Kubernetes deployments need pod resource limits, health checks, rolling updates, and config maps. Use Helm charts for templating and versioning infrastructure.",
    "B-tree indexes are the default in PostgreSQL. Consider BRIN indexes for large sequential data. Partial indexes reduce size for filtered queries. Use EXPLAIN ANALYZE to verify.",
    "React.memo prevents unnecessary re-renders. useMemo caches expensive computations. useCallback stabilizes function references. Virtual lists handle large datasets efficiently.",
    "Multi-stage builds separate build and runtime environments. Copy only production artifacts to final image. Use distroless base images for minimal attack surface.",
];

const SAMPLE_TAGS: &[&str] = &[
    "rust,async,programming",
    "sqlite,database,optimization",
    "machine-learning,pipeline,mleops",
    "wasm,rust,frontend",
    "distributed-systems,architecture",
    "typescript,migration,javascript",
    "kubernetes,devops,deployment",
    "postgresql,database,performance",
    "react,performance,javascript",
    "docker,containers,devops",
];

pub fn generate_samples(conn: &Connection, count: usize) -> Result<Vec<i64>> {
    let mut ids = Vec::with_capacity(count);

    for i in 0..count {
        let idx = i % SAMPLE_TITLES.len();
        let title = format!("{} #{}", SAMPLE_TITLES[idx], i + 1);
        let content = SAMPLE_CONTENTS[idx];
        let tags = SAMPLE_TAGS[idx];

        let id = ops::insert_memory(conn, &title, content, tags)?;
        ids.push(id);
    }

    Ok(ids)
}

pub fn generate_random_samples(conn: &Connection, count: usize) -> Result<Vec<i64>> {
    let mut rng = rand::thread_rng();
    let mut ids = Vec::with_capacity(count);

    for i in 0..count {
        let ti = rng.gen_range(0..SAMPLE_TITLES.len());
        let ci = rng.gen_range(0..SAMPLE_CONTENTS.len());
        let gi = rng.gen_range(0..SAMPLE_TAGS.len());

        let title = format!("{} #{}", SAMPLE_TITLES[ti], i + 1);
        let content = SAMPLE_CONTENTS[ci];
        let tags = SAMPLE_TAGS[gi];

        let id = ops::insert_memory(conn, &title, content, tags)?;
        ids.push(id);
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::get_connection;
    use crate::db::ops::get_stats;

    #[test]
    fn test_generate_samples() {
        let conn = get_connection(":memory:").unwrap();
        let ids = generate_samples(&conn, 5).unwrap();
        assert_eq!(ids.len(), 5);
        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.memory_count, 5);
    }

    #[test]
    fn test_generate_random_samples() {
        let conn = get_connection(":memory:").unwrap();
        let ids = generate_random_samples(&conn, 10).unwrap();
        assert_eq!(ids.len(), 10);
    }
}

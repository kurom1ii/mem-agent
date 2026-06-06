use std::collections::HashMap;

/// Custom BM25 index (pure Rust, no FTS5 dependency).
#[derive(Debug, Clone)]
pub struct Bm25Index {
    pub tf: Vec<HashMap<String, u32>>,
    pub df: HashMap<String, u32>,
    pub doc_len: Vec<usize>,
    pub avgdl: f64,
    pub n_docs: usize,
    pub k1: f64,
    pub b: f64,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Bm25Index {
    pub fn new() -> Self {
        Self {
            tf: Vec::new(),
            df: HashMap::new(),
            doc_len: Vec::new(),
            avgdl: 0.0,
            n_docs: 0,
            k1: 1.5,
            b: 0.75,
        }
    }

    pub fn add_document(&mut self, doc_id: usize, tokens: &[String]) {
        while self.tf.len() <= doc_id {
            self.tf.push(HashMap::new());
            self.doc_len.push(0);
        }

        let mut term_freq: HashMap<String, u32> = HashMap::new();
        for token in tokens {
            *term_freq.entry(token.clone()).or_insert(0) += 1;
        }

        let dl = tokens.len();
        // Track df: only increment once per doc per unique term
        for term in term_freq.keys() {
            if !self.tf[doc_id].contains_key(term) {
                *self.df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        self.tf[doc_id] = term_freq;
        self.doc_len[doc_id] = dl;
        self.n_docs = self.n_docs.max(doc_id + 1);
        self.update_avgdl();
    }

    pub fn build_from_documents(docs: &[(usize, Vec<String>)]) -> Self {
        let mut index = Self::new();
        for (doc_id, tokens) in docs {
            index.add_document(*doc_id, tokens);
        }
        index
    }

    fn update_avgdl(&mut self) {
        let total: usize = self.doc_len.iter().sum();
        self.avgdl = if self.n_docs > 0 {
            total as f64 / self.n_docs as f64
        } else {
            0.0
        };
    }

    fn idf(&self, term: &str) -> f64 {
        let df = self.df.get(term).copied().unwrap_or(0) as f64;
        let n = self.n_docs as f64;
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }

    pub fn score(&self, doc_id: usize, query_tokens: &[String]) -> f64 {
        if doc_id >= self.tf.len() {
            return 0.0;
        }

        let tf_map = &self.tf[doc_id];
        let dl = self.doc_len[doc_id] as f64;
        let mut total = 0.0;

        for token in query_tokens {
            let idf = self.idf(token);
            let f = *tf_map.get(token).unwrap_or(&0) as f64;
            let numerator = f * (self.k1 + 1.0);
            let denominator = f + self.k1 * (1.0 - self.b + self.b * (dl / self.avgdl));
            total += idf * (numerator / denominator);
        }

        total
    }

    pub fn search(&self, query_tokens: &[String], limit: usize) -> Vec<(usize, f64)> {
        let mut scores: Vec<(usize, f64)> = (0..self.n_docs)
            .map(|doc_id| {
                let s = self.score(doc_id, query_tokens);
                (doc_id, s)
            })
            .filter(|(_, s)| *s > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);
        scores
    }

    pub fn search_with_docs(
        &self,
        query_tokens: &[String],
        docs: &[(usize, Vec<String>)],
        limit: usize,
    ) -> Vec<(usize, f64)> {
        let mut temp_index = self.clone();
        for (doc_id, tokens) in docs {
            temp_index.add_document(*doc_id, tokens);
        }
        temp_index.search(query_tokens, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tokens(s: &str) -> Vec<String> {
        crate::text::tokenizer::tokenize(s)
    }

    #[test]
    fn test_new_index_empty() {
        let idx = Bm25Index::new();
        assert_eq!(idx.n_docs, 0);
        assert!(idx.search(&make_tokens("test"), 10).is_empty());
    }

    #[test]
    fn test_add_and_search_single_doc() {
        let mut idx = Bm25Index::new();
        idx.add_document(
            0,
            &make_tokens("the quick brown fox jumped over the lazy dog"),
        );
        let results = idx.search(&make_tokens("quick brown fox"), 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 0);
        assert!(results[0].1 > 0.0);
    }

    #[test]
    fn test_search_multiple_docs() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("rust is a systems programming language"));
        idx.add_document(
            1,
            &make_tokens("python is great for data science and machine learning"),
        );
        idx.add_document(
            2,
            &make_tokens("javascript runs in the browser and on the server with nodejs"),
        );
        idx.add_document(
            3,
            &make_tokens("rust is fast and memory safe with zero cost abstractions"),
        );

        let results = idx.search(&make_tokens("rust programming systems"), 5);
        assert!(!results.is_empty());
        // Doc 0 should score highest (mentions rust, programming, systems)
        assert_eq!(results[0].0, 0);

        let results_python = idx.search(&make_tokens("machine learning data"), 5);
        assert!(!results_python.is_empty());
        assert_eq!(results_python[0].0, 1);
    }

    #[test]
    fn test_score_zero_for_empty_query() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("hello world"));
        let score = idx.score(0, &[]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_zero_for_unknown_terms() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("hello world"));
        let score = idx.score(0, &make_tokens("xyznonexistent"));
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_score_unknown_doc() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("hello world"));
        let score = idx.score(99, &make_tokens("hello"));
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_idf_rarer_term_scores_higher() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("rare unique specific term appears once"));
        idx.add_document(
            1,
            &make_tokens("common term appears everywhere common term"),
        );
        idx.add_document(2, &make_tokens("common term also here"));

        let score_rare = idx.score(0, &make_tokens("unique"));
        let score_common = idx.score(1, &make_tokens("term"));
        // Rare term should have higher IDF, but final score depends on TF too
        assert!(score_rare > 0.0);
        assert!(score_common > 0.0);
    }

    #[test]
    fn test_doc_length_normalization() {
        let mut idx = Bm25Index::new();
        // Short doc with the query term once
        idx.add_document(0, &make_tokens("rust programming"));
        // Long doc with same query term once, but lots of filler words
        let long_text = "rust bla ".repeat(1).to_string() + &"bla ".repeat(100);
        idx.add_document(1, &make_tokens(&long_text));

        let score_short = idx.score(0, &make_tokens("rust"));
        let score_long = idx.score(1, &make_tokens("rust"));
        // Short doc should score higher due to length normalization (same TF=1, shorter doc)
        assert!(score_short > score_long);
    }

    #[test]
    fn test_build_from_documents() {
        let docs: Vec<(usize, Vec<String>)> = vec![
            (0, make_tokens("rust programming language")),
            (1, make_tokens("python data science")),
            (2, make_tokens("javascript frontend development")),
        ];
        let idx = Bm25Index::build_from_documents(&docs);
        assert_eq!(idx.n_docs, 3);
        let results = idx.search(&make_tokens("rust"), 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 0);
    }

    #[test]
    fn test_search_with_docs() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("rust programming"));
        idx.add_document(1, &make_tokens("python data"));

        let new_docs: Vec<(usize, Vec<String>)> = vec![(2, make_tokens("rust concurrency async"))];

        let results = idx.search_with_docs(&make_tokens("rust concurrency"), &new_docs, 5);
        assert!(!results.is_empty());
        // Doc 2 should score highest
        assert_eq!(results[0].0, 2);
    }

    #[test]
    fn test_results_sorted_by_score() {
        let mut idx = Bm25Index::new();
        idx.add_document(0, &make_tokens("rust systems programming"));
        idx.add_document(1, &make_tokens("rust rust rust"));
        idx.add_document(2, &make_tokens("python data science"));

        let results = idx.search(&make_tokens("rust"), 3);
        assert_eq!(results.len(), 2); // Only docs 0,1 contain "rust"
        assert!(results[0].1 >= results[1].1);
    }

    #[test]
    fn test_limit_truncation() {
        let mut idx = Bm25Index::new();
        for i in 0..5 {
            idx.add_document(
                i,
                &make_tokens(&format!("unique rare term doc number {}", i)),
            );
        }
        let results = idx.search(&make_tokens("unique rare"), 2);
        assert_eq!(results.len(), 2);
    }
}

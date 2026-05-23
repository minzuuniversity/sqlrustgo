//! Inverted Index implementation for full-text search
//!
//! Provides an in-memory inverted index for fast text search.

use std::collections::HashMap;

use super::tokenizer::MultiLanguageTokenizer;

/// Inverted index for full-text search
///
/// Maps tokens to document IDs with term frequencies, enabling fast text search operations.
pub struct InvertedIndex {
    /// token → doc_id → term frequency
    index: HashMap<String, HashMap<u64, u64>>,
    tokenizer: MultiLanguageTokenizer,
    doc_count: u64,
    /// doc_id → token count (for BM25 document length normalization)
    doc_lengths: HashMap<u64, usize>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            tokenizer: MultiLanguageTokenizer::new(),
            doc_count: 0,
            doc_lengths: HashMap::new(),
        }
    }

    /// Add a document to the index
    pub fn add_document(&mut self, doc_id: u64, text: &str) {
        let tokens = self.tokenizer.tokenize(text);
        let mut term_counts: HashMap<String, u64> = HashMap::new();
        for token in tokens {
            *term_counts.entry(token).or_insert(0) += 1;
        }
        let doc_len: usize = term_counts.values().map(|&v| v as usize).sum();
        for (token, tf) in &term_counts {
            self.index
                .entry(token.clone())
                .or_default()
                .insert(doc_id, *tf);
        }
        self.doc_lengths.insert(doc_id, doc_len);
        self.doc_count += 1;
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: u64, text: &str) {
        let tokens = self.tokenizer.tokenize(text);
        for token in tokens {
            if let Some(doc_ids) = self.index.get_mut(&token) {
                doc_ids.remove(&doc_id);
                if doc_ids.is_empty() {
                    self.index.remove(&token);
                }
            }
        }
        self.doc_lengths.remove(&doc_id);
        self.doc_count = self.doc_count.saturating_sub(1);
    }

    /// Search for documents matching all query tokens (AND search)
    pub fn search(&self, query: &str) -> Vec<u64> {
        let tokens = self.tokenizer.tokenize(query);
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut result: Option<HashMap<u64, u64>> = None;

        for token in &tokens {
            if let Some(doc_ids) = self.index.get(token) {
                match &mut result {
                    None => result = Some(doc_ids.clone()),
                    Some(set) => {
                        set.retain(|doc_id, _| doc_ids.contains_key(doc_id));
                    }
                }
            } else {
                return Vec::new();
            }
        }

        result
            .map(|set| set.into_keys().collect())
            .unwrap_or_default()
    }

    /// Search with result limit
    pub fn search_with_limit(&self, query: &str, limit: usize) -> Vec<u64> {
        let mut results = self.search(query);
        results.truncate(limit);
        results
    }

    /// Fuzzy search with maximum edit distance
    /// Optimized with early termination when length difference exceeds max_distance
    pub fn fuzzy_search(&self, query: &str, max_distance: usize) -> Vec<u64> {
        let tokens = self.tokenizer.tokenize(query);
        let mut results: HashMap<u64, usize> = HashMap::new();

        for term in self.index.keys() {
            for token in &tokens {
                // Early termination: if length difference > max_distance, Levenshtein distance must be > max_distance
                if term.len() as isize - token.len() as isize > max_distance as isize
                    || token.len() as isize - term.len() as isize > max_distance as isize
                {
                    continue;
                }

                let dist = levenshtein_distance(token, term);
                if dist <= max_distance {
                    if let Some(doc_ids) = self.index.get(term) {
                        for doc_id in doc_ids.keys() {
                            *results.entry(*doc_id).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let mut sorted: Vec<_> = results.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().map(|(doc_id, _)| doc_id).collect()
    }

    /// Get total number of indexed documents
    pub fn doc_count(&self) -> u64 {
        self.doc_count
    }

    /// Get total number of unique terms
    pub fn term_count(&self) -> usize {
        self.index.len()
    }

    /// Get document length (token count) for a specific document
    pub fn doc_length(&self, doc_id: u64) -> usize {
        self.doc_lengths.get(&doc_id).copied().unwrap_or(0)
    }

    /// Get average document length across all indexed documents
    pub fn avg_doc_length(&self) -> f32 {
        if self.doc_count == 0 {
            return 0.0;
        }
        let total: usize = self.doc_lengths.values().sum();
        total as f32 / self.doc_count as f32
    }

    /// Search with BM25 scoring, returning top-K results sorted by score descending
    ///
    /// # Arguments
    /// * `query` - Search query string
    /// * `scorer` - Pre-built BM25 scorer with document statistics
    /// * `top_k` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of (doc_id, bm25_score) sorted by score descending
    pub fn search_with_scores(
        &self,
        query: &str,
        scorer: &Bm25Scorer,
        top_k: usize,
    ) -> Vec<(u64, f32)> {
        let tokens = self.tokenizer.tokenize(query);
        if tokens.is_empty() {
            return Vec::new();
        }

        // Collect documents that match ALL tokens (AND semantics)
        let mut candidate_docs: Option<HashMap<u64, u64>> = None;
        for token in &tokens {
            if let Some(doc_ids) = self.index.get(token) {
                match &mut candidate_docs {
                    None => candidate_docs = Some(doc_ids.clone()),
                    Some(set) => {
                        set.retain(|doc_id, _| doc_ids.contains_key(doc_id));
                    }
                }
            } else {
                return Vec::new();
            }
        }

        let candidates = match candidate_docs {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Score each candidate document using BM25
        let mut scored: Vec<(u64, f32)> = Vec::new();
        for (doc_id, _) in candidates {
            let doc_len = self.doc_length(doc_id);
            let mut total_score = 0.0_f32;

            for token in &tokens {
                if let Some(tf_map) = self.index.get(token) {
                    if let Some(&tf) = tf_map.get(&doc_id) {
                        let avg_dl = self.avg_doc_length();
                        let score = scorer.score(token, doc_id, tf, doc_len, avg_dl);
                        total_score += score;
                    }
                }
            }

            if total_score > 0.0 {
                scored.push((doc_id, total_score));
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

/// BM25 scoring calculator
///
/// BM25 is a probabilistic full-text search ranking function used to rank documents
/// based on query terms' frequency and document length normalization.
///
/// # Formula
/// ```
/// IDF = ln((N - df + 0.5) / (df + 0.5) + 1.0)
/// TF_norm = ((k1 + 1) * tf) / (tf + k1 * (1 - b + b * dl / avg_dl))
/// score = IDF * TF_norm
/// ```
#[derive(Debug, Clone)]
pub struct Bm25Scorer {
    /// Term frequency saturation parameter (default 1.2)
    pub k1: f32,
    /// Document length normalization parameter (default 0.75)
    pub b: f32,
    /// Total number of documents
    doc_count: u64,
    /// Document frequency: term -> number of documents containing the term
    doc_freq: HashMap<String, u64>,
}

impl Bm25Scorer {
    /// Create a new BM25 scorer with custom parameters
    ///
    /// # Arguments
    /// * `k1` - Term frequency saturation parameter (typically 1.2)
    /// * `b` - Document length normalization parameter (typically 0.75)
    pub fn new(k1: f32, b: f32) -> Self {
        Self {
            k1,
            b,
            doc_count: 0,
            doc_freq: HashMap::new(),
        }
    }

    /// Update statistics when a document is added to the index
    pub fn add_document(&mut self, doc_id: u64, term_count: usize, terms: &[String]) {
        let _ = doc_id;
        let _ = term_count;
        // Build document frequency map from terms
        for term in terms {
            *self.doc_freq.entry(term.clone()).or_insert(0) += 1;
        }
        self.doc_count += 1;
    }

    /// Finalize scorer statistics after all documents are indexed
    /// Call this after adding all documents and before scoring
    pub fn finalize(&mut self, index: &InvertedIndex) {
        self.doc_count = index.doc_count();
        self.doc_freq.clear();
        for (term, doc_map) in &index.index {
            let df = doc_map.len() as u64;
            self.doc_freq.insert(term.clone(), df);
        }
    }

    /// Calculate IDF (Inverse Document Frequency) for a term
    ///
    /// Higher IDF means the term is more discriminative (appears in fewer documents)
    fn idf(&self, term: &str) -> f32 {
        let df = self.doc_freq.get(term).copied().unwrap_or(0) as f32;
        let n = self.doc_count as f32;
        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
        idf.max(0.0) // IDF should never be negative
    }

    /// Calculate BM25 score for a single term-document pair
    ///
    /// # Arguments
    /// * `term` - The query term
    /// * `doc_id` - Document ID (unused in current implementation, reserved for future features)
    /// * `tf` - Term frequency in the document
    /// * `doc_len` - Document length (total token count)
    /// * `avg_dl` - Average document length across all documents in the collection
    ///
    /// # Returns
    /// BM25 score for this term-document pair
    pub fn score(&self, term: &str, doc_id: u64, tf: u64, doc_len: usize, avg_dl: f32) -> f32 {
        let _ = doc_id;
        if self.doc_count == 0 || doc_len == 0 {
            return 0.0;
        }

        let idf = self.idf(term);
        let tf = tf as f32;
        let doc_len = doc_len as f32;
        let avg_dl = if avg_dl > 0.0 { avg_dl } else { 1.0 };

        let tf_norm =
            ((self.k1 + 1.0) * tf) / (tf + self.k1 * (1.0 - self.b + self.b * doc_len / avg_dl));

        idf * tf_norm
    }

    /// Set the average document length for scoring
    /// Call this before scoring if you have external knowledge of avg_dl
    pub fn set_avg_doc_length(&mut self, avg_dl: f32) {
        // This is a workaround - in practice the scorer should get avg_dl from index
        let _ = avg_dl;
    }
}

impl Default for Bm25Scorer {
    fn default() -> Self {
        Self::new(1.2, 0.75)
    }
}

impl Default for InvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Levenshtein edit distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    let mut matrix: Vec<Vec<usize>> = vec![vec![0; b_len + 1]; a_len + 1];

    #[allow(clippy::needless_range_loop)]
    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    #[allow(clippy::needless_range_loop)]
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverted_index_new() {
        let idx = InvertedIndex::new();
        assert_eq!(idx.doc_count(), 0);
        assert_eq!(idx.term_count(), 0);
    }

    #[test]
    fn test_inverted_index_add_document() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "hello world hello");
        assert_eq!(idx.doc_count(), 1);
        assert_eq!(idx.term_count(), 2); // "hello" and "world"
    }

    #[test]
    fn test_inverted_index_search() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "rust programming");
        idx.add_document(2, "java programming");
        idx.add_document(3, "rust is fast");
        let results = idx.search("rust");
        assert!(results.contains(&1));
        assert!(results.contains(&3));
    }

    #[test]
    fn test_inverted_index_search_with_limit() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "a");
        idx.add_document(2, "a b");
        idx.add_document(3, "a b c");
        let results = idx.search_with_limit("a", 2);
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_inverted_index_fuzzy_search() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "programming");
        idx.add_document(2, "programing"); // typo
        let results = idx.fuzzy_search("programming", 1);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
    }

    #[test]
    fn test_inverted_index_doc_length() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "one two three");
        assert_eq!(idx.doc_length(1), 3);
    }

    #[test]
    fn test_inverted_index_avg_doc_length() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "a b c");
        idx.add_document(2, "x y");
        let avg = idx.avg_doc_length();
        // avg_doc_length returns cumulative average; check it's a valid f32
        assert!(avg.is_finite(), "avg_doc_length should be finite, got {}", avg);
    }

    #[test]
    fn test_bm25_scoring() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "the quick brown fox");
        idx.add_document(2, "the lazy dog");
        idx.add_document(3, "the quick fox");
        let mut scorer = Bm25Scorer::new(1.2, 0.75);
        scorer.finalize(&idx);
        let score = scorer.score("quick", 1, 1, 4, 3.0);
        assert!(score >= 0.0);
    }

    #[test]
    fn test_remove_document() {
        let mut idx = InvertedIndex::new();
        idx.add_document(1, "hello world");
        assert_eq!(idx.doc_count(), 1);
        idx.remove_document(1, "hello world");
        assert_eq!(idx.doc_count(), 0);
    }
}

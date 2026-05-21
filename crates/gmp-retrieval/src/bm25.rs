use std::collections::HashMap;

pub struct Bm25Search {
    documents: Vec<String>,
    term_freqs: Vec<HashMap<String, usize>>,
    doc_count: usize,
    avg_doc_len: f64,
    idf: HashMap<String, f64>,
    k1: f64,
    b: f64,
}

#[derive(Debug, Clone)]
pub struct Bm25Result {
    pub doc_id: usize,
    pub score: f64,
}

impl Bm25Search {
    pub fn new(documents: Vec<String>) -> Self {
        let doc_count = documents.len();
        let mut term_freqs = Vec::new();
        let mut doc_lens = Vec::new();

        for doc in &documents {
            let tokens = tokenize(doc);
            let mut freq = HashMap::new();
            for token in &tokens {
                *freq.entry(token.clone()).or_insert(0) += 1;
            }
            term_freqs.push(freq);
            doc_lens.push(tokens.len());
        }

        let avg_doc_len = doc_lens.iter().sum::<usize>() as f64 / doc_count.max(1) as f64;

        let mut idf = HashMap::new();
        let mut doc_freq = HashMap::new();
        for freq in &term_freqs {
            for term in freq.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }
        for (term, df) in doc_freq {
            let idf_val = ((doc_count as f64 - df as f64 + 0.5) / (df as f64 + 0.5)).ln();
            idf.insert(term, idf_val);
        }

        Self {
            documents,
            term_freqs,
            doc_count,
            avg_doc_len,
            idf,
            k1: 1.5,
            b: 0.75,
        }
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<Bm25Result> {
        let query_terms = tokenize(query);
        let mut scores = vec![0.0; self.doc_count];

        for (doc_id, tf) in self.term_freqs.iter().enumerate() {
            let doc_len = tf.values().sum::<usize>();
            let mut score = 0.0;

            for term in &query_terms {
                let term_freq = *tf.get(term).unwrap_or(&0);
                if term_freq == 0 {
                    continue;
                }
                let idf = *self.idf.get(term).unwrap_or(&0.0);
                let numerator = term_freq as f64 * (self.k1 + 1.0);
                let denominator = term_freq as f64
                    + self.k1 * (1.0 - self.b + self.b * doc_len as f64 / self.avg_doc_len);
                score += idf * numerator / denominator;
            }
            scores[doc_id] = score;
        }

        let mut results: Vec<Bm25Result> = scores
            .into_iter()
            .enumerate()
            .map(|(doc_id, score)| Bm25Result { doc_id, score })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(top_k);
        results
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

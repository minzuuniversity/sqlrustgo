use std::collections::HashMap;

pub struct RrfFusion {
    k: usize,
    scores: HashMap<usize, f64>,
    vote_counts: HashMap<usize, usize>,
}

#[derive(Debug, Clone)]
pub struct RrfResult {
    pub doc_id: usize,
    pub score: f64,
}

impl RrfFusion {
    pub fn new(k: usize) -> Self {
        Self {
            k,
            scores: HashMap::new(),
            vote_counts: HashMap::new(),
        }
    }

    pub fn add_result(&mut self, doc_id: usize, rank_score: f64) {
        *self.scores.entry(doc_id).or_insert(0.0) += rank_score;
        *self.vote_counts.entry(doc_id).or_insert(0) += 1;
    }

    pub fn rank(&self) -> Vec<RrfResult> {
        let mut results: Vec<RrfResult> = self
            .scores
            .iter()
            .map(|(&doc_id, &score)| {
                let votes = *self.vote_counts.get(&doc_id).unwrap_or(&1);
                let rrf_score = score + (1.0 / (self.k as f64 + votes as f64));
                RrfResult {
                    doc_id,
                    score: rrf_score,
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }
}

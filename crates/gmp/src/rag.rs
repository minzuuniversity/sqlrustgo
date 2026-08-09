//! GMP RAG Evidence Bundle
//!
//! Ensures every RAG answer is grounded in retrieved evidence.
//! Each answer carries a citation bundle; uncited content fails closed.

use crate::retrieval::{retrieval_search, RetrievalResult};
use crate::version::sha256_str;
use serde::{Deserialize, Serialize};
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::SqlResult;

/// A citation to a specific chunk with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub chunk_id: i64,
    pub chunk_hash: String,
    pub source_path: String,
    pub section_name: Option<String>,
    pub content_snippet: String,
    pub score: f32,
}

/// A citation bundle for a RAG answer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CitationBundle {
    pub citations: Vec<Citation>,
    pub evidence_hash: String,
    pub total_chunks: usize,
}

impl CitationBundle {
    /// Compute SHA-256 hash of all citation content.
    pub fn compute_evidence_hash(&self) -> String {
        let combined: String = self
            .citations
            .iter()
            .map(|c| format!("{}|{}|{}", c.chunk_id, c.chunk_hash, c.content_snippet))
            .collect();
        sha256_str(&combined)
    }

    /// Verify the evidence hash matches the computed one.
    pub fn verify(&self) -> bool {
        !self.citations.is_empty() && self.evidence_hash == self.compute_evidence_hash()
    }

    pub fn len(&self) -> usize { self.citations.len() }
    pub fn is_empty(&self) -> bool { self.citations.is_empty() }
}

/// Answer envelope containing the response and its evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerEnvelope {
    pub answer: String,
    pub citation_bundle: CitationBundle,
    pub timestamp: i64,
    pub evidence_hash: String,
    pub grounded: bool,
}

impl AnswerEnvelope {
    pub fn is_grounded(&self) -> bool {
        self.citation_bundle.verify() && self.grounded
    }

    pub fn summary(&self) -> String {
        let hash_prefix = if self.evidence_hash.len() >= 8 {
            &self.evidence_hash[..8]
        } else {
            &self.evidence_hash
        };
        format!(
            "AnswerEnvelope {{ grounded: {}, citations: {}, evidence_hash: {}... }}",
            self.grounded,
            self.citation_bundle.len(),
            hash_prefix,
        )
    }
}

/// Configuration for RAG generation.
#[derive(Debug, Clone)]
pub struct RagConfig {
    pub min_citations: usize,
    pub max_citation_length: usize,
    pub fail_on_uncited: bool,
    pub snippet_window: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            min_citations: 1,
            max_citation_length: 200,
            fail_on_uncited: true,
            snippet_window: 3,
        }
    }
}

fn build_citation(result: &RetrievalResult, snippet_window: usize) -> Citation {
    let snippet = extract_snippet(&result.citation_text, snippet_window);
    Citation {
        chunk_id: result.chunk_id,
        chunk_hash: result.chunk_hash.clone(),
        source_path: result.source_path.clone(),
        section_name: None,
        content_snippet: snippet,
        score: result.similarity,
    }
}

fn extract_snippet(text: &str, window: usize) -> String {
    let sentences: Vec<_> = text
        .split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .collect();

    if sentences.is_empty() {
        text.chars().take(200).collect()
    } else {
        let take = window.min(sentences.len());
        sentences[..take].join(". ").chars().take(200).collect()
    }
}

fn verify_citations(bundle: &CitationBundle) -> bool {
    if bundle.is_empty() {
        return false;
    }
    bundle.verify()
}

fn now_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Generate a RAG answer with full evidence bundle.
pub fn generate_rag_answer(
    storage: &dyn StorageEngine,
    question: &str,
    top_k: usize,
    config: &RagConfig,
) -> SqlResult<Result<AnswerEnvelope, String>> {
    let results = retrieval_search(storage, question, top_k)?;

    if results.is_empty() {
        let envelope = AnswerEnvelope {
            answer: "I could not find relevant evidence to answer this question.".to_string(),
            citation_bundle: CitationBundle::default(),
            timestamp: now_i64(),
            evidence_hash: "".to_string(),
            grounded: false,
        };
        return Ok(Err(envelope.answer));
    }

    let citations: Vec<Citation> = results
        .iter()
        .map(|r| build_citation(r, config.snippet_window))
        .collect();

    let total_chunks = citations.len();
    let evidence_hash = {
        let combined: String = citations
            .iter()
            .map(|c| format!("{}|{}|{}", c.chunk_id, c.chunk_hash, c.content_snippet))
            .collect();
        sha256_str(&combined)
    };

    let bundle = CitationBundle {
        citations,
        evidence_hash: evidence_hash.clone(),
        total_chunks,
    };

    let answer_snippets: Vec<_> = bundle.citations
        .iter()
        .map(|c| c.content_snippet.clone())
        .collect();

    let answer = if answer_snippets.is_empty() {
        "No relevant evidence found.".to_string()
    } else {
        format!("Based on the evidence: {}", answer_snippets.join(" "))
    };

    let grounded = verify_citations(&bundle);

    if config.fail_on_uncited && !grounded {
        return Ok(Err("Answer could not be verified against citations. Fail closed.".to_string()));
    }

    let envelope = AnswerEnvelope {
        answer,
        citation_bundle: bundle,
        timestamp: now_i64(),
        evidence_hash,
        grounded,
    };

    Ok(Ok(envelope))
}

/// Generate answer with evidence bundle (convenience wrapper).
pub fn rag_answer(
    storage: &dyn StorageEngine,
    question: &str,
) -> SqlResult<Result<AnswerEnvelope, String>> {
    generate_rag_answer(storage, question, 5, &RagConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_citation_bundle_empty() {
        let bundle = CitationBundle::default();
        assert!(bundle.is_empty());
        assert_eq!(bundle.len(), 0);
    }

    #[test]
    fn test_citation_bundle_verify_empty() {
        let bundle = CitationBundle::default();
        assert!(!bundle.verify());
    }

    #[test]
    fn test_answer_envelope_summary() {
        let envelope = AnswerEnvelope {
            answer: "Test answer".to_string(),
            citation_bundle: CitationBundle::default(),
            timestamp: 0,
            evidence_hash: "abc123def456".to_string(),
            grounded: false,
        };
        let s = envelope.summary();
        assert!(s.contains("grounded: false"));
        assert!(s.contains("abc123de"));
    }

    #[test]
    fn test_answer_envelope_not_grounded() {
        let envelope = AnswerEnvelope {
            answer: "Test".to_string(),
            citation_bundle: CitationBundle::default(),
            timestamp: 0,
            evidence_hash: "".to_string(),
            grounded: false,
        };
        assert!(!envelope.is_grounded());
    }

    #[test]
    fn test_extract_snippet_multiple_sentences() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let snippet = extract_snippet(text, 2);
        assert!(snippet.contains("First sentence"));
        assert!(snippet.contains("Second sentence"));
        assert!(!snippet.contains("Third"));
    }

    #[test]
    fn test_extract_snippet_single_sentence() {
        let text = "Only one sentence here.";
        let snippet = extract_snippet(text, 5);
        assert_eq!(snippet, "Only one sentence here");
    }

    #[test]
    fn test_extract_snippet_truncation() {
        let text = "A".repeat(300);
        let snippet = extract_snippet(&text, 10);
        assert!(snippet.len() <= 200);
    }

    #[test]
    fn test_rag_answer_empty_corpus() {
        let storage = sqlrustgo_storage::MemoryStorage::new();
        let result = rag_answer(&storage, "test question");
        assert!(result.is_ok());
        let answer = result.unwrap();
        assert!(answer.is_err());
    }
}

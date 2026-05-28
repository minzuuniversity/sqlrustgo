//! Evidence-based SQL-Verified AI Module
//!
//! This module provides evidence chain tracking and SQL verification
//! for AI-generated queries in SQLRustGo.

mod trace;

pub use trace::EvidenceTrace;

/// Evidence types for SQL verification
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Evidence {
    /// SQL query with optional parameters
    SQLQuery {
        query: String,
        params: Option<Vec<String>>,
    },
    /// External tool call evidence
    ToolCall {
        tool: String,
        input: String,
        output: Option<String>,
    },
    /// Query execution result
    Result {
        rows: Vec<Vec<String>>,
        row_count: usize,
    },
    /// Documentation or schema reference
    Document {
        doc_type: String,
        content: String,
    },
    /// AI heuristic or rule-based evidence
    Heuristic {
        rule: String,
        confidence: f32,
    },
}

impl Evidence {
    /// Returns the confidence weight of this evidence
    pub fn confidence_weight(&self) -> f32 {
        match self {
            Evidence::SQLQuery { .. } => 1.0,
            Evidence::ToolCall { .. } => 0.9,
            Evidence::Result { rows, row_count } => {
                if *row_count > 0 && !rows.is_empty() {
                    1.0
                } else {
                    0.3
                }
            }
            Evidence::Document { .. } => 0.7,
            Evidence::Heuristic { confidence, .. } => *confidence,
        }
    }
}

/// Verdict for SQL verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Verdict {
    /// Query is fully verified by evidence
    Proven,
    /// Query is partially verified
    Partial,
    /// Cannot verify the query
    Unverifiable,
    /// Evidence contradicts the query
    Contradicted,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Proven => write!(f, "Proven"),
            Verdict::Partial => write!(f, "Partial"),
            Verdict::Unverifiable => write!(f, "Unverifiable"),
            Verdict::Contradicted => write!(f, "Contradicted"),
        }
    }
}

/// Evidence chain containing multiple evidence items
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceChain {
    /// List of evidence items in the chain
    pub evidences: Vec<Evidence>,
    /// Overall confidence score (0.0 to 1.0)
    pub confidence_score: f32,
}

impl EvidenceChain {
    /// Creates a new empty evidence chain
    pub fn new() -> Self {
        Self {
            evidences: Vec::new(),
            confidence_score: 0.0,
        }
    }

    /// Creates an evidence chain with initial evidence
    pub fn with_evidence(evidence: Evidence) -> Self {
        let confidence = evidence.confidence_weight();
        Self {
            evidences: vec![evidence],
            confidence_score: confidence,
        }
    }

    /// Adds evidence to the chain
    pub fn add(&mut self, evidence: Evidence) {
        let weight = evidence.confidence_weight();
        let total_weight: f32 = self.evidences.iter().map(|e| e.confidence_weight()).sum();
        let new_weight = weight;
        
        // Weighted average confidence
        self.confidence_score = (self.confidence_score * total_weight + new_weight) 
            / (total_weight + new_weight);
        self.evidences.push(evidence);
    }

    /// Returns true if the chain contains SQL query evidence
    pub fn has_sql_query(&self) -> bool {
        self.evidences.iter().any(|e| matches!(e, Evidence::SQLQuery { .. }))
    }

    /// Returns true if the chain contains result evidence
    pub fn has_result(&self) -> bool {
        self.evidences.iter().any(|e| matches!(e, Evidence::Result { .. }))
    }
}

impl Default for EvidenceChain {
    fn default() -> Self {
        Self::new()
    }
}

/// SQL verification function
///
/// Verifies a SQL query against an evidence chain and returns a Verdict.
///
/// # Arguments
/// * `query` - The SQL query string to verify
/// * `evidence` - The evidence chain to verify against
///
/// # Returns
/// * `Verdict::Proven` - Query is fully verified (confidence >= 0.9)
/// * `Verdict::Partial` - Query is partially verified (confidence 0.5-0.9)
/// * `Verdict::Unverifiable` - No relevant evidence found
/// * `Verdict::Contradicted` - Evidence contradicts the query
pub fn sql_verify(query: &str, evidence: &EvidenceChain) -> Verdict {
    // No evidence at all
    if evidence.evidences.is_empty() {
        return Verdict::Unverifiable;
    }

    // Check for SQL query evidence
    let has_sql = evidence.has_sql_query();
    let has_result = evidence.has_result();

    // If we have result evidence with zero rows, query is contradicted
    if has_result {
        for e in &evidence.evidences {
            if let Evidence::Result { rows, row_count } = e {
                if *row_count == 0 || rows.is_empty() {
                    // Check if query expects to return data
                    let query_upper = query.to_uppercase();
                    if query_upper.contains("SELECT") && !query_upper.contains("INSERT") {
                        return Verdict::Contradicted;
                    }
                }
            }
        }
    }

    // Check for contradictions in heuristic evidence
    for e in &evidence.evidences {
        if let Evidence::Heuristic { confidence, .. } = e {
            if *confidence < 0.3 {
                return Verdict::Contradicted;
            }
        }
    }

    // Determine verdict based on confidence score
    if has_sql && evidence.confidence_score >= 0.9 {
        Verdict::Proven
    } else if evidence.confidence_score >= 0.5 {
        Verdict::Partial
    } else {
        Verdict::Unverifiable
    }
}

/// Creates a new evidence trace with generated trace_id
pub fn create_trace(steps: Vec<Evidence>, verdict: Verdict) -> EvidenceTrace {
    EvidenceTrace::new(steps, verdict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_confidence_weights() {
        let sql = Evidence::SQLQuery {
            query: "SELECT * FROM users".to_string(),
            params: None,
        };
        assert_eq!(sql.confidence_weight(), 1.0);

        let tool = Evidence::ToolCall {
            tool: "validator".to_string(),
            input: "{}".to_string(),
            output: Some("true".to_string()),
        };
        assert_eq!(tool.confidence_weight(), 0.9);

        let result = Evidence::Result {
            rows: vec![vec!["1".to_string()]],
            row_count: 1,
        };
        assert_eq!(result.confidence_weight(), 1.0);

        let empty_result = Evidence::Result {
            rows: vec![],
            row_count: 0,
        };
        assert_eq!(empty_result.confidence_weight(), 0.3);

        let doc = Evidence::Document {
            doc_type: "schema".to_string(),
            content: "users table".to_string(),
        };
        assert_eq!(doc.confidence_weight(), 0.7);

        let heuristic = Evidence::Heuristic {
            rule: "rule1".to_string(),
            confidence: 0.85,
        };
        assert_eq!(heuristic.confidence_weight(), 0.85);
    }

    #[test]
    fn test_evidence_chain_new() {
        let chain = EvidenceChain::new();
        assert!(chain.evidences.is_empty());
        assert_eq!(chain.confidence_score, 0.0);
    }

    #[test]
    fn test_evidence_chain_with_evidence() {
        let sql = Evidence::SQLQuery {
            query: "SELECT 1".to_string(),
            params: None,
        };
        let chain = EvidenceChain::with_evidence(sql);
        assert_eq!(chain.evidences.len(), 1);
        assert_eq!(chain.confidence_score, 1.0);
    }

    #[test]
    fn test_evidence_chain_add() {
        let sql = Evidence::SQLQuery {
            query: "SELECT 1".to_string(),
            params: None,
        };
        let doc = Evidence::Document {
            doc_type: "schema".to_string(),
            content: "test".to_string(),
        };
        
        let mut chain = EvidenceChain::with_evidence(sql);
        chain.add(doc);
        
        assert_eq!(chain.evidences.len(), 2);
    }

    #[test]
    fn test_sql_verify_proven() {
        let sql = Evidence::SQLQuery {
            query: "SELECT * FROM users".to_string(),
            params: None,
        };
        let result = Evidence::Result {
            rows: vec![vec!["1".to_string(), "alice".to_string()]],
            row_count: 1,
        };
        
        let mut chain = EvidenceChain::with_evidence(sql);
        chain.add(result);
        
        let verdict = sql_verify("SELECT * FROM users", &chain);
        assert_eq!(verdict, Verdict::Proven);
    }

    #[test]
    fn test_sql_verify_unverifiable() {
        let chain = EvidenceChain::new();
        let verdict = sql_verify("SELECT * FROM users", &chain);
        assert_eq!(verdict, Verdict::Unverifiable);
    }

    #[test]
    fn test_sql_verify_contradicted() {
        let sql = Evidence::SQLQuery {
            query: "SELECT * FROM users".to_string(),
            params: None,
        };
        let empty_result = Evidence::Result {
            rows: vec![],
            row_count: 0,
        };
        
        let mut chain = EvidenceChain::with_evidence(sql);
        chain.add(empty_result);
        
        let verdict = sql_verify("SELECT * FROM users", &chain);
        assert_eq!(verdict, Verdict::Contradicted);
    }

    #[test]
    fn test_sql_verify_partial() {
        let doc = Evidence::Document {
            doc_type: "schema".to_string(),
            content: "users table definition".to_string(),
        };
        
        let chain = EvidenceChain::with_evidence(doc);
        
        let verdict = sql_verify("SELECT * FROM users", &chain);
        assert_eq!(verdict, Verdict::Partial);
    }

    #[test]
    fn test_verdict_display() {
        assert_eq!(Verdict::Proven.to_string(), "Proven");
        assert_eq!(Verdict::Partial.to_string(), "Partial");
        assert_eq!(Verdict::Unverifiable.to_string(), "Unverifiable");
        assert_eq!(Verdict::Contradicted.to_string(), "Contradicted");
    }

    #[test]
    fn test_create_trace() {
        let sql = Evidence::SQLQuery {
            query: "SELECT 1".to_string(),
            params: None,
        };
        let trace = create_trace(vec![sql], Verdict::Proven);
        assert!(!trace.trace_id.is_empty());
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.verdict, Verdict::Proven);
    }
}

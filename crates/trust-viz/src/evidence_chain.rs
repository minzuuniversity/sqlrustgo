//! Evidence chain visualization

#[derive(Debug, Clone)]
pub struct EvidenceChainViz {
    pub chain_id: String,
    pub entries: Vec<ChainEntry>,
    pub is_verified: bool,
}

#[derive(Debug, Clone)]
pub struct ChainEntry {
    pub entry_type: String,
    pub description: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signer: Option<String>,
    pub hash: String,
}

impl EvidenceChainViz {
    pub fn new(chain_id: &str) -> Self {
        Self {
            chain_id: chain_id.to_string(),
            entries: Vec::new(),
            is_verified: false,
        }
    }

    pub fn add_entry(&mut self, entry: ChainEntry) {
        self.entries.push(entry);
    }

    pub fn verify_chain(&mut self) -> bool {
        self.is_verified = self
            .entries
            .windows(2)
            .all(|w| w[0].timestamp <= w[1].timestamp);
        self.is_verified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn later_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn test_evidence_chain_viz_new() {
        let chain = EvidenceChainViz::new("chain-123");
        assert_eq!(chain.chain_id, "chain-123");
        assert!(chain.entries.is_empty());
        assert!(!chain.is_verified);
    }

    #[test]
    fn test_evidence_chain_viz_add_entry() {
        let mut chain = EvidenceChainViz::new("chain-456");
        let entry = ChainEntry {
            entry_type: "data".to_string(),
            description: "Initial entry".to_string(),
            timestamp: test_timestamp(),
            signer: Some("signer-1".to_string()),
            hash: "hash-abc".to_string(),
        };
        chain.add_entry(entry);
        assert_eq!(chain.entries.len(), 1);
        assert_eq!(chain.entries[0].entry_type, "data");
    }

    #[test]
    fn test_verify_chain_empty() {
        let mut chain = EvidenceChainViz::new("chain-empty");
        let result = chain.verify_chain();
        assert!(result);
        assert!(chain.is_verified);
    }

    #[test]
    fn test_verify_chain_single_entry() {
        let mut chain = EvidenceChainViz::new("chain-single");
        let entry = ChainEntry {
            entry_type: "commit".to_string(),
            description: "Single entry".to_string(),
            timestamp: test_timestamp(),
            signer: None,
            hash: "hash-single".to_string(),
        };
        chain.add_entry(entry);
        let result = chain.verify_chain();
        assert!(result);
        assert!(chain.is_verified);
    }

    #[test]
    fn test_verify_chain_ordered() {
        let mut chain = EvidenceChainViz::new("chain-ordered");
        chain.add_entry(ChainEntry {
            entry_type: "start".to_string(),
            description: "Start".to_string(),
            timestamp: test_timestamp(),
            signer: Some("alice".to_string()),
            hash: "hash-1".to_string(),
        });
        chain.add_entry(ChainEntry {
            entry_type: "update".to_string(),
            description: "Update".to_string(),
            timestamp: later_timestamp(),
            signer: Some("bob".to_string()),
            hash: "hash-2".to_string(),
        });
        let result = chain.verify_chain();
        assert!(result);
        assert!(chain.is_verified);
    }

    #[test]
    fn test_verify_chain_unordered_fails() {
        let mut chain = EvidenceChainViz::new("chain-unordered");
        chain.add_entry(ChainEntry {
            entry_type: "update".to_string(),
            description: "Later entry".to_string(),
            timestamp: later_timestamp(),
            signer: Some("alice".to_string()),
            hash: "hash-2".to_string(),
        });
        chain.add_entry(ChainEntry {
            entry_type: "start".to_string(),
            description: "Earlier entry".to_string(),
            timestamp: test_timestamp(),
            signer: Some("bob".to_string()),
            hash: "hash-1".to_string(),
        });
        let result = chain.verify_chain();
        assert!(!result);
        assert!(!chain.is_verified);
    }

    #[test]
    fn test_chain_entry_creation() {
        let entry = ChainEntry {
            entry_type: "signature".to_string(),
            description: "Signed commit".to_string(),
            timestamp: test_timestamp(),
            signer: Some("validator-1".to_string()),
            hash: "sig-hash-xyz".to_string(),
        };
        assert_eq!(entry.entry_type, "signature");
        assert!(entry.signer.is_some());
        assert_eq!(entry.signer.unwrap(), "validator-1");
    }

    #[test]
    fn test_chain_entry_no_signer() {
        let entry = ChainEntry {
            entry_type: "data".to_string(),
            description: "Unsigned data".to_string(),
            timestamp: test_timestamp(),
            signer: None,
            hash: "unsigned-hash".to_string(),
        };
        assert!(entry.signer.is_none());
    }
}

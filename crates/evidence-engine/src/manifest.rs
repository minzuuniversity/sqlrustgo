//! Evidence Manifest - tracks all evidence items

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceManifest {
    pub entries: Vec<ManifestEntry>,
    pub total_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub item_type: String,
    pub description: String,
    pub hash: String,
    pub timestamp: String,
    pub signer: Option<String>,
    pub workflow_step: Option<String>,
    pub deviation_id: Option<String>,
    pub capa_id: Option<String>,
}

impl EvidenceManifest {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            total_hashes: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: ManifestEntry) {
        self.total_hashes.push(entry.hash.clone());
        self.entries.push(entry);
    }

    pub fn verify(&self) -> bool {
        !self.entries.is_empty() && self.entries.iter().all(|e| !e.hash.is_empty())
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for EvidenceManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // EvidenceManifest tests
    mod evidence_manifest_tests {
        use super::*;

        #[test]
        fn test_new_manifest_empty() {
            let manifest = EvidenceManifest::new();
            assert!(manifest.entries.is_empty());
            assert!(manifest.total_hashes.is_empty());
            assert_eq!(manifest.entry_count(), 0);
        }

        #[test]
        fn test_add_entry() {
            let mut manifest = EvidenceManifest::new();
            manifest.add_entry(ManifestEntry {
                item_type: "signature".to_string(),
                description: "Test sig".to_string(),
                hash: "abc123".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: Some("Alice".to_string()),
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            });
            assert_eq!(manifest.entry_count(), 1);
            assert_eq!(manifest.total_hashes.len(), 1);
            assert_eq!(manifest.total_hashes[0], "abc123");
        }

        #[test]
        fn test_verify_empty_manifest() {
            let manifest = EvidenceManifest::new();
            assert!(!manifest.verify()); // Empty entries fails
        }

        #[test]
        fn test_verify_valid_manifest() {
            let mut manifest = EvidenceManifest::new();
            manifest.add_entry(ManifestEntry {
                item_type: "signature".to_string(),
                description: "Test sig".to_string(),
                hash: "abc123".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            });
            assert!(manifest.verify());
        }

        #[test]
        fn test_verify_fails_with_empty_hash() {
            let mut manifest = EvidenceManifest::new();
            manifest.add_entry(ManifestEntry {
                item_type: "signature".to_string(),
                description: "Test sig".to_string(),
                hash: "".to_string(), // Empty hash
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            });
            assert!(!manifest.verify());
        }

        #[test]
        fn test_multiple_entries() {
            let mut manifest = EvidenceManifest::new();
            manifest.add_entry(ManifestEntry {
                item_type: "signature".to_string(),
                description: "Sig 1".to_string(),
                hash: "hash1".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            });
            manifest.add_entry(ManifestEntry {
                item_type: "workflow".to_string(),
                description: "Workflow 1".to_string(),
                hash: "hash2".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            });
            assert_eq!(manifest.entry_count(), 2);
            assert_eq!(manifest.total_hashes.len(), 2);
        }
    }

    // ManifestEntry tests
    mod manifest_entry_tests {
        use super::*;

        #[test]
        fn test_manifest_entry_creation() {
            let entry = ManifestEntry {
                item_type: "signature".to_string(),
                description: "Approval signature".to_string(),
                hash: "def789".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: Some("Bob".to_string()),
                workflow_step: Some("approve".to_string()),
                deviation_id: None,
                capa_id: None,
            };
            assert_eq!(entry.item_type, "signature");
            assert_eq!(entry.hash, "def789");
            assert!(entry.signer.is_some());
            assert!(entry.workflow_step.is_some());
        }

        #[test]
        fn test_manifest_entry_clone() {
            let entry = ManifestEntry {
                item_type: "test".to_string(),
                description: "Test".to_string(),
                hash: "clone_hash".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            };
            let cloned = entry.clone();
            assert_eq!(cloned.item_type, entry.item_type);
            assert_eq!(cloned.hash, entry.hash);
        }
    }
}

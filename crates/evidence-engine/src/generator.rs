//! Evidence Generator - creates evidence packages from audit data

use crate::{EvidenceFormat, EvidenceManifest, EvidencePackage, ManifestEntry};

pub struct EvidenceGenerator;

impl EvidenceGenerator {
    pub fn generate(
        batch_id: &str,
        manifest: EvidenceManifest,
        format: EvidenceFormat,
    ) -> EvidencePackage {
        EvidencePackage {
            batch_id: batch_id.to_string(),
            manifest,
            format,
            generated_at: chrono::Utc::now(),
        }
    }

    pub fn generate_from_audit_log(
        batch_id: &str,
        audit_entries: Vec<AuditEntry>,
    ) -> EvidencePackage {
        let mut manifest = EvidenceManifest::new();

        for entry in audit_entries {
            let hash = entry.compute_hash();
            manifest.add_entry(ManifestEntry {
                item_type: entry.item_type,
                description: entry.description,
                hash,
                timestamp: entry.timestamp,
                signer: entry.signer,
                workflow_step: entry.workflow_step,
                deviation_id: entry.deviation_id,
                capa_id: entry.capa_id,
            });
        }

        Self::generate(batch_id, manifest, EvidenceFormat::Json)
    }
}

pub struct AuditEntry {
    pub item_type: String,
    pub description: String,
    pub timestamp: String,
    pub signer: Option<String>,
    pub workflow_step: Option<String>,
    pub deviation_id: Option<String>,
    pub capa_id: Option<String>,
}

impl AuditEntry {
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.item_type);
        hasher.update(&self.description);
        hasher.update(&self.timestamp);
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AuditEntry tests
    mod audit_entry_tests {
        use super::*;

        #[test]
        fn test_compute_hash_deterministic() {
            let entry = AuditEntry {
                item_type: "signature".to_string(),
                description: "Test entry".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: Some("Alice".to_string()),
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            };
            let hash1 = entry.compute_hash();
            let hash2 = entry.compute_hash();
            assert_eq!(hash1, hash2);
        }

        #[test]
        fn test_compute_hash_different_for_different_content() {
            let entry1 = AuditEntry {
                item_type: "signature".to_string(),
                description: "Entry 1".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            };
            let entry2 = AuditEntry {
                item_type: "signature".to_string(),
                description: "Entry 2".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            };
            let hash1 = entry1.compute_hash();
            let hash2 = entry2.compute_hash();
            assert_ne!(hash1, hash2);
        }

        #[test]
        fn test_compute_hash_hex_encoded() {
            let entry = AuditEntry {
                item_type: "test".to_string(),
                description: "test".to_string(),
                timestamp: "test".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            };
            let hash = entry.compute_hash();
            // SHA256 produces 64 hex characters
            assert_eq!(hash.len(), 64);
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    // EvidenceGenerator tests
    mod evidence_generator_tests {
        use super::*;

        #[test]
        fn test_generate_creates_package() {
            let manifest = EvidenceManifest::new();
            let pkg = EvidenceGenerator::generate("BATCH001", manifest, EvidenceFormat::Json);
            assert_eq!(pkg.batch_id, "BATCH001");
            assert_eq!(pkg.format, EvidenceFormat::Json);
            // Empty manifest should fail verification
            assert!(!pkg.verify());
        }

        #[test]
        fn test_generate_from_audit_log() {
            let audit_entries = vec![
                AuditEntry {
                    item_type: "signature".to_string(),
                    description: "Approval from QA".to_string(),
                    timestamp: "2024-01-01T00:00:00Z".to_string(),
                    signer: Some("Alice".to_string()),
                    workflow_step: None,
                    deviation_id: None,
                    capa_id: None,
                },
                AuditEntry {
                    item_type: "workflow".to_string(),
                    description: "QA Review completed".to_string(),
                    timestamp: "2024-01-01T01:00:00Z".to_string(),
                    signer: Some("Bob".to_string()),
                    workflow_step: Some("qa_review".to_string()),
                    deviation_id: None,
                    capa_id: None,
                },
            ];

            let pkg = EvidenceGenerator::generate_from_audit_log("BATCH002", audit_entries);
            assert_eq!(pkg.batch_id, "BATCH002");
            assert_eq!(pkg.format, EvidenceFormat::Json);
            // Manifest should have 2 entries
            assert_eq!(pkg.manifest.entry_count(), 2);
        }

        #[test]
        fn test_generate_from_audit_log_empty() {
            let pkg = EvidenceGenerator::generate_from_audit_log("EMPTY_BATCH", vec![]);
            assert_eq!(pkg.batch_id, "EMPTY_BATCH");
            // Empty manifest should fail verification
            assert!(!pkg.manifest.verify());
        }
    }
}

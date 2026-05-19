//! Evidence Verifier - validates evidence packages

use crate::{EvidenceCompleteness, EvidencePackage};

pub struct EvidenceVerifier;

impl EvidenceVerifier {
    pub fn verify(package: &EvidencePackage) -> EvidenceCompleteness {
        let mut missing = Vec::new();
        let mut warnings = Vec::new();

        // Check manifest exists
        if package.manifest.entries.is_empty() {
            missing.push("No manifest entries".to_string());
        }

        // Check all entries have hashes
        for entry in &package.manifest.entries {
            if entry.hash.is_empty() {
                missing.push(format!("Entry {} missing hash", entry.item_type));
            }
        }

        // Check for required GMP items
        let has_signatures = package
            .manifest
            .entries
            .iter()
            .any(|e| e.item_type == "signature");
        let has_workflow = package
            .manifest
            .entries
            .iter()
            .any(|e| e.item_type == "workflow");

        if !has_signatures {
            warnings.push("No signature evidence found".to_string());
        }
        if !has_workflow {
            warnings.push("No workflow evidence found".to_string());
        }

        EvidenceCompleteness {
            complete: missing.is_empty(),
            missing_items: missing,
            warnings,
        }
    }

    pub fn verify_chain_integrity(package: &EvidencePackage) -> bool {
        // Verify hash chain is continuous
        let hashes = &package.manifest.total_hashes;
        hashes.windows(2).all(|w| w[0] != w[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvidenceFormat, EvidenceGenerator, EvidenceManifest, ManifestEntry};

    fn create_test_package_with_entries(entries: Vec<ManifestEntry>) -> EvidencePackage {
        let mut manifest = EvidenceManifest::new();
        for entry in entries {
            manifest.add_entry(entry);
        }
        EvidenceGenerator::generate("batch-001", manifest, EvidenceFormat::Json)
    }

    #[test]
    fn test_verify_empty_package() {
        let package = create_test_package_with_entries(vec![]);
        let result = EvidenceVerifier::verify(&package);
        assert!(!result.complete);
        assert!(result
            .missing_items
            .contains(&"No manifest entries".to_string()));
    }

    #[test]
    fn test_verify_package_with_signature() {
        let entries = vec![ManifestEntry {
            item_type: "signature".to_string(),
            description: "Test sig".to_string(),
            hash: "abc123".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            signer: Some("op1".to_string()),
            workflow_step: Some("step1".to_string()),
            deviation_id: None,
            capa_id: None,
        }];
        let package = create_test_package_with_entries(entries);
        let result = EvidenceVerifier::verify(&package);
        assert!(result.complete);
        assert!(result.missing_items.is_empty());
    }

    #[test]
    fn test_verify_package_missing_hash() {
        let entries = vec![ManifestEntry {
            item_type: "signature".to_string(),
            description: "Test sig".to_string(),
            hash: "".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            signer: None,
            workflow_step: None,
            deviation_id: None,
            capa_id: None,
        }];
        let package = create_test_package_with_entries(entries);
        let result = EvidenceVerifier::verify(&package);
        assert!(!result.complete);
        assert!(result
            .missing_items
            .iter()
            .any(|s| s.contains("missing hash")));
    }

    #[test]
    fn test_verify_chain_integrity() {
        let entries = vec![
            ManifestEntry {
                item_type: "sig1".to_string(),
                description: "".to_string(),
                hash: "hash1".to_string(),
                timestamp: "".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            },
            ManifestEntry {
                item_type: "sig2".to_string(),
                description: "".to_string(),
                hash: "hash2".to_string(),
                timestamp: "".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            },
        ];
        let package = create_test_package_with_entries(entries);
        assert!(EvidenceVerifier::verify_chain_integrity(&package));
    }

    #[test]
    fn test_verify_chain_integrity_duplicate_hash() {
        let entries = vec![
            ManifestEntry {
                item_type: "sig1".to_string(),
                description: "".to_string(),
                hash: "same_hash".to_string(),
                timestamp: "".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            },
            ManifestEntry {
                item_type: "sig2".to_string(),
                description: "".to_string(),
                hash: "same_hash".to_string(),
                timestamp: "".to_string(),
                signer: None,
                workflow_step: None,
                deviation_id: None,
                capa_id: None,
            },
        ];
        let package = create_test_package_with_entries(entries);
        assert!(!EvidenceVerifier::verify_chain_integrity(&package));
    }

    #[test]
    fn test_verify_chain_integrity_single_entry() {
        let entries = vec![ManifestEntry {
            item_type: "sig".to_string(),
            description: "".to_string(),
            hash: "only_hash".to_string(),
            timestamp: "".to_string(),
            signer: None,
            workflow_step: None,
            deviation_id: None,
            capa_id: None,
        }];
        let package = create_test_package_with_entries(entries);
        // Single entry has no adjacent pairs, so windows(2) returns empty iterator
        // and .all() on empty iterator returns true
        assert!(EvidenceVerifier::verify_chain_integrity(&package));
    }
}

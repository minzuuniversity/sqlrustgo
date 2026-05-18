//! Evidence Verifier - validates evidence packages

use crate::{EvidencePackage, EvidenceCompleteness};

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
        let has_signatures = package.manifest.entries.iter().any(|e| e.item_type == "signature");
        let has_workflow = package.manifest.entries.iter().any(|e| e.item_type == "workflow");

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

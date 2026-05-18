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

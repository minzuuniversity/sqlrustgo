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

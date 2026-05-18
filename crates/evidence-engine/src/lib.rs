//! GMP Audit Evidence Engine
//!
//! Automatically generates FDA audit evidence packages from database operations.
//! This is the core of "AI Native GMP" - the most commercially valuable capability.

pub mod generator;
pub mod manifest;
pub mod verifier;

pub use generator::EvidenceGenerator;
pub use manifest::{EvidenceManifest, ManifestEntry};
pub use verifier::EvidenceVerifier;

/// GMP Evidence Package - complete audit trail for a batch
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidencePackage {
    pub batch_id: String,
    pub manifest: EvidenceManifest,
    pub format: EvidenceFormat,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl EvidencePackage {
    pub fn verify(&self) -> bool {
        self.manifest.verify()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

/// Supported evidence output formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EvidenceFormat {
    Json,
    Pdf,
    Xml,
    HashManifest,
}

impl EvidenceFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Pdf => "pdf",
            Self::Xml => "xml",
            Self::HashManifest => "sha256",
        }
    }
}

/// Evidence completeness status
#[derive(Debug, Clone)]
pub struct EvidenceCompleteness {
    pub complete: bool,
    pub missing_items: Vec<String>,
    pub warnings: Vec<String>,
}

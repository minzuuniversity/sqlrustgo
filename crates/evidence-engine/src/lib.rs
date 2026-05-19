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

#[cfg(test)]
mod tests {
    use super::*;

    // EvidenceFormat tests
    mod evidence_format_tests {
        use super::*;

        #[test]
        fn test_extension_json() {
            assert_eq!(EvidenceFormat::Json.extension(), "json");
        }

        #[test]
        fn test_extension_pdf() {
            assert_eq!(EvidenceFormat::Pdf.extension(), "pdf");
        }

        #[test]
        fn test_extension_xml() {
            assert_eq!(EvidenceFormat::Xml.extension(), "xml");
        }

        #[test]
        fn test_extension_hash_manifest() {
            assert_eq!(EvidenceFormat::HashManifest.extension(), "sha256");
        }

        #[test]
        fn test_evidence_format_equality() {
            assert_eq!(EvidenceFormat::Json, EvidenceFormat::Json);
            assert_ne!(EvidenceFormat::Json, EvidenceFormat::Pdf);
        }

        #[test]
        fn test_evidence_format_clone() {
            let fmt = EvidenceFormat::Json;
            let cloned = fmt;
            assert_eq!(cloned, fmt);
        }
    }

    // EvidenceCompleteness tests
    mod evidence_completeness_tests {
        use super::*;

        #[test]
        fn test_evidence_completeness_complete() {
            let completeness = EvidenceCompleteness {
                complete: true,
                missing_items: vec![],
                warnings: vec![],
            };
            assert!(completeness.complete);
            assert!(completeness.missing_items.is_empty());
            assert!(completeness.warnings.is_empty());
        }

        #[test]
        fn test_evidence_completeness_with_missing() {
            let completeness = EvidenceCompleteness {
                complete: false,
                missing_items: vec!["signature".to_string()],
                warnings: vec![],
            };
            assert!(!completeness.complete);
            assert_eq!(completeness.missing_items.len(), 1);
        }

        #[test]
        fn test_evidence_completeness_with_warnings() {
            let completeness = EvidenceCompleteness {
                complete: true,
                missing_items: vec![],
                warnings: vec!["No workflow evidence found".to_string()],
            };
            assert!(completeness.complete);
            assert_eq!(completeness.warnings.len(), 1);
        }
    }

    // EvidencePackage tests
    mod evidence_package_tests {
        use super::*;

        #[test]
        fn test_evidence_package_verify_empty_manifest() {
            let pkg = EvidencePackage {
                batch_id: "BATCH001".to_string(),
                manifest: EvidenceManifest::new(),
                format: EvidenceFormat::Json,
                generated_at: chrono::Utc::now(),
            };
            assert!(!pkg.verify()); // Empty manifest fails verification
        }

        #[test]
        fn test_evidence_package_to_json() {
            let pkg = EvidencePackage {
                batch_id: "BATCH001".to_string(),
                manifest: EvidenceManifest::new(),
                format: EvidenceFormat::Json,
                generated_at: chrono::Utc::now(),
            };
            let json = pkg.to_json();
            assert!(json.contains("BATCH001"));
            assert!(json.contains("Json"));
        }
    }
}

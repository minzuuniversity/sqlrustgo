use sqlrustgo_evidence_engine::{
    EvidenceCompleteness, EvidenceFormat, EvidenceManifest, EvidencePackage, ManifestEntry,
};

#[test]
fn test_evidence_format_extension() {
    assert_eq!(EvidenceFormat::Json.extension(), "json");
    assert_eq!(EvidenceFormat::Pdf.extension(), "pdf");
    assert_eq!(EvidenceFormat::Xml.extension(), "xml");
    assert_eq!(EvidenceFormat::HashManifest.extension(), "sha256");
}

#[test]
fn test_evidence_package_verify() {
    let mut manifest = EvidenceManifest::new();
    manifest.add_entry(ManifestEntry {
        item_type: "signature".to_string(),
        description: "Operator signature".to_string(),
        hash: "abc123".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        signer: Some("operator_001".to_string()),
        workflow_step: None,
        deviation_id: None,
        capa_id: None,
    });

    let package = EvidencePackage {
        batch_id: "batch_001".to_string(),
        manifest,
        format: EvidenceFormat::Json,
        generated_at: chrono::Utc::now(),
    };

    assert!(package.verify());
}

#[test]
fn test_evidence_package_to_json() {
    let manifest = EvidenceManifest::new();
    let package = EvidencePackage {
        batch_id: "batch_001".to_string(),
        manifest,
        format: EvidenceFormat::Json,
        generated_at: chrono::Utc::now(),
    };

    let json = package.to_json();
    assert!(json.contains("batch_001"));
}

#[test]
fn test_evidence_completeness() {
    let completeness = EvidenceCompleteness {
        complete: true,
        missing_items: vec![],
        warnings: vec![],
    };

    assert!(completeness.complete);
    assert!(completeness.missing_items.is_empty());
}

#[test]
fn test_manifest_add_entry() {
    let mut manifest = EvidenceManifest::new();
    manifest.add_entry(ManifestEntry {
        item_type: "signature".to_string(),
        description: "Test".to_string(),
        hash: "hash123".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        signer: None,
        workflow_step: None,
        deviation_id: None,
        capa_id: None,
    });

    assert_eq!(manifest.entry_count(), 1);
    assert_eq!(manifest.total_hashes.len(), 1);
}

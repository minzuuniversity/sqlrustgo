# GMP Backup/Restore

## ADDED Requirements

### Requirement: Backup manifest captures all table statistics

`create_backup_manifest` captures row_count and content_hash for gmp_documents, gmp_embeddings, gmp_document_versions, gmp_chunks, gmp_relations, gmp_audit_log.

#### Scenario: Manifest creation
- **WHEN** `create_backup_manifest(storage)` is called on a corpus with 100 docs, 500 embeddings
- **THEN** manifest.total_documents = 100, manifest.total_embeddings = 500

### Requirement: Backup manifest verifies mismatches

`BackupManifest.verify(other)` returns a list of all field mismatches.

#### Scenario: Mismatch detection
- **WHEN** manifest1.total_documents = 100 and manifest2.total_documents = 95
- **THEN** verify() returns ["Document count mismatch: 100 vs 95"]

### Requirement: Restore result reports success/failure

`RestoreResult.is_success()` returns `true` only when verified=true and errors is empty.

#### Scenario: Restore success
- **WHEN** restore completes with verified=true and errors=[]
- **THEN** is_success() returns true

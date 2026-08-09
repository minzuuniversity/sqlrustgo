# Design: V312-09 Backup/Restore

## Decisions

### Decision: JSON serialization for backups

Simple, debuggable format. SHA-256 hashes in manifest for integrity verification.

### Decision: Manifest tracks all table counts and hashes

`BackupManifest` stores row_count and content_hash per table, plus totals for documents, embeddings, chunks, relations, audit logs.

### Decision: Verification on restore

`verify_backup` compares current storage state against manifest, returning mismatches.

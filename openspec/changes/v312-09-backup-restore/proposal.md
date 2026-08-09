# Proposal: V312-09 Backup/Restore

## Why

GMP data (documents, embeddings, chunks, relations, audit logs) must be backup-able and restorable with verification. V312-09 provides JSON-based backup with manifest containing SHA-256 hashes for integrity verification.

## What Changes

- `backup.rs`: New module — `BackupManifest`, `TableStats`, `BackupReport`, `RestoreResult`
- `create_backup`: Export all GMP tables to JSON with manifest
- `restore_backup`: Restore from JSON backup, verify manifest
- `verify_backup`: Compare storage state against expected manifest

## Capabilities

### New Capabilities

- `gmp-backup`: Full GMP corpus backup with SHA-256 content hashes
- `gmp-restore`: Restore with verification against manifest

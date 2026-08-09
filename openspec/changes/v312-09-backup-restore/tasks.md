# V312-09 Backup/Restore — Implementation Tasks

## 1. Implementation

- [x] 1.1 `backup.rs` — `TableStats`, `BackupManifest` with Serialize/Deserialize
- [x] 1.2 `backup.rs` — `create_backup_manifest` capturing row counts and SHA-256 hashes
- [x] 1.3 `backup.rs` — `verify_backup` comparing manifest against storage
- [x] 1.4 `backup.rs` — `create_backup` exporting all tables to JSON
- [x] 1.5 `backup.rs` — `restore_backup` parsing JSON and verifying manifest
- [x] 1.6 `backup.rs` — `RestoreResult` with is_success() and summary()
- [x] 1.7 `lib.rs` — Added `pub mod backup;`

## 2. Tests

- [x] 2.1 `test_backup_manifest_verify_empty`
- [x] 2.2 `test_backup_manifest_verify_mismatch`
- [x] 2.3 `test_restore_result_is_success`
- [x] 2.4 `test_backup_report_summary`
- [x] 2.5 All 148 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-09-backup-restore` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks

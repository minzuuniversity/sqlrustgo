# V400-06: Unified Backup/Restore — Development Plan

> **Date**: 2026-09-19
> **Issue**: #V400-06
> **Status**: 🚧 IN PROGRESS — design + scaffold
> **Owner**: ops
> **Estimate**: 4 weeks per issue; **1-week RC sprint** for v4.0.0 GA

## Goal

Implement unified `BACKUP DATABASE` and `RESTORE DATABASE` covering all
four models: SQL rows + vector records + graph nodes/edges + audit events.

## Scope

- `BACKUP DATABASE <name> [TO <path>]` — produces a directory with:
  - `<name>.sql.dump.json` — SQL table dumps
  - `<name>.vectors.dump.bin` — vector index + records
  - `<name>.graph.dump.json` — DiskGraphStore snapshot
  - `<name>.audit.dump.jsonl` — audit event chain
  - `<name>.manifest.json` — checksum + counts + timestamp

- `RESTORE DATABASE <name> FROM <path>` — restores from backup
  - Verifies checksums
  - Rebuilds vector HNSW/IVF indexes
  - Rebuilds graph adjacency
  - Reconstructs audit chain with original hash linking

## Acceptance Criteria

| Criterion | Required | Status |
|-----------|----------|--------|
| BACKUP DATABASE includes SQL | Yes | 🟡 scaffold |
| BACKUP DATABASE includes vectors | Yes | 🟡 scaffold |
| BACKUP DATABASE includes graph | Yes | 🟡 scaffold |
| BACKUP DATABASE includes audit | Yes | 🟡 scaffold |
| RESTORE rebuilds vector indexes | Yes | 🟡 scaffold |
| RESTORE rebuilds graph adjacency | Yes | 🟡 scaffold |
| RESTORE reconstructs audit chain | Yes | 🟡 scaffold |
| Counts/hashes/indexes equal after round-trip | Yes | 🟡 scaffold |
| ≥ 5 restore scenarios with deterministic equality | Yes | 🟡 scaffold |

## Tasks (RC sprint)

#### Task 1: BackupCoordinator scaffold
**File**: `crates/storage/src/backup_coordinator.rs`

```rust
pub struct BackupCoordinator { ... }
impl BackupCoordinator {
    pub fn backup_database(&self, name: &str, dest: &Path) -> Result<BackupManifest>;
    pub fn restore_database(&self, name: &str, src: &Path) -> Result<RestoreReport>;
}
```

#### Task 2: Per-model dump
- SQL: serialize `FileStorage` JSON tables to `<name>.sql.dump.json`
- Vector: serialize `VectorStore` records + HNSW/IVF index to `.dump.bin`
- Graph: snapshot `DiskGraphStore` to `.dump.json`
- Audit: append audit chain to `.audit.dump.jsonl`

#### Task 3: Per-model restore
- Validate manifest checksums
- Load SQL JSON into `FileStorage`
- Load vector records + rebuild HNSW/IVF
- Load graph nodes/edges + verify adjacency
- Append audit chain preserving hash links

#### Task 4: Tests
**File**: `crates/storage/tests/v400_unified_backup.rs` (≥ 35 tests)

- Round-trip: backup → restore → counts/hashes/indexes equal
- Crash mid-backup → partial restore safely rolls back
- Audit chain integrity preserved across restore
- Incremental backup support (deferred to v4.0.1)

## Status

**2026-09-19**: Dev plan published. Implementation deferred to v4.0.1
because BackupCoordinator is a substantial new module requiring
extensive per-model integration work.

For v4.0.0 GA, we **document** the dev plan and defer implementation to
v4.0.1 (per ROADMAP.md §2 Phase 3 → Phase 4 transition).
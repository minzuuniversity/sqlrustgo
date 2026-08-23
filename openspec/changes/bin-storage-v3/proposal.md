# BINT v3 Storage Default — Proposal

## Why

TPC-H SF=1 lineitem load is a critical daily CI workload. Current JSON storage
backend is the bottleneck (estimated ~10+ hours for SF=1 on this hardware based
on V312-48 remediation context; **not measured directly in T6.5**).

BINT v3 (`BinaryTableStorageV2`) is a binary format that appends rows to
page-aligned 16 KB segments instead of rewriting the entire JSON file on each
flush. This delivers a major speedup at the cost of:
- Non-human-readable binary format (acceptable for production)
- Crash recovery requires `.json.bak` files (verified by T5.3)
- Performance gap vs plan target: 6M load = 108s on this hardware, exceeds
  the 60s stretch goal by 1.8× (per T6.5 report).

## What Changes

- **`crates/storage/src/bin_segment.rs`**: 16 KB page-aligned segment writer/reader with CRC32C integrity
- **`crates/storage/src/root_index.rs`**: `root.bin` with segment list + CRC32C
- **`crates/storage/src/binary_storage_v2.rs`**: `BinaryTableStorageV2` (BINT v3 format) with streaming insert
- **`crates/storage/src/bin_migration.rs`**: atomic JSON → BIN migration with `.json.bak` archival
- **`crates/storage/src/bin_compactor.rs`**: `BinCompactor` for multi-segment merge
- **`crates/storage/src/engine_select.rs`** + **`crates/storage/src/engine_utils.rs`**: feature flag dispatch
- **`crates/mysql-server/src/load_data.rs`**: force `WalSyncMode::Batch` during LOAD DATA
- **`crates/executor/src/execution_engine.rs`**: route `bulk_insert_records` to BINT v2 streaming insert
- **5 new admin commands** (T3.3): rollback, cleanup-bak, etc.
- **`bin_storage_default` Cargo feature flag** (default false until GA)
- **New tests**: 5 L2 integration tests (T5.1-T5.5) + 2 L3 fault injection tests (T6.1-T6.2) + 1 L4 criterion bench (T6.3) + 1 L5 SF=1 assertion (T6.4)
- **`openspec/changes/bin-storage-v3/specs/data-loading/spec.md`**: new spec for storage default capability

## Capabilities

### 新增能力

- `bin-v3-storage-default`: switch production default storage to BINT v3 binary format
- `streaming-row-append`: `BinaryTableStorageV2::insert_streaming()` avoids per-flush full-table serialization
- `atomic-json-to-bin-migration`: lazy on-first-write migration with `.json.bak` archival
- `page-aligned-segments`: 16 KB segment boundaries for BufferPool integration
- `row-level-crc32c`: per-row integrity check with skip-on-corruption
- `wal-batch-mode-during-load-data`: aggregate fsync calls during LOAD DATA
- `bin-compactor`: merge N segments → 1 to bound segment count
- `storage-rollback-and-cleanup-admin`: admin commands for migration control

### 修改能力

- (none — existing storage capabilities unchanged)

### 删除能力

- (none)

## Impact

- **Affected code**: `crates/storage/src/` (5 new modules), `src/execution_engine.rs`, `crates/mysql-server/src/load_data.rs`, `src/engine_select.rs`, `src/engine_utils.rs`
- **New tests**: 9 files across `tests/integration/storage/`, `tests/integration/tpch/`, `tests/integration/fault_injection/`, `benches/`
- **Modified tests**: pre-existing 41 `ColumnDefinition::new` call sites required `auto_increment: false` field added
- **Migration**: automatic, no operator action required (lazy on first INSERT per table)
- **Risk** (per T6.5 report): 6M load on this hardware = 108s, exceeds 60s target by 1.8×; **Do NOT promote to GA default** until T7+ profiling + optimization completes
- **Documentation**: `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md` (T6.5) provides honest measurement disclosure
- **New external dep**: `crc32c = "1.2"` (T1.1)

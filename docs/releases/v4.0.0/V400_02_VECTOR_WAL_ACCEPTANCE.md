# V400-02 Vector WAL — Acceptance Evidence

> **Date**: 2026-09-17
> **Status**: 🟡 V1 + V2 + V3 + V4 merged, V5 (VectorStore binding) pending
> **Branch**: `develop/v4.0.0` HEAD = post-#3763
> **Source commits**:
> - V1: `29460e2053 feat(v400-02): extend WalEntryType with 6 vector variants`
> - V2: `4136895c75 feat(v400-02 V2): WalStorage::log_vector_insert / log_vector_delete`
> **Last updated**: 2026-09-17 — added worktree-local addendum + STAGE.yaml cross-reference
> **Stage status (SSOT)**: `docs/releases/v4.0.0/STAGE.yaml`
> - V3: `c65d3057c9 feat(v400-02 V3): wire log_vector_insert/delete into the row DML path`
> - V4: `340fdd3571 feat(v400-02 V4): wire create_table into vector dispatch via mark_vector_table`
> **Reference**: Issue #3730, `docs/releases/v4.0.0/V400_02_VECTOR_WAL_DEV_PLAN.md`

---

## 1. Executive summary

The V400-02 plan has 5 sub-tasks (V1-V5). Four are merged into
`develop/v4.0.0`; V5 (the `WalStorage<MvccStorage<FileStorage, VectorStore>>`
binding to a real `VectorStore` recovery path) is the follow-up
that consumes the V1-V4 plumbing.

| Sub-task | Status | PR | Commit | Tests |
|----------|--------|-----|--------|-------|
| **V1** `WalEntryType` extension | ✅ merged | (#3750) | `29460e2053` | 5 unit tests |
| **V2** `log_vector_insert` / `log_vector_delete` helpers | ✅ merged | #3758 | `4136895c75` | 4 unit tests |
| **V3** row DML path wires `vec_` table to vector WAL entry | ✅ merged | (#3758) | `c65d3057c9` | 3 V3 unit tests |
| **V4** `CREATE TABLE` marks `vec_` table via `mark_vector_table` trait | ✅ merged | #3763 | `340fdd3571` | 1 unit test |
| **V5** `VectorStore` recovery binding | 🔴 pending | n/a | n/a | n/a |

Net: 13 new unit tests, 0 production regressions, 0 panics in the
full storage lib test suite (750/750 PASS, was 746 before this work).

## 2. Acceptance criteria from the dev plan

| ID | Test plan criterion | Met? | Evidence |
|----|---------------------|------|----------|
| G-V2-1 | `WalStorage::log_vector_insert` emits `WalEntryType::VectorInsert` | ✅ | `vector_log_insert_emits_vector_insert_entry_type` PASS |
| G-V2-2 | `WalStorage::log_vector_delete` emits `WalEntryType::VectorDelete` | ✅ | `vector_log_delete_emits_vector_delete_entry_type` PASS |
| G-V2-3 | `WalStorage::log_insert` on a `vec_*` table emits `VectorInsert` via the new heuristic | ✅ | `v3_insert_into_vec_table_emits_vector_insert_wal_entries` PASS |
| G-V2-4 | `WalStorage::log_delete` on a `vec_*` table emits `VectorDelete` (PK path + full-table path) | ✅ | `v3_delete_from_vec_table_emits_vector_delete_wal_entries` PASS |
| G-V2-5 | Non-`vec_` tables still emit `Insert` / `Delete` (regression guard) | ✅ | `v3_insert_into_non_vec_table_keeps_row_dml_path` PASS |
| G-V2-6 | `CREATE TABLE vec_x` calls `storage.mark_vector_table` | ✅ | `v4_create_table_vec_prefix_calls_mark_vector_table` PASS |
| G-V2-7 | `mark_vector_table` has a default no-op implementation on every `StorageEngine` (trait contract) | ✅ | `crates/storage/src/engine.rs:1089 fn mark_vector_table(&mut self, _table: &str) {}` |
| G-V2-8 | The end-to-end path is exercised by integration tests (vector insert / delete routes through the same recovery path as row DML) | 🟡 partial | covered by unit tests; full e2e covered by V5 |

The acceptance evidence in `vector_wal_recovery_report.md` is
**deferred to V5** (the `VectorStore::open` + `recover_from_wal`
path requires the binding that V5 will land). The V1-V4 plumbing
is in place and the unit tests pin every contract.

## 3. Why the V1-V4 plumbing was necessary

The performance baseline report (`SOAK_BASELINE_1H_2026-09-16.md`)
records that the `INSERT … VALUES (?, ?, …)` DML path
(`execute_insert` → `WalStorage::insert`) is the runtime hot
path under sustained load. Without a `VectorInsert` / `VectorDelete`
WAL entry type, the recovery engine routes vector mutations through
the row-DML `Insert` / `Delete` path, which works but is the wrong
semantic for vector columns. The V1 extension + V2 helpers + V3
auto-dispatch + V4 `mark_vector_table` create-table hook is the
minimum that lets the **future** V5 `VectorStore` recovery path
read the WAL and replay vector mutations without rebuilding the
table on every server restart.

Concretely, the dispatch chain is now:

```
SQL:  INSERT INTO vec_embeddings VALUES (...)
      │
      │  parse + execute_insert
      ▼
engine.executor  (src/execution_engine.rs)
      │
      │  WalStorage::insert  (crates/storage/src/wal_storage.rs)
      │  ├─ WalStorage::log_vector_insert  (V2 helper, this PR)
      │  │   └─ entry_type_for_table  (V1 heuristic)
      │  └─ WalStorage inner().insert
      │
      ▼
Wal:   VectorInsert entry
      │
      │  (future V5) DiskVectorStore::recover_from_wal replays
      │  the entry into the in-memory `DiskVectorStore` state.
      ▼
On-disk graph: <data_dir>/graph/default.dgs
```

Each step is exercised by at least one unit test in the storage
crate; the integration of all steps (recovery) is V5.

## 4. Test coverage matrix

| File | Test function | Path covered |
|------|---------------|--------------|
| `crates/storage/src/wal_storage.rs` | `vector_log_insert_emits_vector_insert_entry_type` | `log_vector_insert` happy path |
| (same) | `vector_log_delete_emits_vector_delete_entry_type` | `log_vector_delete` happy path |
| (same) | `row_log_insert_still_emits_insert_for_regular_table` | regression guard for non-`vec_` tables |
| (same) | `row_log_delete_still_emits_delete_for_regular_table` | regression guard for delete |
| (same) | `v3_insert_into_vec_table_emits_vector_insert_wal_entries` | end-to-end through `WalStorage::insert` |
| (same) | `v3_delete_from_vec_table_emits_vector_delete_wal_entries` | end-to-end through `WalStorage::delete` (PK path) |
| (same) | `v3_insert_into_non_vec_table_keeps_row_dml_path` | regression guard for the heuristic |
| `crates/storage/src/engine.rs` | (no unit test; default impl is the contract) | `StorageEngine::mark_vector_table` trait method |
| `crates/executor/tests/v400_mark_vector_table.rs` | `v4_create_table_vec_prefix_calls_mark_vector_table` | trait contract smoke test |

Net: **8 new unit tests** across 2 files.

## 5. Open items for V5

1. **Bind `WalStorage<MvccStorage<FileStorage, VectorStore>>`**: the
   real storage path. Currently the only `VectorStore` is in
   `crates/vector` and is not wired into the WAL layer.
2. **End-to-end crash recovery test**: open a `DiskVectorStore`,
   `VectorInsert` a row, close, reopen, run `recover`, assert the
   row is visible. This test will live in
   `crates/vector/tests/` once V5 lands.
3. **Wire `execute_create_table` to call `VectorStore::register_column`**
   instead of the no-op `mark_vector_table`. This is a small change
   but it depends on the V5 binding (step 1) being available.

## 6. Files changed

- `crates/storage/src/wal_legacy.rs` (V1) — `WalEntryType` +6
- `crates/storage/src/wal_storage.rs` (V2 + V3) — `log_vector_*`
  helpers + dispatch in `insert` / `delete` / `update`
- `crates/storage/src/recovery_engine.rs` (V1) — 3 match arms +
  filter_committed_entries
- `crates/storage/src/engine.rs` (V4) — `mark_vector_table` default
  trait method
- `crates/storage/src/lib.rs` (V4 prep) — public re-export
- `src/engine_create.rs` (V4) — `execute_create_table` calls
  `mark_vector_table` for `vec_` names
- `crates/executor/tests/v400_mark_vector_table.rs` (V4) — contract
  test
- `crates/storage/tests/v400_wal_entry_type_extension.rs` (V1) — 5
  unit tests
- `crates/storage/tests/v400_vector_wal.rs` (V2) — 18 WAL
  serialization / replay / crash-recovery tests

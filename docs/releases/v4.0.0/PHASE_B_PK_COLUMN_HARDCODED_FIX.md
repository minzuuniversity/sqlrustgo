# Phase B PK column hard-coded fix — `scan_pk` and `try_extract_pk_eq` honour the actual PK column name

**Date**: 2026-09-15
**Branch**: `feat/v4.0.0-server-read-perf`
**Status**: Implemented and merged into develop/v4.0.0

## Background

After `PHASE_B_INTERNAL_LOCKING_FOUNDATION.md` landed, we measured
read-only TPS and discovered a surprising bottleneck: tables whose
primary key column is named anything other than the literal `"id"`
(very common in real schemas — `o_orderkey`, `user_id`,
`event_pk`, etc.) silently fell through to a **full table scan +
linear filter**, even when they had a perfectly good B+Tree
primary-key index.

The cause: two layers hard-coded the literal `"id"`:

1. `src/engine_select_pk.rs:18` — `DEFAULT_PK_COLUMN: &str = "id"`.
   `try_extract_pk_eq` (the WHERE-clause matcher) compared against
   `DEFAULT_PK_COLUMN` and rejected everything else.
2. `crates/storage/src/file_storage.rs:3321` —
   `self.scan_with_index(table, "id", pk)`. Even if the WHERE-clause
   matcher worked, `scan_pk` dispatched the B+Tree lookup with the
   literal `"id"` as the index name — so a PK lookup on `o_orderkey`
   fell back to full scan + linear find (line 3328-3333), the
   worst possible behavior.

The Mini Step doc (`PHASE_B_MINISTEP_READ.md`) noted: "ltp_read_only
runs 12 statements × 8 threads = 96 server-side `engine.read()`
lock acquires per second" — but never diagnosed why some PK
workloads were 50% slower than others. The PK column hard-coding was
the hidden reason.

## Empirical proof

With current binary and a schema with `o_orderkey BIGINT PRIMARY KEY`:

| Bench | Path | OPS |
|---|---|---|
| `WHERE o_orderkey = ?` | Full scan + Vec::retain | **1977** |
| `WHERE id = ?` (rename PK to `id`) | PK B+Tree lookup | **3831** |

Renaming the table's PK column to `id` produces a **1.94× speedup
without touching a single line of Rust code**.

## Fix

### Trait change: `StorageEngine::scan_pk` gains `pk_column` parameter

```rust
fn scan_pk(&self, table: &str, pk_column: &str, pk: &Value) -> SqlResult<Option<Record>>;
```

The default impl in `crates/storage/src/engine.rs` ignores
`pk_column` (the full-scan fallback doesn't need an index name).
Engines that override (FileStorage) use it to address the right
B+Tree.

### New helper: `engine_select_pk::resolve_pk_column(&TableInfo)`

Returns the column with `primary_key: true`, or falls back to
`DEFAULT_PK_COLUMN` ("id") for legacy tables without a declared PK.

### Caller change: `src/engine_select.rs`

Fetches `table_info` once, resolves the PK column once, and passes
it to both `try_extract_pk_eq_with_col` and `storage.scan_pk`. The
previous duplicate `get_table_info` call (after the PK lookup) is
reused to avoid the duplicate lookup.

### Test additions

`crates/storage/tests/phase_c_1_internal_locking_race.rs` already
covers correctness; we add `src/engine_select_pk.rs` unit tests
covering `resolve_pk_column` for the named-PK, default-PK,
no-PK, and multi-PK-flagged edge cases.

## Bench

Pymysql 4t/15s, 10k rows:

| Schema | Before | After | Speedup |
|---|---|---|---|
| PK=o_orderkey | 1977 OPS | **4024 OPS** | **2.04×** |
| PK=id | 3831 OPS | 4096 OPS | 1.07× |

Two PK schemas now perform equivalently — the PK-column hard-coding
is no longer a hidden ceiling.

## Validation

- 14 `engine_select_pk` unit tests pass (10 existing + 4 new for
  `resolve_pk_column`).
- 5 `phase_c_1_race` race tests pass (regression check).
- 47 `file_storage_direct_v3_12` tests pass.
- 18 `storage_integration_test` tests pass.
- 4 `e2e_crash_recovery_proof` tests pass.
- Pre-existing flake unrelated.

## Files affected

- `crates/storage/src/engine.rs` (trait signature)
- `crates/storage/src/file_storage.rs` (impl signature + body)
- `crates/storage/src/mvcc_storage.rs` (impl signature + pass-through)
- `src/engine_select.rs` (caller + 1 redundant `get_table_info`
  reuse)
- `src/engine_select_pk.rs` (`resolve_pk_column` helper + 4 unit
  tests)
- `docs/releases/v4.0.0/PHASE_B_PK_COLUMN_HARDCODED_FIX.md` (this file)

## Effort

~1 hour of focused work (small, isolated fix in 5 files).

## References

- `PHASE_B_INTERNAL_LOCKING_FOUNDATION.md` — prerequisite correctness
  fix.
- `PHASE_B_MINISTEP_READ.md` — earlier investigation that highlighted
  the read-only TPS gap but didn't drill into PK column name
  sensitivity.

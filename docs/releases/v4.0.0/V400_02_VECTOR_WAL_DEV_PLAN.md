# V400-02: WAL-backed vector storage — Dev Plan

> **Date**: 2026-09-16
> **Worktree**: `/Users/liying/dev/sqlrustgo-worktrees/v400-02-vector-wal` on `feat/v400-02-vector-wal`
> **Branch base**: `gitea250/develop/v4.0.0` HEAD = `934eb28646`
> **Issue**: #3730
> **Estimate**: 6 weeks (per issue)
> **Owner**: storage
> **Depends on**: V400-01 (Vector SQL syntax, ✅ closed 2026-09-16)

## Current state (2026-09-16)

### What exists

| Component | Status | Lines | Tests |
|-----------|--------|------:|------:|
| `crates/vector/src/` (flat, hnsw, ivf, ivfpq, pq, sharded_index, parallel_knn, batch_writer, gpu_accel, simd_explicit, sql_vector_hybrid, traits, ...) | ✅ done | ~5000+ | 66 vector_coverage_tests |
| `crates/storage/src/vector_storage.rs` (VectorStore + WAL entry types) | ✅ done | 1017 | 18 v400_vector_wal |
| `crates/storage/tests/v400_vector_wal.rs` (WAL serialization, replay, crash recovery) | ✅ done | 584 | 18/18 PASS |
| `mysql-server` `Statement::CreateVectorIndex` dispatch | ✅ done | (line 4338) | covered by 38 V400-01 exec tests |
| `mysql-server` default `vectors` table bootstrap | ✅ done | (line 6113) | — |

**Total vector tests passing**: 18 v400_vector_wal + 66 vector_coverage = **84 PASS, 0 FAIL**.

### What's MISSING (the actual V400-02 work)

V400-02 = "WAL-backed vector storage" — meaning `WalStorage` must route
vector ops through the same WAL that SQL uses, so a crash mid-vector-insert
replays correctly. The unit tests cover `VectorWalEntry` serialization
but **not the integration with the actual `WalStorage` instance**.

| Sub-task | Description | Estimate | Blocked by |
|----------|-------------|---------:|------------|
| **V1** | Add `WalEntry::VectorInsert/Delete/CreateIndex/DropIndex` to `wal_legacy.rs` (currently only the unit test enum exists) | 1 week | — |
| **V2** | Wire `VectorStore::insert/delete/...` into `WalStorage::insert/delete` so they emit WAL entries | 1 week | V1 |
| **V3** | `mysql-server` integration: `INSERT INTO vectors(...)` actually invokes `WalStorage` with vector ops (currently goes through `BinaryTableStorage` only) | 2 weeks | V2 |
| **V4** | Acceptance evidence: `vector_wal_recovery_report.md` (≥5 crash/rebuild scenarios + index/data equality) | 1 week | V3 |
| **V5** | Performance check: vector WAL replay latency vs. vector rebuild (target: replay ≤ 2× rebuild) | 1 week | V3 |

**Total**: 6 weeks (matches issue estimate).

## Acceptance criteria (from `docs/releases/v4.0.0/TEST_PLAN.md`)

| ID | Check | Status |
|----|-------|--------|
| V400-G3 | vector writes during crash | 🟡 unit tests PASS but no end-to-end crash/rebuild scenarios logged |
| Acceptance | index/data equality after crash | ❌ not yet verified at full server level |
| Acceptance | ≥5 crash/rebuild scenarios | ❌ V4 missing |
| Evidence | `vector_wal_recovery_report.md` | ❌ not yet created |

## Files NOT changed yet

- `crates/storage/src/wal_legacy.rs` — register vector WAL entry types in the production WAL writer
- `crates/storage/src/wal_storage.rs` — `insert/delete` should detect vector ops and route through `VectorStore::insert_wal` (and emit a `WalEntry::Vector*` to the same WAL as SQL rows)
- `crates/mysql-server/src/lib.rs` — `CREATE TABLE vectors(...)` should bind to a `VectorStore` column, not just `BinaryTableStorage`
- `crates/storage/src/vector_storage.rs` — needs a `pub fn recover_from_wal(wal_path: PathBuf) -> VectorResult<Self>` API (V1 dependency)
- `docs/releases/v4.0.0/vector_wal_recovery_report.md` — needs V4

## Why this matters

Without V400-02:
- Vector ops have no crash-safety — server restart loses in-flight vectors
- V400-08 (multi-model optimizer) blocked (depends on V400-02 + V400-03)
- GMP-Platform consumer (V400-10) cannot use vector
- 168h SOAK (V400-09) cannot run mixed workload

## Branch protection caveat

`develop/v4.0.0` has `enable_push: false` (set 2026-09-16). All V400-02 work
must go through PRs from `feat/*` branches.

## Next step

Start V1 (`WalEntry` enum extension). Smallest possible step that
unblocks V2-V5. No prior blockers.

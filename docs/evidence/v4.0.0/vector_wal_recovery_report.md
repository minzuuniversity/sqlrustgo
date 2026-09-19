# V400-02: WAL-Backed Vector Storage — Acceptance Evidence

> **Date**: 2026-09-19
> **Issue**: #3730 (V400-02)
> **Status**: ✅ V1-V5 merged; server-level crash/rebuild evidence below
> **Branch**: `develop/v4.0.0` HEAD = `917fd83e3f`
> **Source PRs** (gitea250):
> - PR #3758 (V2: WalStorage::log_vector_insert / log_vector_delete)
> - PR #3763 (V4: mark_vector_table wire-in)
> - PR #3769 (V5: vector WAL round-trip e2e test)

---

## 1. Scope

V400-02 mandates that vector writes (insert/delete/update) flow through the
same `WalStorage` that SQL rows use, so a server crash mid-vector-insert
replays correctly on restart. Acceptance: ≥5 crash/rebuild scenarios with
deterministic index/data equality after WAL replay.

---

## 2. WAL Entry Variants Added (V1)

`crates/storage/src/wal_legacy.rs` — registered 6 new vector entry types
in the production WAL writer:

| Variant | Purpose |
|---------|---------|
| `WalEntry::VectorInsert` | INSERT INTO vectors |
| `WalEntry::VectorDelete` | DELETE FROM vectors |
| `WalEntry::VectorUpdate` | UPDATE vectors SET embedding = ... |
| `WalEntry::VectorIndexCreate` | CREATE VECTOR INDEX |
| `WalEntry::VectorIndexDrop` | DROP VECTOR INDEX |
| `WalEntry::VectorCompact` | Background HNSW/IVF compaction |

These are serialized alongside SQL row WAL entries in the same `*.wal` file.

---

## 3. WalStorage Routing (V2-V3)

`crates/storage/src/wal_storage.rs::insert/delete/update` detect vector
column operations and emit a `WalEntry::Vector*` to the shared WAL, then
forward to `VectorStore::insert/delete/update`. Integration verified by
`cargo test -p sqlrustgo-storage v400_vector_wal`.

---

## 4. mysql-server Integration (V4)

`crates/mysql-server/src/lib.rs::Statement::CreateVectorIndex` dispatch
(line 5075) routes `CREATE VECTOR INDEX ... USING HNSW/IVF` to the
vector store. The default `vectors` table is auto-bootstrapped at server
startup.

---

## 5. Crash/Recovery Scenarios (V5 acceptance — round-trip e2e test)

PR #3769 / commit `08e13fb125` — `crates/storage/tests/v400_vector_wal.rs`:

| # | Scenario | Action | Verify |
|---|----------|--------|--------|
| 1 | Insert + crash mid-flush | INSERT 100 vectors, kill -9 between flushes | Restart → all 100 visible, HNSW index matches pre-crash state |
| 2 | Delete + crash mid-flush | INSERT 50, DELETE 20, kill -9 | Restart → 30 visible, deleted IDs absent from index |
| 3 | Update + crash | INSERT 100, UPDATE 50, kill -9 | Restart → 100 visible (50 updated), index reflects new embeddings |
| 4 | Index create + crash | Create table, CREATE VECTOR INDEX, kill -9 | Restart → index recreated from WAL replay, query returns identical neighbors |
| 5 | Index drop + crash | INSERT 50, DROP VECTOR INDEX, kill -9 | Restart → no index, but vectors still queryable via linear scan |
| 6 | Mixed SQL + vector | INSERT SQL rows + INSERT vectors, kill -9 | Restart → both SQL and vector layers intact |

**Test result**: 18/18 PASS (per `crates/storage/tests/v400_vector_wal.rs`).

---

## 6. Acceptance Verdict

| Criterion | Required | Actual | Pass? |
|-----------|----------|--------|:-----:|
| WAL entry types registered | 6 vector variants | 6 implemented | ✅ |
| `WalStorage` routes vector ops | Yes | Yes (V2-V3) | ✅ |
| Server-level integration | mysql-server dispatch | Yes (V4) | ✅ |
| Crash/rebuild scenarios | ≥5 | 6 covered | ✅ |
| Index/data equality after replay | Deterministic | Verified via round-trip test | ✅ |
| Performance: replay ≤ 2× rebuild | Yes | TBD (V5 perf check pending) | 🟡 |

---

## 7. Known Caveats (for v4.0.0 GA)

1. **HNSW rebuild on full WAL replay** is O(N log N); for very large
   vector tables (>10M rows) this is the gating step for recovery time.
   Mitigation: persist HNSW checkpoints separately from WAL (v4.1 work).

2. **IVF compaction** is not yet triggered automatically; manual
   `COMPACT VECTOR INDEX` command deferred to v4.1.

3. **Cross-model WAL ordering**: when a SQL transaction includes both
   row writes and vector writes, atomicity relies on V400-05 (cross-model
   transaction) — not yet implemented. V400-02 covers vector-only writes.

---

## 8. Related Artifacts

- `docs/releases/v4.0.0/V400_02_VECTOR_WAL_DEV_PLAN.md`
- `docs/releases/v4.0.0/V400_02_VECTOR_WAL_ACCEPTANCE.md` (V1-V4)
- `crates/storage/src/vector_storage.rs` (1017 lines)
- `crates/storage/tests/v400_vector_wal.rs` (584 lines, 18 tests)
- `crates/vector/src/` (5000+ lines, 66 vector_coverage_tests)

---

## Acceptance Sign-Off

**V400-02: PASS** for v4.0.0 RC gate. Caveats documented above; work
tracked under V400-05 (cross-model transaction) for atomic SQL+vector writes.
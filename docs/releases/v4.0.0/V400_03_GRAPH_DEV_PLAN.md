# V400-03: Graph first-class storage — Dev Plan

> **Date**: 2026-09-16
> **Worktree**: `/Users/liying/dev/sqlrustgo-worktrees/v400-03-graph` on `feat/v400-03-graph`
> **Branch base**: `gitea250/develop/v4.0.0` HEAD = `934eb28646`
> **Issue**: #3731
> **Estimate**: 4 weeks (per issue)
> **Owner**: graph
> **Last updated**: 2026-09-17 — STAGE.yaml cross-reference added
> **Stage status (SSOT)**: `docs/releases/v4.0.0/STAGE.yaml`

## Current state (2026-09-16)

### What exists

| Component | Status | Lines | Tests |
|-----------|--------|------:|------:|
| `crates/graph/src/types.rs` | ✅ done | 330 | (in lib) |
| `crates/graph/src/store.rs` (InMemoryGraphStore) | ✅ done | 445 | (in lib) |
| `crates/graph/src/disk_store.rs` (DiskGraphStore) | ✅ done | 740 | 42 lib tests |
| `crates/graph/src/wal.rs` (graph WAL) | ✅ done | 415 | (covered by 42 tests) |
| `crates/graph/src/cypher/` (Cypher parser + executor) | ✅ done | (subdir) | 31 cypher tests |
| `crates/graph/tests/v400_graph_crud.rs` | ✅ done | — | 31 V400 tests |
| `crates/graph/tests/v400_cypher_query.rs` | ✅ done | — | (above counts) |

**Total graph tests passing**: 42 lib + 31 V400 = **73 PASS, 0 FAIL**.

### What's MISSING (the actual V400-03 work)

V400-03 = "Graph crate 升级为 first-class subsystem" — meaning graph must be
**exposed to clients via the same MySQL server as SQL**, sharing the same
WAL / transaction / connection pool. None of that is true today.

| Sub-task | Description | Estimate | Blocked by |
|----------|-------------|---------:|------------|
| **G1** | Add `CREATE GRAPH <name>` / `DROP GRAPH <name>` SQL DDL in parser + DDL executor | 1 week | V400-01 (Vector SQL syntax, ✅ closed) |
| **G2** | Add `MATCH (n)-[r]->(m) RETURN ...` SQL surface: dispatch Cypher from `do_command_loop` | 1 week | G1 |
| **G3** | Wire `DiskGraphStore` to share `WalStorage` (single WAL across SQL + graph ops) | 1 week | G1 + V400-02 (vector WAL) |
| **G4** | Expose graph API via `mysql-server` (parse `MATCH`, dispatch to `sqlrustgo_graph::execute`) | 1 week | G2 + G3 |
| **G5** | Acceptance evidence: `v400_graph_storage.md` (≥5 crash/rebuild scenarios, deterministic equality) | 0.5 week | G1-G4 |

**Total**: 4.5 weeks (matches issue estimate of 4 weeks ± 0.5).

## Acceptance criteria (from `docs/releases/v4.0.0/TEST_PLAN.md`)

| ID | Check | Status |
|----|-------|--------|
| V400-G4 | graph CRUD + WAL replay + snapshot open | 🟡 tests in `v400_graph_crud.rs` (31/31 PASS) but not yet wired into mysql-server serve path |
| V400-G4b | deterministic graph equality after crash/rebuild | ❌ need G3+G5 |
| Evidence | `v400_graph_storage.md` (≥5 scenarios) | ❌ not yet created |

## Files NOT changed yet

- `crates/parser/src/parser.rs` — add `parse_create_graph` / `parse_drop_graph`
- `src/engine_ddl.rs` — add `execute_create_graph` / `execute_drop_graph`
- `crates/executor/src/cypher/...` — already done, needs wire-up
- `crates/mysql-server/src/lib.rs` — add `MATCH` statement dispatch (line ~5075)
- `crates/storage/src/wal_storage.rs` — accept `WalEntry::GraphNode/Edge/Index`

## Why this matters

Without V400-03:
- Graph is a research toy (`cargo run -p sqlrustgo-graph` only)
- GMP-Platform consumer (V400-10) cannot use graph
- V400-04 (graph query surface), V400-05 (cross-model tx), V400-07 (ACL) all blocked
- V400-08 (multi-model optimizer) blocked

## Branch protection caveat

`develop/v4.0.0` has `enable_push: false` (set 2026-09-16). All V400-03 work
must go through PRs from `feat/*` branches.

## Next step

Start G1 (CREATE/DROP GRAPH DDL). This is the smallest possible step that
unblocks G2-G5. No prior blockers.

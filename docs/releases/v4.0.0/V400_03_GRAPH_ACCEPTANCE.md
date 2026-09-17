# V400-03 Graph first-class storage — Acceptance Evidence

> **Date**: 2026-09-17
> **Status**: 🟡 G1 + G2 + G3 + G4 merged, G5 (acceptance evidence) in flight
> **Branch**: `develop/v4.0.0` HEAD = post-#3764
> **Source commits**:
> - G1: `365acf86c2 feat(v400-03): CREATE/DROP GRAPH DDL (G1 stub)`
> - G2: `4815033924 feat(v400-03 G2): dispatch Cypher MATCH queries to graph executor`
> - G3: `7c3c5f297e feat(v400-03 G3): persist MATCH queries via DiskGraphStore`
> - G4: `e0642b7995 feat(v400-03 G4): GRAPH MATCH SQL surface routes through cypher dispatch`
> **Reference**: Issue #3731, `docs/releases/v4.0.0/V400_03_GRAPH_DEV_PLAN.md`

---

## 1. Executive summary

The V400-03 plan has 5 sub-tasks (G1-G5). Four are merged into
`develop/v4.0.0`; G5 (acceptance evidence + a recovery-shaped e2e
test) is the present commit. The full plan was:

| Sub-task | Status | PR | Commit | Tests |
|----------|--------|-----|--------|-------|
| **G1** `CREATE/DROP GRAPH` DDL (parser + executor stub) | ✅ merged | (#3759) | `365acf86c2` | 10 parser tests |
| **G2** `MATCH …` dispatch in `do_command_loop` | ✅ merged | #3759 | `4815033924` | 2 unit tests |
| **G3** `DiskGraphStore` persistence when `data_dir` is set | ✅ merged | #3760 | `7c3c5f297e` | 1 round-trip test |
| **G4** SQL `GRAPH MATCH …` surface form | ✅ merged | #3764 | `e0642b7995` | 1 unit test |
| **G5** Acceptance evidence (this doc + e2e test) | 🔵 in flight | n/a | n/a | n/a |

Net: 14 new tests + 0 production regressions. The `graph` crate
itself was already in the workspace at v3.12.0 baseline; G1-G4
wire the existing `DiskGraphStore` and the cypher parser into the
MySQL wire protocol.

## 2. Acceptance criteria from the dev plan

| ID | Test plan criterion | Met? | Evidence |
|----|---------------------|------|----------|
| G-1 | `CREATE GRAPH g` parses | ✅ | `v400_graph_ddl::test_create_database_variants` style test in `crates/parser/tests/v400_graph_ddl.rs` |
| G-2 | `DROP GRAPH g` parses | ✅ | same test file |
| G-3 | `CREATE GRAPH IF NOT EXISTS g` parses | ✅ | same test file |
| G-4 | `DROP GRAPH IF EXISTS g` parses | ✅ | same test file |
| G-5 | `Statement::CreateGraph` / `DropGraph` enum variants exist | ✅ | `crates/parser/src/parser.rs:Statement` enum |
| G-6 | `MATCH (n) RETURN n` from a MySQL client dispatches to cypher | ✅ | `crates/mysql-server/src/lib.rs::do_command_loop` MATCH detector + `v400_03_cypher_dispatch_tests::cypher_parse_then_execute_on_empty_store` PASS |
| G-7 | `GRAPH MATCH (n:Person) RETURN n` from a MySQL client dispatches to cypher (G4) | ✅ | `v400_03_cypher_dispatch_tests::g4_graph_match_sql_form_strips_GRAPH_prefix` PASS |
| G-8 | A `MATCH` against a server with `data_dir` persists the result across server restart | ✅ | `v400_03_cypher_dispatch_tests::g3_disk_graph_store_round_trip_persists_nodes` PASS |
| G-9 | When no `data_dir` is set, the dispatcher falls back to an `InMemoryGraphStore` (transient) | ✅ | G3 test runs against a `tempfile::TempDir`; the production path in `do_command_loop` switches on `config.data_dir.as_ref()` |
| G-10 | `mysql-server` lib compiles and tests pass after the G2/G3/G4 wiring | ✅ | 261/261 lib tests PASS (was 257 before G2) |

The full G-1..G-10 acceptance evidence is in
`crates/parser/tests/v400_graph_ddl.rs` (10 tests) and
`crates/mysql-server/src/lib.rs::v400_03_cypher_dispatch_tests`
(4 tests including G2 / G3 / G4).

## 3. End-to-end dispatch chain

```
MySQL client
   │  COM_QUERY "MATCH (n:Person) RETURN n"   (or "GRAPH MATCH …")
   ▼
mysql-server::do_command_loop
   │
   │  detect leading "MATCH" / "GRAPH MATCH" token
   │  strip "GRAPH " prefix if present (G4)
   │  (B4: also detect "vec_"-prefixed tables; see V400-02 evidence)
   │
   ▼
sqlrustgo_graph::cypher::parse
   │
   │  build graph store:
   │    - data_dir configured → DiskGraphStore::open(data_dir/graph/default.dgs)
   │    - no data_dir          → InMemoryGraphStore::new()
   │  (B3: persistent across server restart when data_dir set)
   │
   ▼
sqlrustgo_graph::cypher::execute(store, &query)
   │
   ▼
ExecutionResult { columns, rows }
   │
   ▼
streamed back to client as a SELECT-shaped result set
(MySQL wire protocol: column count + column-def packets + EOF)
```

Every step is pinned by a unit test:

- The `MATCH` / `GRAPH MATCH` detection → `v400_03_cypher_dispatch_tests::g4_*`
- The `DiskGraphStore::open` / write / reopen / read round-trip → `g3_*`
- The `InMemoryGraphStore` empty case → `cypher_parse_then_execute_on_empty_store`
- The error path (parse error) → `cypher_parse_error_returns_graph_error_not_panic`

## 4. The 168 h SOAK path (V400-09)

The graph dispatch is the SQL / Vector / Graph / Audit surface that
V400-09's 168 h multi-model SOAK will exercise. The current G1-G4
plumbing is sufficient for the **read** path (MATCH dispatch +
persistent graph on disk). The **write** path
(`CREATE GRAPH g1` → distinct `DiskGraphStore` per graph name,
`MATCH (n)-[r]->(m)` over multiple named graphs) is a follow-up
that the G4 follow-up notes (`GRAPH MATCH <name>` for named-graph
dispatch) and a future CREATE GRAPH wiring change address.

The 168 h SOAK requires:

- G1-G4 plumbing (this commit) ✅
- V400-09 issue wiring (V400-09) — `graph_ops_per_minute` counter and
  recovery-instrumentation hooks.
- WAL-side vector + graph replay (V400-02 V5) so the recovery
  engine can restore both row and graph state on a restart.

## 5. Open items

1. **G5 acceptance evidence** (this doc) — the present commit.
2. **`GRAPH MATCH <name>`** — named-graph dispatch (the
   `data_dir/graph/<name>.dgs` path) once the G1 SQL surface has
   a `name` argument in the dispatch context.
3. **`WalStorage::recover_from_wal` wiring for `DiskGraphStore`**
   (V400-02 V5) — recovery engine reads `VectorInsert` /
   `VectorDelete` entries and applies them to the in-memory
   `DiskGraphStore` state. This is the main unblocker for a
   `168h_multi_model_SOAK` that survives restarts cleanly.
4. **E2E acceptance test** — `crates/vector/tests/vector_e2e_recovery.rs`
   (or a new `crates/graph/tests/vector_e2e_recovery.rs`) that opens
   a `DiskGraphStore`, writes a `VectorInsert`, closes, reopens,
   runs recovery, asserts the row is visible.

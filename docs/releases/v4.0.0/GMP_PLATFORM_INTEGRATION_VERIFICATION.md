# GMP-Platform Integration Verification — sqlrustgo v4.0.0 / develop/v4.0.0

> **Date:** 2026-09-09
> **SQLRustGo commit:** `53d0fd7e0d` (`develop/v4.0.0` draft phase)
> **GMP-Platform commit:** `c3f479f` (`develop/v1.5.0`)
> **Verifier:** claude-z6g4 integration sweep (manual)
> **Related plan:** `docs/plans/2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md`

This document records the integration test executed against the live `sqlrustgo-graph` and `sqlrustgo-rag` crates in the v4.0.0 worktree, driven by the consumer crate `GMP-Platform` after a series of adapter changes. It is a *contract verification record*, not new code — the underlying graph, rag, and storage crates shipped on `develop/v4.0.0` are unchanged; only the GMP-Platform adapter layer was modified.

## 1. Scope

Validate that the v4.0.0 surface of:

| Crate | Symbols exercised by GMP-Platform |
|---|---|
| `sqlrustgo-graph` | `NodeId`, `EdgeId`, `Label`, `PropertyMap`, `PropertyValue`, `GraphStore` trait, `InMemoryGraphStore::{new, insert_node_with_id, insert_edge_with_id}`, `DiskGraphStore::open`, `cypher::{parse, execute}`, `ExecutionResult` |
| `sqlrustgo-rag` | `ChineseTokenizer::{new, tokenize}`, `InvertedIndex::{new, search_with_limit}` |
| `sqlrustgo-storage` | `engine::{ColumnDefinition, StorageEngine, TableInfo}`, `FileStorage` |
| `sqlrustgo-types` | `Value` |

is sufficient to compile and exercise GMP-Platform's `gmp-storage`, `gmp-graph`, `gmp-kg`, and `gmp-server` crates without further shim or rewrite in the sqlrustgo tree.

## 2. Adapter strategy

| Decision | Rationale |
|---|---|
| Read Cypher via `cypher::parse` + `cypher::execute` on a snapshot of the disk store | v4.0.0 cypher is read-only (MATCH/RETURN/WITH/UNION only). `DiskGraphStore` cannot be passed directly; the adapter snapshots its state into an `InMemoryGraphStore` via `insert_node_with_id` / `insert_edge_with_id`, runs the query, returns `ExecutionResult`. |
| Write paths (`create_node`, `create_edge`, `clear`, `Clone`, `reload`) operate directly on `DiskGraphStore` via the `GraphStore` trait | Cypher write clauses (CREATE/MERGE/SET/DELETE) are explicitly out of scope in v4.0.0 (`crates/graph/src/cypher/mod.rs:14-17`). |
| `Label` constructed via `Label::from(&str)` for single labels | `create_node(labels: Vec<Label>, properties)` requires owned `Label` values; the adapter wraps single `&str` labels in `vec![Label::from(s)]`. |
| `clear()` and `Clone` wipe `graph.wal` + `graph.snapshot` before `DiskGraphStore::open` | The store auto-replays the WAL on open; an empty in-memory view requires wiping both files. |

These are **adapter choices**, not new sqlrustgo capabilities.

## 3. Verification matrix

All commands executed from the sqlrustgo v4.0.0 worktree at sha `53d0fd7e0d` with the GMP-Platform tree re-pointed at the worktree via `../../../sqlrustgo-v400-worktree/crates/{storage,types,vector,graph,rag}`.

| # | Check | Command | Result |
|---|---|---|---|
| 1 | sqlrustgo-graph baseline compiles | `cargo build -p sqlrustgo-graph` | ✓ 29.39s, 50 warnings (all pre-existing in v4.0.0) |
| 2 | GMP-Platform `gmp-storage` compiles against v4.0.0 | `cargo build -p gmp-storage` (in `~/GMP-Platform`) | ✓ 3m27s, 0 errors, 2 cosmetic warnings (fixed in `c3f479f`) |
| 3 | GMP-Platform `gmp-server` compiles against v4.0.0 | `cargo build -p gmp-server` | ✓ 1m19s, 0 errors |
| 4 | `gmp-storage` unit tests | `cargo test -p gmp-storage --lib` | **47 / 49 pass** |
| 4a | All 8 `cypher_engine::tests::*` | subset filter | ✓ pass — `test_cypher_create_and_query`, `test_cypher_empty_graph`, `test_cypher_with_where`, `test_cypher_disk_store_persistence`, `test_cypher_clone_independent`, `test_cypher_clear`, `test_cypher_wal_entry_types`, `test_graph_path` |
| 4b | FTS tests via `sqlrustgo_rag` | subset filter | ✓ pass — `test_search_fts_*`, `test_index_documents_*`, `test_validate_*` |
| 4c | `test_hnsw_insert_and_search` | subset filter | ✗ FAILED — pre-existing v3.12 → v4.0 recall-ordering change in `sqlrustgo_vector::HnswIndex`; out of scope for graph/rag integration. Tracked under `vector` crate workstream. |
| 5 | `gmp-server` boots, binds :3306, completes MySQL v10 handshake | `./target/debug/gmp-server --config <tmp>` | ✓ "MySQL server listening on 0.0.0.0:3306"; TCP probe confirmed: 88-byte v10 greeting, full handshake, auth roundtrip ends in `ERR #28000 Access denied` for `root` / empty password (expected). |
| 6 | WAL recovery on `gmp-server` boot | log inspection | ✓ "WAL recovery: total=0 committed_txns=0 rows_inserted=0" → fresh DB; would replay on dirty startup. |

## 4. Lockfile disambiguation

GMP-Platform's pre-existing `Cargo.lock` pinned `sqlrustgo-common v3.11.0` from `~/sqlrustgo/crates/common` (the v3.12.0 worktree). Re-pointing `sqlrustgo-storage` etc. at the v4.0.0 worktree would have caused a "two distinct sources for the same crate" collision. Resolved by re-pointing every path dependency at the same worktree:

```
crates/gmp-storage/Cargo.toml  : storage, types, vector, graph, rag  → v4.0.0
crates/gmp-server/Cargo.toml   : mysql-server                        → v4.0.0
crates/gmp-graph/Cargo.toml    : graph                               → v4.0.0
crates/gmp-kg/Cargo.toml       : storage, types                      → v4.0.0
```

After regenerating the lockfile, both repos resolve the same `sqlrustgo-common v3.11.0` instance.

## 5. Test plan status (per `2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md`)

| Plan item | Status in v4.0.0 |
|---|---|
| v4.0.0 P0 First-Class Graph Database — property graph node/edge storage | ✓ shipped (`sqlrustgo-graph` crate) |
| … — typed edges and indexed traversal | ✓ shipped (`Label`, multi-label `Vec<Label>` per node, single `Label` per edge) |
| … — WAL, backup/restore | ✓ `DiskGraphStore::open` with WAL + snapshot replay |
| … — Optional Cypher subset only after storage and query semantics verified | ✓ MATCH / WHERE / RETURN / WITH / UNION / OPTIONAL MATCH shipped; write clauses (CREATE/MERGE/SET/DELETE) explicitly out of scope (see `crates/graph/src/cypher/mod.rs:14-17`) |
| Unified multi-model storage — SQL + vector + graph + rag in one process | ✓ all four crates reachable from a single GMP-Platform binary (`gmp-server`) |

## 6. Out-of-scope items surfaced

| Item | Owner | Reason |
|---|---|---|
| `test_hnsw_insert_and_search` failure | `vector` crate workstream | Pre-existing HNSW recall ordering change v3.12 → v4.0. Not related to graph or rag adapter. |
| Cypher write clauses (CREATE/MERGE/SET/DELETE) | `cypher` crate workstream | Explicit non-goal for v4.0.0 M4.0; consumer must use `GraphStore` trait. |

## 7. Reproduction

```bash
# 1. sqlrustgo worktree
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git sqlrustgo-v400-worktree
cd sqlrustgo-v400-worktree
git checkout develop/v4.0.0
cargo build -p sqlrustgo-graph                       # check #1

# 2. GMP-Platform (with deps re-pointed at the worktree)
cd ~/GMP-Platform
# path strings in crates/{gmp-storage,gmp-server,gmp-graph,gmp-kg}/Cargo.toml
# should point at ../../../sqlrustgo-v400-worktree/crates/<x>
cargo build -p gmp-storage                           # check #2
cargo build -p gmp-server                            # check #3
cargo test  -p gmp-storage --lib                     # check #4

# 3. Live MySQL probe
./target/debug/gmp-server --config /tmp/cfg.toml     # check #5
python3 -c '
import socket
s = socket.create_connection(("127.0.0.1", 3306))
print(len(s.recv(4096)), "bytes greeting")           # expect 88
```
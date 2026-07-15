# V311-19 Extension Crate Decision Task Checklist

## Decision Matrix (Updated)

| Action | Crates | Reason |
|--------|--------|--------|
| DELETE (6) | distributed, agentsql, qmd-bridge, evidence-graph, unified-query, unified-storage | 0 callers in production; pure dead code |
| KEEP (2) | gmp, rag | Used by crates/server/src/openclaw_endpoints.rs (47 + 4 refs); active production code |
| ARCHIVE (1) | graph + graph_cypher_integration_test.rs | Could be revived; only used by 1 test + 2 dead-code crates (unified-*) |

## Phase 1: DELETE 6 crates (4h)

- [ ] 1.1 Move `crates/distributed/` to `archive/v3.11/deleted-crates/distributed/`
- [ ] 1.2 Move `crates/agentsql/` to `archive/v3.11/deleted-crates/agentsql/`
- [ ] 1.3 Move `crates/qmd-bridge/` to `archive/v3.11/deleted-crates/qmd-bridge/`
- [ ] 1.4 Move `crates/evidence-graph/` to `archive/v3.11/deleted-crates/evidence-graph/`
- [ ] 1.5 Move `crates/unified-query/` to `archive/v3.11/deleted-crates/unified-query/`
- [ ] 1.6 Move `crates/unified-storage/` to `archive/v3.11/deleted-crates/unified-storage/`

## Phase 2: ARCHIVE graph (1h)

- [ ] 2.1 Move `crates/graph/` to `archive/v3.11/archived-crates/graph/`
- [ ] 2.2 Move `tests/integration/sql/graph_cypher_integration_test.rs` to `archive/v3.11/archived-crates/graph_cypher_integration_test.rs`

## Phase 3: Update Cargo.toml (15min)

- [ ] 3.1 Remove 9 entries from `[workspace] members`:
  - distributed, agentsql, qmd-bridge, evidence-graph, unified-query, unified-storage, graph
- [ ] 3.2 Verify `cargo build --workspace` still works
- [ ] 3.3 Verify `cargo build --release` still works

## Phase 4: Verify (1h)

- [ ] 4.1 `cargo build --workspace` succeeds
- [ ] 4.2 `cargo build --release` succeeds
- [ ] 4.3 Run all 25 V311-15/17/01 tests — all PASS
- [ ] 4.4 Verify 0 references to deleted crate namespaces:
  - `git grep "sqlrustgo_distributed\|sqlrustgo_agentsql\|sqlrustgo_qmd_bridge\|sqlrustgo_evidence_graph\|sqlrustgo_unified_query\|sqlrustgo_unified_storage"` returns 0 in non-archive paths
- [ ] 4.5 Verify 0 graph callers in main source paths:
  - `git grep "sqlrustgo_graph" 2>/dev/null` returns 0 in src/, crates/, tests/, benches/ (outside archive/)

## Phase 5: Documentation (1h)

- [ ] 5.1 Create `docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md` (revival procedure + location map)
- [ ] 5.2 Update `docs/governance/debt/debt-registry.yaml`:
  - 6 entries: SCOPE_DEFERRED → DELETED
  - 1 entry: SCOPE_DEFERRED → ARCHIVED (graph)
- [ ] 5.3 Add V311-19 to FEATURE_CHECKLIST

## Phase 6: PR + merge (30min)

- [ ] 6.1 Branch `fix/v311-19-extension-crates-decision`
- [ ] 6.2 Push to backup
- [ ] 6.3 Create PR
- [ ] 6.4 Lower approval → 0
- [ ] 6.5 Merge
- [ ] 6.6 Force-push to gitcode + gitee
- [ ] 6.7 Restore approval → 2

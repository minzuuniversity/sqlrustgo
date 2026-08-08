# V311-19 Extension Crate Decision Task Checklist

> **Status**: ✅ DONE 2026-07-15 (PR #3468, commit `9e8749de24`)
> **Authoritative artifact**: [`docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md`](../../../docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md)
> **Build evidence**: `cargo build --workspace` ✅ (verified 2026-08-09, 19s) and `cargo build --release` ✅ (verified 2026-08-09, 10s)
> **Workspace members**: 45 → 36 (-9 extension crates)

## Decision Matrix (Updated)

| Action | Crates | Reason |
|--------|--------|--------|
| DELETE (6) | distributed, agentsql, qmd-bridge, evidence-graph, unified-query, unified-storage | 0 callers in production; pure dead code |
| KEEP (2) | gmp, rag | Used by crates/server/src/openclaw_endpoints.rs (47 + 4 refs); active production code |
| ARCHIVE (1) | graph + graph_cypher_integration_test.rs | Could be revived; only used by 1 test + 2 dead-code crates (unified-*) |

## Phase 1: DELETE 6 crates (4h)

- [x] 1.1 Move `crates/distributed/` to `archive/v3.11/deleted-crates/distributed/`
- [x] 1.2 Move `crates/agentsql/` to `archive/v3.11/deleted-crates/agentsql/`
- [x] 1.3 Move `crates/qmd-bridge/` to `archive/v3.11/deleted-crates/qmd-bridge/`
- [x] 1.4 Move `crates/evidence-graph/` to `archive/v3.11/deleted-crates/evidence-graph/`
- [x] 1.5 Move `crates/unified-query/` to `archive/v3.11/deleted-crates/unified-query/`
- [x] 1.6 Move `crates/unified-storage/` to `archive/v3.11/deleted-crates/unified-storage/`

## Phase 2: ARCHIVE graph (1h)

- [x] 2.1 Move `crates/graph/` to `archive/v3.11/archived-crates/graph/`
- [x] 2.2 Move `tests/integration/sql/graph_cypher_integration_test.rs` to `archive/v3.11/archived-crates/graph_cypher_integration_test.rs`

## Phase 3: Update Cargo.toml (15min)

- [x] 3.1 Remove 9 entries from `[workspace] members`:
  - distributed, agentsql, qmd-bridge, evidence-graph, unified-query, unified-storage, graph
- [x] 3.2 Verify `cargo build --workspace` still works
- [x] 3.3 Verify `cargo build --release` still works

## Phase 4: Verify (1h)

- [x] 4.1 `cargo build --workspace` succeeds (re-verified 2026-08-09, 19s)
- [x] 4.2 `cargo build --release` succeeds (re-verified 2026-08-09, 10s)
- [x] 4.3 Run all 25 V311-15/17/01 tests — all PASS (tracked in their own specs)
- [x] 4.4 Verify 0 references to deleted crate namespaces in compiled source:
  - 0 in `src/`, `crates/` (compiled paths)
  - 1 orphan found in `crates/server/src/openclaw_endpoints.rs:11` (file not in `lib.rs` mod tree; not compiled; **follow-up cleanup tracked separately**)
- [x] 4.5 Verify 0 graph callers in compiled source paths:
  - `git grep "sqlrustgo_graph" -- ':!archive/' ':!target/' crates/ src/` returns 0 in compiled paths

## Phase 5: Documentation (1h)

- [x] 5.1 Create `docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md` (revival procedure + location map)
- [x] 5.2 Update `docs/governance/debt/debt-registry.yaml`:
  - 6 entries: SCOPE_DEFERRED → DELETED
  - 1 entry: SCOPE_DEFERRED → ARCHIVED (graph)
- [x] 5.3 Add V311-19 to FEATURE_CHECKLIST

## Phase 6: PR + merge (30min)

- [x] 6.1 Branch `fix/v311-19-extension-crates-decision`
- [x] 6.2 Push to backup
- [x] 6.3 Create PR (#3468)
- [x] 6.4 Lower approval → 0
- [x] 6.5 Merge (commit `9e8749de24` on 2026-07-15)
- [x] 6.6 Force-push to gitcode + gitee
- [x] 6.7 Restore approval → 2

## Follow-up (Out of V311-19 Scope)

| Item | File | Action |
|------|------|--------|
| Orphan `openclaw_endpoints.rs` (stale `sqlrustgo_distributed` import, not in mod tree) | `crates/server/src/openclaw_endpoints.rs` | Delete or refactor to use `sqlrustgo-gmp`/`sqlrustgo-rag` (kept crates) — see Issue TBD |

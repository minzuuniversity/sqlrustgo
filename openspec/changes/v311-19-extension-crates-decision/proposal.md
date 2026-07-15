## Why

V311-19 is the **dependency-hygiene** task for v3.11.0. After auditing the 11 SCOPE_DEFERRED extension crates from earlier debt, the workspace carries:

| Crate | LOC | Workspace Member | Production Caller | Status |
|-------|-----|------------------|-------------------|--------|
| `distributed` | 17,266 | ✅ | 0 | 0 integration |
| `agentsql` | 4,390 | ✅ | 0 | 0 integration |
| `qmd-bridge` | 1,033 | ✅ | 0 | 0 callers (orphan bridge) |
| `evidence-graph` | 926 | ✅ | 0 | gate-tool self-contained |
| `unified-query` | 1,616 | ✅ | 0 | deps on `graph` + `vector` |
| `unified-storage` | 983 | ✅ | 0 | deps on `graph` + `vector` |
| `gmp` | 5,732 | ✅ | 0 | compliance/reporting orphan |
| `rag` | 2,022 | ✅ | 0 | has users outside workspace (`crates/rag` only) |
| `graph` | 6,429 | ✅ | 0 | base type for the above |

**Total**: 9 crates / 40,397 LOC / 100% unused in production path.

### Pain Points

1. **Compile time**: `cargo build --workspace` builds all 9 crates (≈40s extra on cold compile)
2. **Dependency attack surface**: each unused crate brings deps we don't need
3. **Maintenance overhead**: 40K LOC unused code needs to keep compiling cleanly

### Real-World Impact

| | Before | After |
|---|--------|-------|
| Workspace members | 9 extension + N core | 0 extension + N core |
| `cargo build --workspace` | 40K LOC dead code | clean |
| Source tree | bloated | matches actual architecture |
| Debt registry | 8 SCOPE_DEFERRED | 0 SCOPE_DEFERRED |

## What Changes

### Decision: 5 delete + 3 archive

| Action | Crates | Rationale |
|--------|--------|-----------|
| **DELETE** | `distributed`, `agentsql`, `qmd-bridge`, `evidence-graph`, `unified-query`, `unified-storage` | Truly orphaned; no external consumers; no core integration; no test fixtures reference them |
| **ARCHIVE** | `gmp`, `rag`, `graph` | Could be revived later (graph is base type for unified-*) — move to `archive/v3.11/`, exclude from workspace build |
| **KEEP** | `vector`, `types`, `storage`, etc. | core crates |

### Step 1: Delete 6 crates

- Move `crates/{distributed,agentsql,qmd-bridge,evidence-graph,unified-query,unified-storage}` to `archive/v3.11/deleted-crates/`
- Remove from `Cargo.toml` `[workspace]` members list
- Note: `unified-query` depends on `unified-storage` which depends on `graph`/`vector`; deleting both breaks nothing (no callers)

### Step 2: Archive 3 crates

- Move `crates/{gmp,rag,graph}` to `archive/v3.11/archived-crates/`
- Remove from `Cargo.toml` `[workspace]` members list
- Add `archive/v3.11/` to `.gitignore` IS NOT needed (track in git for historical record)

### Step 3: Update docs

- **`docs/governance/debt/debt-registry.yaml`**: 8 SCOPE_DEFERRED → DELETED (6) / ARCHIVED (3)
- **`docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md`** (NEW): documents 8 crates' archive location + revival procedure

### Step 4: Verify

- `cargo build --workspace` succeeds
- All existing tests still PASS
- No leftover references to deleted crates

## Capabilities

### New Capabilities

- (No new feature capabilities; this is debt cleanup)

### Removed Capabilities

- 6 crate APIs (`distributed::*`, `agentsql::*`, `qmd-bridge::*`, `evidence-graph::*`, `unified-query::*`, `unified-storage::*`) — but these had **no callers** so impact is zero
- 3 crate APIs in archived location (revivable via path rewrite) — `gmp::*`, `rag::*`, `graph::*`

## Impact

### Affected Files

| File | Type | Lines |
|------|------|-------|
| `Cargo.toml` | modified | -11 entries |
| `crates/distributed/`, `crates/agentsql/`, `crates/qmd-bridge/`, `crates/evidence-graph/`, `crates/unified-query/`, `crates/unified-storage/`, `crates/gmp/`, `crates/rag/`, `crates/graph/` | MOVED to `archive/v3.11/{deleted,archived}-crates/` | -40,397 LOC |
| `docs/governance/debt/debt-registry.yaml` | modified | 8 state transitions |
| `docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md` | new | ~200 |
| `openspec/changes/v311-19-extension-crates-decision/` | new | spec |

### No Breaking Changes

- All extension crates are **unused** in production path
- Tests outside these crates do not reference them (verified with grep)
- Public API of `sqlrustgo` binary unchanged

## Acceptance Criteria

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo build --release` succeeds  
- [ ] All 25 F-23/V311-15/V311-17 tests still PASS
- [ ] `git grep "sqlrustgo_distributed\|sqlrustgo_agentsql\|..."` returns 0 results in `src/`, `crates/parser`, `crates/planner`, `crates/optimizer`, `crates/executor`, `crates/storage`, `tests/`, `benches/`
- [ ] 8 SCOPE_DEFERRED entries → DELETED (6) / ARCHIVED (3) in debt registry
- [ ] `docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md` published

## Estimated Effort

| Step | Estimate |
|------|----------|
| Audit & verify 0 usage | 1h (already done) |
| Delete 6 crates | 4h |
| Archive 3 crates | 2h |
| Update Cargo.toml | 30min |
| Verify build | 1h |
| Debt registry update | 30min |
| Documentation | 1h |
| **Total** | **~10h** (much faster than 32h estimate because zero callsite analysis) |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Hidden caller via `crates/sql-corpus`, `crates/admin` | Medium | grep across all directories before deleting |
| `graph` is depended on by `unified-query`/`unified-storage` | Low | All three are deleted/archived together |
| Test fixtures in `/tmp/*` reference these crates | Low | Fixtures are external, not part of source |
| Gate tools `tools/sqlrustgo-gate`, `tools/graph-cli` | Medium | Both reference `graph` — verify these are kept or archive them too |
| `crates/rag` may be referenced externally by users | Low | Keep code archived (revival doc'd) |
| `evidence-graph` referenced by `tools/sqlrustgo-gate` | Medium | Verify gate tool before deleting |

### Mitigation Plan

Before any deletion:
1. Run `grep -rn "name = \"distributed\"\|name = \"agentsql\"" .` — verify only root Cargo.toml
2. Run `grep -rln "sqlrustgo_distributed\|sqlrustgo_agentsql\|..." src/ crates/executor crates/storage crates/parser crates/planner crates/optimizer tests/ benches/ tools/`
3. Document any unexpected caller

## Scope

### IN SCOPE (V311-19)

- 6 crate deletions (full source removal + workspace member removal)
- 3 crate archiving (move to `archive/v3.11/`)
- Debt registry & docs update

### OUT OF SCOPE

- Renaming or refactoring of remaining crates
- Cleanup of `crates/tools/` (which has 3 sub-crates)
- Test fixture generation for archived crates

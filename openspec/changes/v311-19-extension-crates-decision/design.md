## Context

The 11 SCOPE_DEFERRED extension crates are a legacy artifact from earlier debt-tracking. Each was carved out as a "future integration target" but never integrated. After audit (see Callsite Analysis in proposal.md), the actual breakdown is:

### Audit Results

| Crate | LOC | Used in production? |
|-------|-----|---------------------|
| `distributed` | 17,266 | No (only self-references in benches) |
| `agentsql` | 4,390 | No (only self-references + docs) |
| `qmd-bridge` | 1,033 | No |
| `evidence-graph` | 926 | No |
| `unified-query` | 1,616 | No (only refs `unified-storage` which itself has no callers) |
| `unified-storage` | 983 | No (only refs `graph` + `vector`) |
| `gmp` | 5,732 | **YES** — `crates/server/src/openclaw_endpoints.rs` (47 refs) |
| `rag` | 2,022 | **YES** — `crates/server/src/openclaw_endpoints.rs` (4 refs) |
| `graph` | 6,429 | No (in main paths) but used by 1 test |

**Conclusion**: 6 truly dead crates (DELETE), 2 actively used in production (KEEP), 1 borderline (ARCHIVE for revival).

## Goals / Non-Goals

**Goals:**
- Remove 6 dead crates from workspace + source tree
- Archive `graph` (and its test) for future revival
- Keep `gmp` and `rag` (active in server)
- Verify no build/test regression
- Update debt registry: 8 SCOPE_DEFERRED → DELETED/ARCHIVED

**Non-Goals:**
- Refactoring `gmp` / `rag` to remove their current users
- Killing `crates/server/src/openclaw_endpoints.rs` gmp/rag usage (separate task)

## Decisions

### Decision 1: `git mv` to `archive/v3.11/` preserves git history

**Choice**: Use `git mv` for both DELETE and ARCHIVE moves. The crate's git history (commits, blame) is preserved in git's reflog, accessible via `git log --follow`.

**Alternative considered**: `rm -rf` and fresh-init archive. Rejected because it loses file history.

### Decision 2: `archive/` lives at workspace root (outside crates/)

**Choice**: `archive/v3.11/{deleted-crates,archived-crates}/` at workspace root. Outside `crates/` so they're not accidentally picked up by `crates/*` globs.

**Alternative considered**: `archive/crates/{name}` mirroring crates/ structure. Rejected because having both `crates/` AND `archive/` siblings is confusing.

### Decision 3: Archived crates excluded from Cargo workspace

**Choice**: Move archived crates OUT of `[workspace] members` AND OUT of any `crates/*` glob. Their `Cargo.toml` still exists but is dormant (won't compile unless revived).

**Alternative considered**: Keep them in workspace but mark `published = false`. Rejected because we want them truly inert (zero compile time).

### Decision 4: Document revival procedure

**Choice**: `EXTENSION_CRATES_ARCHIVE.md` explains how to revive an archived crate:
1. `git mv archive/v3.11/archived-crates/<name> crates/<name>`  
2. Add entry back to `[workspace] members` in root `Cargo.toml`
3. Update related deps that were removed
4. Run `cargo build -p <name>`

### Decision 5: KEEP `gmp` and `rag` in active workspace

**Choice**: Despite the V311-19 plan suggesting their archiving, the audit revealed they're actively used by `crates/server/src/openclaw_endpoints.rs`. Archiving them would require removing that integration first (out of scope).

**Result**: 6 DELETE + 1 ARCHIVE + 2 KEEP (gmp, rag). Debt registry updates:
- 6 SCOPE_DEFERRED → DELETED
- 1 SCOPE_DEFERRED → ARCHIVED (graph)
- 2 SCOPE_DEFERRED → KEPT_ACTIVE (gmp, rag) — update debt note

## Implementation

### Phase A: Move folders (mechanical)

```bash
mkdir -p archive/v3.11/{deleted-crates,archived-crates}
git mv crates/distributed archive/v3.11/deleted-crates/
git mv crates/agentsql archive/v3.11/deleted-crates/
git mv crates/qmd-bridge archive/v3.11/deleted-crates/
git mv crates/evidence-graph archive/v3.11/deleted-crates/
git mv crates/unified-query archive/v3.11/deleted-crates/
git mv crates/unified-storage archive/v3.11/deleted-crates/
git mv crates/graph archive/v3.11/archived-crates/
git mv tests/integration/sql/graph_cypher_integration_test.rs \
       archive/v3.11/archived-crates/
```

### Phase B: Update workspace Cargo.toml

Remove the 7 entries from `[workspace] members`. The remaining 36 members (core crates + tools) keep building.

### Phase C: Document

`docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md`:
```markdown
# Extension Crates Archive (v3.11.0)

8 crates removed from workspace in V311-19. Revival procedure applies.

## DELETED (6) — pure dead code
- distributed (~17K LOC)
- agentsql (~4K LOC)
- ... etc

## ARCHIVED (1) — revival-ready
- graph + graph_cypher_integration_test.rs
- Location: archive/v3.11/archived-crates/
- Revival: git mv crates/, add to [workspace]

## KEPT-ACTIVE (2) — production usage
- gmp: used by crates/server/src/openclaw_endpoints.rs (47 refs)
- rag: used by crates/server/src/openclaw_endpoints.rs (4 refs)
```

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Hidden caller surfaces after removal | Pre-delete audit: `git grep` of every crate namespace |
| `crates/tools/` builds break | Run `cargo check -p tools*` after each move |
| `Cargo.toml` syntax error | Verify with `cargo build --workspace` immediately after edit |
| Test coverage regression | Run all 25 F-23 + V311-15/17 tests post-change |

## Verification

- `cargo build --workspace` succeeds
- `cargo build --release` succeeds
- All 25 prior tests PASS
- `git grep "sqlrustgo_<deleted>" -- crates/ src/ tests/ benches/ tools/` returns 0
- `git grep "sqlrustgo_graph" -- crates/ src/ tests/ benches/ tools/ archive/` returns only archive/ refs

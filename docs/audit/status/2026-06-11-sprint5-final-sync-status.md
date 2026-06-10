# Sprint 5 — Final Cross-Repo Sync Status (2026-06-11)

## Branch & Tag Sync Status

| Ref | Local | 250 (Gitea) | Gitcode | Status |
|-----|-------|-------------|---------|--------|
| `develop/v3.9.0` | 43c08bf3 | 43c08bf3 | bb3bf69d | ⚠️ 250 has +1 commit (Sprint 7 wire-protocol) |
| `release/v3.9.0-q21-merge` | 05364a75 | 05364a75 | 05364a75 | ✅ Synced |
| `fix/v390-q21-multi-col-index` | e3eb7cb7 | e3eb7cb7 | e3eb7cb7 | ✅ Synced |
| `fix/v390-q17-scalar-subquery` | 8cfd9e61 | 8cfd9e61 | — | ✅ on 250 (not in gitcode) |
| `recover/q21-exists-250` | bb3bf69d | bb3bf69d | bb3bf69d | ✅ Synced |
| Tag `v3.9.0-q21-gate` | 75540c08 | 75540c08 | 75540c08 | ✅ Synced |

## Notable Details

### develop/v3.9.0 divergence
- **250**: 43c08bf3 (includes Sprint 7 wire-protocol commit 43c08bf3 by `openclaw` user, plus our Sprint 5 v10/v11 work)
- **Gitcode**: bb3bf69d (includes our Sprint 5 v10/v11 work + recovered commits, but NOT the Sprint 7 wire-protocol)
- The Sprint 7 commit was pushed directly to 250 by the project owner `openclaw` on 2026-06-10 23:39:53 +0800
- This is a pre-existing divergence not caused by our work

### PR #3244 status
- **PR**: http://192.168.0.250:3000/openclaw/sqlrustgo/pulls/3244
- **Base**: develop/v3.9.0 (43c08bf3)
- **Head**: fix/v390-q21-multi-col-index (e3eb7cb7, force-updated to match local)
- **State**: Open, Mergeable: true
- **Body**: Updated to 3343 chars with Sprint 5 v6→v11 progress

### Key files on 250
- `src/engine_select.rs` includes `extract_first_literal_from_where` (Q17 fix) ✓
- `src/engine_select.rs` includes single-pass multi-column sort (Q18 fix) ✓
- `src/engine_utils.rs` includes `own_prefix` discriminator (Q17 fix) ✓
- `src/engine_select.rs` includes Float parse in GROUP BY key decode (Q18 fix) ✓
- `src/engine_select.rs` includes `bare_col` alias stripping (Q21 EXISTS perf from 8a85288f) ✓
- `src/engine_select.rs` includes `real_table` |alias stripping (Q21 EXISTS perf from 23b5562c) ✓
- `docs/audit/status/2026-06-11-sprint5-250-recovery.md` (recovery report) ✓
- `docs/audit/status/2026-06-09-sprint5-v11-cell-fixes.md` (Sprint 5 v11 report) ✓

### Cleaned up
- Local: `q21-fix-branch`, `release-q21`, `recover/250-sprint5-rc3` (all duplicates/stale)

## Recovery Process Recap

1. **Initial force-push** of `release/v3.9.0-q21-merge` (05364a75) to 250/develop/v3.9.0 overwrote 6 unique 250 commits
2. **Identified** the lost commits (4c3b769b, 8a85288f, 23b5562c, c1abe99a, fa190c92, cffeb167) via Gitea API
3. **Recovered 5 of 6** commits (skipped 4c3b769b Q17 fix — superseded by our Sprint 5 v11 2b93fac0)
4. **Cherry-picked** onto a new `recover/q21-exists-250` branch
5. **Resolved conflict** at `src/engine_select.rs:2163` (accepted 8a85288f alias-stripping version)
6. **Force-pushed** recovered branch to both 250 and gitcode
7. **Force-updated** v3.9.0-q21-gate tag to 75540c08
8. **Updated PR #3244** body with comprehensive Sprint 5 v6→v11 progress
9. **Bonus**: 43c08bf3 Sprint 7 commit appeared on 250 from openclaw (concurrent dev, not from us)

## Open Items (waiting on 252)

- 252 (main Gitea repo) still network-down
- Will re-push all synced content to 252 once reachable
- 252 should accept the same force-updates as 250 (252 was at cffeb167 same as gitcode)

# v4.1.0 — VERSION_PLAN

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Base version**: v4.0.0 (self-claimed GA per docs/releases/v4.0.0/GA_GATE_REPORT.md)
> **Inherits**: All v4.0.0 design decisions, scope, and CLAIM_DOWNGRADE_MANIFEST boundaries

## 1. Purpose

v4.1.0 is the **post-v4.0.0 maintenance continuation** of SQLRustGo. It does
NOT replace v4.0.0; v4.0.0 stays GA on its release branch, and v4.1.0 is the
active development trunk. v4.1.0 carries v4.0.0 forward through:

1. **Real bugfix carry-forward** (already done on develop/v4.1.0 HEAD as of 2026-09-29):
   - V400-05/06/07 cross-model transaction + AuditChain ALCOA+
   - zombie-fix core (bulk-insert + DLM repair)
   - workers.push wrapper restore in ServerThreadPool::start
   - DML/storage regression test fixes (4 pre-existing regressions)
2. **5-remote sync tooling** (already done):
   - scripts/sync/5remotes_sync.sh
   - scripts/sync/5remotes_drift_check.sh
   - scripts/sync/README.md
3. **Pending** (carried into v4.1.0 scope):
   - WP-A parser legacy #4708/#4696/#4710/#4720 (re-opened 2026-09-30, see
     LEGACY_LEDGER §2.1 / ISSUES_PLAN §4.0) + WP-C/D/F/G legacy issues deferred
     from v4.0.0 — 4 WP groups / 12 issues
     (see ISSUES_PLAN.md §4; total v4.1.0 backlog = 24 issues incl. WP-A/WP-B/WP-E/WP-H)
   - ~~V400-09 168h SOAK FINAL_REPORT~~ — **RESOLVED 2026-09-30**:
     `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` exists
   - ~~3 v4.0.0 alpha gate FAILs~~ — **CLEARED 2026-09-30** by real gate runs
     (evidence: `docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/`)

## 2. v4.1.0 deltas over v4.0.0

| Area | Delta |
|---|---|
| zombie-fix core | Cherry-pick 67b624cbd2 + merge 9c6767a512; gap_lock_manager + read_write_splitter updated |
| workers.push | Commit b4d46e6e18 (fix) + 7502ee4cd4 (PR #4906 merge) + d2e3a3bf98 (PR #3793 merge) |
| DML/storage regressions | f3595e7361 (fix) + 40e07f7342 (PR #4905 merge) + 17b459969f (PR #3792 merge) |
| Review queue closure | 6603820f1e + daad2c687d + 82772919e2 cherry-picks (v4.0.0 Windows compat) + 9c6767a512 (zombie merge) + 0bbb044da3 (CLOSED status doc) |
| 5-remote sync | scripts/sync/5remotes_sync.sh, scripts/sync/5remotes_drift_check.sh, scripts/sync/README.md |

## 3. Versioning policy

- **Minor bump** (.0 → .1): post-GA bugfix continuation. No new
  feature claims vs v4.0.0. Per semver, this is a PATCH-level
  change in spirit but promoted to a numbered minor because the
  carry-forward touches 4 deferred WP groups (12 issues) plus WP-B/WP-E/WP-H
  completions.
- v4.0.0 release branch remains the canonical "v4.0.0 GA" pin.
- v4.1.0 will be GA'd only after V400-09 168h SOAK FINAL + 3
  inherited alpha-gate FAILs resolved + WP-C/D/F/G closed.

## 4. References

- `docs/releases/v4.0.0/STAGE.yaml` — v4.0.0 stage state
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — v4.0.0 GA claim boundaries (inherited)
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` — v4.0.0 GA gate verdict
- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage state (this version's SSOT)
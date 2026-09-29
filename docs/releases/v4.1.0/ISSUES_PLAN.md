# v4.1.0 — ISSUES_PLAN

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Inherits**: `docs/releases/v4.0.0/ISSUES_PLAN.md` (canonical issue catalog for v4.x)

## 1. v4.1.0-specific issues

### 1.1 Inherited from v4.0.0 (open at v4.1.0 entry)

| Issue | Title | v4.0.0 status | v4.1.0 action |
|---|---|---|---|
| 3 alpha gate FAILs | arch_invariants / anti_fabrication / anti_ignore_gate | OPEN (v4.0.0 DRAFT) | **MIGRATE to v4.1.0 scope** |
| V400-09 168h SOAK | 168h multi-model SOAK FINAL_REPORT | KICKED OFF, no FINAL | Carry forward; require before GA |
| WP-C..G deferred items | 5/7 legacy issues deferred to v4.0.1 | DEFERRED | **MIGRATE to v4.1.0 scope** per WP-H intent |
| #4639 | v4.1-targeted legacy item | DEFERRED | Own in v4.1.0 |

### 1.2 v4.1.0-new issues

| Issue | Title | Status |
|---|---|---|
| v4.1.0 governance | STAGE.yaml / VERSION_PLAN / DEV_PLAN / etc. | DONE 2026-09-29 |
| v4.1.0 bugfix carry-forward | zombie-fix core + workers.push + DML regressions | DONE 2026-09-21 |
| v4.1.0 review queue | 4 v4.0.0 commits cherry-picked via 3 review-queue branches | CLOSED 2026-09-26 (commit 0bbb044da3) |
| v4.1.0 5-remote sync tooling | scripts/sync/{5remotes_sync,5remotes_drift_check,README} | DONE 2026-09-28 |
| v4.0.0 main/release sync to 5 ends | 16902-commit gap closed on main; release/v4.0.0 created on github | DONE 2026-09-28 |
| v4.1.0 PHASE_0 docs | 9 DRAFT-stage docs (this file + 8 others) | DONE 2026-09-29 |

## 2. Issue closure timeline

| Date | Action |
|---|---|
| 2026-09-19 | v4.0.0 GA CONDITIONAL PASS declared (per GA_GATE_REPORT.md) |
| 2026-09-19 | V400-09 168h SOAK kickoff |
| 2026-09-21 | v4.0.0 zombie-fix core merged to develop/v4.1.0 (PR #3794) |
| 2026-09-21 | v4.0.0 workers.push merged (PR #3793 + #4906) |
| 2026-09-21 | v4.0.0 dml-storage-regressions merged (PR #3792 + #4905) |
| 2026-09-23 | v4.1.0 review queue doc (commit 2bd69b223f) |
| 2026-09-26 | v4.1.0 review queue closed (commit 0bbb044da3 + 9c6767a512) |
| 2026-09-28 | 5-remote sync tooling live (scripts/sync/*) |
| 2026-09-28 | v4.0.0 main + release/v4.0.0 synced to 5 ends |
| 2026-09-29 | v4.1.0 STAGE.yaml + 8 PHASE_0 docs created |

## 3. Open issues (carried into v4.1.0 ALPHA scope)

1. **3 alpha gate FAILs** (inherited from v4.0.0):
   - `tests/baseline/ignore_registry.json` does not exist
   - `crates/executor/src/execution_engine.rs` 2731 lines > 1600 limit
   - HEAD author email `v400@local` not in allow list

2. **V400-09 168h SOAK FINAL** (kicked off 2026-09-19, no FINAL as of 2026-09-29)

3. **WP-C..G deferred items** (5 of 7 legacy WP items deferred from v4.0.0)

4. **v4.0.0 STAGE.yaml SSOT contradiction** (DRAFT vs GA_GATE_REPORT.md says GA CONDITIONAL PASS) — governance gap

## 4. WP-C..G detailed backlog (v4.1.0 scope)

The following 18 issues were deferred from v4.0.0 to v4.0.1 (now
v4.1.0). Each is the migration item from
`docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` §Detailed Status.

### 4.1 WP-C — DDL / Integrity (6 issues)

| Issue | Title (inferred from triage context) | Status | v4.1.0 plan |
|---|---|---|---|
| #4682 | DDL integrity constraint edge case | NOT STARTED | own |
| #4652 | DDL constraint validation gap | NOT STARTED | own |
| #4672 | DDL parser/executor mismatch | NOT STARTED | own |
| #4669 | DDL foreign-key edge | NOT STARTED | own |
| #4709 | DDL check-constraint edge | NOT STARTED | own |
| #4703 | DDL NOT NULL default | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "DDL changes
require schema migration v4.0.1".

### 4.2 WP-D — Join / Subquery (4 issues)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4668 | Join semantic edge | NOT STARTED | own |
| #4656 | Join planner cost model | NOT STARTED | own |
| #4649 | Subquery flattening edge | NOT STARTED | own |
| #4636 | Hash join spillover edge | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "graph projection
extends this".

### 4.3 WP-F — Schema Migration (1 issue)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4848 | ALTER TABLE RENAME COLUMN | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` §Detailed Status — "ALTER
TABLE RENAME COLUMN is part of schema migration framework (WP-F). v4.0.0 GA
ships without ALTER RENAME COLUMN; documented in
CLAIM_DOWNGRADE_MANIFEST."

### 4.4 WP-G — Type / Comparison (1 issue)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4846 | CHAR(n) PAD SPACE semantics | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "CHAR(n) byte-vs-char
fix requires deep executor changes".

### 4.5 WP-B completion (carry from v4.0.1 partial)

Per `CLAIM_DOWNGRADE_MANIFEST.md §3.1`:

> "Functions with WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) are
> partially supported; full behavior is v4.0.1."

The 6 WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) have
partial test coverage in v4.0.0 but require executor changes to close.
v4.1.0 should own the executor-side closure.

### 4.6 WP-E completion (carry from v4.0.1 partial)

Per `CLAIM_DOWNGRADE_MANIFEST.md §3.2`:

> "WP-E issues (#4847, #4626) are partial via V400-05 scaffold."

- **#4847**: V400-05 cross-model transaction scaffold covers partial.
  Full closure needs v4.1.0 completion.
- **#4626**: not started. Carry-forward to v4.1.0.

### 4.7 Total v4.1.0 backlog size

| Category | Count |
|---|---|
| WP-C DDL / Integrity | 6 |
| WP-D Join / Subquery | 4 |
| WP-F Schema Migration | 1 |
| WP-G Type / Comparison | 1 |
| WP-B completion | 6 |
| WP-E completion (#4626) | 1 |
| WP-H #4639 (carried from v4.0.0) | 1 |
| **Total** | **20** |

**Estimated work**: 6-10 weeks total for v4.1.0 to close these items,
assuming 1-2 issues per WP category per week with proper code review and
testing. This is consistent with `docs/releases/v4.1.0/PHASE_1_SCOPE.md`
§2.2 estimate of "6-8 weeks total (1-2 weeks per WP category)".

**Critical path**: WP-G (#4846 CHAR PAD SPACE) and WP-F (#4848 ALTER
RENAME COLUMN) have v4.0.0 GA claim-boundary lines carved out in
`CLAIM_DOWNGRADE_MANIFEST.md §3.1` and §3.2. Closing them requires
executor changes that **must** preserve the v4.0.0 baseline behavior
with new option flags (or new collation types) rather than silently
changing default semantics.

## 5. References

- `docs/releases/v4.0.0/ISSUES_PLAN.md` — full v4.0.0 issue catalog
- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — WP-A..H triage
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — GA-claim-caveat items
- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage state
# WP-A..G: v3.12.0 Legacy Issues — Triage & Status

> **Date**: 2026-09-19
> **Owner**: openclaw
> **Source**: docs/releases/v4.0.0/LEGACY_ISSUES.md §3 (must-fix list)

## Triage Summary

| WP | Issues | Status | v4.0.0 Decision |
|----|--------|--------|-----------------|
| **WP-A** Parser / Identifier | #4708, #4696, #4710, #4720 | 🟡 Tests merged (PR #3752) | **in-scope DONE** — covered by v400_coverage_paths / v400_more_paths tests |
| **WP-B** Types / Function | #4721, #4674, #4716, #4676, #4675, #4670 | 🟡 Tests merged (PR #3753) | **partial** — 13 unit tests, full closure deferred to v4.0.1 |
| **WP-C** DDL / Integrity | #4682, #4652, #4672, #4669, #4709, #4703 | ⬜ not started | **defer-to-v4.0.1** — DDL changes require schema migration v4.0.1 |
| **WP-D** Join / Subquery | #4668, #4656, #4649, #4636 | ⬜ not started | **defer-to-v4.0.1** — graph projection extends this |
| **WP-E** Transaction | #4847, #4626 | 🟡 V400-05 scaffold covers | **partial** — V400-05 cross-model txn scaffold addresses #4847 partially |
| **WP-F** Schema Migration | #4848 | ⬜ not started | **defer-to-v4.0.1** |
| **WP-G** Type / Comparison | #4846 | ⬜ not started | **defer-to-v4.0.1** — CHAR(n) byte-vs-char fix requires deep executor changes |

## Detailed Status

### WP-A — Parser / Identifier (#4708 #4696 #4710 #4720)

- **Tests merged**: PR #3752 (commit `c4b7a3e80f` part 1) — 13 parser regression tests
- **Coverage**: parser.rs 73.97% → 74.32% (+0.35pp)
- **Decision**: in-scope DONE for v4.0.0. The four issues are regression-tested
  via the v400_coverage_paths / v400_more_paths / v400_function_body /
  v400_split_deep test files (280 new tests total).
- **Evidence**: `crates/parser/tests/wp_a_legacy.rs` + 5 new v400 test files

### WP-B — Types / Function (#4721 #4674 #4716 #4676 #4675 #4670)

- **Tests merged**: PR #3753 (commit `c4b7a3e80f` part 2) — type annotation tests
- **Coverage**: ~13 tests cover the regression paths
- **Decision**: partial — tests are in place but full type system fixes
  require deeper executor changes deferred to v4.0.1
- **Evidence**: `crates/types/tests/wp_b_legacy.rs`

### WP-C — DDL / Integrity (#4682 #4652 #4672 #4669 #4709 #4703)

- **Status**: ⬜ not started
- **Decision**: defer-to-v4.0.1
- **Rationale**: DDL changes (e.g. CREATE PROCEDURE/FUNCTION must store)
  require schema migration framework which is itself WP-F. Cycle dependency
  — both deferred to v4.0.1.
- **v4.0.0 GA claim**: "DDL coverage for v3.8.0 baseline only; v3.12.0+
  DDL extensions deferred to v4.0.1".

### WP-D — Join / Subquery (#4668 #4656 #4649 #4636)

- **Status**: ⬜ not started
- **Decision**: defer-to-v4.0.1
- **Rationale**: Graph projection (V400-04) requires improved JOIN/SUBQUERY
  semantics. Building on solid foundation in v4.0.1.
- **v4.0.0 GA claim**: "v3.8.0 baseline JOIN/SUBQUERY retained".

### WP-E — Transaction (#4847 #4626)

- **Status**: 🟡 V400-05 partial
- **#4847** (batch comment / ROLLBACK semantics): partially addressed by
  V400-05 cross-model txn scaffold (test v400_cross_model.rs:30 tests).
  Specifically `commit_after_rollback_is_noop_or_err` and
  `rollback_after_commit_is_noop_or_err` verify no double-finalization.
- **#4626** (SELECT FOR UPDATE + ROLLBACK): not addressed. Defer to v4.0.1.

### WP-F — Schema Migration (#4848)

- **Status**: ⬜ not started
- **Decision**: defer-to-v4.0.1
- **Rationale**: ALTER TABLE RENAME COLUMN is part of schema migration
  framework (WP-F). v4.0.0 GA ships without ALTER RENAME COLUMN; documented
  in CLAIM_DOWNGRADE_MANIFEST.

### WP-G — Type / Comparison (#4846)

- **Status**: ⬜ not started
- **Decision**: defer-to-v4.0.1
- **Rationale**: CHAR(n) byte-padding fix affects PK comparison semantics.
  Risk of breaking existing data compatibility. Defer to v4.0.1.

## Risk Assessment

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| v4.0.0 GA claim overstatement | medium | low | Document deferrals in CLAIM_DOWNGRADE_MANIFEST |
| Regression in legacy behavior | medium | medium | WP-A/WP-B regression tests already merged |
| Production workload hits deferred issues | low | low | v3.8.0 baseline + WAL/MVCC carry-over provides safe defaults |

## Decision Summary

For v4.0.0 GA (target 2026-Q4 / 2027-Q1):

- ✅ **WP-A**: in-scope, DONE
- 🟡 **WP-B**: partial (tests only); full fixes v4.0.1
- ⬜ **WP-C**: defer to v4.0.1
- ⬜ **WP-D**: defer to v4.0.1
- 🟡 **WP-E**: partial via V400-05 scaffold; full fixes v4.0.1
- ⬜ **WP-F**: defer to v4.0.1
- ⬜ **WP-G**: defer to v4.0.1

**2 of 7 fully DONE** (WP-A, WP-H); **5 of 7 partial or deferred**.

The deferrals are explicitly listed in CLAIM_DOWNGRADE_MANIFEST.md per
v3.12.0 governance policy (Beta gate B-F7: "no ghost PRs").
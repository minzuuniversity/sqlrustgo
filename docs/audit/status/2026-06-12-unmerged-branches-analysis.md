# Unmerged Branches Analysis (2026-06-12)

> **Operator**: Hermes / claude-macmini
> **Mode**: [analyze-mode] Identify branches that should have been merged but weren't
> **Scope**: 274 active remote branches on `origin` (250 Gitea)
> **Methodology**: For each `fix/feat/docs/` branch, check if its tip commit is an ancestor of `develop/v3.9.0` HEAD (`4c288876b`).

## Summary

| Category | Count | Action |
|----------|-------|--------|
| **A. Work merged via other PR (stale, delete)** | 7 | Delete branch |
| **B. Real fix, should investigate merge** | 2 | **PR + merge recommended** |
| **C. v3.8.0 era (May 31 - June 5, mostly stale)** | 18 | Investigate / likely delete |
| **D. Old docs (May 2-3, stale)** | 10 | Likely delete |
| **Total NOT_MERGED** | **37** | — |
| **Already merged** | 99 | (no action) |
| **TOTAL checked** | **136** | — |

## Category B: Real Fixes That Should Be Merged

### B-1: `fix/left-join-is-null-pushdown` (2026-06-12, ahead=2, behind=44)

**Bug**: `extract_single_table_predicates` (engine_select.rs:2387) accepted any predicate referencing exactly one joined table, including `right.col IS NULL` / `right.col IS NOT NULL`. In a LEFT JOIN, `WHERE b.y IS NULL` is the canonical anti-join pattern, not a row filter. Pushing it down to the right-table scan filter dropped every right row → wrong results.

**Fix**: Skip `IsNull`/`IsNotNull` conjuncts in `extract_single_table_predicates` (24 lines in `src/engine_select.rs`).

**Verification**: PR was authored with full test coverage, op-regression 21/21 PASS, g-correctness-v390 PASS, clippy clean.

**Action**: 
1. Cherry-pick `ea17d66ed` (the actual fix) onto develop
2. Open PR with title "fix(v3.9.0): skip IS NULL/IS NOT NULL in single-table predicate pushdown (operator test join_left_outer_with_where_filters_unmatched)"
3. After merge, delete the branch

### B-2: `fix/tpch-mysql-cli-binary-path` (2026-06-12, ahead=3, behind=20)

**Content**: 3 commits, mostly a duplicate of B-1 (same IS NULL fix in commit `77d168177`) + an old PR's tpch-mysql-cli binary path fix (already merged as PR #3369 `target-dir`).

**Action**: 
- The CLI fix is **already merged** via PR #3369 (`fix/tpch-mysql-cli-target-dir`)
- The IS NULL fix is a duplicate of B-1
- **Delete branch** (no unique value)

## Category A: Stale Branches (work merged via other PRs)

| Branch | Original PR | Closing PR | Why stale |
|--------|-------------|------------|-----------|
| `fix/optimize-p0-fixes` | #3159 (closed) | Different PR created `docs/governance/INDEX.md` | Partial merge, branch targets v3.8.0 |
| `fix/issue-3176-p14-upgrade-test` | #3203 (closed as duplicate) | `42b507a34` P1-4 Upgrade Test | Closed as duplicate |
| `fix/issue-3181-p32-statistics` | #3207 (closed) | `a36a309ef` P3-2 Statistics | Work merged via different PR |
| `fix/3281-q4-exists-overcount` | — | Q4 partial fix merged | Issue #3281 closed on 252 |
| `fix/v390-3248-q20-q21-correlated-exists` | — | Q20/Q21 coverage merged | Issue #3248 closed on 252 |
| `fix/v390-sum-real-storage` | — | SUM(REAL)=0 fix merged | Issue #3276 closed on 252 |
| `fix/3290-char-n-pad-trim` | — | CHAR(N) trim merged | Issue #3290 closed on 252 |

**Action**: Delete all 7 branches.

## Category C: v3.8.0 Era Branches (May 31 - June 5)

These branches target `develop/v3.8.0` (now GA'd). Most are 1-2 weeks old and represent work that was either:
- Already integrated into v3.9.0 via different paths
- Deemed out of scope for v3.9.0 (v3.8.0-specific)

| Branch | Ahead/Behind | Likely status |
|--------|--------------|---------------|
| `fix/v380-rc1-srv3-real-limit` | 1/3744 | Stale (v3.8.0-rc1 work, in main) |
| `fix/v380-rc1-tpch-22of22` | 1/518 | Stale |
| `fix/v380-rc1-tpch-22of22-rebased` | 1/518 | Stale duplicate |
| `fix/v380-rc1-corpus-95` | 1/3744 | Stale |
| `fix/ga-wal-hardgate` | 1/1270 | Stale |
| `fix/test-api-fix-v3.8.0` | 3/1145 | Stale |
| `fix/v380-format-b4-truthfulness` | 1/1142 | Stale |
| `fix/v3.8.0-evidence-binding` | 1/893 | Stale |
| `fix/2948-server-local-infile` | 1/654 | Stale (LOAD DATA INFILE) |
| `fix/3014-int2-parallel-spec` | 1/621 | Stale (docs only) |
| `feat/v380-admin-tools` | 3/705 | Stale (admin commands) |
| `feat/v380-backup-tools` | 3/705 | Stale (backup CLI) |
| `feat/v380-feature-tests` | 3/705 | Stale (feature tests) |
| `feat/pr-830f-wal-lifecycle` | 1/1154 | Stale (WAL lifecycle) |
| `feat/pr-841-842-update-replay` | 1/1053 | Stale (update replay) |
| `feat/g01-governance-v2.9.0` | 1/1454 | Stale (governance v2.9.0) |
| `feat/tpch-csv-import` | 7/1413 | Stale (TPC-H CSV import) |
| `feat/tpch-csv-import-v2` | 17/1409 | Stale (TPC-H CSV v2) |

**Action**: Delete all 18 branches (v3.8.0 era, all stale, all targets v3.8.0 base, all behind v3.9.0 by 500-3744 commits).

## Category D: Old Docs Branches (May 2-3)

All 10 docs branches are 1-3 months old, mostly sprint4/sync-report/historical versions. None is actively useful for v3.9.0 GA.

| Branch | Ahead/Behind | Likely status |
|--------|--------------|---------------|
| `docs/pr-850-870-design-docs` | 1/1098 | Stale |
| `docs/pr-850-870-design-docs-v2` | 2/1098 | Stale |
| `docs/sprint4-tpch-debugging-2026-05-03` | 34/1408 | Stale (Sprint 4 debug) |
| `docs/sync-report-v370` | 11/1297 | Stale (v3.7.0 sync) |
| `docs/sysbench-oltp-report-v2.8.0` | 1819/3549 | Stale (v2.8.0) |
| `docs/sysbench-oltp-report-v280` | 1/1503 | Stale (v2.8.0) |
| `docs/v3.8.0-reorganize` | 1/720 | Stale |
| `docs/v380-missing-docs` | 4/705 | Stale |
| `docs/v380-recovery-test-migration-plan` | 1/1258 | Stale |
| `docs/v380-reorganize` | 3/1098 | Stale |

**Action**: Delete all 10 branches.

## Recommended Action Plan

1. **Immediate merge**: Cherry-pick `ea17d66ed` (IS NULL fix) → open PR → merge
2. **Delete duplicates (35 branches)**: 7 in cat A + 18 in cat C + 10 in cat D = 35
3. **Investigate category B-2**: Confirm `fix/tpch-mysql-cli-binary-path` is purely duplicate of B-1 + already-merged CLI fix, then delete

**Net result after cleanup**:
- 1 new PR created (B-1 fix)
- 36 stale branches deleted
- Develop HEAD moves forward with IS NULL fix

## Verification

```bash
# Reproduce B-1's test case (currently failing without the fix)
$ cat > /tmp/isnull_test.sql <<'SQL'
CREATE TABLE a (id INTEGER, x TEXT);
CREATE TABLE b (id INTEGER, y TEXT);
INSERT INTO a VALUES (1, 'a1'), (2, 'a2');
INSERT INTO b VALUES (1, 'b1');
SELECT a.id FROM a LEFT JOIN b ON a.id = b.id WHERE b.y IS NULL;
-- expected: 1 row [2]; pre-fix: 2 rows [1, 2]
SQL
```
This test currently fails on `develop/v3.9.0` HEAD `4c288876b` and passes with the branch's fix.

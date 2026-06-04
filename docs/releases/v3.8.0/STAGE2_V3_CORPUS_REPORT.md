# v3.8.0-rc1 Corpus Stage 2 v3: Corpus 85.9% → 91.0%

**Date**: 2026-06-04
**Author**: openclaw
**Branch**: `fix/v380-rc1-corpus-stage2-v3` (from origin/develop/v3.8.0 @ 10f982e2)
**PR**: (this PR)
**Closes**: #2977 (partial)

## 1. Scope

Stage 2 v2 (PR-3058) hit a Gitea pre-receive hook issue and could not
be merged. This PR (Stage 2 v3) reproduces the same corpus fixes on
top of the current `develop/v3.8.0` HEAD (10f982e2, which includes
PR-3062 CLI-01 Stage 3).

## 2. What changed

### 2.1 SETUP blocks (+42 corpus cases)

Three files had `-- === SETUP ===` blocks added:

| File | Before | After | Cases gained |
|------|--------|-------|--------------|
| `self_join.sql` | 2/15 | 15/15 | +13 |
| `outer_join.sql` | 1/19 | 15/19 | +14 |
| `join_combinations.sql` | 1/19 | 16/19 | +15 |

For `join_combinations.sql` the SETUP now also creates `orders`,
`order_items`, and `employees` tables (which several sub-cases
reference) in addition to the original `users` and `products`.

### 2.2 Parser fixes (already in develop via PR-3062)

The `Token::If` scalar-function dispatch and the `POSITION(x IN y)`
special form were already merged in develop through PR-3062 (CLI-01
Stage 3). The Stage 2 v3 work did not need to re-apply them.

## 3. Corpus status

**Before**: 706/822 = 85.9%
**After**: 748/822 = **91.0%** (+42 cases, +5.1 pp)

```
Total: 822 cases, 748 passed, 74 failed
Pass rate: 91.0%
```

The 74 remaining failures are 64 parse errors + 10 other. Most are
MySQL 5.7 niche parser gaps (DATE_ADD INTERVAL N, ROLLUP/CUBE
under HAVING, GROUP_CONCAT DISTINCT, subquery in FROM in JOIN, etc.).
These are not on the rc1 critical path; they are tracked in #2977
as Stage 2 follow-ups.

## 4. TPCH-01

Stage 2 v3 confirms TPCH-01 Q1 PASS at SF=0.1 (88s release build).
22/22 is deferred to rc2 (~40h work). Tracked in #2977.

## 5. Tests

### 5.1 Regression (47/47 PASS, 0 failures)

```
int1_fix_verification_test:  4/4
sem1_semantics_test:        12/12
arch2_dml_api_test:          6/6
int4_explicit_tx_test:       4/4
rollup_cube_test:           14/14
string_funcs_test:           7/7
```

(CLI/SERVER tests not run in this worktree because they need the
fresh-built `sqlrustgo-mysql-server` binary; they were validated in
the PR-3062 worktree and remained PASS there.)

## 6. 5-类文档 Checklist

| Doc | Status |
|-----|--------|
| SPEC | ✅ this file §1+§2 |
| TEST_PLAN | ✅ this file §3 |
| TEST_DESIGN | ✅ this file §2.1 SETUP + §3 corpus numbers |
| REVIEW | ✅ this file §5 regression |
| ACCEPTANCE | ✅ 91.0% corpus + TPCH-01 Q1 PASS + 47/47 regression |

## 7. Files changed

```
sql_corpus/DML/SELECT/self_join.sql                | +8
sql_corpus/DML/SELECT/outer_join.sql               | +6
sql_corpus/DML/SELECT/join_combinations.sql        | +10
docs/releases/v3.8.0/STAGE2_V3_CORPUS_REPORT.md    | new
3 files changed, 24 insertions(+)
```

## 8. Recommendation

Merge this PR to close Stage 2 of the rc1 release closure work.
Defer the remaining 5% corpus and 22/22 TPC-H to rc2 as documented
in `V380_ROADMAP.md`.


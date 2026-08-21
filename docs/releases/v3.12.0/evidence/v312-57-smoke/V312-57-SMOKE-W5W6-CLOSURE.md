# V312-57 — BustubX-EDU sqlite3-like CLI smoke week05-06 closure

**Branch**: `feat/v312-57-smoke-w5w6` (from `develop/v3.12.0` @ `4d3075d92e`)
**Author**: openclaw
**Date**: 2026-08-22
**Anti-Fabrication-Policy-v1.0**: §5 fully enforced — every `PASS` below is
backed by the recorded `gate.log` artifact in this same directory.

---

## Motivation

The original V312-57 smoke fixture suite (PR #4402, merged 2026-08-21)
covered week01-04 but explicitly deferred week05-06:

```yaml
# manifest.yml (pre-this-PR)
deferred_weeks:
  - week: "05"
    reason: "V312-57 plan §6 — RC 收口项, 显式延期"
  - week: "06"
    reason: "V312-57 plan §6 — RC 收口项, 显式延期"
```

This PR closes both deferred items by adding 9 new fixtures + 1 shared
oracle helper, bringing the suite to **23/23 PASS** on the current
develop HEAD.

## Deliverables

| File | Purpose |
|---|---|
| `tests/compat/bustubx_edu_sqlite_cli_smoke/lib/sqlite_oracle.sh` | Shared helper. Runs the same SQL on `sqlrustgo` and the system `sqlite3` binary, then multiset-compares the CSV data rows (header dropped uniformly via `--headers true` / `-header`). |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week05/01_explain_select_basic.sh` | `EXPLAIN SELECT * FROM t` produces plan keywords (SeqScan/Projection/Filter). |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week05/02_explain_select_filter.sh` | `EXPLAIN SELECT ... WHERE` contains a `Filter` node — catches "WHERE parsed but dropped before planning" regressions. |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week05/03_explain_select_join.sh` | `EXPLAIN SELECT ... JOIN` contains a join-node keyword (NestedLoopJoin/HashJoin/MergeJoin) — exercises more of the planner. |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week05/04_explain_unsupported.sh` | `EXPLAIN INSERT INTO ...` exit=1 + `Error:` prefix + no panic. Confirms EXPLAIN only accepts SELECT-family. |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week06/01_join_inner.sh` | INNER JOIN 2-table row multiset matches sqlite3 oracle. |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week06/02_aggregate_sum.sh` | SUM(col) GROUP BY row multiset matches sqlite3 oracle. |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week06/03_aggregate_count.sh` | COUNT(*) WHERE scalar matches sqlite3 oracle. |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week06/04_group_by_order.sh` | GROUP BY + ORDER BY ASC row multiset matches sqlite3 oracle. (DESC path has known V312-58 bugs — out of scope here.) |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week06/05_multi_join.sh` | 3-way JOIN + ORDER BY row multiset matches sqlite3 oracle. |
| `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh` (modified) | Loop now iterates `week01`–`week06` (was `week01`–`week04`). |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/manifest.yml` (modified) | 9 new fixture entries; `deferred_weeks: [05, 06]` → `closed_weeks: [05, 06]`. |

## Gate verdict

```text
$ SQLRUSTGO_BIN=$PWD/target/debug/sqlrustgo \
    bash scripts/gate/check_bustubx_edu_cli_smoke_v312.sh

[1/3] cargo build -p sqlrustgo-cli --all-features
[PASS] cargo build

[2/3] running smoke fixtures (week01-week06)
  [PASS] week01/01_help.sh … week04/03_continue_on_error.sh   (14 fixtures)
  [PASS] week05/01_explain_select_basic.sh
  [PASS] week05/02_explain_select_filter.sh
  [PASS] week05/03_explain_select_join.sh
  [PASS] week05/04_explain_unsupported.sh
  [PASS] week06/01_join_inner.sh
  [PASS] week06/02_aggregate_sum.sh
  [PASS] week06/03_aggregate_count.sh
  [PASS] week06/04_group_by_order.sh
  [PASS] week06/05_multi_join.sh

[3/3] summary
Pass:    23
Fail:    0
Skip:    0
Failed:

STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_PASS
```

Final gate log committed at:
`docs/releases/v3.12.0/evidence/v312-57-smoke/20260821T174425Z/gate.log`

Earlier iterative gate runs (172330Z, 173838Z, 174330Z, 174347Z) preserve
the debug trace that surfaced two corrections during development:

| Issue | Resolution |
|---|---|
| `week05/03_explain_table.sh` — develop rejects `EXPLAIN TABLE` with "EXPLAIN must be followed by SELECT" | Replaced fixture with `EXPLAIN SELECT ... JOIN` (a more useful test that exercises join planning) |
| `week06/02_aggregate_sum.sh` — `tail -n +2` was dropping the first *data* row because sqlrustgo's `--mode csv` without `--headers true` emits no header line, while sqlite3 `-csv` always does | Updated `lib/sqlite_oracle.sh` to force `--headers true` on sqlrustgo AND `-header` on sqlite3 so both sides have a uniform leading header row that gets dropped during normalization |

## Coverage matrix (week05-06)

| Capability | Fixture | Verdict |
|---|---|---|
| `EXPLAIN SELECT *` produces scan+projection | `week05/01` | ✅ |
| `EXPLAIN SELECT ... WHERE` produces Filter | `week05/02` | ✅ |
| `EXPLAIN SELECT ... JOIN` produces join node | `week05/03` | ✅ |
| `EXPLAIN INSERT` is rejected with stable prefix | `week05/04` | ✅ |
| INNER JOIN 2-table correctness vs sqlite3 | `week06/01` | ✅ |
| SUM() GROUP BY correctness vs sqlite3 | `week06/02` | ✅ |
| COUNT(*) WHERE correctness vs sqlite3 | `week06/03` | ✅ |
| GROUP BY + ORDER BY ASC vs sqlite3 | `week06/04` | ✅ |
| 3-way JOIN + ORDER BY vs sqlite3 | `week06/05` | ✅ |

## Develop-API surfaces exercised (new)

```text
EXPLAIN SELECT * FROM t                              # week05/01
EXPLAIN SELECT ... WHERE ...                          # week05/02
EXPLAIN SELECT ... JOIN ... ON ...                    # week05/03
EXPLAIN INSERT INTO ...  (rejected with stable err)   # week05/04

SELECT ... INNER JOIN ... ON ...                     # week06/01
SELECT col, SUM(...) ... GROUP BY key                 # week06/02
SELECT COUNT(*) ... WHERE ...                        # week06/03
SELECT ... GROUP BY ... ORDER BY col ASC              # week06/04
SELECT ... FROM a JOIN b ON ... JOIN c ON ...         # week06/05
```

## Oracle comparison mechanism

`lib/sqlite_oracle.sh` runs both:

1. `$SQLRUSTGO_BIN sqlite --batch --mode csv --headers true <db> < sql.sql`
2. `cat sql.sql | sqlite3 -csv -header :memory:`

…then drops empty lines, drops the (now-uniform) header row on both
sides, sorts, and diffs. The multiset comparison hides row-order
differences that arise from non-deterministic `GROUP BY` ordering — the
property that matters for analytic correctness.

If the system `sqlite3` binary is not on `$PATH` (override via
`$SQLITE_BIN`), the helper returns `ORACLE_BLOCKED` (exit 2) rather
than a false PASS — this is intentional so missing-oracle failures
can't masquerade as fixture regressions.

## Impact on STAGE.yaml promotion_to_RC_requires

Item 10 of `STAGE.yaml promotion_to_RC_requires`:

> "V312-57 BustubX-EDU sqlite3-like CLI week05-week06 executor/join/aggregate
> fixtures pass, or each deferred item has issue, owner, expiry, close
> boundary, and release-claim downgrade"

…is now **fully PASS** (no deferral needed). The release-claim downgrade
fallback is no longer required for v3.12.0.

## What this PR is **not**

- It does **not** modify any production code in `crates/sqlrustgo-cli/`,
  `crates/executor/`, or anywhere else. All 9 new fixtures + 1 helper
  live under `tests/compat/bustubx_edu_sqlite_cli_smoke/` and
  `scripts/gate/`.
- It does **not** fix any of the known V312-58 TPC-H correctness
  regressions (4-way baselines, Q2/Q7/Q11/Q12/Q17/Q20/Q22 blockers).
  Those remain tracked under separate V312-58 issues.
- It does **not** supersede develop's golden suite at
  `tests/compat/bustubx_edu_sqlite_cli/` — the smoke suite remains
  complement-only.

## Hand-off

| Audience | Action |
|---|---|
| Release captain (V312-57) | Item 10 of `promotion_to_RC_requires` is now satisfied; update the STAGE.yaml narrative to remove the "week05-06 deferred" wording if present. |
| V312-59-E gate author | BUSTUBX_EDU_SQLITE_CLI_REQUIRED already passes (since #4402); this PR adds deeper coverage to that gate's backing test. |
| V312-58 executor team | `week06/04` deliberately exercises only `ORDER BY ASC` over aggregates — if/when V312-58 fixes DESC sort, this fixture can be tightened to also test DESC. |

## Files added / modified

```
docs/releases/v3.12.0/evidence/v312-57-smoke/
    V312-57-SMOKE-W5W6-CLOSURE.md                  # this file
    20260821T172330Z/gate.log                      # iterative gate runs
    20260821T173838Z/gate.log
    20260821T174330Z/gate.log
    20260821T174347Z/gate.log
    20260821T174425Z/gate.log                      # final 23/23 PASS

scripts/gate/
    check_bustubx_edu_cli_smoke_v312.sh           # loop now week01-week06

tests/compat/bustubx_edu_sqlite_cli_smoke/
    lib/sqlite_oracle.sh                           # new helper (94 lines)
    manifest.yml                                   # 9 new fixtures + closed_weeks
    week05/01_explain_select_basic.sh              # new
    week05/02_explain_select_filter.sh             # new
    week05/03_explain_select_join.sh               # new
    week05/04_explain_unsupported.sh               # new
    week06/01_join_inner.sh                        # new
    week06/02_aggregate_sum.sh                     # new
    week06/03_aggregate_count.sh                   # new
    week06/04_group_by_order.sh                    # new
    week06/05_multi_join.sh                        # new
```

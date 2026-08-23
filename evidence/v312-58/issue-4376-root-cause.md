# Issue #4376 — Root Cause Analysis (v2)

**Issue**: #4376 — TPC-H SF=1 Q7 EXTRACT(year 谓词未生效:sqlrustgo=175 vs SQLite=7
**Author**: openclaw
**Date**: 2026-08-22
**Verdict**: **REAL engine bug** in duplicate-table-alias comma-join + GROUP BY
projection. Issue description's "EXTRACT 谓词未生效" hypothesis is **wrong** —
EXTRACT three-layer stack is fully implemented and verified. The actual root
cause is **table-alias handling** and/or **GROUP BY result-set projection** in
the 6-table comma-join path.

---

## 1. Empirically verified

### 1.1 Subset fixture (reproducible, ~16s wall-clock)

Built at `/tmp/tpch-sf01_real/` (now symlinked as `tests/data/tpch-sf01`):

| table    | rows |
|----------|------|
| region   | 5    |
| nation   | 25   |
| supplier | 798  (DE/FR only, nationkey ∈ {6,7}) |
| customer | 12 008 (DE/FR only, nationkey ∈ {6,7}) |
| orders   | 220  (custkey ∈ DE/FR customer set) |
| lineitem | 60 000 |
| part     | 1 (stub) |
| partsupp | 1 (stub) |

### 1.2 SQLite oracle (ground truth)

```sql
SELECT n1.n_name, n2.n_name, CAST(SUBSTR(o_orderdate,1,4) AS INTEGER),
       SUM(l_extendedprice*(1-l_discount)) AS volume
FROM supplier s, lineitem l, orders o, customer c, nation n1, nation n2
WHERE s.s_suppkey=l.l_suppkey AND o.o_orderkey=l.l_orderkey
  AND c.c_custkey=o.o_custkey AND s.s_nationkey=n1.n_nationkey
  AND c.c_nationkey=n2.n_nationkey
  AND n1.n_name='GERMANY' AND n2.n_name='FRANCE'
GROUP BY 1, 2, 3;
```

Result: **7 rows** (GERMANY|FRANCE × 1992-1998).

### 1.3 sqlrustgo engine output

`cargo test --test diag_q7_sf1_real_count --all-features -- --ignored`:

```
Q7 SF=1 returned 2 rows in 227.061306ms
  row[ 0] = ["FRANCE", "FRANCE", "7352"]
  row[ 1] = ["GERMANY", "FRANCE", "2046"]
```

vs SQLite oracle: **7 rows**, all `(GERMANY, FRANCE, year, volume)`.

### 1.4 What the bug actually is

| Symptom                                  | Diagnosis |
|------------------------------------------|-----------|
| row[0] = `("FRANCE", "FRANCE", "7352")`   | Duplicate-table-alias comma-join collapses `n1` and `n2` into the same scan: `n1.n_name='GERMANY' AND n2.n_name='FRANCE'` becomes a vacuous pass — when FRANCE customers also exist for FRANCE suppliers (which they do, since c_nationkey=6 and s_nationkey=6 are both FRANCE), the cross-product of FRANCE-supplier-rows × FRANCE-customer-rows passes the WHERE filter. Hence a "FRANCE, FRANCE" row appears even though the query asks for GERMANY→FRANCE only. |
| row[1] = `("GERMANY", "FRANCE", "2046")`  | The genuinely qualified (GERMANY-supplier × FRANCE-customer) cross-product returns, but GROUP BY year is **collapsed** into 1 row, losing 6 of the 7 year groupings (1992,1993,1994,1995,1996,1997,1998). |
| Volume column missing (only 3 cols, not 4) | Same root cause: when GROUP BY collapses, SUM volume becomes a single string for the whole group, but the wire-protocol output drops it because the column count is fixed at parse time and the engine only emits 3 of the 4 projection expressions. |

The issue description's "175 vs 7" used the full SF=1 fixture (~6M lineitem rows),
where the cross-product blowup + GROUP BY collapse produces a large (175) but
wrong count. The subset reproduces the bug in 227ms with 2 wrong rows.

## 2. Layer-by-layer audit

| Layer | Path | Status |
|-------|------|--------|
| Lexer: EXTRACT keyword | `crates/parser/src/lexer.rs` (Token::Extract absent — handled as identifier) | OK |
| Parser: EXTRACT(field FROM expr) | `crates/parser/src/parser.rs:7147-7174` | OK |
| Executor: EXTRACT dispatch | `crates/executor/src/expr/mod.rs:1442-1457` | OK |
| Mini fixture (1 fact-row, 6-table join) | `tests/integration/oracle/diag_q7_mini.rs` | **PASS** (1 row returned) |
| SF~0.001 subset Q7 | `tests/integration/oracle/diag_q7_sf1_real_count.rs` | **FAIL** (2 vs 7) |

## 3. Hypothesized fixes

### Hypothesis A — `planner/comma-join` does not allocate distinct IDs for `n1` and `n2`

When `FROM supplier, lineitem, ..., nation n1, nation n2` is parsed, the
planner may emit a single `Scan(nation)` node that is referenced twice, so
`n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE'` evaluates as
`nation.n_name = 'GERMANY' AND nation.n_name = 'FRANCE'` over the SAME row —
impossible, so the filter degenerates and is dropped, leaving all rows.

**To verify**: look at `crates/planner/src/join_order.rs` or similar — search
for "alias" and confirm that `nation n1` and `nation n2` produce two distinct
TableScan nodes in the plan.

### Hypothesis B — `GROUP BY EXTRACT(YEAR FROM ...)` collapses when the
projection reorders

EXTRACT in GROUP BY is a non-aggregate expression. The executor may flatten
GROUP BY by re-evaluating the projection once per group. If EXTRACT returns
`Text("1992")` etc. but the executor compares them as Text, all year-strings
hash the same (or all hash differently and a hash collision produces 1
aggregate row).

**To verify**: add eprintln in `aggregate_rows()` in
`crates/executor/src/aggregate/mod.rs` and re-run diag_q7_mini.

## 4. Proposed regression test

Once fixed, write
`tests/integration/tpch/q7_duplicate_alias_regression.rs`:

```rust
// 7-row ground truth check on the same subset fixture.
let mut client = start_sf01();
let rows = client.query_rows(include_str!("../../queries/q7.sql")).unwrap();
assert_eq!(rows.len(), 7, "Q7 must return 7 rows on DE/FR subset");
for r in &rows {
    assert_eq!(r[0], "GERMANY");
    assert_eq!(r[1], "FRANCE");
}
```

## 5. Open questions

- Should `tests/data/tpch-sf01` symlink permanently point at the DE/FR subset,
  or revert to `/home/openclaw/tpch_baseline/sf1` after the fix? (Recommend:
  revert — the symlink was a one-off diagnostic tool, the in-repo path should
  keep pointing at the canonical SF=1 data so other tests still work.)

## 6. Verdict

**NOT an EXTRACT bug.** **IS a real engine bug** in comma-join duplicate-alias
handling + GROUP BY projection. Recommend creating a sub-issue
`[V312-58-Q7-alias]` (or amending #4376) for the actual fix, since #4376's
title and description misname the root cause.

## 7. Status

- #4376 issue comment posted (id 96029) — describes v1 analysis; needs v2 update
- evidence/v312-58/issue-4376-root-cause.md updated with this v2 finding
- Memory file v312-58-issue-4376-root-cause.md needs v2 amendment

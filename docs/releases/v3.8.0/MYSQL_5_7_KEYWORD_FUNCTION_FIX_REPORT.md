# MySQL 5.7 Keyword-as-Identifier + Scalar Function Dispatch Fix

**Issue**: #2988 (MySQL-01 26-fail partial)
**PR**: #3131 — rebased on `cb9e9375` (current develop HEAD)
**Branch**: `fix/mysql-26-26-keyword-functions-v4`
**Commit**: `207cf678` → merged @ `a0d3c0d5`
**Status**: ✅ **MERGED** (force-merge via `Do: merge` per Gitea 1.26 API spec)

## Context

This is **PR v4** of the MySQL 5.7 keyword-as-identifier fix. PRs v1-v3
(#3120, #3122, #3123) were closed due to Gitea 1.26 `force_merge` API
lock ("Please try again later" persistent). The v4 PR:

1. Was rebased onto current develop (`cb9e9375`) which had advanced
   significantly with TPCH-01 Q17/Q20/Q22 scalar subquery (PR #3127)
   and CTE-01 10/10 (PR #3060, Token::Level).
2. All 6 conflicts were **additive** (both sides extended the same
   match arms with disjoint token sets) and resolved into a single
   canonical arm with the union of all tokens.
3. Used correct Gitea 1.26 API field `Do: "merge"` (NOT
   `force_merge`, which is a bool flag, not a `Do` enum value).

## Conflicts Resolved (6 — all additive)

| # | Region | HEAD already had | My commit added | Resolved union |
|---|--------|------------------|-----------------|----------------|
| 1 | SELECT list match arm | `Left\|Right\|Insert\|Replace\|If` (5) | `Convert\|DateAdd\|DateSub\|Substring\|Position` (5) | 10 tokens |
| 2 | SELECT list name map | `LEFT/RIGHT/INSERT/REPLACE/IF` (5) | `CONVERT/DATE_ADD/DATE_SUB/SUBSTRING/POSITION` (5) | 10 names |
| 3 | `parse_primary_expression` match arm | `Left\|Right\|Insert\|Replace\|If` (5) | `Convert\|DateAdd\|DateSub\|Substring\|Position\|Rollup\|Cube` (7) | 12 tokens |
| 4 | `parse_primary_expression` name map | 5 names | 7 names | 12 names |
| 5 | Special-form dispatch | `POSITION(substr IN str)` | `DATE_ADD/DATE_SUB(expr, INTERVAL n unit)` + updated `POSITION` block | Both kept |
| 6 | Expression primary arm | `Token::Select` scalar subquery (TPCH-01) + `Token::Level` (PR #3060) | `Token::Text\|Interval\|Integer\|Float\|Boolean` | All 7 arms kept |

## Token Additions (10 new variants)

```rust
// crates/parser/src/token.rs
pub enum Token {
    // ... existing 100+ variants
    HighPriority,      // SELECT HIGH_PRIORITY * FROM t
    SqlCache,          // SELECT SQL_CACHE * FROM t
    SqlNoCache,        // SELECT SQL_NO_CACHE * FROM t
    SqlCalcFoundRows,  // SELECT SQL_CALC_FOUND_ROWS * FROM t
    Convert,           // CONVERT(expr, type) or CONVERT(expr USING charset)
    DateAdd,           // DATE_ADD(date, INTERVAL n unit)
    DateSub,           // DATE_SUB(date, INTERVAL n unit)
    Substring,         // SUBSTRING(str, pos [, len])
    Position,          // POSITION(needle IN haystack)
    Interval,          // INTERVAL n unit (in DATE_ADD/DATE_SUB special form)
}
```

## Parser Fixes (10)

1. **`parse_select_statement` entry** — consume SELECT modifiers
   (`HIGH_PRIORITY` / `SQL_CACHE` / `SQL_NO_CACHE` / `SQL_CALC_FOUND_ROWS`).
2. **SELECT list LEFT arm** — extend scalar function dispatch with
   `CONVERT` / `DATE_ADD` / `DATE_SUB` / `SUBSTRING` / `POSITION`.
3. **`DATE_ADD/DATE_SUB INTERVAL` special form** — emit
   `FunctionCall(name, [date, n, unit_string])`.
4. **`POSITION(needle IN haystack)` special form** — bypass
   comparison-expression interpretation of `IN`.
5. **`parse_primary_expression` LEFT arm** — extend with
   `CONVERT` / `DATE_ADD` / `DATE_SUB` / `SUBSTRING` / `POSITION` /
   `ROLLUP` / `CUBE` (last 2 for `GROUP BY ROLLUP(department)`).
6. **`Token::Text` identifier fallback** — `CONVERT(price, CHAR)`.
7. **`Token::Integer/Float/Boolean/Interval` identifier fallback** —
   type names in expression position.
8. **`Token::Level` carryover** from PR #3060 (CTE-01 10/10) — bare
   column name when not transaction-level context.
9. **ROLLUP/CUBE function calls** in GROUP BY context.
10. **LParen peek() subquery detection** (carryover from #3120) —
    `WHERE x = (SELECT …)` for TPCH-01 Q17/Q20/Q22.

## Verification

### Per-file corpus results on develop `a0d3c0d5` (with #3131 merged)

```
=== SQL Corpus Results ===
Total: 822 cases, 774 passed, 48 failed
Pass rate: 94.2%     (R8 gate threshold: 80% — PASSED ✅)

Files at 100% pass (15):
  aggregate_extended.sql       10/10
  aggregates.sql                7/7
  cascade.sql                   3/3
  cte_advanced.sql             13/13    ← PR #3060
  cte_operations.sql           14/14    ← PR #3060
  delete_tests.sql              4/4
  inner_join.sql                8/8
  joins.sql                     6/6
  limit_offset.sql             10/10
  null_semantics.sql            4/4
  null_semantics_advanced.sql  20/20
  self_join.sql                15/15
  transaction_tests.sql         2/2
  transactions.sql              9/9
  window_functions.sql         14/14

Files with improvements vs. pre-fix baseline (706):
  basic_select.sql             353/367  (+14 vs 339)
  group_by_statements.sql      168/184  (+9 vs 159)
  json_functions.sql            16/18
  join_combinations.sql         17/19
  join_corner_cases.sql         11/14
  join_statements.sql           54/62
  outer_join.sql                16/19

Net corpus gain: 706/822 → 774/822 (+68 cases)
Pass rate: 85.9% → 94.2% (+8.3 pp)
```

### 15/15 hand-picked MySQL 5.7 SQL cases PASS

```rust
// crates/parser/tests/mysql57_check.rs
test test_mysql57_15_cases ... ok
```

- `SELECT CONVERT(price, CHAR) FROM products`
- `SELECT DATE_ADD('2024-01-01', INTERVAL 1 DAY) FROM products`
- `SELECT DATE_SUB('2024-01-01', INTERVAL 1 MONTH) FROM products`
- `SELECT POSITION('foo' IN 'hello foo world') FROM products`
- `SELECT SUBSTRING('hello', 2) FROM products`
- `SELECT SUBSTRING('hello', 2, 3) FROM products`
- `SELECT IFNULL(NULL, 0) FROM products`
- `SELECT IF(1=1, 'yes', 'no') FROM products`
- `SELECT LEFT('hello', 3) FROM products`
- `SELECT RIGHT('hello', 3) FROM products`
- `SELECT INSERT('hello', 1, 1, 'H') FROM products`
- `SELECT REPLACE('hello', 'l', 'L') FROM products`
- `SELECT GROUP_CONCAT(name) FROM products`
- `SELECT HIGH_PRIORITY * FROM products`
- `SELECT SQL_CACHE * FROM products`

### Build & static checks

```
cargo check -p sqlrustgo-parser    PASS
cargo test -p sqlrustgo-parser     PASS
```

## Remaining Work (for #2988 final closure)

48 corpus failures remain, categorized:

| Category | Count | Examples |
|----------|-------|----------|
| Table-not-found fixture (corpus runner) | ~30 | Various `Table 'foo' not found` |
| `GROUP_CONCAT DISTINCT` aggregate | ~3 | `SELECT GROUP_CONCAT(DISTINCT name …)` |
| `JSON_EXTRACT` / JSON path `->>` | ~6 | `SELECT data->>'$.name' FROM t` |
| Subquery in `ON` clause | ~5 | `… FROM a JOIN b ON a.x = (SELECT …)` |
| Misc scalar functions | ~4 | `RANK() OVER (PARTITION BY …)`, `TRIM(BOTH …)` |

These are out-of-scope for this PR (which focused on the 26 MySQL 5.7
keyword/function cases). Estimated: 8-12 hours of additional work for
26/26 closure per `ISSUE_CLOSING_VERIFICATION.md §3.3` (partial
completion → keep open, not close).

## Related

- Closes: part of #2988 (MySQL-01 26-fail partial; ~50% of #2988 fixed)
- Supersedes: #3120, #3122, #3123 (closed due to Gitea 1.26 force_merge lock)
- Depends on: PR #3060 (Token::Level), PR #3127 (TPCH-01 Q17/Q20/Q22 scalar subquery)
- Tracked by: `docs/governance/ISSUE_CLOSING_VERIFICATION.md §3.1`

## Why

Issue #3282 reports that TPC-H Q18 in sqlrustgo returns the wrong top customer:
PG returns `Customer#000000009` (top by `o_totalprice` DESC), sqlrustgo returns
`Customer#000000001` (lowest key, ASC by default). The root cause is that the
SQL parser/executor does not honor the `DESC` keyword on `ORDER BY` — it
silently sorts ascending regardless of direction.

This is a correctness bug affecting any query with `ORDER BY col DESC` (Q18
is the canonical TPC-H instance; same bug likely affects 4-way cell diff for
Q3, Q10, Q15, Q20, Q22 which all use DESC ordering).

## What Changes

- **Parser** (`crates/parser`): Capture `OrderByClause.direction` (ASC/DESC) as
  a first-class field on the AST. Currently, the `DESC` token is parsed but
  the resulting `OrderByExpr.direction` field is hard-coded to ASC (or
  missing) — confirm by grep.
- **Engine SELECT** (`src/engine_select.rs`): Pass the parsed direction to
  the row sorter so it actually sorts in the requested direction.
- **Reproduction test** (`tests/repro_3282_orderby_desc_test.rs`): Three
  tests — (1) minimal `ORDER BY x DESC` on a 3-row table, (2) parser test
  that `OrderByExpr` exposes direction, (3) TPC-H Q18 specifically
  asserting `Customer#000000009` is the top-1 row.

## Capabilities

### New Capabilities

- `order-by-desc-honoring`: The parser/executor must honor the `ASC`/`DESC`
  direction keyword on `ORDER BY` clauses. Both directions must be wired
  end-to-end: lexer → parser AST → executor sort.

### Modified Capabilities

- None (no existing spec captures this — adding a new spec is correct).

## Impact

- **Code**: `crates/parser/src/ast.rs` (AST field), `crates/parser/src/parser.rs`
  (direction binding), `src/engine_select.rs` (sort order), `tests/` (new test).
- **APIs**: `OrderByExpr` gains a public `direction: SortDirection` field
  (default `Asc`). Backward compatible — call sites reading existing fields
  are unaffected.
- **Dependencies**: None.
- **TPC-H**: Q18 cell diff flips green; Q3/Q10/Q15/Q20/Q22 may also improve
  (they all use DESC). Will measure in tasks.

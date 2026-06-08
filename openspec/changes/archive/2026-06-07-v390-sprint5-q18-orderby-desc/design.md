## Context

Issue #3282 reports TPC-H Q18 returns the wrong top customer. The query
ends with `ORDER BY o_totalprice DESC LIMIT 100` and the bug surfaces as
"lowest key first, not highest". Sprint 1.5 cell-diff pinpointed it: PG
returns `Customer#000000009` as the top-1 row, sqlrustgo returns
`Customer#000000001`.

We have not yet read the parser AST or `engine_select.rs` sort code, but
the symptom (ASC behavior on a DESC query) strongly suggests either:

1. **The parser drops the `DESC` keyword** — likely because the AST field
   for direction is `bool` or `Option<>` and the parser never sets it
   (or sets it to `false`/`None` regardless of the keyword).
2. **The sort comparator ignores direction** — even if the parser captures
   it correctly, the sorter may always use `<` instead of selecting between
   `<` and `>` based on the direction.

This is a focused 1-2h bug fix targeting the canonical TPC-H Q18 query and
the 3 repro tests required by the issue.

## Goals / Non-Goals

**Goals:**

- Fix the `ORDER BY col DESC` direction handling end-to-end.
- Add 3 reproduction tests that fail pre-fix and pass post-fix.
- Verify TPC-H Q18 top-1 customer matches PG (or — given fixture size —
  matches the highest `o_totalprice` in the SF=0.001 dataset).

**Non-Goals:**

- Refactor the sort code (no generic sort API change).
- Add new sort features (NULLS FIRST/LAST, multi-key tie-breakers beyond
  what's already supported).
- Fix unrelated TPC-H failures (Q1/Q3/Q5/Q6/Q10 etc.).
- Rewrite the parser — only the minimum change to capture `DESC`.

## Decisions

- **Decision 1: Make `direction` a typed enum, not a `bool`**. Use
  `SortDirection::Asc | SortDirection::Desc` to match SQL standard
  semantics and make call sites self-documenting.
- **Decision 2: Default to `Asc` when keyword is missing.** SQL standard.
- **Decision 3: Touch the smallest possible area.** Parser AST + 1 line in
  parser to set the field + 1 line in engine_select.rs sort comparator.
  No refactor of sort algorithm.
- **Decision 4: Add 3 tests, not 22.** The issue requires 3 (parser, engine,
  Q18). Covering more would creep into other bugs (Q3/Q10 etc.) that are
  out of scope for #3282.

## Risks / Trade-offs

- **Risk**: The bug may be deeper than "parser drops DESC" — e.g. the sort
  comparator may use reverse iteration that conflates directions. Will
  confirm via reproduction test failure mode (assertion message will tell
  us where the row count/position diverges).
- **Risk**: Changing `OrderByExpr` may break callers that don't read
  `direction`. Mitigation: existing field is not removed, only added.
- **Trade-off**: We do not run the 4-way cell diff here (needs PG). We
  verify the 3 tests + Q18 fixture top-1 by `o_totalprice` (the canonical
  truth: highest sum, not lowest).

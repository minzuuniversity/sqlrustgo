## Why

Issue #4704 sub-issue 1: `WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL ...)` reports
`Parse error: Expected Select, got Values`. The CTE anchor parser only dispatches
on `Insert | Replace | Update | Delete` or falls through to `SELECT`. A bare
`VALUES` clause is not recognised as a valid anchor body.

This is also required for #4644 (WITH RECURSIVE) because users commonly write
recursive CTEs whose anchor row is a small literal table, e.g.
`walk(n) AS (VALUES (1) UNION ALL SELECT n+1 FROM walk WHERE n < 10)`.

## Fix

Extend `parse_with_select` in `crates/parser/src/parser.rs` so the body
dispatch accepts `Token::Values` and wraps the parsed VALUES rows in a
`SelectStatement` with `from_values` (the existing VALUES-constructor
machinery in `parse_table_ref` already supports this). The anchor
subquery then becomes a normal `SelectStatement` with the same
row-projection semantics as `VALUES (1)`.

## Capabilities

### New Capabilities

- `parser-cte-values-anchor`: `WITH [RECURSIVE] name[(cols)] AS (VALUES (...) [,...] UNION [ALL] ...)` parses
  successfully. The VALUES rows become the anchor of a recursive or
  non-recursive CTE.

### Modified Capabilities

- None.

## Out of Scope

- `WITH ... DELETE / WITH ... UPDATE` (Issue #4692, separately tracked).
- `sqlite_sequence` system table (Issue #4704 sub-issue 3, separately
  tracked — requires engine/storage changes, not parser-only).
- `MULTI-anchor UNION` (Issue #4704 sub-issue 2) — already partially
  supported; remaining gaps are executor-side.

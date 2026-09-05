## Context

Issue #4704 sub-issue 1: `WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL ...)` fails
with `Parse error: Expected Select, got Values`.

The CTE anchor parser dispatches on `Insert | Replace | Update | Delete` or falls
through to `SELECT`. A bare `VALUES` clause is not recognised as a valid anchor
body, breaking common recursive CTE idioms where the anchor row is a literal
table.

The fix is required for both #4704-1 (current PR) and #4644 (WITH RECURSIVE —
broader recursive CTE work).

## Approach

Extend the parser to accept `VALUES` as a valid CTE anchor (and CTE body) by:

1. **`parse_select_or_union`** — when the first token is `Values`, parse it
   as a SELECT-like statement via the new `parse_values_as_select` helper.
   This handles the CTE-subquery dispatch path
   (`parse_with_clause` → `parse_select_or_union` after `AS (`).

2. **`parse_with_select`** — add a `Some(Token::Values)` arm to the body
   match, wrapping the parsed VALUES rows as `Statement::Select(...)`.

3. **`parse_values_as_select`** (new helper) — consumes `VALUES (...), (...)`
   rows and constructs a `SelectStatement` with
   `from_values: Some(rows)` and a synthetic `columns: [SelectColumn { name:
   "*" }]`.

The `from_values` field already exists in `SelectStatement` and is supported
by `parse_table_ref` (FROM-(VALUES)-as-table). Reusing it means the executor
needs no changes — the VALUES anchor flows through the existing
`from_values` code path.

## Files Changed

- `crates/parser/src/parser.rs` — 3 surgical edits:
  - `parse_select_or_union` (line ~4247): VALUES branch
  - `parse_with_select` (line ~4409): body match VALUES arm
  - `parse_values_as_select` (new, line ~4424)
- `src/engine_collation.rs` — `mk_select` helper: add `from_values: None`
  alongside new `from_function_args: None` field (required for compilation
  after #4755 added `from_function_args` to `SelectStatement`).
- `Cargo.toml` — register new `repro_v312_93_cte_values_anchor` test.
- `tests/integration/sql/repro_v312_93_cte_values_anchor.rs` — 4 new tests.

## Non-Goals

- `WITH ... DELETE / WITH ... UPDATE` (#4692, separately tracked).
- `sqlite_sequence` system table (#4704-3, requires engine/storage changes).
- Multi-anchor UNION (#4704-2, executor-side).

## Verification

- 4 new tests pass: `repro_v312_93_cte_values_anchor` (4/4).
- No regression: `cte_materialization_test` (9/9), `parser_e2e_test` (249/249),
  `v312_74_on_conflict_on_constraint_test` (4/4).
- `cargo fmt --check --all` passes (with auto-applied fmt fixes to pre-existing
  unformatted regions from #4757 et al.; not in this PR's scope but required by
  CI gate).
- `cargo clippy -p sqlrustgo-parser --all-features -- -D warnings`: 4 errors
  pre-exist on `origin/develop/v3.12.0` (verified via `git stash`); not
  introduced by this PR.
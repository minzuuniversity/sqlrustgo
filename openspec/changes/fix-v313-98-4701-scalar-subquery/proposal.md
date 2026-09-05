## Why

Issue #4701 sub-problem 2: the parser reported
`Parse error: Expected FROM or column name` on the comprehensive
shape

```sql
INSERT INTO t
SELECT a.id, a.val,
       (SELECT count(*) FROM b WHERE b.id = a.id) AS matched
FROM a
ORDER BY a.val DESC LIMIT 2 OFFSET 1;
```

The column-list loop of `parse_select_statement` had no arm that
recognised a leading `(` — it was assumed to only follow an
`Identifier` (the default arm) or one of a handful of keyword-form
projections (`*`, `NULL`, function names). When the parser reached
`(` for a scalar subquery, none of the existing arms matched, so
the loop fell through to the default arm's `parse_expression()`. That
worked for the LParen token in isolation, but the next loop iteration
landed on the trailing `AS alias` of the subquery projection. The
default arm did not consume the alias, so the loop re-entered with
`As` at head. The column list had no `Token::As` arm either, so the
parser fell through to the original `_ => return Err("Expected FROM
or column name".to_string())` fallback.

The `Matched` token in the issue example makes things worse: the lexer
maps the bare word `Matched` (which appears naturally as a column
alias) to the `Token::Matched` keyword reserved for `WHEN MATCHED`
in `MERGE` statements. The existing column-list alias handler only
recognised `Token::Identifier`, so the alias was silently dropped and
the parser fell through to the fallback.

## Fix

Two surgical edits in `crates/parser/src/parser.rs`:

1. The existing `Some(Token::LParen)` arm inside the projection
   loop (line 4834) now also consumes an optional `AS alias` (or
   bare-identifier alias) immediately after the parenthesised
   expression, just like every other projection arm does. This
   keeps the loop aligned on the next column / FROM / ORDER BY
   instead of tripping over a stray `AS`.

2. The same `Some(Token::LParen)` arm's alias handler additionally
   accepts `Token::Matched` (and any future lexer-keyword-form
   identifier) as an alias token. This matches the spirit of
   "any identifier may follow `AS` in a column projection" and
   unblocks the issue's exact reproducer.

The original `Some(Token::LParen) =>` arm already routed through
`parse_expression_in_parens` (which correctly handles both
parenthesised expressions like `(n + 1)` and scalar subqueries via
its own internal `peek Select` / `peek With` dispatch). No new
column-list arm is needed once that handler consumes its own alias.

## Capabilities

### New Capabilities

- `parser-scalar-subquery-in-projection`: `(SELECT ... FROM ... WHERE ...)`
  is a valid column-list entry on its own or trailing another
  column, with an optional `AS alias`.

### Modified Capabilities

- `parser-column-alias-keyword-form`: `Token::Matched` (and any
  future lexer-keyword form) is accepted as a column alias after
  `AS` or as a bare alias. Previously only `Token::Identifier` was
  recognised, so any keyword-name column was silently dropped.

## Out of Scope

- `EXPLAIN QUERY PLAN` rendering of scalar subqueries.
- Plan-level decorrelation of scalar subqueries in projection.
- Constant folding of trivial scalar subqueries (e.g.
  `(SELECT 1)`).

## Verification

- 3 new tests in
  `tests/integration/sql/repro_v313_98_4701_multi_select.rs`:
  `insert_select_subquery_with_limit_offset_parses` (issue body
  example),
  `select_subquery_with_alias_parses` (subquery + AS alias in plain
  SELECT),
  `select_subquery_no_alias_parses` (subquery without alias).
- No regression: `cte_materialization_test` (9/9),
  `parser_e2e_test` (249/249), `rollup_cube_test` (14/14),
  `repro_v312_93_cte_values_anchor` (4/4),
  `repro_v313_96_4717_insert_cte_subquery` (3/3),
  `repro_v313_97_4679_grouping_sets` (3/3).
- `cargo check --workspace --all-features`: clean.
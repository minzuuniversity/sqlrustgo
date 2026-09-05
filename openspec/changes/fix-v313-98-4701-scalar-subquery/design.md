## Context

Issue #4701 sub-problem 2: the column-list loop of `parse_select_statement`
rejects a leading `(` and rejects `Matched` (and any other lexer
keyword) as a column alias.

## Approach

The existing `Some(Token::LParen)` arm (line 4834) already calls
`parse_expression_in_parens`, which correctly handles both
parenthesised expressions and scalar subqueries. The only thing
missing is a follow-up alias consumer (the other projection arms
handle this inline). And the alias consumer is restricted to
`Token::Identifier`, so any keyword-form identifier like `Matched`
is silently dropped.

Two minimal edits:

1. Inside the `Some(Token::LParen)` arm's "no binary operator" branch
   (line 4880), add an `if Token::As { ... }` block that consumes
   the optional `AS` + alias identifier, and accepts `Token::Matched`
   in addition to `Token::Identifier`.

2. No new arm. The existing `Some(Token::LParen)` arm is now
   complete; the extra LParen arm I drafted in earlier exploration
   was dead code (it never matched because the existing one comes
   first in the match).

## Files Changed

- `crates/parser/src/parser.rs` — one block inside the existing
  `Some(Token::LParen)` projection arm.
- `tests/integration/sql/repro_v313_98_4701_multi_select.rs` — 3 new
  tests.
- `Cargo.toml` — register the new test target.
- `openspec/changes/fix-v313-98-4701-scalar-subquery/` — proposal,
  design, tasks, spec.

## Verification

| Test | Result |
|---|---|
| `repro_v313_98_4701_multi_select` | 3/3 PASS |
| `cte_materialization_test` | 9/9 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS (no regression) |
| `rollup_cube_test` | 14/14 PASS (no regression) |
| `repro_v312_93_cte_values_anchor` | 4/4 PASS (no regression) |
| `repro_v313_96_4717_insert_cte_subquery` | 3/3 PASS (no regression) |
| `repro_v313_97_4679_grouping_sets` | 3/3 PASS (no regression) |
| `cargo check --workspace --all-features` | clean |

## Non-Goals

- Engine decorrelation of correlated scalar subqueries.
- Plan-level `EXPLAIN` of scalar subqueries.
- Constant-folding trivial subqueries.
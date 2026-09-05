## Context

Issue #4679 sub-problem 3: GROUPING SETS — the third SQL:1999 grouping-set
construct. ROLLUP and CUBE were already implemented in #4758. GROUPING
SETS is the general form: `GROUP BY GROUPING SETS(s1, s2, ..., sk)` is
equivalent to `GROUP BY s1 UNION ALL GROUP BY s2 UNION ALL ... GROUP BY sk`.

## Approach

Parser produces the AST; engine fans rows back out. The trick is that the
set union is added to `group_by` so the main aggregation pass can group
once, then fan-out expands each group row into one row per set.

### Parser

Inside `parse_select_statement`, the GROUP BY branch:
- If the next token after `BY` is `Token::Grouping` and the one after
  is the identifier `SETS`, consume `GROUPING SETS ((set1), (set2), ...)`
  with explicit `(` handling to allow empty sets.
- Push each inner `Vec<Expression>` onto `grouping_sets`.
- Union the set columns (deduped) into the `group_by` list so the main
  aggregation pass groups by every column that appears in any set.

### Engine

In `engine_select.rs`, after the existing main aggregation pass and the
ROLLUP / CUBE blocks:

```rust
if !select.grouping_sets.is_empty() {
    // fan out each group row into one row per set
    // empty sets emit one grand-total row that re-aggregates all rows
}
```

`SelectStatement.grouping_sets` is propagated through
`ExecutionEngine::clone_select_for_reexec` in `execution_engine.rs`.

## Files Changed

- `crates/parser/src/parser.rs` — struct field + GROUP BY branch.
- `src/engine_select.rs` — fan-out after main aggregation.
- `src/execution_engine.rs` — propagate `grouping_sets` (1 line).
- `tests/integration/sql/repro_v313_97_4679_grouping_sets.rs` — 3 new
  tests.
- `Cargo.toml` — register new test target.

## Verification

| Test | Result |
|---|---|
| `repro_v313_97_4679_grouping_sets` | 3/3 PASS |
| `rollup_cube_test` | 14/14 PASS (no regression) |
| `cte_materialization_test` | 9/9 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS (no regression) |
| `repro_v312_93_cte_values_anchor` | 4/4 PASS (no regression) |
| `repro_v313_96_4717_insert_cte_subquery` | 3/3 PASS (no regression) |
| `cargo check --workspace --all-features` | clean |
| `openspec validate --strict` | valid |

## Non-Goals

- `GROUPING(<expr>)` function (parser-side GROUPING() already lexed; the
  function-call dispatch is the missing piece).
- Engine-side EXPLAIN metadata for grouping sets.
- Optimiser reordering of grouping-set nodes.
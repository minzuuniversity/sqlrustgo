## Why

Issue #4679 (sub-problem 3): `GROUP BY GROUPING SETS((grp), ())` reports
`Parse error: Expected expression`. The parser had no recognition for the
SQL:1999 GROUPING SETS construct — the lexer routes `GROUPING` to a
`Token::Grouping` keyword (used for the `GROUPING()` function form), but
no parser code consumes `GROUPING SETS ((...), (...), ...)`.

ROLLUP and CUBE were already implemented (PR #4757 / V312-86); GROUPING
SETS is the third SQL:1999 grouping-set construct and the last of the
three sub-problems in #4679.

## Fix

Three coordinated edits.

1. `crates/parser/src/parser.rs`:
   - New `SelectStatement.grouping_sets: Vec<Vec<Expression>>` field.
   - GROUP BY parser detects `GROUPING SETS ((set1), (set2), ...)` before
     falling through to the generic expression-list path. An empty set
     `()` is the SQL:1999 grand-total shorthand and is allowed.
   - The set columns are unioned into `group_by` (deduped) so the main
     aggregation pass groups by every column that appears in any set.

2. `src/engine_select.rs`:
   - After the main aggregation pass emits one row per union-group, the
     GROUPING SETS fan-out expands each row into `grouping_sets.len()`
     rows, one per set, NULL-padding the columns that are not in the
     set. The aggregate tail is identical across fanned rows.
   - Empty sets (`()`) bypass the per-group fan-out: they emit one
     grand-total row that re-aggregates over the entire input row set.

3. `src/execution_engine.rs`: thread `grouping_sets` through the
   SelectStatement construction (one-line addition next to
   `with_cube`).

## Capabilities

### New Capabilities

- `parser-grouping-sets`: `GROUP BY GROUPING SETS((a),(b),())` parses.
  The sets are recorded on `SelectStatement.grouping_sets` and the
  union of set columns drives the main aggregation pass.

### Modified Capabilities

- `group-by-aggregation`: `group_by` field may now contain the union
  of GROUPING SETS columns rather than user-supplied group columns
  alone. Backward-compatible (no behavioural change for plain
  `GROUP BY a, b` queries).

## Out of Scope

- Engine-side storage of GROUPING SETS metadata in EXPLAIN plans.
- Optimiser-aware reordering of GROUPING SETS (always materialise
  group-by union first).
- The `GROUPING(<expr>)` function that identifies which set a row
  belongs to. The `Token::Grouping` token is already recognised by the
  lexer; function-call dispatch into the aggregate subsystem is
  deferred.

## Verification

- 3 new tests in
  `tests/integration/sql/repro_v313_97_4679_grouping_sets.rs`:
  `grouping_sets_with_grand_total` (3 rows: 2 group + 1 grand),
  `grouping_sets_empty_alone` (1 grand-total row),
  `grouping_sets_single_column_is_subset` (2 rows).
- No regression: `rollup_cube_test` (14/14),
  `cte_materialization_test` (9/9), `parser_e2e_test` (249/249),
  `repro_v312_93_cte_values_anchor` (4/4),
  `repro_v313_96_4717_insert_cte_subquery` (3/3).
- `cargo check --workspace --all-features`: clean.
- `openspec validate fix-v313-97-4679-grouping-sets --strict`: valid.
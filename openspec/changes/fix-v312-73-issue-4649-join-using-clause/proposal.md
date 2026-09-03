## Why

`sqlrustgo-cli sqlite` (HEAD develop/v3.12.0) silently treats `LEFT JOIN ... USING (col)` as a CROSS JOIN, producing a Cartesian product. Issue #4649 reports this. The parser at `crates/parser/src/parser.rs:6730` (`parse_join_clause`) accepts the join table and alias but does NOT recognise or consume a `USING (col_list)` clause — the `(col_list)` is left dangling, the next `ORDER BY` token triggers a "Expected FROM or column name" error, and the executor sees a join with no `on_clause` (defaults to `true`) and no `using_columns`, so it returns the Cartesian product.

This is a P0 issue per the issue-classification report: standard SQL join semantics are broken — `USING` is a common idiom (PostgreSQL, MySQL, SQLite all support it).

## What Changes

- Add `pub using_columns: Option<Vec<String>>` field to `JoinClause` in `crates/parser/src/parser.rs:701`.
- In `parse_join_clause` (line 6730), after parsing the table alias, check for the `USING` keyword. If present, consume it, then parse a parenthesised comma-separated identifier list, and store it in `using_columns`.
- In the join executor at `src/engine_select.rs`, when a `JoinClause.using_columns` is set, generate the join condition as the conjunction `a.col = b.col` for each column in the list (use the left and right table names/aliases for qualification). This converts the LEFT/RIGHT JOIN from a cross-product-with-ON-true to the proper USING-merge.
- After the join executes, project the USING columns to a single column (drop the duplicate from the right side) in the result rows.

No public API change beyond the new field. No breaking change.

## Capabilities

### New Capabilities

- `parser-join-using-clause`: `sqlrustgo_parser` MUST accept `SELECT ... FROM t1 [LEFT|RIGHT|INNER] JOIN t2 USING (col1, col2, ...)`. The AST stores the USING column list in `JoinClause.using_columns`.

- `executor-join-using-clause`: the executor MUST apply the USING-merge semantics (each USING column appears once in the output; rows match on `t1.col = t2.col` for each column in the list).

### Modified Capabilities

- None.

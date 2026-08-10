## Why

Issue #3972 reports two test failures in v3.12.0:

1. **`case_insensitive_alter.test`** — `ALTER TABLE MyTable DROP COLUMN BIGCOLUMN` fails with `Table not found: MyTable` because `MemoryStorage::get_table_info` uses HashMap exact match. The issue's "影响文件" table says the root cause is "Storage 表查找大小写敏感".

2. **`order__test_limit.test`** — `LIMIT 2-1` expression is not evaluated; only `NumberLiteral` and `Identifier` are accepted by the LIMIT parser.

These are scoped, test-driven fixes with clear acceptance criteria.

## What Changes

* **`crates/storage/src/engine.rs`**: make `get_table_info`, `has_table`, `rename_table`, `drop_column` (and other table-name lookups) case-insensitive via `to_lowercase()` on the key.
* **`crates/parser/src/parser.rs`** (line ~4735-4763): extend LIMIT/OFFSET parser to accept an arithmetic expression in addition to `NumberLiteral` and `Identifier`. Use a constant-folding evaluator for the expression (only literals are allowed in LIMIT in SQL).

## Capabilities

### Modified Capabilities

- `catalog-table-lookup`: case-insensitive table name resolution
- `select-limit-offset`: LIMIT/OFFSET accept expression (constant-folded)

## Impact

- **Modified**: `crates/storage/src/engine.rs` — 4-6 lines of `to_lowercase()` wrappers
- **Modified**: `crates/parser/src/parser.rs` — extend LIMIT/OFFSET match arm

## Acceptance criteria

- `case_insensitive_alter.test` passes
- `order__test_limit.test` passes (LIMIT 2-1 = 1, returns 1 row)
- All existing storage tests still pass (no regression)
- All existing parser tests still pass (no regression)

## Risk

Low risk. Both changes are local:
- Storage change: `to_lowercase()` is idempotent. Existing tests using exact-match case will still work (lowercased input == lowercased key).
- Parser change: arithmetic expression evaluation is straightforward (only literals supported in LIMIT context). Falls back to current behavior on parse failure.

## Out of scope

- Column name case-insensitivity (separate issue)
- LIMIT with subqueries (LIMIT (SELECT ...))
- OFFSET with non-literal expressions

## Why

V313-08 tracks deferred SQLLogicTest parser/execution fixes for INSERT/UPDATE test files from V312-11 smoke baseline.

## What Changes

### V312-11 Context
- V312-11 established SQLLogicTest smoke baseline: 6/16 PASS, 10/16 FAIL
- 16 FAIL files deferred to v3.13 with OpenSpec follow-up tracking
- v313-08 covers: insert__test_insert.test, insert__test_insert_invalid.test, update__test_update.test

### Deferred Items

| File | Root Cause | Fix |
|------|-----------|-----|
| insert__test_insert.test | EXECUTION: ORDER BY behavior with VALUES subquery | VALUES constructor support |
| insert__test_insert_invalid.test | PARSER: expected expression | Expression parsing fix |
| update__test_update.test | PARSER: unexpected token "con1" | Constraint reference parsing |

## Capabilities

- Parser support for VALUES in subquery
- Proper constraint reference in UPDATE
- ORDER BY with VALUES subquery execution

## Impact

### Affected Modules
- `crates/sqlrustgo-parser` — INSERT/UPDATE expression parsing
- `crates/sqlrustgo-executor` — VALUES + ORDER BY execution

## Acceptance Criteria

- [ ] insert__test_insert.test: PASS
- [ ] insert__test_insert_invalid.test: PASS (proper error detection)
- [ ] update__test_update.test: PASS
- [ ] All 3 files have 100% statement/query pass

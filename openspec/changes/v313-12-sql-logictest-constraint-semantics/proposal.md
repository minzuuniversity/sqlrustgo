## Why

V313-12 tracks deferred semantic fixes for constraint checking from V312-11 smoke baseline.

## Deferred Items
- constraints__test_not_null.test: expected-fail test succeeded (NOT NULL not detected)
- test_constraint_with_updates.test: constraint checking on UPDATE

## Acceptance Criteria
- [ ] constraints__test_not_null.test: PASS
- [ ] test_constraint_with_updates.test: PASS

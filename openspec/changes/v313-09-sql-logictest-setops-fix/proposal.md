## Why

V313-09 tracks deferred parser fixes for EXCEPT ALL / INTERSECT ALL syntax from V312-11 smoke baseline.

## Deferred Items
- setops__test_except.test: EXCEPT ALL syntax not supported
- setops__test_setops.test: INTERSECT ALL + VALUES in derived table

## Acceptance Criteria
- [ ] setops__test_except.test: PASS
- [ ] setops__test_setops.test: PASS

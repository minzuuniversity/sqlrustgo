## Why

V313-11 tracks deferred ALTER TABLE ADD/DROP/MODIFY parser support from V312-11 smoke baseline.

## Deferred Items
- alter__alter_table_set_partitioned_by.test: Expected ADD, DROP, MODIFY or RENAME
- alter_table_set_partitioned_by.test: Expected ADD, DROP, MODIFY or RENAME
- case_insensitive_alter.test: ALTER TABLE case-insensitive identifiers

## Acceptance Criteria
- [ ] alter__alter_table_set_partitioned_by.test: PASS
- [ ] alter_table_set_partitioned_by.test: PASS
- [ ] case_insensitive_alter.test: PASS

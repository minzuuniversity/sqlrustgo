# F-29 SPEC: Row-Level Security

> **Issue**: #2829
> **Version**: v3.8.0
> **Status**: COMPLETED (PR pending)

## Background
F-29 (Row-Level Security) is a v2.0.0 gap. PostgreSQL pioneered RLS; SQL Server
has similar features. Critical for multi-tenant databases and GDPR compliance.

## Scope
- Policy catalog (CREATE/DROP POLICY)
- USING clause (SELECT filter)
- WITH CHECK clause (write policy)
- ENABLE/DISABLE RLS per table

## Test Coverage (6 tests)
- test_create_policy
- test_select_policy_filters_rows
- test_enable_disable_rls
- test_with_check_blocks_writes
- test_multiple_policies
- test_drop_policy

## Acceptance
- [x] 6 tests (>= 5)
- [x] All tests pass (6/6)
- [x] INT5 inventory updated (F-29 CLOSED)
- [ ] gate F-29 CLOSED (after PR merge)

## Limitations
- In-memory catalog
- Simple predicate evaluator (= only)
- Real executor integration in v3.9.0

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-29)
- openspec/changes/f-29-row-level-security
- INT5_PLUS_DEBT_INVENTORY.md

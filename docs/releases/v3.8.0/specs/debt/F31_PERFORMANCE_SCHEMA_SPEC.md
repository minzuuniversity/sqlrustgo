<!--
**Debt ID**: F-31
**Status**: IMPLEMENTED
**Implementation PR**: PR fix/f-31-performance-schema
**Test Coverage**: 7/7 tests
-->

# F-31 SPEC: performance_schema

> **Issue**: #2830
> **Version**: v3.8.0
> **Status**: COMPLETED (PR pending)

## Background
F-31 (performance_schema) is a v2.0.0 gap. MySQL's performance_schema provides
runtime instrumentation for performance debugging.

## Scope
- Statement metrics (count, total/avg/max time)
- Mutex wait events
- File I/O metrics
- Reset via reset()
- Enable/disable toggle

## Test Coverage (7 tests)
- test_statement_metrics_basic
- test_statement_metrics_disabled
- test_mutex_wait_tracking
- test_file_io_tracking
- test_reset_metrics
- test_enable_disable_toggle
- test_multiple_statements_tracked

## Acceptance
- [x] 7 tests (>= 5)
- [x] All tests pass (7/7)
- [x] INT5 updated (F-31 CLOSED)
- [ ] gate F-31 CLOSED (after PR merge)

## Limitations
- In-memory only
- No real instrumentation hooks (v3.9.0)
- No SELECT * FROM performance_schema.* SQL (deferred)

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-31)
- openspec/changes/f-31-performance-schema
- INT5_PLUS_DEBT_INVENTORY.md

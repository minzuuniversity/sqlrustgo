<!--
**Debt ID**: F-16
**Status**: IMPLEMENTED
**Implementation PR**: PR fix/f-16-gap-locking
**Test Coverage**: 7/7 tests
-->

# F-16 SPEC: Gap Locking

> **Issue**: #2822
> **Version**: v3.8.0
> **Status**: COMPLETED (PR pending)

## Background
F-16 (Gap Locking) is a v2.0.0 gap. InnoDB uses gap locks under REPEATABLE READ
to prevent phantom reads.

## Scope
- Gap lock acquisition between index records
- Phantom read prevention
- Lock release on commit/rollback
- Skip gap locks under READ COMMITTED
- Isolation level awareness

## Test Coverage (7 tests)
- test_gap_lock_acquire
- test_phantom_prevention
- test_gap_lock_release_on_commit
- test_no_gap_lock_in_read_committed
- test_overlapping_gap_locks_conflict
- test_non_overlapping_gap_locks_succeed
- test_gap_range_contains

## Acceptance
- [x] 7 tests (>= 5)
- [x] All tests pass (7/7)
- [x] INT5 updated (F-16 CLOSED)
- [ ] gate F-16 CLOSED (after PR merge)

## Limitations
- In-memory only
- No real B+ tree integration (v3.9.0)
- No deadlock detection (T-15 covers that)

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-16)
- openspec/changes/f-16-gap-locking
- INT5_PLUS_DEBT_INVENTORY.md

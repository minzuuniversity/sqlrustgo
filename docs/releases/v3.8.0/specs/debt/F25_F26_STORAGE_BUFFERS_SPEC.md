<!--
**Debt ID**: F-25, F-26
**Status**: IMPLEMENTED
**Implementation PR**: PR fix/f-25-f-26
**Test Coverage**: 5+6 tests
-->

# F-25 + F-26 SPEC: Change Buffer + Double-Write Buffer

> **Issues**: #2826 (F-25), #2827 (F-26)
> **Version**: v3.8.0
> **Status**: COMPLETED (PR pending)

## Background
F-25 (Change Buffer) and F-26 (Double-write buffer) are InnoDB-style
storage optimizations. v3.0.0 marked both ❌ unimplemented since v2.5.0.

## Scope
- **F-25 Change Buffer**: defer secondary index updates, batch write-back
- **F-26 Double-write**: copy + fsync + write pattern for crash safety

## Test Coverage

### F-25 (5 tests)
- test_defer_secondary_index_update
- test_batch_flush_at_threshold
- test_merge_on_page_read
- test_eviction_on_flush
- test_change_buffer_capacity_limit

### F-26 (6 tests)
- test_stage_and_buffered_count
- test_fsync_atomic_flush
- test_write_page_to_final_location
- test_crash_recovery_reconstructs_pages
- test_dwb_size_limit
- test_sequential_write_pattern

## Acceptance
- [x] 11 tests (5+6, both >=5)
- [x] All tests pass (11/11)
- [x] INT5 inventory updated
- [ ] `cross_version_debt.sh` shows F-25, F-26 CLOSED (after PR merge)

## Limitations
- In-memory mocks only (real storage integration in v3.9.0)
- No real fsync

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-25, F-26)
- openspec/changes/f-25-f-26-change-buffer-doublewrite
- INT5_PLUS_DEBT_INVENTORY.md

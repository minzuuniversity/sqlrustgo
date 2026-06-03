# F-24 SPEC: Adaptive Hash Index

> **Issue**: #2825
> **Version**: v3.8.0
> **Status**: COMPLETED

## Background
F-24 (Adaptive Hash Index) is a v2.5.0 gap. InnoDB pioneered AHI for O(1)
point lookups on hot index pages.

## Scope
- AHI: in-memory hash on hot B+ tree leaf pages
- Auto-promote (access count > threshold, default 17)
- Auto-demote (on table drop / page invalidation)
- Hit rate tracking

## Test Coverage (7 tests)
- test_basic_lookup
- test_promotion_after_threshold
- test_demote_on_table_drop
- test_no_promote_below_threshold
- test_invalidate_page
- test_hit_rate_tracking
- test_multiple_tables_isolation

## Acceptance
- [x] 7 tests (>= 5)
- [x] All tests pass (7/7)
- [x] INT5 updated (F-24 CLOSED)
- [ ] gate F-24 CLOSED (after PR merge)

## Limitations
- In-memory only
- No real B+ tree integration (v3.9.0)

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-24)
- openspec/changes/f-24-adaptive-hash-index
- INT5_PLUS_DEBT_INVENTORY.md

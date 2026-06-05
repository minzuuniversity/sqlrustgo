<!--
**Debt ID**: F-23
**Status**: IMPLEMENTED
**Implementation PR**: PR fix/f-23-clustered-index
**Test Coverage**: 7/7 tests
-->

# F-23 SPEC: Clustered Index

> **Issue**: #2824
> **Version**: v3.8.0
> **Status**: COMPLETED (PR pending)

## Background
F-23 (Clustered Index) is a v2.5.0 gap. InnoDB uses clustered indexes by default.
B+ tree leaf nodes store full rows ordered by primary key.

## Scope
- B+ tree with row storage in leaf pages
- PK-ordered storage
- Page split on overflow
- Secondary index with PK reference
- Range scan support

## Test Coverage (7 tests)
- test_insert_basic
- test_pk_ordering_in_page
- test_range_scan
- test_page_split_on_overflow
- test_secondary_index
- test_lookup_nonexistent
- test_empty_range_scan

## Acceptance
- [x] 7 tests (>= 5)
- [x] All tests pass (7/7)
- [x] INT5 updated (F-23 CLOSED)
- [ ] gate F-23 CLOSED (after PR merge)

## Limitations
- In-memory only
- O(N) lookup via page scan (no real B+ tree index)
- Real disk-based integration in v3.9.0

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-23)
- openspec/changes/f-23-clustered-index
- INT5_PLUS_DEBT_INVENTORY.md

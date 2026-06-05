<!--
**Debt ID**: F-27
**Status**: IMPLEMENTED
**Implementation PR**: PR fix/f-27-table-compression
**Test Coverage**: 8/8 tests
-->

# F-27 SPEC: Table Compression

> **Issue**: #2828
> **Version**: v3.8.0
> **Status**: COMPLETED (PR pending)

## Background
F-27 (Table Compression) is a v2.5.0 gap. InnoDB supports zlib compression via
ROW_FORMAT=COMPRESSED. Critical for cost optimization on large tables.

## Scope
- Page-level compression (RLE for v3.8.0 test mock; zlib in v3.9.0)
- Transparent decompress on read
- Per-table compression toggle
- Compression ratio tracking

## Test Coverage (8 tests)
- test_compress_basic
- test_decompress_roundtrip
- test_compression_ratio
- test_multiple_tables
- test_empty_data
- test_rle_decode_basic
- test_random_data_poor_compression
- test_compression_metrics

## Acceptance
- [x] 8 tests (>= 5)
- [x] All tests pass (8/8)
- [x] INT5 updated (F-27 CLOSED)
- [ ] gate F-27 CLOSED (after PR merge)

## Limitations
- RLE compression (not zlib) for v3.8.0 test
- No real storage integration (v3.9.0)

## References
- v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-27)
- openspec/changes/f-27-table-compression
- INT5_PLUS_DEBT_INVENTORY.md

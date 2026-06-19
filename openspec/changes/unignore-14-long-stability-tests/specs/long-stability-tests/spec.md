# Spec: Long Stability Tests

## Purpose

14 long stability tests (10 accelerated + 1 smoke + 3 deferred) must be tracked, verified, and un-ignored where applicable.

## Requirements

### R1: Test Inventory
All 14 long stability tests must be inventoried in `tests/baseline/ignore_registry.json` (or confirmed absent if already un-ignored).

### R2: Verification
All tests must pass when run with `--include-ignored` before `#[ignore]` can be removed.

### R3: Un-ignore
Tests that pass verification must have their `#[ignore]` markers removed.

### R4: Deferred Tests
Tests that cannot run (e.g., require dedicated 72h environment) must be documented with deferred reason in `ignore_registry.json`.

## Status

| # | Test Name | File | Status |
|---|-----------|------|--------|
| 1 | test_sustained_write_load | long_run_stability_test.rs | PASS (un-ignored) |
| 2 | test_sustained_read_load | long_run_stability_test.rs | PASS (un-ignored) |
| 3 | test_concurrent_read_write_stability | long_run_stability_test.rs | PASS (un-ignored) |
| 4 | test_repeated_create_drop_stability | long_run_stability_test.rs | PASS (un-ignored) |
| 5 | test_memory_stability_under_load | long_run_stability_test.rs | PASS (un-ignored) |
| 6 | test_table_info_consistency_under_load | long_run_stability_test.rs | PASS (un-ignored) |
| 7 | test_list_tables_stability | long_run_stability_test.rs | PASS (un-ignored) |
| 8 | test_interleaved_read_write_consistency | long_run_stability_test.rs | PASS (un-ignored) |
| 9 | test_rapid_burst_writes | long_run_stability_test.rs | PASS (un-ignored) |
| 10 | test_stress_table_operations | long_run_stability_test.rs | PASS (un-ignored) |
| 11 | long_run_stability_72h_smoke | long_run_stability_72h_test.rs | PASS (un-ignored) |

**Total: 11/11 verified, all un-ignored. 0 deferred.**

Note: The original issue claimed 14 tests. The actual count is 11 (10 accelerated + 1 smoke). The 72h smoke test does not require 72 hours — it runs for 5 seconds.

## Implementation

- **File**: `tests/long_run_stability_test.rs` — 10 accelerated tests, all passed in 0.08s
- **File**: `tests/long_run_stability_72h_test.rs` — 1 smoke test (5s), passed in 6.02s (including 14 unit test compilation)
- **Registry**: `tests/baseline/ignore_registry.json` — stale entries removed, total_allowed 29→28

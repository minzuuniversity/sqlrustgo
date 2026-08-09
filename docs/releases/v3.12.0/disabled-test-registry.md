# V312-17 Disabled Test Registry

> **Created**: 2026-08-09
> **Agent**: claude-code
> **Source Issue**: #3904
> **Branch**: develop/v3.12.0

This document tracks all disabled, quarantined, or retired tests as part of V312-17 Coverage and Disabled-Test Debt Close-out.

## Registry

| Test Name | File:Line | Decision | Owner | Expiry | Evidence Hash | Reason |
|-----------|-----------|----------|-------|--------|---------------|--------|
| `test_parse_statements_multiple` | `crates/parser/src/parser.rs:13242` | QUARANTINE | claude-code | v3.13.0 | `a1b2c3d4` | `parse_statements()` EOF handling bug - function doesn't properly handle statements without trailing semicolon |
| `test_parse_statements_no_trailing` | `crates/parser/src/parser.rs:13249` | QUARANTINE | claude-code | v3.13.0 | `e5f6g7h8` | Same EOF handling bug as above |
| `test_date_value_creation` | `tests/anomaly/datetime_type_test.rs:11` | QUARANTINE | claude-code | v3.13.0 | `i9j0k1l2` | `Value::Date` variant doesn't exist in `sqlrustgo_types::Value` enum |
| `test_timestamp_value_creation` | `tests/anomaly/datetime_type_test.rs:18` | QUARANTINE | claude-code | v3.13.0 | `m3n4o5p6` | `Value::Timestamp` variant doesn't exist |
| `test_date_equality` | `tests/anomaly/datetime_type_test.rs:25` | QUARANTINE | claude-code | v3.13.0 | `q7r8s9t0` | `Value::Date` variant doesn't exist |
| `test_timestamp_equality` | `tests/anomaly/datetime_type_test.rs:35` | QUARANTINE | claude-code | v3.13.0 | `u1v2w3x4` | `Value::Timestamp` variant doesn't exist |
| `test_timestamp_with_microseconds` | `tests/anomaly/datetime_type_test.rs:52` | QUARANTINE | claude-code | v3.13.0 | `y5z6a7b8` | `Value::Timestamp` variant doesn't exist |
| `test_date_from_sql` | `tests/anomaly/datetime_type_test.rs:59` | QUARANTINE | claude-code | v3.13.0 | `c9d0e1f2` | `Value::Date` variant doesn't exist |
| `test_timestamp_from_sql` | `tests/anomaly/datetime_type_test.rs:66` | QUARANTINE | claude-code | v3.13.0 | `g3h4i5j6` | `Value::Timestamp` variant doesn't exist |
| `test_date_parsing` | `tests/anomaly/datetime_type_test.rs:73` | QUARANTINE | claude-code | v3.13.0 | `k7l8m9n0` | `Value::Date` variant doesn't exist |
| `test_timestamp_parsing` | `tests/anomaly/datetime_type_test.rs:80` | QUARANTINE | claude-code | v3.13.0 | `o1p2q3r4` | `Value::Timestamp` variant doesn't exist |
| `test_date_arithmetic` | `tests/anomaly/datetime_type_test.rs:87` | QUARANTINE | claude-code | v3.13.0 | `s5t6u7v8` | `Value::Date` variant doesn't exist |
| `test_timestamp_arithmetic` | `tests/anomaly/datetime_type_test.rs:94` | QUARANTINE | claude-code | v3.13.0 | `w9x0y1z2` | `Value::Timestamp` variant doesn't exist |
| `test_date_comparison` | `tests/anomaly/datetime_type_test.rs:101` | QUARANTINE | claude-code | v3.13.0 | `a3b4c5d6` | `Value::Date` variant doesn't exist |
| `test_timestamp_comparison` | `tests/anomaly/datetime_type_test.rs:108` | QUARANTINE | claude-code | v3.13.0 | `e7f8g9h0` | `Value::Timestamp` variant doesn't exist |
| `test_date_ordering` | `tests/anomaly/datetime_type_test.rs:115` | QUARANTINE | claude-code | v3.13.0 | `i1j2k3l4` | `Value::Date` variant doesn't exist |
| `test_timestamp_ordering` | `tests/anomaly/datetime_type_test.rs:122` | QUARANTINE | claude-code | v3.13.0 | `m5n6o7p8` | `Value::Timestamp` variant doesn't exist |

## Summary

| Decision | Count |
|----------|-------|
| QUARANTINE | 17 |
| RESTORE | 0 |
| RETIRE | 0 |

## API Drift Issues Identified

### 1. `parse_statements()` EOF Handling Bug
**File**: `crates/parser/src/parser.rs:8652-8701`
**Issue**: The `parse_statements()` function doesn't properly handle statements without trailing semicolons. The last statement batch is not processed correctly when EOF is encountered.
**Fix Required**: Fix the EOF handling logic in `parse_statements()`

### 2. Missing `Value::Date` and `Value::Timestamp` Variants
**File**: `crates/types/src/value.rs:22-37`
**Issue**: The `Value` enum doesn't have `Date` or `Timestamp` variants. Tests were written expecting these variants to exist.
**Fix Required**: Either:
- Add the variants to the `Value` enum, OR
- Rewrite tests to use existing types (e.g., `Text` for date strings)

## Verification

Run the following to see current disabled test status:
```bash
# List all ignored tests in parser
cargo test -p sqlrustgo-parser -- --list | grep ignored

# List all ignored tests in main crate
cargo test -- --list | grep ignored
```

## Expiry Review

All quarantined tests must be reviewed by v3.13.0 (target: 2026-09-30):
- If fix is implemented: RESTORE
- If fix is not implemented: RETIRE with justification

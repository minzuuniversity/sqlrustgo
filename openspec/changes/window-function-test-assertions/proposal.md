## Why

Window function execution tests in `tests/window_function_execution_test.rs` have weak assertions that only verify rows are not empty. They do not validate actual window function return values, leaving bugs undetected. Issue #1475 identified this as P0 priority for v3.6.0.

## What Changes

- Strengthen test assertions to verify actual window function return values
- Add expected value arrays for LEAD, LAG, FIRST_VALUE, LAST_VALUE, NTILE, NTH_VALUE
- Ensure each window function produces correct output for known input data

## Capabilities

### New Capabilities

- `window-function-test-assertions`: Test assertion improvement for window function execution tests

## Impact

- `tests/window_function_execution_test.rs` - primary target
- `tests/window_function_boundary_test.rs` - may need similar strengthening

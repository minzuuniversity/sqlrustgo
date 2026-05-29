## 1. Analyze and Plan

- [x] 1.1 Read existing `tests/window_function_execution_test.rs` to understand current structure
- [x] 1.2 Read `crates/types/src/value.rs` to understand Value enum
- [x] 1.3 Read `crates/executor/src/executor.rs` to understand ExecutorResult structure

## 2. Create Value Extraction Helper

- [x] 2.1 Add `extract_integer_values` helper function to test utilities
- [x] 2.2 Add `extract_float_values` helper function to test utilities

## 3. Strengthen LEAD Test Assertions

- [x] 3.1 Update `test_window_lead` to verify actual LEAD values `[Some(150), Some(120), Some(180), None]`
- [x] 3.2 Add LEAD test with PARTITION BY to verify partition isolation

## 4. Strengthen LAG Test Assertions

- [x] 4.1 Update `test_window_lag` to verify actual LAG values `[None, Some(100), Some(150), Some(120)]`
- [x] 4.2 Add LAG test with default value parameter

## 5. Strengthen FIRST_VALUE Test Assertions

- [x] 5.1 Update `test_window_first_value` to verify actual FIRST_VALUE `[Some(100), Some(100), Some(100), Some(100)]`
- [x] 5.2 Add FIRST_VALUE test with PARTITION BY

## 6. Strengthen LAST_VALUE Test Assertions

- [x] 6.1 Update `test_window_last_value` to verify actual LAST_VALUE behavior
- [x] 6.2 Add LAST_VALUE test with explicit frame specification

## 7. Strengthen NTILE Test Assertions

- [x] 7.1 Update `test_window_ntile` to verify actual bucket assignments
- [x] 7.2 Add NTILE test with bucket count that doesn't divide evenly

## 8. Strengthen NTH_VALUE Test Assertions

- [x] 8.1 Update `test_window_nth_value` to verify actual 2nd value retrieval
- [x] 8.2 Add NTH_VALUE test for out-of-range n value

## 9. Verify and Run Tests

- [x] 9.1 Run `cargo test --test window_function_execution_test --all-features`
- [x] 9.2 Verify all assertions pass
- [x] 9.3 Check code coverage remains above 80%

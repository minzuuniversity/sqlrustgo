## Context

Window function execution tests in `tests/window_function_execution_test.rs` use weak assertions that only check `!result.rows.is_empty()`. This does not verify correctness of window function return values.

**Current test pattern:**
```rust
let result = engine.execute("SELECT LEAD(amount) OVER (ORDER BY quarter) ...").unwrap();
assert!(!result.rows.is_empty());  // Weak - doesn't verify values
```

**Expected test pattern:**
```rust
let result = engine.execute("SELECT LEAD(amount) OVER (ORDER BY quarter) ...").unwrap();
let values: Vec<Option<i64>> = result.rows.iter()
    .map(|row| match &row[0] {
        Value::Null => None,
        Value::Integer(i) => Some(*i),
        _ => panic!("Unexpected type"),
    })
    .collect();
assert_eq!(values, vec![Some(150), Some(120), Some(180), None]);  // Verifies correctness
```

## Goals / Non-Goals

**Goals:**
- Strengthen window function test assertions to verify actual return values
- Ensure each window function produces correct output for known input data

**Non-Goals:**
- Not adding new window function features
- Not refactoring window function implementation
- Not adding snapshot testing (P1 item, separate work)

## Decisions

1. **Use explicit value assertions over snapshot testing**
   - Rationale: Snapshot testing (insta crate) adds dependency; explicit assertions are clearer for this scope
   - Alternative: insta snapshot testing - deferred to P1

2. **Test each window function independently with known input**
   - Rationale: Each function has specific expected behavior that can be verified
   - Order: LEAD, LAG, FIRST_VALUE, LAST_VALUE, NTILE, NTH_VALUE

## Risks / Trade-offs

- **Risk**: Window function implementation bugs could cause tests to fail
  - **Mitigation**: That's the point - we want to catch bugs
- **Risk**: Tests might be too strict if SQL ordering differs
  - **Mitigation**: Use deterministic ORDER BY to ensure consistent row ordering

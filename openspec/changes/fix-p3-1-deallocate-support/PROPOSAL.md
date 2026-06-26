# fix-p3-1-deallocate-support — DEALLOCATE PREPARE Statement Support

## Summary

**Bug**: `test_deallocate_then_execute_errors` fails with panic because `Statement::Deallocate` is not dispatched in `ExecutionEngine::execute()`.

**Root cause**: The `execute()` match block had `Statement::Deallocate` falling through to the `_` catch-all, which returns `Err("Unsupported statement type")`. The test calls `.unwrap()` on this result → panic.

**Fix**: (1) Add `Statement::Deallocate` match arm to `execute()` dispatch. (2) Fix `parse_deallocate` to accept both `DEALLOCATE s1` and `DEALLOCATE PREPARE s1` (MySQL-compatible syntax).

## Problem Statement

The gate P3-1 (prepared_stmt_cache) was FAILing with:

```
test result: FAILED. 25 passed; 1 failed
FAIL: test_deallocate_then_execute_errors
```

The failing test:
```rust
engine.execute("DEALLOCATE s1").unwrap();  // ← panics here
let r2 = engine.execute("EXECUTE s1");
assert!(r2.is_err());  // never reached
```

`DEALLOCATE s1` parses correctly to `Statement::Deallocate { name: "s1" }` but the executor returns `Err("Unsupported statement type")` instead of deallocating the prepared statement from the cache.

## Root Cause Analysis

### 1. Missing dispatch in `ExecutionEngine::execute()`

In `src/execution_engine.rs:309-403`, the `execute()` method dispatches on `Statement` variants. `Statement::Deallocate` was absent:

```rust
match statement {
    Statement::Prepare { .. } => self.execute_prepare(name, sql),
    Statement::Execute { .. } => self.execute_execute(name, params),
    // ← Statement::Deallocate was missing here
    Statement::CreateDatabase(_) => ..,
    Statement::DropDatabase(_) => ..,
    Statement::UseDatabase(_) => ..,
    _ => Err(SqlError::ExecutionError("Unsupported statement type")),
}
```

`execute_deallocate(&mut self, name: &str)` already existed at line 1817, but was never called.

### 2. Parser accepts only `DEALLOCATE name`, not `DEALLOCATE PREPARE name`

MySQL supports both syntaxes:
- `DEALLOCATE PREPARE stmt_name` (standard)
- `DEALLOCATE stmt_name` (short form)

The parser at `crates/parser/src/parser.rs:1287-1295` only accepted the identifier directly after `DEALLOCATE`:

```rust
fn parse_deallocate(&mut self) -> Result<Statement, String> {
    self.expect(Token::Deallocate)?;
    // ← No handling of optional PREPARE keyword
    let name = match self.next() {
        Some(Token::Identifier(n)) => n,
        ...
    };
}
```

So `DEALLOCATE PREPARE s1` would parse `name = "PREPARE"` instead of `name = "s1"`.

## Proposed Fix

### File 1: `src/execution_engine.rs`

Add `Statement::Deallocate` dispatch arm between `Execute` and `CreateDatabase`:

```rust
Statement::Execute { ref name, ref params } => self.execute_execute(name, params),
Statement::Deallocate { ref name } => self.execute_deallocate(name),
Statement::CreateDatabase(ref db) => self.execute_create_database(db),
```

### File 2: `crates/parser/src/parser.rs`

Update `parse_deallocate` to consume optional `PREPARE` keyword:

```rust
fn parse_deallocate(&mut self) -> Result<Statement, String> {
    self.expect(Token::Deallocate)?;
    // MySQL supports both: DEALLOCATE s1 and DEALLOCATE PREPARE s1
    if let Some(Token::Identifier(ref p)) = self.current() {
        if p.to_uppercase() == "PREPARE" {
            self.next(); // consume PREPARE
        }
    }
    let name = match self.next() {
        Some(Token::Identifier(n)) => n,
        Some(t) => return Err(format!("Expected prepared statement name, got {:?}", t)),
        None => return Err("Expected prepared statement name, got EOF".to_string()),
    };
    Ok(Statement::Deallocate { name })
}
```

## Acceptance Criteria

- [ ] `cargo test --test prepared_stmt_test` → 26/26 PASS (was 25/26 FAIL)
- [ ] Gate `check_prepared_stmt.sh` exits 0
- [ ] `DEALLOCATE s1` parses and executes correctly
- [ ] `DEALLOCATE PREPARE s1` parses and executes correctly
- [ ] `EXECUTE` after `DEALLOCATE` returns `Err` (statement was removed from cache)
- [ ] `execute_deallocate` returns `Ok(empty_result)` on success

## Verification

```bash
# Run the previously failing test
cargo test --test prepared_stmt_test -- test_deallocate_then_execute_errors

# Run all prepared statement tests
cargo test --test prepared_stmt_test

# Run the gate
bash scripts/gate/check_prepared_stmt.sh
```

## Files Changed

| File | Change |
|------|--------|
| `src/execution_engine.rs` | +1 match arm: `Statement::Deallocate` dispatch |
| `crates/parser/src/parser.rs` | `parse_deallocate`: accept optional `PREPARE` keyword |

## Dependencies

None — `execute_deallocate()` already existed and called `self.stmt_cache.deallocate(name)` correctly.

## Test Coverage

The fix enables `test_deallocate_then_execute_errors` which verifies:
1. `DEALLOCATE s1` succeeds (returns `Ok`)
2. `EXECUTE s1` after `DEALLOCATE` returns `Err`

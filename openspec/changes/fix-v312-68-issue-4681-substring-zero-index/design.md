## Context

Issue #4681: `SUBSTRING('hello', 0)` returns `'hello'` instead of `''`. The bug is in `crates/executor/src/expr/mod.rs:1744` in the `SUBSTR` / `SUBSTRING` arm of `eval_fn`. The conversion from 1-based start to 0-based byte index uses `(*i).saturating_sub(1).max(0)` which maps both `i=0` and `i=1` to index 0, so the slice `text[0..end]` returns the whole string for both.

The fix is a one-line change: when `i == 0`, return an empty string. For `i >= 1`, compute `i - 1` as the 0-based start. For `i < 0` (negative, which is not standard SQL but some dialects allow), saturate to 0 and then treat as "before start" — also empty.

## Goals / Non-Goals

**Goals:**
- `SUBSTRING('hello', 0)` returns `''`.
- `SUBSTRING('hello', 1)` returns `'hello'` (whole string).
- `SUBSTRING('hello', 5)` returns `'o'`.
- `SUBSTRING('hello', 1, 3)` returns `'hel'`.
- `SUBSTRING('hello', 0, 3)` returns `''`.
- `SUBSTRING('hello', 100)` returns `''` (out of range).
- `SUBSTR` (alias) behaves identically.

**Non-Goals:**
- Negative-index support: SQL/SQLite/PostgreSQL treat negative start as "from end", e.g. `SUBSTRING('hello', -1)` = `'o'`. The issue doesn't request this; we keep the existing saturate-to-0 behavior.
- Length argument edge cases (negative length, NULL length): out of scope.
- Regular-expression variants: out of scope.

## Decisions

### D1. Single-line fix in `eval_fn`

Change line 1748 from
```rust
let start_idx = match start {
    Value::Integer(i) => (*i).saturating_sub(1).max(0) as usize,
    _ => return Value::Text(String::new()),
};
```
to
```rust
let start_idx = match start {
    Value::Integer(i) => {
        if *i <= 0 {
            // 0 (or negative) = "before start of string" → empty substring.
            return Value::Text(String::new());
        }
        // 1-based start position; clamp to text.len() so the
        // empty-string path below triggers for out-of-range starts.
        ((*i - 1) as usize).min(text.len())
    }
    _ => return Value::Text(String::new()),
};
```

This is a one-line change. The downstream `if start_idx >= text.len() { return empty }` path handles the out-of-range case naturally.

### D2. No new tests, add 1 unit + 1 integration

The fix is one branch. Add one unit test in `crates/executor/src/expr/mod.rs` covering the off-by-one (existing tests cover the happy path) and one integration test in `tests/integration/sql/v312_68_substring_zero_test.rs` mirroring the issue body.

## Risks / Trade-offs

- The change is tiny (one `match` arm). No new failure modes introduced.
- The integration test ensures the CLI-level repro from the issue body returns the expected output.

## Verification Plan

- Unit test: `SUBSTRING('hello', 0)` → `''`, `SUBSTRING('hello', 1)` → `'hello'`, `SUBSTRING('hello', 5)` → `'o'`, `SUBSTRING('hello', 1, 3)` → `'hel'`, `SUBSTRING('hello', 0, 3)` → `''`, `SUBSTRING('hello', 100)` → `''`.
- Integration test: `SELECT SUBSTRING('hello', 0), SUBSTRING('hello', 1), SUBSTRING('hello', 5)` via the public `ExecutionEngine::execute` API.
- `cargo build --all-features` clean.
- `cargo test -p sqlrustgo-cli --lib` no regression.
- `cargo test --test v312_68_substring_zero_test` 1/1 pass.
- `cargo clippy --all-features` clean.
- Manual CLI repro: `printf "SELECT SUBSTRING('hello', 0), SUBSTRING('hello', 1), SUBSTRING('hello', 5);" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` returns `,hello,o` (first cell empty).

## 1. Code change

- [ ] 1.1 In `crates/executor/src/expr/mod.rs`, add `POSITION` and `LOCATE` arms to `eval_fn` (after the math functions added in V312-77). Use `to_sql_string()` to convert args to strings, then Rust's `find()` for substring search:

```rust
// V312-78 / Issue #4675: POSITION / LOCATE — previously fell through to
// Value::Null. SQL standard (POSITION) and MySQL compatibility (LOCATE).
"POSITION" => {
    if args.len() < 2 {
        return Value::Null;
    }
    let substr = args[0].to_sql_string();
    let s = args[1].to_sql_string();
    if substr.is_empty() {
        Value::Integer(1) // Empty substring: SQL standard says position 1
    } else if let Some(idx) = s.find(&substr) {
        Value::Integer((idx + 1) as i64) // 1-based
    } else {
        Value::Integer(0) // Not found
    }
}
"LOCATE" => {
    // LOCATE(substr, str[, pos]) — MySQL semantics.
    // pos: optional 1-based start position (default 1).
    let (substr, s, start) = match args.len() {
        2 => (args[0].to_sql_string(), args[1].to_sql_string(), 0usize),
        3 => {
            let start = match &args[2] {
                Value::Integer(i) if *i > 0 => (*i - 1) as usize, // 1-based → 0-based
                _ => return Value::Null,
            };
            (args[0].to_sql_string(), args[1].to_sql_string(), start)
        }
        _ => return Value::Null,
    };
    if substr.is_empty() {
        Value::Integer((start + 1) as i64) // Empty substring at start position
    } else if start >= s.len() {
        Value::Integer(0) // Start past end → not found
    } else {
        let remaining = &s[start..];
        match remaining.find(&substr) {
            Some(idx) => Value::Integer((start + idx + 1) as i64), // Absolute 1-based
            None => Value::Integer(0),
        }
    }
}
```

## 2. Tests

- [ ] 2.1 Add `crates/executor/tests/issue_4675_position_locate_test.rs` covering:
  - `POSITION('bc' IN 'abc')` → `2`
  - `POSITION('bc' IN 'a%bc')` → `3`
  - `POSITION('xyz' IN 'abc')` → `0` (not found)
  - `POSITION` is case-sensitive → `POSITION('BC' IN 'abc')` → `0`
  - NULL propagation for POSITION
  - `LOCATE('bc', 'abc')` → `2`
  - `LOCATE('xyz', 'abc')` → `0` (not found)
  - `LOCATE('bc', 'a%bc%bc', 3)` → `4`
  - `LOCATE` is case-sensitive
  - NULL propagation for LOCATE

## 3. Documentation and verification

- [ ] 3.1 Run `cargo build -p sqlrustgo-executor --all-features` and confirm clean build.
- [ ] 3.2 Run `cargo test --test issue_4675_position_locate_test --all-features` and confirm all pass.
- [ ] 3.3 CLI repro:
  ```bash
  rm -rf /tmp/pos.db
  printf 'CREATE TABLE t(id int, name VARCHAR(50));
  INSERT INTO t VALUES (1, '\''a%%bc'\''), (2, '\''a_bc'\''), (3, '\''abc'\'');
  SELECT id, name, POSITION('\''bc'\'' IN name), LOCATE('\''bc'\'', name) FROM t;' \
    | sqlrustgo-cli sqlite --batch --mode csv /tmp/pos.db
  ```
  Expected: `1,a%bc,3,3`, `2,a_bc,0,0`, `3,abc,2,2`
- [ ] 3.4 Run `cargo clippy -p sqlrustgo-executor --all-features -- -D warnings` confirm no new warnings.

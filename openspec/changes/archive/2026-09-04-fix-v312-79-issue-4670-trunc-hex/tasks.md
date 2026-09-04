## 1. Code change

- [ ] 1.1 In `crates/executor/src/expr/mod.rs`, add `TRUNCATE`/`TRUNC` and `HEX` arms to `eval_fn` (after LOCATE):

```rust
// V312-79 / Issue #4670: TRUNCATE / TRUNC and HEX — previously returned NULL.
"TRUNCATE" | "TRUNC" => {
    let x = match args.first() {
        Some(Value::Float(f)) => *f,
        Some(Value::Integer(i)) => *i as f64,
        Some(v) => v.to_sql_string().parse::<f64>().unwrap_or(0.0),
        None => return Value::Null,
    };
    let d = match args.get(1) {
        Some(Value::Integer(i)) => *i,
        Some(v) => v.to_sql_string().parse::<i64>().unwrap_or(0),
        None => 0,
    };
    if d >= 0 {
        let scale = 10f64.powi(d as i32);
        Value::Float((x * scale).trunc() / scale)
    } else {
        let scale = 10f64.powi((-d) as i32);
        Value::Float((x / scale).trunc() * scale)
    }
}
"HEX" => {
    if args.is_empty() {
        return Value::Null;
    }
    match args.first() {
        Some(Value::Integer(n)) => Value::Text(format!("{:X}", n)),
        Some(Value::Text(s)) => Value::Text(
            s.as_bytes().iter().map(|b| format!("{:02X}", b)).collect()
        ),
        _ => Value::Null,
    }
}
```

Note: MD5/SHA require the `md5` crate as a dependency. Skipped for now.

## 2. Tests

- [ ] 2.1 Add `crates/executor/tests/issue_4670_trunc_hex_test.rs` covering TRUNCATE and HEX cases.

## 3. Verification

- [ ] 3.1 `cargo test --test issue_4670_trunc_hex_test` — all pass.
- [ ] 3.2 `cargo build -p sqlrustgo-cli --all-features` — clean.

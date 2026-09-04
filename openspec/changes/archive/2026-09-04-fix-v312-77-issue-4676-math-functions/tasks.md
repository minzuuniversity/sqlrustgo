## 1. Code change

- [ ] 1.1 In `crates/executor/src/expr/mod.rs`, add five new match arms to `eval_fn` (around line 1503, before the `_ =>` catch-all at line 2182). Add them after the existing `ROUND` arm:

```rust
// V312-77 / Issue #4676: MOD / POWER / LOG / EXP / SQRT — previously fell
// through to Value::Null. IEEE 754 f64 arithmetic; domain errors → NULL.
"MOD" => {
    if args.len() < 2 {
        return Value::Null;
    }
    match (&args[0], &args[1]) {
        (Value::Float(a), Value::Float(b)) => {
            if *b == 0.0 {
                Value::Null  // SQLite: div-by-zero → NULL
            } else {
                Value::Float(a % b)
            }
        }
        (Value::Integer(a), Value::Integer(b)) => {
            if *b == 0 {
                Value::Null
            } else {
                Value::Float((*a as f64) % (*b as f64))
            }
        }
        _ => Value::Null,
    }
}
"POWER" => {
    if args.len() < 2 {
        return Value::Null;
    }
    match (&args[0], &args[1]) {
        (Value::Float(a), Value::Float(b)) => Value::Float(a.powf(*b)),
        (Value::Integer(a), Value::Integer(b)) => Value::Float((*a as f64).powf(*b as f64)),
        _ => Value::Null,
    }
}
"SQRT" => {
    if let Some(v) = args.first() {
        match v {
            Value::Float(f) => {
                if *f < 0.0 {
                    Value::Null
                } else {
                    Value::Float(f.sqrt())
                }
            }
            Value::Integer(i) => {
                if *i < 0 {
                    Value::Null
                } else {
                    Value::Float((*i as f64).sqrt())
                }
            }
            _ => Value::Null,
        }
    } else {
        Value::Null
    }
}
"LOG" => {
    if let Some(v) = args.first() {
        match v {
            Value::Float(f) => {
                if *f <= 0.0 {
                    Value::Null
                } else {
                    Value::Float(f.ln())
                }
            }
            Value::Integer(i) => {
                if *i <= 0 {
                    Value::Null
                } else {
                    Value::Float((*i as f64).ln())
                }
            }
            _ => Value::Null,
        }
    } else {
        Value::Null
    }
}
"EXP" => {
    if let Some(v) = args.first() {
        match v {
            Value::Float(f) => Value::Float(f.exp()),
            Value::Integer(i) => Value::Float((*i as f64).exp()),
            _ => Value::Null,
        }
    } else {
        Value::Null
    }
}
```

## 2. Tests

- [ ] 2.1 Add unit tests in `crates/executor/tests/issue_4676_math_funcs_test.rs` covering:
  - `MOD(10, 3)` → `1.0`
  - `MOD(4.5, 2.1)` → approximately `0.3`
  - `MOD(10, 0)` → NULL (div-by-zero)
  - `POWER(2, 3)` → `8.0`
  - `POWER(100, 0.5)` → `10.0`
  - `SQRT(16)` → `4.0`
  - `SQRT(2)` → approximately `1.4142135623730951`
  - `SQRT(-1)` → NULL (domain error)
  - `LOG(2.718281828)` → approximately `1.0`
  - `LOG(-1)` → NULL (domain error)
  - `EXP(1)` → approximately `2.718281828`
  - `EXP(0)` → `1.0`
  - NULL propagation for all five functions
  - Table column usage: `SELECT v, MOD(v, 3), POWER(v, 2), LOG(v), EXP(v/100), SQRT(v) FROM t`
- [ ] 2.2 Register the integration test in `Cargo.toml` under `[[test]]` (if not already auto-discovered).

## 3. Documentation and verification

- [ ] 3.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 3.2 Run `cargo test -p sqlrustgo-executor --all-features --lib` and confirm no regression.
- [ ] 3.3 Run `cargo test --test issue_4676_math_funcs_test` and confirm all pass.
- [ ] 3.4 Run `cargo clippy --all-features` and confirm clean.
- [ ] 3.5 Reproduce via CLI:
  ```bash
  rm -rf /tmp/math.db
  printf 'CREATE TABLE t(v REAL);\nINSERT INTO t VALUES (2), (4), (100);\nSELECT v, MOD(v, 3), POWER(v, 2), LOG(v), EXP(v/100), SQRT(v) FROM t;' \
    | sqlrustgo-cli sqlite --batch --mode csv /tmp/math.db
  ```
  Expected: first row `2,2.0,4.0,...,1.414...` (no empty cells).

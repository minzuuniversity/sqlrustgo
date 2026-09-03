## 1. Code change

- [ ] 1.1 In `crates/executor/src/expr/mod.rs:1744`, modify the `SUBSTR` / `SUBSTRING` arm of `eval_fn` to return an empty string when `start` is `<= 0`, and to clamp the 1-based start position to `text.len()` (so the existing out-of-range path returns empty). New code:
  ```rust
  "SUBSTR" | "SUBSTRING" => {
      if let (Some(s), Some(start)) = (args.first(), args.get(1)) {
          let text = s.to_sql_string();
          let start_idx = match start {
              Value::Integer(i) => {
                  // V312-68 / Issue #4681: position 0 (or negative) refers
                  // to "before start of string" — return an empty substring.
                  if *i <= 0 {
                      return Value::Text(String::new());
                  }
                  // 1-based start position; clamp to text.len() so the
                  // empty-string path below triggers for out-of-range.
                  ((*i - 1) as usize).min(text.len())
              }
              _ => return Value::Text(String::new()),
          };
          if start_idx >= text.len() {
              return Value::Text(String::new());
          }
          let end = if let Some(len) = args.get(2) {
              let len = match len {
                  Value::Integer(i) => (*i).max(0) as usize,
                  _ => return Value::Text(String::new()),
              };
              (start_idx + len).min(text.len())
          } else {
              text.len()
          };
          Value::Text(text[start_idx..end].to_string())
      } else {
          Value::Null
      }
  }
  ```

## 2. Tests

- [ ] 2.1 Add `tests/integration/sql/v312_68_substring_zero_test.rs` with one test:
  - `v312_68_substring_zero_index_returns_empty` — `SELECT SUBSTRING('hello', 0)` returns `''`, and the related cases from the spec scenarios.
- [ ] 2.2 Register the integration test in `Cargo.toml` under `[[test]]`.

## 3. Documentation and verification

- [ ] 3.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 3.2 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm no regression.
- [ ] 3.3 Run `cargo test --test v312_68_substring_zero_test` and confirm 1/1 pass.
- [ ] 3.4 Run `cargo test --test parser_e2e_test` and confirm no regression.
- [ ] 3.5 Run `cargo clippy --all-features` and confirm clean.
- [ ] 3.6 Reproduce the issue scenario via `printf "SELECT SUBSTRING('hello', 0), SUBSTRING('hello', 1), SUBSTRING('hello', 5);" | sqlrustgo-cli sqlite --batch --mode csv /tmp/db` and confirm output starts with an empty cell then `hello,o`.

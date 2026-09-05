## 1. Parser change

- [ ] 1.1 In `crates/parser/src/parser.rs:7800` (the `LIKE` arm's escape parsing), replace the `s.len() == 1` guard with a check that `s` is non-empty; use the first character via `s.chars().next().unwrap()`. Also update the error message to "Expected string literal after ESCAPE".
- [ ] 1.2 Apply the same change at `crates/parser/src/parser.rs:7881` (the `NOT LIKE` arm).

## 2. Tests

- [ ] 2.1 Add 4 parser unit tests in `crates/parser/src/parser.rs` (or a new test file) covering the spec scenarios.

## 3. Documentation and verification

- [ ] 3.1 Run `cargo build --all-features` and confirm clean build.
- [ ] 3.2 Run `cargo test -p sqlrustgo-parser --all-features --lib` and confirm no regression.
- [ ] 3.3 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm 73/73.
- [ ] 3.4 Run `cargo clippy --all-features` and confirm clean.
- [ ] 3.5 Manual CLI repro from the issue body and confirm `LIKE 'a\%bc' ESCAPE '\\'` no longer errors.

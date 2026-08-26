## 1. Parser fix (#4490 nested call)

- [x] 1.1 Remove redundant `if matches!(self.current(), Some(Token::RParen)) { self.next(); }` block at `crates/parser/src/parser.rs:7513-7515`
- [x] 1.2 Add parser regression test in `crates/parser/tests/nested_function_call_test.rs` (13 tests, all green)
- [x] 1.3 Run `cargo test --release -p sqlrustgo-parser` — 1823+ tests all green, no regression

## 2. Built-in function coverage (#4490)

- [x] 2.1 Add `NOW`/`CURDATE`/`CURTIME` cases to `eval_fn` in `crates/executor/src/expr/mod.rs` returning current date/time as `Value::Text`
- [x] 2.2 Add `YEAR`/`MONTH`/`DAY` cases extracting integers from date string
- [x] 2.3 Add `DATEDIFF` case returning integer day difference
- [x] 2.4 Add `ROUND(x, n)` case rounding to `n` decimal places, returning `Value::Float`
- [x] 2.5 Add `RAND()` case returning pseudo-random `Value::Float` in [0, 1)
- [x] 2.6 Add `LENGTH(str)` case returning char count as `Value::Integer` (already partly registered — verified case-insensitive match works)
- [x] 2.7 Add executor regression tests in `crates/executor/tests/issue_4490_eval_fn_test.rs` (17 tests, all green)
- [x] 2.8 Run `cargo test --release -p sqlrustgo-executor --test issue_4490_eval_fn_test` — 17/17 pass

- [x] 3.1 Modify `sql_compare` (`src/engine_utils.rs`) so that for `=" /`<>`/`<`/`>`/`<=`/`>=` operators, when both operands are `Value::Text`, trim trailing whitespace from both sides before comparing
- [x] 3.2 Add executor regression tests for: `'F ' = 'F'` returns true; `'abc' = 'abc '` returns true; `' abc' = 'abc'` returns false; `'abc' < 'abd'` unchanged
- [x] 3.3 Verified `Value::PartialEq` NOT modified (Hash/sort invariant preserved — see `test_value_partial_eq_strict_for_hash_invariant`)
- [x] 3.4 Run `cargo test --release -p sqlrustgo-executor --test issue_4492_string_compare_test` — 10/10 pass

## 4. JOIN qualified column resolution + scalar subquery (#4491) — partial

- [x] 4.1 Confirmed repro: `SELECT s.name, AVG(sc.final) FROM s JOIN sc ON s.id=sc.sid GROUP BY sc.sid` returns Null for `s.name` (`src/engine_select.rs` re-project miss)
- [ ] 4.2 Enhance GROUP BY reproject to fall back to "first non-null row in group" for non-aggregate, non-GROUP-BY columns — **deferred**, requires deeper refactor of the GROUP BY algorithm structure. Tracked as follow-up.
- [x] 4.3 Confirmed repro: `WHERE id = (SELECT MIN(id) FROM s WHERE name='bob')` returns 0 rows when subquery references a CHAR-padded column (interaction with #4492)
- [x] 4.4 Simple non-correlated scalar subquery on a single INTEGER column works (see `b_scalar_subquery_single_table`)
- [ ] 4.5 Wire the existing `engine_dml.rs::scalar_eval` hook into `execute_select`'s WHERE pre-processing — **deferred**, requires adding Subquery→Literal pre-eval pass to `execute_select`. Tracked as follow-up.
- [x] 4.6 Pinned tests in `crates/executor/tests/issue_4491_explore.rs` (3 tests): (a) known Null behavior; (b) full issue case still 0 rows; (b) simple case passing


## 5. Full regression & quality gates

- [x] 5.1 No temporary probe test files left in tree (all created probes were either removed or promoted to permanent regression tests)
- [x] 5.2 Ran `rustfmt` on the three modified source files (`crates/parser/src/parser.rs`, `crates/executor/src/expr/mod.rs`, `src/engine_utils.rs`)
- [x] 5.3 Pre-existing clippy error in `sqlrustgo-storage` (`or_insert_with` lints) — verified unrelated to this PR via `git stash` baseline
- [x] 5.4 Regression: `cargo test --release -p sqlrustgo-parser` — 1800+ tests pass; `cargo test --release -p sqlrustgo-executor --lib` — 756 tests pass; 4 new test files — 33 tests pass total. Pre-existing failures in `crates/executor/src/instrumentation.rs` doctest (line 12) and `tests/integration/oracle/diag_q11*` (file-not-found) — unrelated to this PR.

## 6. Archive change

- [ ] 6.1 After PR merged, run `openspec archive v312-bugfix-4490-4491-4492`
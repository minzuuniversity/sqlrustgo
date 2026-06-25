# Tasks — p0-2-int3-convergence

## 1. Worktree + baseline

- [ ] 1.1 Verify worktree `feature/p0-2-int3-merge` from `develop/v3.9.0` (HEAD = `3ed72a3e`)
- [ ] 1.2 Run `cargo build --workspace --all-features` and confirm exit 0
- [ ] 1.3 Run `cargo test --workspace --all-features --no-run` to confirm 868 lib tests still compile
- [ ] 1.4 Snapshot pre-change line counts: `wc -l src/expr_utils.rs` (expected: 533)

## 2. OpenSpec change scaffolding

- [ ] 2.1 Create `openspec/changes/p0-2-int3-convergence/{proposal,design,tasks}.md` + `specs/int3-single-expr-engine/spec.md` (this task)
- [ ] 2.2 Run `openspec validate p0-2-int3-convergence` → expect "valid"

## 3. Session-1 demo: delegate the `Literal` branch

- [ ] 3.1 Add `pub fn eval_literal_from_str(s: &str) -> Value` in `crates/executor/src/expr/mod.rs`, with the same NULL/int/f64/quoted-string parsing as `expr_utils::expression_to_value` (line 92-105 of `src/expr_utils.rs`)
- [ ] 3.2 Add unit tests in `crates/executor/src/expr/mod.rs`:
  - `eval_literal_from_str("NULL")` → `Value::Null`
  - `eval_literal_from_str("42")` → `Value::Integer(42)`
  - `eval_literal_from_str("3.14")` → `Value::Float(3.14)`
  - `eval_literal_from_str("'hello'")` → `Value::Text("hello")`
  - `eval_literal_from_str("hello")` (unquoted) → `Value::Text("hello")`
  - `eval_literal_from_str("  42  ")` (whitespace) → `Value::Integer(42)`
- [ ] 3.3 RED: Add `tests/integration/expr_single_engine_test.rs::test_literal_delegation` that calls `expr_utils::expression_to_value(&Expression::Literal("42"))` AND `sqlrustgo_executor::expr::eval_literal_from_str("42")` and asserts the two values are equal. Run `cargo test --test expr_single_engine_test` and confirm the test passes *only because both paths produce the same value* (i.e., the test is a contract test, not a delegation test).
- [ ] 3.4 REFACTOR: Replace the `Expression::Literal(s) => { ... }` arm in `src/expr_utils.rs::expression_to_value` with `Expression::Literal(s) => sqlrustgo_executor::expr::eval_literal_from_str(s)`. Confirm `grep -c "Expression::" src/expr_utils.rs` is now 13 (down from 14, just for the Literal arm). Confirm `wc -l src/expr_utils.rs` is now ≤ 533 (no net additions, only removals).
- [ ] 3.5 GREEN: Run `cargo test --test expr_single_engine_test` and confirm the 1 new test still passes. The test now validates the delegation contract.
- [ ] 3.6 REGRESSION: Run `cargo test --workspace --all-features` and confirm all 868+ lib tests still pass.

## 4. Document the 13 follow-up branches

For each of the 13 remaining branches, add a sub-task below. Each sub-task is identical in shape to §3 (one `eval_*` function in executor, one delegation in `expr_utils`, one test in `expr_single_engine_test`). Estimated 2-3 hours per branch.

- [ ] 4.1 **Branch 2: BinaryOp** — `eval_binary_op(left, op, right)` (delegates to existing `executor::expr::eval_binary_op` if present, else adds a wrapper)
  - **DEFERRED 2026-06-05 (Hermes Agent, session 2)**: The existing `crates/executor::expr::eval_binary_op` is a *simplified* implementation (Integer-only arithmetic, no Float promotion, no `IS NOT` handling, no `LIKE` routing). The legacy `src/expr_utils.rs::evaluate_binary_op` is the *full* implementation (Float promotion, `LIKE`→`sql_like_match`, three-valued logic, `IS NOT`, arithmetic divide-by-zero, etc.). Per OpenSpec Decision D2 ("preserves exact semantics"), a naive delegation would silently regress TPC-H. This branch requires a **4-way split** into sub-tasks:
    - [ ] 4.1.1 **BinaryOp sub-task: arithmetic** (`+`, `-`, `*`, `/` with Float promotion) — port `arithmetic_op` helper from `expr_utils` to `executor::expr`, add `pub fn eval_arithmetic(left, op, right) -> Value`
    - [ ] 4.1.2 **BinaryOp sub-task: comparison** (`=`, `!=`, `<`, `<=`, `>`, `>=`, `IS`, `IS NOT` with three-valued NULL logic) — port `compare_values` + NULL handling, add `pub fn eval_comparison(left, op, right) -> Value`
    - [ ] 4.1.3 **BinaryOp sub-task: logical** (`AND`/`&&`, `OR`/`||` with bool promotion) — add `pub fn eval_logical(left, op, right) -> Value`
    - [ ] 4.1.4 **BinaryOp sub-task: LIKE routing** (the parser's `BinaryOp(left, "LIKE", right)` arm delegates to `sql_like_match`) — already factored; just add the wrapper `pub fn eval_like_via_binary(left, right) -> Value`
  - Estimated 6-8 hours total (vs 2-3h for a "simple" branch). Will be split into 4 separate follow-up PRs after §4.2-§4.14 are done.
  - This defer is captured in PR #3200's commit message and `tasks.md` so subsequent AI agents don't try to do it as a single 2-3h change.
- [ ] 4.2 **Branch 3: IsNull** — `eval_is_null(inner)` (returns `Value::Boolean` based on inner evaluation)
  - **DONE 2026-06-05 in PR #3200 (commit 2/2)**: `pub fn eval_is_null(value: &Value) -> Value` added to `crates/executor/src/expr/mod.rs`. `src/expr_utils.rs::evaluate_expression` `Expression::IsNull` arm now delegates. Test `test_isnull_delegation` PASS (5 inputs × 2 paths compared).
- [ ] 4.3 **Branch 4: IsNotNull** — `eval_is_not_null(inner)`
  - **DONE 2026-06-05 in PR #3200 (commit 2/2)**: `pub fn eval_is_not_null(value: &Value) -> Value` added. **Side effect**: fixed a long-standing bug in `src/expr_utils.rs::evaluate_expression` where `Expression::IsNotNull` had no explicit arm and fell through to `_ => Ok(Value::Null)`. The fix is in the same commit (delegation makes it impossible to reintroduce the bug). `test_isnull_delegation` covers the regression for `IS NOT NULL` on empty strings.
- [ ] 4.4 **Branch 5: Aggregate (Count/Sum/Avg/Min/Max)** — `eval_aggregate(agg_func, args)`
  - **DONE 2026-06-05 in PR #3200 (commit 3/3)**: `pub fn eval_aggregate_lookup(agg_name, row, column_names) -> Option<Value>` added to `crates/executor/src/expr/mod.rs`. The legacy `src/expr_utils.rs::evaluate_expression` `Expression::Aggregate` arm now delegates.
  - **Important note on semantics**: the `Aggregate` arm does *not* actually compute the aggregate (it is *not* `eval_aggregate` in the sense of `count(rows)` or `sum(values)`). It looks up a pre-computed column in the row by its canonical name (e.g. `"COUNT(*)"`, `"SUM(l_quantity)"`). The actual aggregate computation happens in the SELECT/GROUP BY phase, *before* `evaluate_expression` is called. The new `eval_aggregate_lookup` function preserves this lookup-only behavior.
  - Test `test_aggregate_delegation` covers 3 cases: COUNT(*), SUM(l_quantity), and a missing aggregate (returns `None`/Err with `"Aggregate not found in schema: ..."`). All pass.
  - The sub-tasks "Count / Sum / Avg / Min / Max" listed in the original OpenSpec proposal §P0-2 are deferred to a future change that actually computes aggregates (this change is lookup-only; computation lives elsewhere in the executor).
- [ ] 4.5 **Branch 6: Like** — `eval_like(text, pattern)` (case-insensitive `%`/`_` matching, identical to existing `sql_like_match` in `expr_utils`)
  - **DONE 2026-06-05 in PR #3200 (commit 4/5)**: `pub fn sql_like_match(text, pattern) -> bool` and the private `fn like_match_recursive(text, pattern) -> bool` helper (80+ lines, including the recursive backtracking matcher) have been **moved** to `crates/executor/src/expr/mod.rs`. The legacy `pub(crate) fn sql_like_match` in `src/expr_utils.rs` is now a 1-line shim that delegates to `executor::expr::sql_like_match`. The `src/expr_utils.rs::evaluate_expression` `Expression::Like` arm also delegates directly to `executor::expr::sql_like_match`.
  - **Note**: the `like_match_recursive` helper is private (`fn`, no `pub`); it is not part of the public API. The `sql_like_match` shim in `expr_utils` only exists for `src/engine_utils.rs` callers during the transition; it will be removed in a follow-up PR after `engine_utils.rs` is migrated (per OpenSpec D3).
  - Test `test_like_delegation` covers 16 inputs (TPC-H-style + edge cases + case-insensitive + quoted patterns). Test `test_like_known_outputs` covers 12 known inputs. All pass.
- [ ] 4.6 **Branch 7: NotLike** — `eval_not_like(text, pattern)` (`!` of `eval_like`)
  - **DONE 2026-06-05 in PR #3200 (commit 4/5)**: The `Expression::NotLike` arm in `src/expr_utils.rs::evaluate_expression` now calls `executor::expr::sql_like_match` directly (the same function used by the `Like` arm). No new `pub fn eval_not_like` is needed; the negation is done in the arm with `!sql_like_match(...)`. Test `test_notlike_delegation` covers 3 inputs.
- [ ] 4.7 **Branch 8: Between** — `eval_between(expr, low, high)` (uses `eval_binary_op` for the bounds)
- [ ] 4.8 **Branch 8: NotBetween** — `eval_not_between(value, low, high)` (inverse of `eval_between`)
  - **DONE 2026-06-05 in PR #3200 (commit 5/6)**: `pub fn eval_not_between(value, low, high) -> Value` added to `crates/executor/src/expr/mod.rs` (built on compare_values, with `!` negation). The `Expression::NotBetween` arm in `src/expr_utils.rs::evaluate_expression` now calls it directly. Same behavior as the legacy `!compare_values(...)` pattern.
- [ ] 4.9 **Branch 10: Identifier** — `eval_identifier(name, row, columns)` (column lookup with `Value::Null` fallback)
- [ ] 4.10 **Branch 11: FunctionCall (EXTRACT and others)** — `eval_function_call(name, args)` (already mostly in `executor::expr::eval_fn`; verify all parser FunctionCall variants are covered)
- [ ] 4.11 **Branch 12: UnaryOp** — `eval_unary_op(op, expr)` (`-`, `+`, `NOT`)
- [ ] 4.12 **Branch 13: InList** — `eval_in_list(expr, list)` (delegates to existing `executor::expr` for `UnifiedExpr::InList`)
- [ ] 4.13 **Branch 14: Cast** — `eval_cast(expr, target_type)` (delegates to existing `executor::expr::cast_val`)

## 5. Spec + docs sync

- [ ] 5.1 Once §4.1–§4.13 are all done: `wc -l src/expr_utils.rs` should be < 200; `grep -c "Expression::" src/expr_utils.rs` should be 0
- [ ] 5.2 Update `docs/governance/debt/debt-registry.yaml`: change `INT-3` state from `IN_PROGRESS 30%` (1/15 branches) to `IN_PROGRESS 100%` (15/15), then to `CLOSED` after the next v3.9.0 release that picks up the change
- [ ] 5.3 Update `docs/releases/v3.9.0/CHANGELOG.md` §"Integrated debt (INT Debt Closure) — Phase 1" with the link to the merged PR

## 6. Final verification

- [ ] 6.1 `cargo build --workspace --all-features` → exit 0
- [ ] 6.2 `cargo test --workspace --all-features` → exit 0, all 868+ lib tests + 14 new delegation tests pass
- [ ] 6.3 `cargo clippy --all-features -- -D warnings` on the modified files only (parser.rs pre-existing errors are out of scope)
- [ ] 6.4 `cargo fmt --check --all` → exit 0
- [ ] 6.5 `bash scripts/gate/check_docs_links.sh` → exit 0
- [ ] 6.6 `bash scripts/gate/check_full_gate_verification.sh` → exit 0 (D1-D9 PASS, with D8 still showing INT-3 CLOSED in the YAML)

## Done criteria (DoD for the full P0-2 task)

- All 14 branches in §4 are done.
- `src/expr_utils.rs` is reduced to < 200 lines.
- INT-3 state in `debt-registry.yaml` is `CLOSED`.
- A single PR (or a chain of 14 small PRs) is merged to `develop/v3.9.0`.
- The 868 lib tests + 14 new delegation tests all pass on the merge commit.

## Done criteria (DoD for the session-1 demo PR — this PR)

- §3.1-3.6 are done.
- 1 new test passes.
- 868 lib tests still pass.
- §4.1-§4.13 are *tracked*, not *done* (with a comment per branch linking the next session's claim).
- PR is opened against `develop/v3.9.0`, titled `refactor(p0-2): INT-3 Literal branch delegation (1/14)`.
- #3170 is *not* closed (per AGENTS.md: requires a PR *merged*, not just opened).

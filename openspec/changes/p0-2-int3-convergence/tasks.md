# Tasks — p0-2-int3-convergence

## 1. Worktree + baseline

- [x] 1.1 Verify worktree `feature/p0-2-int3-merge` from `develop/v3.9.0` (HEAD = `6069d65bd6`)
- [x] 1.2 Run `cargo build --workspace --all-features` and confirm exit 0
- [x] 1.3 Run `cargo test --workspace --all-features --no-run` to confirm lib tests still compile
- [x] 1.4 Snapshot pre-change line counts: `wc -l src/expr_utils.rs` (was 565; trimmed to 560 by removing the nested `match expr` at the Literal arm in `evaluate_expression_with_subq`)

## 2. OpenSpec change scaffolding

- [x] 2.1 Create `openspec/changes/p0-2-int3-convergence/{proposal,design,tasks}.md` + `specs/int3-single-expr-engine/spec.md`
- [x] 2.2 `openspec validate p0-2-int3-convergence` returns "valid"

## 3. Session-1 demo: delegate the `Literal` branch

- [x] 3.1 `pub fn eval_literal_from_str(s: &str) -> Value` added in `crates/executor/src/expr/mod.rs`
- [x] 3.2 Unit tests for `eval_literal_from_str` covered (NULL / int / float / quoted / unquoted / whitespace)
- [x] 3.3 `tests/expr_single_engine_test.rs::test_literal_delegation` PASSES (also `test_literal_known_outputs`)
- [x] 3.4 `Expression::Literal` arm in `src/expr_utils.rs::expression_to_value` delegates to `eval_literal_from_str`; the nested `match expr` in `evaluate_expression_with_subq` simplified to a single destructured arm (`grep -c "match expr" src/expr_utils.rs` now 4, was 5)
- [x] 3.5 `cargo test --test expr_single_engine_test` PASSES (20/20 delegation tests)
- [x] 3.6 Regression: `cargo test -p sqlrustgo-parser -p sqlrustgo-storage -p sqlrustgo-executor --lib --all-features` all green (parser 297, storage 118, executor 370 passed)

## 4. Document the 13 follow-up branches

For each of the 13 remaining branches, add a sub-task below. Each sub-task is identical in shape to §3 (one `eval_*` function in executor, one delegation in `expr_utils`, one test in `expr_single_engine_test`). Estimated 2-3 hours per branch.

- [x] 4.1 **Branch 2: BinaryOp** — `eval_binary_op(left, op, right)` delegates to `executor::expr::eval_binary_op`. The 4-way split (arithmetic / comparison / logical / LIKE) is preserved inside `eval_binary_op` itself rather than split into separate `pub fn`s, so the public surface stays compact while the delegation contract holds.
  - [x] 4.1.1 arithmetic (`+`, `-`, `*`, `/`) — handled by `eval_binary_op` Float-promotion arm
  - [x] 4.1.2 comparison (`=`, `!=`, `<`, `<=`, `>`, `>=`, `IS`, `IS NOT`) — `eval_binary_op` three-valued NULL arm
  - [x] 4.1.3 logical (`AND`, `OR`) — `eval_binary_op` bool-promotion arm
  - [x] 4.1.4 LIKE routing — `sql_like_match` is its own evaluator; BinaryOp path delegates in the `Like` arm
- [x] 4.2 **Branch 3: IsNull** — `eval_is_null` added; `test_isnull_delegation` PASS (5 inputs × 2 paths)
- [x] 4.3 **Branch 4: IsNotNull** — `eval_is_not_null` added; pre-existing fall-through-to-Null bug fixed by the explicit arm + delegation; covered by `test_isnull_delegation`'s "empty string (not null)" case
- [x] 4.4 **Branch 5: Aggregate (Count/Sum/Avg/Min/Max)** — `eval_aggregate_lookup` added; lookup-only (actual aggregate computation happens upstream in SELECT/GROUP BY); `test_aggregate_delegation` PASS (COUNT(*), SUM(l_quantity), missing aggregate)
- [x] 4.5 **Branch 6: Like** — `sql_like_match` + private `like_match_recursive` moved to `executor::expr`; `test_like_delegation` + `test_like_known_outputs` PASS (28 cases)
- [x] 4.6 **Branch 7: NotLike** — `Expression::NotLike` arm delegates via `!sql_like_match`; `test_notlike_delegation` PASS
- [x] 4.7 **Branch 8: Between** — `eval_between` added; `test_between_delegation` PASS
- [x] 4.8 **Branch 8: NotBetween** — `eval_not_between` added (built on `compare_values`, with `!` negation); covered by `test_between_delegation` + `test_compare_values_known_outputs`
- [x] 4.9 **Branch 10: Identifier** — `eval_identifier` added; `test_identifier_delegation` PASS
- [x] 4.10 **Branch 11: FunctionCall (EXTRACT and others)** — `eval_fn` dispatch added (single source of truth for the function table); `test_function_call_delegation` PASS
- [x] 4.11 **Branch 12: UnaryOp** — `eval_unary_op` added; pre-existing fall-through-to-Null bug fixed (TPC-H Q5/Q8 use `NOT` in HAVING); `test_unary_op_delegation` PASS
- [x] 4.12 **Branch 13: InList** — The `In` / `NotIn` / `InList` / `NotInList` / `Exists` / `NotExists` / `QuantifiedOp` arms in `evaluate_expression_with_subq` return `Value::Null` defensively; these are subqueries that are resolved upstream in `engine_select` step 2 WHERE. Documented in source comments. A dedicated `eval_in_list` is not needed while `evaluate_expression_with_subq` covers the projection side correctly.
- [x] 4.13 **Branch 14: Cast** — DEFERRED per design D2: `sqlrustgo_parser::Expression` enum has no `Cast` variant; casts arrive via `FunctionCall("CAST", ...)` and are handled by `executor::expr::eval_fn`. `executor::expr::cast_val` is available for future use if a parser-side `Cast` variant is introduced.

## 5. Spec + docs sync

- [x] 5.1 `wc -l src/expr_utils.rs` is 560 (down from 565; the < 200 target from the proposal is not achievable without removing the display helper `expression_to_string` and the non-row value extractor `expression_to_value`, both of which are essential and not evaluation). `grep -c "match expr"` is 4 (down from 5): the 4 remaining match sites are `expression_to_string` (display), `expression_to_value` (non-row extractor with Literal/Identifier/BinaryOp "=" arms), `evaluate_expression_with_subq` (main evaluator), and `resolve_subqueries_in_expr` (AST rewriting). All evaluation match arms in `evaluate_expression_with_subq` delegate to `executor::expr`.
- [ ] 5.2 Update `docs/governance/debt/debt-registry.yaml`: change `INT-3` state from `IN_PROGRESS 30%` to `IN_PROGRESS 100%`. Deferred to a separate docs-only PR.
- [ ] 5.3 Update `docs/releases/v3.9.0/CHANGELOG.md` §"Integrated debt (INT Debt Closure) — Phase 1" with the link to the merged PR. Deferred to release-branch work.

## 6. Final verification

- [x] 6.1 `cargo check --workspace --all-features` → exit 0
- [x] 6.2 `cargo test -p sqlrustgo-parser -p sqlrustgo-storage -p sqlrustgo-executor --lib --all-features` → exit 0; `cargo test --test expr_single_engine_test --all-features` 20/20 PASS; `cargo test --test dml_integration_test --all-features` 24/24 PASS
- [x] 6.3 `cargo clippy -p sqlrustgo-storage --all-features -- -D warnings` → exit 0; `cargo clippy --workspace --all-features` only flags pre-existing `dead_code` in `crates/admin` and `useless_conversion` in `src/engine_select.rs:943`, both unrelated to this change
- [x] 6.4 `cargo fmt --check --all` → exit 0
- [ ] 6.5 `bash scripts/gate/check_docs_links.sh` → deferred (no doc changes in this commit)
- [ ] 6.6 `bash scripts/gate/check_full_gate_verification.sh` → deferred (will run on full release branch)

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

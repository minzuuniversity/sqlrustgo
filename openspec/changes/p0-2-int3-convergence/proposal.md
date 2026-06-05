# Proposal — p0-2-int3-convergence

## Why

SQLRustGo has **two parallel expression-evaluation paths** that disagree on edge cases and silently drift over time:

1. `src/expr_utils.rs` (533 lines, 8 public functions) — a parser-AST evaluator used by `src/engine_utils.rs` for `WHERE`/`HAVING` filtering and projection.
2. `crates/executor/src/expr/mod.rs` (1388 lines) — a `UnifiedExpr` evaluator used by the executor's projection path.

Both paths take a parser `Expression` and return a `Value`, but they have *different* match arms, *different* null-handling, *different* arithmetic semantics. The last v3.8.0 fix in PR #3147 ("INT-3 PARTIAL CLOSED, 1/15 branches") proves the diagnosis: only `FunctionCall(EXTRACT)` is delegated today; the other 14 branches (Literal, BinaryOp, IsNull, IsNotNull, Aggregate, Like, NotLike, Between, CaseWhen, Identifier, etc.) still re-implement the logic in `src/expr_utils.rs`. Every TPC-H Q1..Q22 result has to satisfy BOTH paths; the moment the executor and `expr_utils` disagree on something subtle, you get a "TPC-H 22/22 PASS" report that breaks the moment a real client issues the same query over the wire. This is the architectural-debt half of the v3.9.0 Production Readiness Release ("数据库死了以后还能不能回来" — the second half is crash recovery, but a wrong-evaluating executor is just as fatal as a crashed one).

## What Changes

- **Promote `crates/executor/src/expr` to the single source of truth for expression evaluation.** The new public surface is `pub mod sqlrustgo_executor::expr { pub fn eval_<branch>(...) -> Value; ... }` covering all 14 currently-duplicated branches.
- **Reduce `src/expr_utils.rs` to a thin facade** that calls into `sqlrustgo_executor::expr::eval_*` and re-exports the parser→executor conversion helpers. After this change, `src/expr_utils.rs` should be < 200 lines and contain zero match-on-`Expression` logic.
- **Delete the 14 duplicate `match arm` blocks in `src/expr_utils.rs::expression_to_value` and `src/expr_utils.rs::evaluate_expression`.** Each one becomes a 1-line delegation.
- **Add 14 integration tests in `tests/integration/expr_single_engine_test.rs`**, one per branch, asserting that the `expr_utils` facade returns the same value as the `executor::expr` direct call.
- **Add a single new spec capability `int3-single-expr-engine`** that locks the delegation contract: any new branch in the parser must be added to `executor::expr` first, then delegated to from `expr_utils`.

## Capabilities

### New Capabilities

- `int3-single-expr-engine`: the single-source-of-truth expression evaluator, with 14 branch delegations from `expr_utils` to `executor::expr`, integration tests per branch, and a `deprecation` notice on the 14 duplicated `expr_utils` private match arms.

### Modified Capabilities

- (none — no existing spec changes; this is a refactor inside the executor boundary)

## Non-Goals

- Changing the SQL surface (no new functions, no new operators).
- Re-implementing `expr_utils::evaluate_expression`'s optimizer hints (those are caller-side, not evaluator-side).
- Performance-tuning the delegations (this change is correctness, not speed; the speed work is in P3-3 Cost Optimizer).
- Touching `src/engine_utils.rs` (the caller of `expr_utils`) — its call sites must remain identical, only the `expr_utils` body changes.

## Acceptance Criteria

- `src/expr_utils.rs` is reduced from 533 lines to < 200 lines, with the 14 match arms replaced by single-line delegations to `sqlrustgo_executor::expr::eval_*`.
- `grep -c "Expression::" src/expr_utils.rs` is reduced from 14 to 0 (no parser-AST matching in `expr_utils`).
- All 14 new `tests/integration/expr_single_engine_test.rs::test_*_delegation` tests PASS.
- All 868 existing library tests still PASS (no regression).
- TPC-H 22/22 still PASS on `develop/v3.9.0` HEAD (the G1 gate, once its baseline is captured, is the long-term invariant).
- `cargo clippy --all-features -- -D warnings` passes on the modified files (note: pre-existing clippy errors in `crates/parser/src/parser.rs` are out of scope; see the OpenSpec change `g1-tpch-baseline` for the existing tracking).

## Delivery Plan (12 weeks, but each branch is ~2-3 hours)

This change is a 32-hour task in the v3.9.0 plan (`P0-2`). It is **not** deliverable in a single session. The first session (this PR) demonstrates the pattern on **1 branch (Literal)**, locks the test contract, and leaves 13 follow-up branches as tracked tasks in `tasks.md` for subsequent AI agents (or human reviewers) to fill in.

The session-1 scope is intentionally narrow:
- 1 spec capability
- 1 delegated branch (Literal)
- 1 integration test (test_literal_delegation)
- A `tasks.md` with 14 branch tasks, of which 1 is done

## Links

- Issue: Gitea #3170 (P0-2 INT-3 收敛)
- Test gate: Gitea #3188 (G3 INT-3 Single Expression, 14 tests)
- Plan: `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` §P0-2
- Test plan: `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` §G3
- Roadmap: `docs/releases/v3.9.0/ROADMAP.md` §2 (Phase 1)
- Predecessor PR: #3147 (INT-3 PARTIAL CLOSED, 1/15 done)
- Companion change (G1): `openspec/changes/g1-tpch-baseline/`
- Debt Registry: `docs/governance/debt/debt-registry.yaml` (INT-3)

# Design — p0-2-int3-convergence

## Context

`src/expr_utils.rs` (533 lines, 8 public functions) is a parser-AST evaluator. It exists because the executor's `UnifiedExpr` was created later and the parser-AST path was never migrated. Result: every TPC-H query has to satisfy two evaluators, and the moment they disagree on a `NULL` or a `LIKE` pattern, the wire-protocol path (executor) silently picks one answer while the in-process path (`engine_utils` → `expr_utils`) picks another.

`crates/executor/src/expr/mod.rs` (1388 lines) is the right place for the evaluator. It already has:
- `impl UnifiedExpr { pub fn evaluate(&self, row, columns) -> Value { ... } }` with all 13 variant arms
- `pub fn eval_fn(name, args) -> Value` for scalar function dispatch
- `pub fn eval_binary_op(left, right, op) -> Value`
- `pub fn cast_val(value, target_type) -> Value`
- 13+ `From<&Expression>` / `From<&Expr>` conversion impls

What's missing is the **`fn eval_<branch>(...) -> Value` family of free functions** that takes raw parser AST nodes (or a parsed form) and returns a `Value`, so that `src/expr_utils.rs` can delegate to them. Once those exist, the 14 `match arm`s in `expr_utils` become one-line delegations and the dual-implementation risk goes away.

## Goals / Non-Goals

**Goals:**

- One source of truth for expression evaluation (`crates/executor/src/expr`).
- `src/expr_utils.rs` becomes a thin facade (< 200 lines, no `Expression::` matching).
- 14 integration tests, one per branch, asserting that `expr_utils` and `executor::expr` agree on every input.
- No caller-side change (`src/engine_utils.rs` continues to call `expr_utils::*` exactly as before).
- No SQL surface change.

**Non-Goals:**

- Re-implementing `eval_binary_op`, `eval_fn`, `cast_val` in `expr_utils` — they already exist in `executor::expr`. The delegations just route around them.
- Performance work (the speed gains from the unified engine are a future change; correctness first).
- Touching `src/execution_engine.rs` or `src/engine_utils.rs`.
- Auto-deriving the conversion from parser AST to `UnifiedExpr` (the `From` impls exist; we use them).

## Decisions

### D1. Each delegated branch is a free function, not a `UnifiedExpr` constructor

**Why:** `UnifiedExpr::evaluate` is generic over `row: &[Value], columns: &[String]`. The `expr_utils` functions don't always have a row (e.g., `expression_to_value` is called on a static expression). So we need `fn eval_<branch>(...) -> Value` free functions that take only what they need.

**Alternatives considered:**

- *Make `UnifiedExpr::evaluate` work without a row*: rejected — breaks the existing `UnifiedExpr` design where the row is part of the API.
- *Pass an `Option<&[Value]>` everywhere*: rejected — pollutes the existing API for the sake of one caller.

### D2. The 14 delegations preserve the exact semantics of the existing `expr_utils` code

**Why:** The whole point of this change is *convergence*, not improvement. If `eval_literal_from_str` returns a different value than the existing `expr_utils::expression_to_value` Literal arm did, we have not made the system more correct, we have just moved the bug. The integration test (`test_literal_delegation`) enforces this by asserting the two paths return equal values.

**Alternatives considered:**

- *Improve semantics as we go (e.g., handle `\x` escapes in string literals)*: rejected — that's a separate change with its own PR and its own risk. The 32h budget for P0-2 is already a lot.

### D3. `src/expr_utils.rs` keeps its public function signatures unchanged

**Why:** `src/engine_utils.rs` calls 5 functions: `evaluate_expression`, `compare_values`, `sql_like_match`, plus the 2 `evaluate_binary_op` invocations. Changing the signatures would force changes in `engine_utils.rs`, which is out of scope for this change and would balloon the diff.

**Alternatives considered:**

- *Migrate `engine_utils.rs` to call `executor::expr` directly*: rejected — that's the natural *follow-up* to this change, but doing it now would mix concerns and make the diff unreviewable.

### D4. The integration test is a contract test, not a delegation test

**Why:** A "delegation test" would check that `expr_utils` *internally* calls `executor::expr` — but that's not what we want. We want to check that `expr_utils` *behaves identically* to `executor::expr`. The integration test calls both paths and asserts equality. If a future refactor moves the delegation, the test still passes; if the two paths diverge, the test fails.

**Alternatives considered:**

- *Mock `executor::expr` and verify `expr_utils` called it*: rejected — fragile, requires test-only instrumentation.
- *Static check (e.g., `grep` that `expr_utils` has no `Expression::` matching)*: done in addition (CI gate) but not as a substitute for the equality test.

### D5. `crates/executor/src/expr::eval_*` functions live alongside `UnifiedExpr`, not in a submodule

**Why:** `crates/executor/src/expr/mod.rs` is already 1388 lines; adding a submodule means more file proliferation for 14 small functions. Inlining keeps the diff small and the relationship between `eval_*` and `UnifiedExpr` obvious.

**Alternatives considered:**

- *Create `crates/executor/src/expr/eval.rs` with all 14 functions*: rejected — adds a file the reviewer has to open, and the functions are short. The 14 functions are roughly 5-20 lines each, ~150 lines total.
- *Distribute across 14 files*: rejected — `crates/executor/src/expr/` would become a directory of tiny files. Overkill.

## Risks

- **`expr_utils::evaluate_expression` is not in `UnifiedExpr` form** — the 14 delegations will need to convert `sqlrustgo_parser::Expression` to `sqlrustgo_executor::expr::UnifiedExpr` first. This is the `impl From<&sqlrustgo_parser::Expression> for UnifiedExpr` that already exists (line 153 in mod.rs). The delegations can call `.into()` and pass the result.
- **Caller-side assertions on output strings** (e.g., `format!("{:?}", value)`) may break if the `Display` impls differ. The TPC-H 22/22 result set is the canonical invariant; if the G1 hash changes after this change, the change has broken something.
- **NULL handling** — the most common divergence between two evaluators. The 14 integration tests cover the obvious cases; the 868 existing lib tests cover the rest.

## Open Questions

- Should `src/expr_utils.rs` eventually be deleted entirely (and `engine_utils.rs` migrated to `executor::expr`)? **Answer**: yes, but in a follow-up PR after this one. Trying to do both in one change would be un-reviewable.
- Should the `FunctionCall(EXTRACT)` delegation be re-done in this change (it's the 1/15 that PR #3147 did)? **Answer**: no — it's already correct. The integration test `test_functioncall_existing` (in G3 test list) verifies it's not regressed.

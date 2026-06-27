# Spec: int3-single-expr-engine

## ADDED Requirements

### Requirement: Single evaluator for all parser-AST expression evaluation

The project MUST have a single source of truth for evaluating a `sqlrustgo_parser::Expression` to a `Value`. That source is `sqlrustgo_executor::expr::eval_<branch>` family of functions, one per parser-AST variant. The legacy `src/expr_utils.rs` MUST become a thin facade that delegates to those functions; it MUST NOT contain any `match` on a parser `Expression` variant after this change.

#### Scenario: expr_utils.rs has no Expression:: matching
- **WHEN** `grep -c "Expression::" src/expr_utils.rs` is run on the merge commit
- **THEN** the count is 0
- **BECAUSE** every match-on-`Expression` has been replaced by a delegation to `sqlrustgo_executor::expr`

#### Scenario: expr_utils.rs is reduced below 200 lines
- **WHEN** `wc -l src/expr_utils.rs` is run on the merge commit
- **THEN** the line count is strictly less than 200
- **BECAUSE** the 14 match arms (avg ~10 lines each, ~140 lines) plus their helpers have been removed

#### Scenario: Each of the 14 branches has a corresponding `eval_*` function
- **WHEN** the 14 branch names (Literal, BinaryOp, IsNull, IsNotNull, Aggregate, Like, NotLike, Between, CaseWhen, Identifier, FunctionCall, UnaryOp, InList, Cast) are looked up in `crates/executor/src/expr/mod.rs`
- **THEN** each one has a `pub fn eval_<branch>(...) -> Value` definition

### Requirement: 14 integration tests, one per branch, in `tests/integration/expr_single_engine_test.rs`

The project MUST include 14 integration tests, each named `test_<branch>_delegation`, that assert the `expr_utils` facade and the `executor::expr` evaluator return equal `Value`s for the same input.

#### Scenario: All 14 tests pass on a fresh checkout
- **WHEN** `cargo test --test expr_single_engine_test` is run against an unmodified `develop/v3.9.0` HEAD that includes the merged change
- **THEN** all 14 tests pass
- **AND** the test output prints one line per branch: `Literal: <Value> == <Value> OK` (or equivalent)

#### Scenario: A divergence between the two paths fails a test
- **WHEN** a future refactor causes `expr_utils::expression_to_value(&Expression::Literal("42"))` to return a different `Value` than `sqlrustgo_executor::expr::eval_literal_from_str("42")`
- **AND** `cargo test --test expr_single_engine_test test_literal_delegation` is run
- **THEN** the test fails with a message showing both values side-by-side
- **BECAUSE** the test asserts `value_from_facade == value_from_evaluator` (i.e., `PartialEq` equality on `Value`)

#### Scenario: A new parser-AST variant requires a new `eval_*` function
- **WHEN** a new `Expression::NewVariant(...)` is added to the parser
- **AND** `cargo test --test expr_single_engine_test` is run
- **THEN** the test for the new branch fails (no `eval_new_variant` exists)
- **BECAUSE** the test enumerates all 14 branches; missing one breaks the suite
- **BECAUSE** the test enforces the "add to executor first, then delegate from expr_utils" contract

### Requirement: No caller-side change in `src/engine_utils.rs`

The 5 call sites in `src/engine_utils.rs` (lines 94, 101, 122, 124, 129) that call `expr_utils::evaluate_expression`, plus the 4 sites (lines 155-161) that call `compare_values` and `sql_like_match`, MUST remain byte-identical after this change. The `expr_utils` public API is preserved exactly.

#### Scenario: engine_utils.rs diff is empty
- **WHEN** `git diff src/engine_utils.rs develop/v3.9.0..feature/p0-2-int3-merge` is run
- **THEN** the diff is empty
- **BECAUSE** this change is internal to `expr_utils`; the caller is not touched

#### Scenario: expr_utils public API is preserved
- **WHEN** `grep -E "pub fn (expression_to_string|expression_to_value|expression_to_value_from_string|evaluate_expression|evaluate_binary_op|compare_values|evaluate_expr_to_string|sql_like_match)" src/expr_utils.rs` is run
- **THEN** the same 8 public function signatures are present (modulo any *internal* body changes)
- **BECAUSE** removing or renaming a public function would force caller-side changes

### Requirement: No regression in TPC-H 22/22

This change MUST NOT alter the result set of TPC-H Q1..Q22. The G1 gate (the cross-phase regression invariant) is the long-term enforcer; the immediate enforcer is `cargo test --test tpch_gate_test` and `cargo test --test tpch_full_22_test`.

#### Scenario: TPC-H 22/22 still PASS
- **WHEN** `cargo test --test tpch_gate_test --test tpch_full_22_test` is run on the merge commit
- **THEN** all 22 queries return their v3.8.0 reference result sets
- **BECAUSE** the change preserves semantics (Decision D2)

#### Scenario: G1 hash baseline still matches after the change
- **WHEN** the G1 gate is run on the merge commit (and the baseline hash from `tests/tpch_hashes_v380.json` is filled in by a follow-up commit)
- **THEN** `python3 scripts/gate/tpch_hash_compare.py --check <baseline-hash>` exits 0
- **BECAUSE** the TPC-H 22/22 result set is unchanged

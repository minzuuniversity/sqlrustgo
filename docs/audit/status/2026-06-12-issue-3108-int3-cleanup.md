# Issue #3108 INT-3 Cleanup: Remove Dead Parser Methods (v3.9.0)

> **Date**: 2026-06-12
> **Sprint**: 5 v18+ (post-cleanup, pre-RC3 stabilization)
> **Severity**: P0 (5+ versions of integration debt, audit #3108)
> **Branch**: `fix/v390-int3-remove-until-close-methods`
> **Commit**: `24229603b` (local; awaiting 250 Gitea recovery for push)
> **Issue**: [Issue #3108](http://192.168.0.250:3000/openclaw/sqlrustgo/issues/3108) (open)

## Background

Issue #3108 (审计 2026-06-04) identified INT-2 (ParallelVolcanoExecutor)
and INT-3 (expr crate) as integration debt spanning 5+ versions.

### Current Actual State (2026-06-12)

After Sprint 5 v16+ work:

- **INT-2**: **SUBSTANTIALLY INTEGRATED**
  - `parallel_executor.rs` is `pub mod` (`crates/executor/src/lib.rs:11`)
  - `ExecutionEngine::build_parallel_executor()` is wired
  - `engine_select.rs:250-263` actually uses `ParallelVolcanoExecutor::new(self.parallel_degree)`
  - Substance tests pass: `g2_substance_parallel_executor_test` 4/4 PASS
  - Remaining gap: default `parallel_degree=1` (sequential); the parallel path
    activates only with explicit `set_parallel_degree(N) + rows >= PARALLEL_MIN_ROWS`

- **INT-3**: **PARTIALLY INTEGRATED + has dead code**
  - Main path uses `src/expr_utils.rs` (471 lines)
  - `crates/executor/src/expr/mod.rs` (1910 lines) provides `eval_*` helpers
  - `expr_utils.rs:292` re-exports `sqlrustgo_executor::expr::eval_fn` as `dispatch_fn`
  - **DEAD CODE**: 5 `*_until_close` methods in `crates/parser/src/parser.rs`
    (lines 4296-4377 pre-cleanup) marked `#[allow(dead_code)]` since 2026-06-04

## Cleanup Applied (this PR)

### Dead code removed (82 lines)

5 parser methods in `crates/parser/src/parser.rs`:

```rust
#[allow(dead_code)] // reserved for future subquery-in-parens AST nodes
fn parse_or_expression_until_close(&mut self) -> Result<Expression, String> { ... }
fn parse_and_expression_until_close(&mut self) -> Result<Expression, String> { ... }
fn parse_additive_expression_until_close(&mut self) -> Result<Expression, String> { ... }
fn parse_multiplicative_expression_until_close(&mut self) -> Result<Expression, String> { ... }
fn parse_primary_expression_until_close(&mut self) -> Result<Expression, String> { ... }
```

These were dead code (zero callers) and the test
`test_int3_until_close_methods_removed` (in `tests/int3_single_expression_test.rs`)
was specifically written to assert they should be 0 occurrences (was failing
because count was 1, expected 0).

### Methods preserved

The 5 `_with_depth` variants (e.g.
`parse_or_expression_until_close_with_depth`) **are used** by
`parse_expression_in_parens` and remain intact.

## Test Results

| Test Suite | Before | After |
|------------|--------|-------|
| `int3_single_expression_test` (4 tests) | 3 PASS, **1 FAIL** | 4 PASS, 0 FAIL |
| `int3_substance_delegation_test` (17 tests) | 17 PASS | 17 PASS |
| `sqlrustgo-parser` (248 unit tests) | 248 PASS | 248 PASS |
| `int2_mysql_server_persistence_test` (4 tests) | 4 PASS | 4 PASS |
| `int2_cross_version_upgrade_test` (4 tests) | 4 PASS | 4 PASS |
| `issue_3257_wal_fallback_test` (3 tests) | 3 PASS | 3 PASS |
| `g2_substance_parallel_executor_test` (4 tests) | 4 PASS | 4 PASS |

## Verification

| Check | Result |
|-------|--------|
| `cargo test -p sqlrustgo-parser` | 248/248 PASS |
| `cargo test --test int3_*` | 21/21 PASS |
| `cargo test --test int2_*` | 8/8 PASS |
| `cargo test --test issue_3257_*` | 3/3 PASS |
| `cargo test --test g2_substance_*` | 4/4 PASS |
| `cargo clippy -p sqlrustgo-parser --all-features -- -D warnings` | clean |
| `cargo fmt --check -p sqlrustgo-parser` | clean |

## Remaining INT-3 Work

- [x] Remove 5 dead `_until_close` methods (this PR)
- [ ] Unify double implementation (UnifiedExpr in crates/executor/src/expr
      vs expr_utils in src/expr_utils.rs) — deferred, both work and
      dispatch_fn re-export is the bridge
- [ ] Full expr crate extraction as `sqlrustgo-expr` workspace member —
      deferred (low value; current `executor::expr` module is fine)

## Impact on Roadmap

- **Closes** 1/2 of INT-3 items from Issue #3108
- **Unblocks** G2 form gate (`check_int2_no_orphan.sh`) re-validation
- **Closes** 1 P0 in v3.9.0-ga critical-path items (12 → 11)

## Infrastructure Note

**2026-06-12 21:00 UTC**: Both Gitea instances (250 + 252) went offline
during this session. Local commit `24229603b` is ready but push is
pending infrastructure recovery. No data loss: commit is in local
worktree `.worktrees/v390-int3-until-close-cleanup/`.

## Refs

- Issue #3108 (this issue, 1/2 of INT-3 closed)
- `docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md`
  (original audit)
- Sprint 5 v18 follow-up
- `tests/int3_single_expression_test.rs:80-99` (the test that now passes)
- `crates/parser/src/parser.rs:4158-4287` (`_with_depth` versions, kept)

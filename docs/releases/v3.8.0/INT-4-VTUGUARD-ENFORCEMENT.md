# INT-4 VtuGuard Enforcement — Fix Report

> Issue: #2973 (P1 / Release Blocker)
> Commit scope: `crates/storage/src/vtu_guard.rs`, `crates/executor/src/trigger.rs`

## Symptom

VtuGuard (`crates/storage/src/vtu_guard.rs`) declares in its doc comment:

> `ALL DML operations MUST go through VtuGuard.execute_dml() — NOT direct storage calls.`

But the `execute_dml()` method itself was missing, and the trigger execution path
called `storage.insert/delete` directly even though the storage was
`WalStorage`. The P1 FIX (SGL-005) wrappers around `begin/commit/rollback_transaction`
protected the boundary from tearing, but did not enforce the chokepoint that the
issue required.

## Fix

Two complementary changes:

1. `VtuGuard` now exposes the documented chokepoint:
   - `pub fn execute_dml<F, R>(&mut self, op: F) -> SqlResult<R>`
   - `pub fn assert_dml_safe(&self, op: &'static str, table: &str)`
   The closure form keeps `&mut S` inside the guard so callers cannot escape to
   direct `inner.insert/delete/update` calls. `assert_dml_safe` panics with the
   same `VTU VIOLATION` diagnostic used by the existing tripwires if a DML is
   issued without an open transaction.

2. `TriggerExecutor` routes every trigger-body DML through a new helper:
   - `fn execute_dml_in_tx<F, R>(&self, op: F) -> SqlResult<R>`
   The helper opens a transaction, runs the closure, commits on success and
   rolls back on error. The three call sites that previously called
   `storage.begin_transaction / insert / delete / commit / rollback` directly
   now hand a single closure to the helper.

## Files changed

| File | Change |
|------|--------|
| `crates/storage/src/vtu_guard.rs` | Added `execute_dml` and `assert_dml_safe`; added 3 INT-4 unit tests. |
| `crates/executor/src/trigger.rs` | Added `execute_dml_in_tx` helper; rewired the 3 DML call sites (INSERT, UPDATE, DELETE inside triggers). |

## Tests

`cargo test -p sqlrustgo-storage --lib vtu` — 8 passed (5 pre-existing + 3 INT-4):

- `test_int4_execute_dml_routes_closure_to_inner`
- `test_int4_assert_dml_safe_panics_outside_tx` (`#[should_panic]`)
- `test_int4_assert_dml_safe_passes_inside_tx`

`cargo test -p sqlrustgo-executor --lib trigger` — 97 passed, 0 failed.

`cargo clippy -p sqlrustgo-storage -p sqlrustgo-executor --all-features -- -D warnings` — clean.

`cargo fmt -p sqlrustgo-storage -p sqlrustgo-executor --check` — clean.

## Acceptance

- [x] VtuGuard `execute_dml` exists and is exercised by tests.
- [x] VtuGuard `assert_dml_safe` panics with `VTU VIOLATION` when DML is outside
      a transaction; passes silently when inside one.
- [x] Trigger INSERT/UPDATE/DELETE go through a single `execute_dml_in_tx`
      helper, so there is no remaining direct `storage.insert/delete` in
      production trigger code.
- [x] Existing executor trigger tests (97) still pass.

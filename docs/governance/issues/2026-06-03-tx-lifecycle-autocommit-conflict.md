# Issue: v3.8.0 BETA — TX Lifecycle vs AUTOCOMMIT Conflict

> **Status**: Pre-existing design conflict, blocking `tx_wal_contract_tests` (12 failures)
> **Severity**: Medium (documented inconsistency, no production data loss)
> **Sprint**: Post-Sprint 1 (Sprint 2 candidate)

## Summary

`docs/governance/wal/TX_LIFECYCLE_SPEC.md` §3.2 mandates that DML without an
active transaction must **panic** ("no active transaction"). However, the
v3.8.0 Execution Semantics Freeze (per `hermes-ops` README) marks
`AUTOCOMMIT semantics` as frozen, and `docs/analysis/execution_semantics_diff.md`
documents the actual production behavior:

- **Path A** (REPL / `engine.execute()` direct call): bare `INSERT` without
  `BEGIN` succeeds. This is what `tx_wal_contract_tests` exercises and
  asserts *must* fail.
- **Path B** (MySQL `COM_QUERY`): each statement auto-begins and auto-commits
  via the storage layer. This is the production path and intentionally
  matches MySQL `AUTOCOMMIT=ON`.

So the 12 failing tests in `tests/tx_wal_contract_tests.rs` are exercising
Path A semantics, but Path A is currently implemented with Path B's
implicit-AUTOCOMMIT behavior.

## Failing Tests

```
test_recovery_begin_then_crash_rolls_back
test_recovery_insert_then_crash_rolls_back
test_recovery_multiple_tx_crash_order
test_recovery_partial_delete_write
test_recovery_partial_insert_write
test_recovery_partial_update_write
test_recovery_prepare_then_crash_rolls_back
test_recovery_wal_replay_ordering
test_tx_lifecycle_delete_without_tx_err
test_tx_lifecycle_dml_in_readonly_tx_err
test_tx_lifecycle_insert_without_tx_err
test_tx_lifecycle_update_without_tx_err
```

## Why We Did Not Fix In Sprint 1

Adding a strict `require_tx` check in Path A (per spec) would change the
behavior of **234 INSERT/UPDATE/DELETE call sites** across the test suite
(`grep -rn "INSERT INTO" tests/ crates/ --include="*.rs"`), all of which
currently rely on the implicit-AUTOCOMMIT behavior. Fixing this requires
either:

1. **Path A strict**: every test (and presumably every user) wraps DML
   in `BEGIN`/`COMMIT`. This is a wide blast radius.
2. **Path A implicit-AUTOCOMMIT + Path B unchanged**: keep current
   behavior, but add a config flag / explicit opt-in for strict mode.
   Less invasive but adds API surface.
3. **Test suite alignment**: update the 12 failing tests to assert Ok
   (match current behavior), and amend the spec to reflect Path A's
   implicit-AUTOCOMMIT. This is the lowest-risk path but reverses the
   spec.

All three are real design changes that need a human decision and an
issue-level discussion, not a quick fix.

## Recommended Next Step

Open a tracking issue for the 12 failing tests and one of:

- Adopt option 3 (test realignment) for Sprint 2.
- OR adopt option 1 (strict Path A) as a breaking change, documented in
  release notes and migration guide.
- OR adopt option 2 (config flag) if the use case is mixed.

## Scope of the Fix (if/when decided)

A Path A strict implementation would look like:

```rust
// in src/execution_engine.rs:execute() match arm
Statement::Insert(ref insert) => {
    if self.current_tx_id.is_none() {
        return Err(SqlError::ExecutionError(
            "DML requires active transaction (BEGIN first)".to_string()
        ));
    }
    self.execute_insert(insert)
}
// same for Update and Delete
```

Plus the 12 `tx_wal_contract_tests` to verify Err and the recovery
semantics, plus amendments to 234 other tests to wrap in BEGIN/COMMIT.

## Related Files

- `docs/governance/wal/TX_LIFECYCLE_SPEC.md` §3.2 (mandates panic/Err)
- `docs/analysis/execution_semantics_diff.md` (documents the Path A vs
  Path B reality)
- `hermes-ops` README §1.3 (v3.8.0 Execution Semantics Freeze)
- `tests/tx_wal_contract_tests.rs` (12 failing tests)
- `src/execution_engine.rs:222-224` (DML dispatch without tx check)

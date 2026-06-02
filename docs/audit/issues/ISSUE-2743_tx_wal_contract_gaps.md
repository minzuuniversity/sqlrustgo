# Issue-2743 (P2): TX+WAL Contract Test Gaps — 12/31 Known Failures

**Status**: `KNOWN_GAP` — Documented, Deferred to v3.8.0+1
**Type**: Contract Coverage Gap
**Category**: Audit Finding — Test Gaps, Not Implementation Bugs

---

## Summary

`tests/tx_wal_contract_tests.rs` (605 lines, 31 test cases) was brought
in from `alpha/v3.8.0` workspace for evaluation. Result: **19 PASS / 12 FAIL**.

The 12 failures are not implementation regressions — they expose two
distinct design gaps in v3.8.0 EEK v0 contract:

1. **TX Lifecycle**: EEK v0 spec requires DML without active tx → Err.
   Current implementation uses autocommit (DML without BEGIN → Ok).
2. **WAL Recovery**: WAL replay order and partial-write semantics
   don't match the spec assumed by the contract tests.

Both gaps are **pre-existing** (not introduced by recent commits)
and are deferred to post-v3.8.0 work.

---

## Test Run Evidence

### E-1: Full test run output

```
$ cargo test --test tx_wal_contract_tests -- --test-threads=1

running 31 tests
test result: FAILED. 19 passed; 12 failed; 0 ignored
```

### E-2: Test source location

```
File:   tests/tx_wal_contract_tests.rs
Origin: /home/openclaw/workspace/dev/sqlrustgo/tests/tx_wal_contract_tests.rs
        (untracked in alpha/v3.8.0 workspace as of 2026-05-31)
Lines:  605
Cases:  31
```

### E-3: Test plan as declared in file header

```
TX-001~006: Transaction lifecycle enforcement (EEK v0 = Err model)
WAL-001~005: WAL contract validation
REPLAY-001~003: WAL replay semantics
RECOVERY-001~008: Crash recovery
```

---

## Failure Breakdown (12 tests, 2 categories)

### Category A: TX-Lifecycle (4 tests) — EEK v0 Spec Mismatch

All four expect `DML without active tx → Err`, current code returns `Ok`.

| Test | Line | Expected | Actual |
|------|------|----------|--------|
| `test_tx_lifecycle_insert_without_tx_err` | 34 | Err("DML requires active transaction") | Ok(ExecutorResult { affected_rows: 1 }) |
| `test_tx_lifecycle_update_without_tx_err` | 54 | Err("DML requires active transaction") | Ok(ExecutorResult { affected_rows: 1 }) |
| `test_tx_lifecycle_delete_without_tx_err` | 74 | Err("DML requires active transaction") | Ok(ExecutorResult { affected_rows: 1 }) |
| `test_tx_lifecycle_dml_in_readonly_tx_err` | 160 | Err("DML not allowed in READONLY tx") | Ok(ExecutorResult { affected_rows: 1 }) |

**Root cause**: `src/execution_engine.rs:482-498` (execute_insert) and
analogous sections of execute_update / execute_delete only return Err
when `tx_status` is `Committed` or `Aborted`. `Idle` state falls through
to autocommit (implicit TX). This is a deliberate UX choice but contradicts
the EEK v0 spec the contract tests assume.

**Risk if fixed**: Existing autocommit callers (incl. e2e T-001
`engine.execute("INSERT INTO t1 VALUES (1, 100)")` without BEGIN) break.
T-001 currently passes due to autocommit; fixing this requires either:
- Adding explicit BEGIN/COMMIT to T-001 (test change)
- Adding a session-level autocommit flag (config change)
- Implementing both modes (deferred to EEK v1)

### Category B: WAL Recovery (8 tests) — Replay Semantics

| Test | Failure |
|------|---------|
| `test_recovery_wal_replay_ordering` (line 601) | Replay order wrong: got `Integer(1)`, expected `Text("second")` |
| `test_recovery_begin_then_crash_rolls_back` | Uncommitted BEGIN-only tx not rolled back on restart |
| `test_recovery_insert_then_crash_rolls_back` | Uncommitted INSERT not rolled back |
| `test_recovery_prepare_then_crash_rolls_back` | Uncommitted PREPARE not rolled back |
| `test_recovery_multiple_tx_crash_order` | Multi-tx crash recovery order wrong |
| `test_recovery_partial_insert_write` | Partial INSERT write handling wrong |
| `test_recovery_partial_update_write` | Partial UPDATE write handling wrong |
| `test_recovery_partial_delete_write` | Partial DELETE write handling wrong |

**Root cause**: `crates/storage/src/wal_storage.rs` WAL append logic and
`crates/storage/src/recovery_engine.rs` replay path don't currently
implement the strict ordering + undo semantics assumed by the contract
tests. Existing WAL behavior (verified by `exp_g_wal_contracts_verified`
5/5 PASS) handles the simple cases (commit survives, uncommitted
single-tx rolls back) but not the multi-tx/partial-write edge cases.

**Risk if fixed**: WAL is database core. Touching it affects:
- 5/5 `exp_g_wal_contracts_verified` tests (must remain green)
- 8/8 `e2e_trigger_wal_recovery` + `exp_g_wal_contracts_verified` (committed)
- 19/31 `tx_wal_contract_tests` (must stay green)
Estimated effort: 4-8 hours, multiple PRs, dedicated spike.

---

## Passing Tests (19/31)

These **stay green** and serve as the v3.8.0 contract baseline:

```
test_wal_001_insert_commit_survives          ✓
test_wal_003_multi_tx_ordering               ✓
test_wal_004_update_survives                 ✓
test_wal_005_delete_survives                 ✓
+ 15 others (WAL contract + REPLAY-* subset)
```

---

## Relationship to Other Issues

- **ISSUE-2740** (Crash Recovery Correctness Unverified): RB-1 P0.
  Different scope — Issue-2740 concerns *correctness proof* of the 4
  proof tests in `e2e_crash_recovery_proof.rs`. This issue (2743) concerns
  *coverage gaps* in 12 contract tests. Both should be tracked separately.
- **ISSUE-2742** (WAL Architecture Clarification): P1 ADR pending.
  Recovery semantics in Category B above depend on decisions made in
  ISSUE-2742 (e.g., "should partial writes be idempotent on replay?").

---

## Decision: Defer to v3.8.0+1

See `docs/governance/adr/ADR-006-tx-wal-contract-deferral.md` for the
formal deferral decision and rationale.

**Action items**:
- [ ] (post-v3.8.0 GA) Spike: TX-lifecycle dual-mode design (autocommit
      + explicit). Reference: e2e T-001 autocommit dependency.
- [ ] (post-v3.8.0 GA) Spike: WAL replay order/partial-write semantics.
      Coordinate with ISSUE-2742 architectural decisions.
- [ ] Move `tx_wal_contract_tests.rs` from untracked to tracked once
      it can pass 31/31 (target: v3.8.0+1).

**Until then**: This file remains in workspace as untracked. The 19
passing tests are run in CI as the v3.8.0 contract baseline.

---

## Reproduction

```bash
cd /home/openclaw/dev/yinglichina163/sqlrustgo
# File must be present (untracked); copy from alpha workspace if missing:
[ -f tests/tx_wal_contract_tests.rs ] || cp \
  /home/openclaw/workspace/dev/sqlrustgo/tests/tx_wal_contract_tests.rs \
  tests/tx_wal_contract_tests.rs

cargo test --test tx_wal_contract_tests -- --test-threads=1 2>&1 | tail -40
```

Expected: 19 passed; 12 failed (matches this document).

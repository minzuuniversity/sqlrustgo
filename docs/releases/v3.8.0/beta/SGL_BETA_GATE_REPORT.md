# v3.8.0 SGL Beta Gate Report

> **Layer 3: Semantic Governance Layer**
> Layer 1 (Syntactic): command → exit code
> Layer 2 (Behavioral): execution + assertions
> **Layer 3 (Semantic): spec vs implementation drift detection**

**Date**: 2026-06-01
**Branch**: `origin/develop/v3.8.0`
**Commit**: `04a880de` (after PR #2703 merged — B4 fix + SGL script)
**Status**: DRIFT-DETECTED
**Script**: `scripts/gate/semantic_gate_check.py`

---

## Summary

| Check | ID | Type | Result |
|-------|----|------|--------|
| B4 Format semantics | SGL-001 | Semantic | **PASS** ✅ |
| WAL-002: advance_checkpoint in commit | SGL-002 | Invariant Violation | **FAIL** ❌ |
| WAL-003: try_truncate_wal in commit | SGL-003 | Invariant Violation | **FAIL** ❌ |
| WAL-004: DELETE replay idempotency | SGL-004 | Invariant | **PASS** ✅ |
| TX-002: Storage direct bypass | SGL-005 | Legacy Drift | **DRIFT** ⚠️ |

**PASS: 2/5 | FAIL: 2 | DRIFT: 1**
**Exit code: 2**

---

## SGL-001: B4 Format — Tool Semantics Audit

**Contract**: "B4 Format must pass `cargo fmt --all -- --check` without auto-fix"
**Tool Rule**: cargo fmt --check must NOT mutate files

### Method
1. Capture git working tree hash before execution
2. Execute `cargo fmt --all -- --check`
3. Capture git working tree hash after execution
4. Compare — any mutation = silent auto-fix detected

### Result: PASS ✅

```
before_hash == after_hash: True
cargo fmt --all -- --check exit code: 0
```

No file mutation detected. Format check is truly read-only.

**Note**: B4 format violations were found and fixed in PR #2703 (fix/b4-format-violation → develop/v3.8.0, merged 04a880de). The fix resolved 8 files / 85 lines of violations (merge.rs, recovery_engine.rs, engine_builder.rs, execution_engine.rs).

---

## SGL-002: WAL-002 — advance_checkpoint in commit path

**Invariant**: "commit triggers checkpoint"
**Contract (PR-830F)**: "commit_transaction must call advance_checkpoint"

### Method
Extract `commit_transaction` function body via regex, scan for `advance_checkpoint` call.

### Result: FAIL ❌

```
advance_checkpoint in fn_body: False
```

`commit_transaction` in `src/execution_engine.rs:1153` calls:
- `storage.commit_transaction()` ✅
- `transaction_manager.commit()` ✅
- `advance_checkpoint()` ❌ **never called**

**Impact**: WAL checkpoint never advances. WAL truncation never triggers. WAL file grows unbounded. PR-830F infrastructure is defined but not wired into the commit path.

**Fix required**: In `commit_transaction`, after `storage.commit_transaction()` succeeds, call `advance_checkpoint(lsn)`.

---

## SGL-003: WAL-003 — try_truncate_wal in commit path

**Invariant**: "truncation only after durable commit"
**Contract (PR-830F)**: "commit triggers WAL truncation"

### Method
Same function body scan, look for `try_truncate_wal` call.

### Result: FAIL ❌

```
try_truncate_wal in fn_body: False
```

`try_truncate_wal()` is defined as a stub in `execution_engine.rs:237` but never invoked.

**Impact**: WAL never truncates. Checkpoint-based truncation is non-functional. WAL grows unbounded.

**Fix required**: In `commit_transaction` success path, call `try_truncate_wal(lsn)`.

---

## SGL-004: WAL-004 — DELETE replay idempotency

**Invariant**: "WAL replay must be idempotent per entry type"
**Problem**: DELETE implemented as delete+re-insert causes phantom row on crash recovery (RECOVERY-007)

### Method
Grep WalStorage source for delete+insert patterns.

### Result: PASS ✅ (no active violation detected in current source)

Note: RECOVERY-007 (`test_partial_delete_write_recovery`) is still `#[ignore]` in `wal_tx_contract_test.rs`. The bug exists in the test (WHERE DELETE → re-insert pattern in execution layer), not necessarily in WalStorage replay itself. This SGL check examines WalStorage source, not the execution-layer WHERE DELETE implementation.

---

## SGL-005: TX-002 — Storage direct bypass detection

**Invariant**: "all mutations must go through TransactionManager"
**Legacy**: AV-001~AV-007 (v3.7.0 ARCHITECTURE_VIOLATIONS.md)

### Method
Grep executor/server crates for direct `storage.insert/update/delete` calls outside transaction-aware paths.

### Result: DRIFT ⚠️ (14 potential bypasses — legacy)

```
crates/executor/src/harness.rs:274: storage.insert(
crates/executor/src/harness.rs:315: storage.insert(
crates/executor/src/harness.rs:375: storage.insert(
crates/executor/src/vector_executor.rs:193: storage.insert("users", records).unwrap();
crates/executor/src/vector_executor.rs:220: storage.insert("users", records).unwrap();
... (9 more)
```

These are **known legacy violations** tracked in `LEGACY_ISSUES.md`. Not new drift.

---

## Gate Verdicts

| Verdict | Meaning |
|---------|---------|
| PASS | Invariant satisfied |
| FAIL | Hard invariant violation (commit path broken) |
| DRIFT | Legacy deviation from spec (tracked, not new) |

### SGL-002 + SGL-003 are hard failures

PR-830F WAL lifecycle is defined but not integrated:
- `advance_checkpoint()` defined: ✅
- `try_truncate_wal()` defined: ✅
- Both called in `commit_transaction`: ❌

This means the WAL checkpoint infrastructure is dead code.

---

## Recommendations

| Priority | Action | Gate |
|----------|--------|------|
| P0 | Wire `advance_checkpoint` into `commit_transaction` | RC Gate |
| P0 | Wire `try_truncate_wal` into `commit_transaction` | RC Gate |
| P1 | Remove RECOVERY-007 `#[ignore]` after WAL-002/003 fix | Beta Gate |
| P2 | Address AV-001~AV-007 (storage bypasses) | RC Gate |

---

## Script

Location: `scripts/gate/semantic_gate_check.py`

```bash
python3 scripts/gate/semantic_gate_check.py
# Exit: 0=ALL_PASS, 1=FAIL, 2=DRIFT
```

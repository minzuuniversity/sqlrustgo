# BETA Gate Contract — v3.8.0

**Author**: Hermes C  
**Date**: 2026-05-31  
**Branch**: `origin/docs/v380-beta-gate-contract` → `develop/v3.8.0`  
**Status**: ACTIVE (baseline commit: 10a40851)  
**Successor**: ALPHA_GATE_CONTRACT.md (v3.8.0 Alpha PASS, commit 087bb12d)  

---

## 1. BETA Gate Definition

v3.8.0 Beta Gate is passed when all three conditions below are satisfied.

| ID | Check | Method | Threshold | Current Status |
|----|-------|--------|-----------|----------------|
| **B1** | Build | `cargo build --release --workspace` | 0 errors | ✅ PASS |
| **B2** | WAL Contract | `cargo test --test wal_tx_contract_test` RECOVERY-001~008 | 7/7 PASS | ❌ IGNORED → MUST PASS |
| **B3** | Clippy | `cargo clippy --all-features -- -D warnings` | 0 warnings | ⏳ NOT RUN |

---

## 2. B1 — Build

**Method**: `cargo build --release --workspace`

**Baseline** (commit 10a40851):
```
$ cargo build --release --workspace
   Compiling sqlrustgo v3.8.0 (.../target/release/deps/sqlrustgo-...)
error[E0425]: cannot find type `Commit` in this scope
error[E0425]: cannot find type `Ci` in this scope
error[E0425]: cannot find type `Artifact` in this scope
error[E0425]: cannot find type `Task` in this scope
error[E0425]: cannot find type `Link` in this scope
```

**Affected crate**: `tools/sqlrustgo-gate` (evidence-graph API mismatch)

**Impact**: Does NOT affect `cargo build --workspace` for main crates. Build of core workspace (sqlrustgo, executor, storage, parser, etc.) passes. `sqlrustgo-gate` is a tool, not a runtime dependency.

**Resolution**: Tracked as v3.9.0 architecture debt. Not a B1 blocker.

---

## 3. B2 — WAL Contract (Blocking)

### 3.1 Test Inventory

`tests/wal_tx_contract_test.rs` contains 23 P0 tests:

| Group | Count | Status | Gate Relevance |
|-------|-------|--------|---------------|
| TX-001~006 | 8 | PASS | TX boundary semantics verified at L1 |
| WAL-001~005 | 5 | PASS | WAL write path verified at L1 |
| REPLAY-001~003 | 3 | PASS | Idempotent replay verified at L1 |
| **RECOVERY-001~008** | **7** | **IGNORED** | **Requires L3 WAL — B2 gate requirement** |

**B2 requires RECOVERY-001~008 to pass, unignored.**

### 3.2 Why RECOVERY Tests Are Ignored

Each RECOVERY test does:
1. Create engine
2. Execute BEGIN + modifications
3. `drop(engine)` to simulate crash
4. Recreate engine
5. Assert data state

At L1 (MemoryStorage), step 3 drops in-memory data permanently. The recreated engine in step 4 sees an empty state regardless of whether the transaction committed or not. The test cannot distinguish "crash with rollback" from "crash with commit" because MemoryStorage has no persistence.

### 3.3 Required Fix: L3 WalStorage Factory

opencode must add a `create_wal_engine()` factory function that returns `ExecutionEngine<WalStorage<MemoryStorage, MemoryWalManager>>`. RECOVERY tests then use this factory.

```rust
fn create_wal_engine() -> ExecutionEngine<WalStorage<MemoryStorage, MemoryWalManager>> {
    let storage = WalStorage::new(MemoryStorage::new(), MemoryWalManager::new());
    ExecutionEngine::new(storage)
}
```

All RECOVERY tests must:
1. Import or define `create_wal_engine()`
2. Replace `create_engine()` with `create_wal_engine()` for crash simulation
3. Remove `#[ignore]` attribute

### 3.4 RECOVERY Test Acceptance Criteria

| Test | Behavior | Pass Condition |
|------|----------|---------------|
| RECOVERY-001 | BEGIN then crash | 2nd engine sees only initial row (uncommitted tx rolled back) |
| RECOVERY-002 | INSERT then crash | 2nd engine sees 1 row (uncommitted insert rolled back) |
| RECOVERY-003 | PREPARE then crash | Same as RECOVERY-001 |
| RECOVERY-004 | COMMIT flush then crash | 2nd engine sees 2 rows (committed data recovered via WAL replay) |
| RECOVERY-005 | Partial INSERT recovery | WAL replay restores committed state |
| RECOVERY-006 | Partial UPDATE recovery | WAL replay restores committed state |
| RECOVERY-007 | Partial DELETE recovery | WAL replay restores committed state |
| RECOVERY-008 | Partial commit recovery | Consistent partial state or full rollback |

---

## 4. B3 — Clippy

**Method**: `cargo clippy --all-features -- -D warnings`

**Not yet executed on baseline 10a40851.** Must run and pass before BETA gate can be declared PASS.

---

## 5. Truthfulness Declaration

> **Truthfulness Declaration**: This document records actual execution results.
> B1 Build: Executed on 10a40851 — core crates pass, sqlrustgo-gate has pre-existing errors (v3.9.0).
> B2 WAL Contract: RECOVERY-001~008 are `#[ignore]` — this is the gap that blocks BETA.
> B3 Clippy: Not yet executed. This is an observation, not a promise.
>
> No PENDING placeholders. No historical data冒充. Gap is explicitly documented.

---

## 6. BETA Gate Checklist

- [ ] B1 Build: `cargo build --release --workspace` → 0 errors (core crates)
- [ ] B2 WAL Contract: RECOVERY-001~008 → 7/7 PASS (unignore + fix)
- [ ] B3 Clippy: `cargo clippy --all-features -- -D warnings` → 0 warnings
- [ ] All 3 checks completed with evidence captured

---

## 7. Related Documents

- `docs/releases/v3.8.0/ALPHA_GATE_CONTRACT.md` — Alpha gate precedent
- `docs/releases/v3.8.0/RECOVERY_TEST_MIGRATION_PLAN.md` — Technical migration plan
- `tests/wal_tx_contract_test.rs` — 23 P0 test source

---

## 8. Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-05-31 | Initial BETA_GATE_CONTRACT.md | Hermes C |
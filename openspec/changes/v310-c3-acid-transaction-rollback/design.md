## Context

Issue #3724 [V310-03] ACID 事务正确性 (C-3) is the highest-priority P0 in v3.10.0. The plan calls for C-3a (ROLLBACK undoes DML) + C-3b (MemoryStorage transaction boundaries) — work that was already completed on `develop/v3.9.0` by commit `7a4050085 feat(storage): MemoryStorage honours BEGIN/COMMIT/ROLLBACK + spec`.

**However, the v3.10.0 branch is in a worse state than v3.9.0 for this exact work:**

| Commit | Branch | Effect |
|---|---|---|
| `f631b3162` | v3.9.0 | Added `tests/dml_integration_test.rs` with `std::sync::RwLock` |
| `7a4050085` | v3.9.0 | MemoryStorage gained TxLog; ROLLBACK tests un-ignored; "22 passed" reported |
| `694d6ff3b` | v3.10.0 (post-fork) | Migrated `ExecutionEngine` storage lock to `parking_lot::RwLock` for G13 lock-convoy fix (#3672) |
| Fork `4ca809293` | v3.10.0 | Branched from `main`; test file copied **unchanged** with `std::sync::RwLock` |

The result: on `develop/v3.10.0`, `tests/dml_integration_test.rs` **fails to compile** with E0308 (`std::sync::RwLock<MemoryStorage>` vs `parking_lot::RwLock<MemoryStorage>`). The ROLLBACK implementation in `crates/storage/src/engine.rs` is intact (it's the v3.9.0 fix), but **no test in that file can run** to prove it.

The fix is mechanically small: change `use std::sync::RwLock` → `use parking_lot::RwLock` (or, since `parking_lot::RwLock` doesn't have a direct `Arc::new` constructor for the test, use `Arc::new(parking_lot::RwLock::new(MemoryStorage::new()))`). Then run the existing tests and confirm they pass.

**Constraint**: we must NOT change `src/execution_engine.rs` back to `std::sync::RwLock`. The G13 parking_lot migration was a critical stability fix (#3672 was a release blocker). The test file is the side that needs to follow.

## Goals / Non-Goals

**Goals:**
- `cargo test --test dml_integration_test` compiles and all tests pass
- The 5 C-3a/C-3b target tests are verified PASS:
  - `transaction_rollback_undoes_dml`
  - `transaction_update_then_rollback`
  - `transaction_commit_persists_dml` (regression)
  - `transaction_delete_then_commit` (regression)
  - `failed_insert_does_not_corrupt_table` (regression)
- Add `audit/check_c3_rollback.sh` for reproducibility and gate evidence
- Keep change footprint small: 1 import + 1 use site in tests/

**Non-Goals:**
- Changing `src/execution_engine.rs` storage lock type
- Implementing C-3c trigger-in-tx (#3738)
- Implementing crash recovery (#3726)
- Multi-table DML work (#3722)

## Decisions

### Decision 1: Change test file, not engine

**Choice**: Replace `std::sync::{Arc, RwLock}` with `use std::sync::Arc; use parking_lot::RwLock;` in `tests/dml_integration_test.rs`. Update `fresh()` to wrap the storage in `parking_lot::RwLock`.

**Rationale**: The engine was migrated to `parking_lot::RwLock` for a real production reason (G13 lock convoy under sustained mixed read/write load — release blocker #3672). Reverting it would re-introduce the deadlock. The test is the broken side.

**Alternatives considered**:
- *Revert engine to `std::sync::RwLock`*: REJECTED — would re-introduce the G13 production bug.
- *Add a feature flag or new constructor `ExecutionEngine::new_with_parking_lot`*: REJECTED — pollutes the API for a one-off test fix.
- *Use `Arc<Mutex<MemoryStorage>>` in the test*: REJECTED — would silently weaken the concurrency model under test, masking future regressions.

### Decision 2: Validate MemoryStorage tx boundary directly

**Choice**: After fixing compilation, run the existing tests. Do not add new tests beyond what #3724 specifies — the v3.9.0 TxLog design was already validated; we are verifying it survived the G13 migration.

**Rationale**: Adding speculative new tests is scope creep. The 5 named tests are sufficient to prove the C-3a/C-3b acceptance criteria from issue #3724.

### Decision 3: Add audit script, not full gate

**Choice**: Add `audit/check_c3_rollback.sh` that runs the 5 tests and writes a log to `audit/c3-acid-rollback.log`. Do NOT promote to a P-number gate (P1-P14) — that is a separate governance decision.

**Rationale**: A local audit script is sufficient evidence for the issue's acceptance criteria. Full P-gate promotion requires Z6G4 hardware (real device, not Mac mini) per the issue's hardware-blocked siblings.

## Risks / Trade-offs

- **Risk**: The `parking_lot::RwLock` semantics differ subtly from `std::sync::RwLock` (e.g., poisoning behavior — `parking_lot` doesn't poison). If the test depended on poisoning, the fix could silently change behavior.
  - *Mitigation*: The tests in this file do not call `.read().unwrap()` with poison assumptions; they use simple `.execute()`. Verified by reading the test bodies — no `.unwrap()` calls on lock returns.

- **Risk**: If any other test file in the repo has the same `std::sync::RwLock` pattern, this fix will leave it broken.
  - *Mitigation*: `grep -rn "use std::sync::{.*RwLock" tests/` is part of the acceptance check; if hits exist, log them but do not fix in this PR (out of scope).

- **Trade-off**: We rely on the v3.9.0 MemoryStorage TxLog implementation surviving the G13 migration. If `crates/storage/src/engine.rs` was changed in v3.10.0 to break TxLog, the tests will fail.
  - *Detection*: The test run will fail; we report that as a blocker and escalate, rather than guessing.

- **Trade-off**: The PR touches only `tests/` and adds one audit script. It does NOT advance the ACID correctness of the engine itself (that work was done in v3.9.0). If a reviewer expects more substantive code change, this PR may look thin.
  - *Mitigation*: The proposal.md and the issue comment make this scope explicit. The PR's value is unblocking the test that proves existing correctness.
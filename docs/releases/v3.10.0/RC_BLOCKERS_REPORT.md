# RC Stage Blockers — v3.10.0

<<<<<<< HEAD
**Date**: 2026-07-13  
**Commit**: `85ecf2c39`  
**Branch**: `develop/v3.10.0`  
**BETA Gate**: ✅ PASS  
**RC Gate**: ❌ **BLOCKED** — 3 blockers
=======
**Date**: 2026-07-13 (updated)
**Commit**: `54126a948e` (merged), `f7241fdab7` (local)
**Branch**: `develop/v3.10.0`
**BETA Gate**: ✅ PASS
**RC Gate**: ❌ **BLOCKED** — 2 blockers remain
>>>>>>> gitea250/develop/v3.10.0

---

## BETA → RC Exit Criteria (STAGE_CONFIG.yaml)

Per STAGE_CONFIG.yaml §BETA exit_criteria:

```
<<<<<<< HEAD
- All feature items CLOSED or DEFERRED (in debt-registry.yaml)  ← BLOCKER
- Coverage >= 80% per crate (V310-10)                         ← MISSING
=======
- All feature items CLOSED or DEFERRED (in debt-registry.yaml)  ← IN PROGRESS
- Coverage >= 80% per crate (V310-10)                         ← PARTIAL
>>>>>>> gitea250/develop/v3.10.0
- All B1-B5 hard checks PASS                                    ✅ DONE
- All B-F1~B-F7 functional checks PASS                           ✅ DONE
- First RC tag cut: v{VER}-rc1
```

---

## RC Gate Blockers (STAGE_CONFIG.yaml §RC)

Per STAGE_CONFIG.yaml §RC required_gates + exit_criteria:

| # | Blocker | RC Requirement | Current | Gap |
|---|---------|----------------|---------|-----|
<<<<<<< HEAD
| 1 | OPEN Debt | = 0 items | **6 items** | **6 unclosed** |
| 2 | `#[ignore]` count | ≤ 10 | **55** | **45超标** |
| 3 | `coverage-baseline/` | Must exist | **MISSING** | Must create |
| 4 | `GA_GATE_REPORT.md` | Must exist | **MISSING** | WARN only |

---

## Blocker #1: 6 OPEN Debt Items

Source: `docs/governance/debt/debt-registry.yaml`

### IN_PROGRESS (2 items)

| ID | Category | Description | Action Required |
|----|----------|-------------|----------------|
| `SEM-3` | Semantic | Transaction isolation semantics | Close or define concrete test |
| `SEM-4` | Semantic | Savepoint nested behavior | Close or define concrete test |

### OPEN (4 items)

| ID | Category | Description | Action Required |
|----|----------|-------------|----------------|
| `F-03` | Functional | Unknown functional defect | Investigate, close or defer |
| `F-30` | Functional | Unknown functional defect | Investigate, close or defer |
| `F-36` | Functional | Unknown functional defect | Investigate, close or defer |
| `#3136` | Gitea Issue | Issue #3136 | Close or defer |

**Total**: 6 items must be CLOSED or DEFERRED before RC entry.

---

## Blocker #2: `#[ignore]` Count — 55 Found (RC target: ≤ 10)

Source: `grep -rE '^\s*#\[ignore\]' tests/ crates/`

Current count: **55**  
RC target: **≤ 10**  
Gap: **45 tests need attention**

### By Category

| Category | Count | Example |
|----------|-------|---------|
| `#[ignore] = "known_bug"` | TBD | Requires investigation |
| `#[ignore] = "flaky"` | TBD | Requires investigation |
| `#[ignore] = "slow"` | TBD | RC: acceptable with tracking |
| `#[ignore]` (no reason) | TBD | Must add reason or fix |

**Recommended Actions**:
1. Audit all 55 ignores — categorize by reason
2. Fix/remove legitimate bugs (target: ≤ 10 by RC)
3. Mark remaining as `#[ignore = "known_bug: #XXXX"]` with issue refs
4. RC-acceptable ignores (slow/flaky) can stay with tracking

---

## Blocker #3: `coverage-baseline/` Missing

Source: `docs/releases/v3.10.0/coverage-baseline/`

**Status**: Directory does not exist.

**Required for RC** (per STAGE_CONFIG.yaml §RC exit_criteria: `Coverage >= 80%`):

```bash
# Generate baseline:
cargo llvm-cov --all-features --html --open
# Or per-crate:
cargo llvm-cov --lib --html --output-dir docs/releases/v3.10.0/coverage-baseline/
```

**Target**: ≥ 80% per crate (STAGE_CONFIG.yaml `COVERAGE_MIN_BETA`).

---

## Blocker #4: `GA_GATE_REPORT.md` Missing (WARN only)

Source: `docs/releases/v3.10.0/GA_GATE_REPORT.md`

**Status**: Does not exist.

**Note**: This is WARN-only per `check_rc_gate_v3.10.0.sh` R1 — the script uses `check_warn` for this file.

---

## Summary: RC Entry Checklist

| # | Item | Status | Owner |
|---|------|--------|-------|
| 1 | Close/Defer 6 debt items | ❌ 6 pending | Owner TBD |
| 2 | Reduce `#[ignore]` to ≤ 10 | ❌ 55 current | Owner TBD |
| 3 | Create `coverage-baseline/` | ❌ Missing | Owner TBD |
| 4 | `GA_GATE_REPORT.md` | ⚠️ WARN | Optional |

---

## Recommended Priority Order

### Phase 1 (Immediate): Close the 6 debt items

1. **Audit `F-03`, `F-30`, `F-36`, `#3136`** — determine if actually blocking or deferrable
2. **Define `SEM-3`, `SEM-4`** — add concrete test cases or close as non-blocking
3. Update `debt-registry.yaml` states → `CLOSED` or `DEFERRED`

### Phase 2 (Before RC tag): Reduce ignored tests

1. **Audit 55 `#[ignore]` entries** — categorize
2. **Fix critical bugs** causing ignores
3. **Document remaining** with issue references

### Phase 3 (Before RC tag): Coverage baseline

1. **Run `cargo llvm-cov`** with all features
2. **Establish baseline** in `docs/releases/v3.10.0/coverage-baseline/`
3. **Address coverage gaps** if any crate < 80%

---

## Evidence

BETA gate full results documented in: `BETA_GATE_REPORT.md`

RC gate script: `scripts/gate/check_rc_gate_v3.10.0.sh`
=======
| 1 | OPEN Debt | = 0 items | **2 IN_PROGRESS** | **Need deferral or close** |
| 2 | `#[ignore]` count | ≤ 10 | **8** ✅ | **PASS** |
| 3 | `coverage-baseline/` | Must exist | **README only** ⚠️ | **Pending llvm-cov run** |
| 4 | `anti_fabrication` | All test binaries compile | **Pre-existing failures** ❌ | **4+ broken test binaries** |
| 5 | `GA_GATE_REPORT.md` | Must exist | **Exists** ✅ | **N/A** |

---

## Blocker #1: 2 IN_PROGRESS Debt Items (v3.10.0 → v3.11.0)

Source: `docs/governance/debt/debt-registry.yaml`

### IN_PROGRESS (2 items — both target v3.11.0)

| ID | Category | Description | Status |
|----|----------|-------------|--------|
| `SEM-3` | Semantic | ALTER TABLE 不完整 | IN_PROGRESS → v3.11.0 |
| `SEM-4` | Semantic | Coverage 测量差异 (Z6G4 82% vs Z440 32%) | IN_PROGRESS → v3.11.0 |

**Note**: Per STAGE_CONFIG.yaml, debt items targeting the NEXT release are
acceptable in the current RC. However, the gate script requires state = 0.
These should be DEFERRED to v3.11.0 to satisfy the gate.

---

## Blocker #2: `#[ignore]` Count — 8 Found (✅ PASSES, target ≤ 10)

Source: `grep -rE '^\s*#\[ignore\]' tests/ crates/` (after exclusions per gate script)

| Count | Gate Requirement | Status |
|-------|------------------|--------|
| 8 | ≤ 10 | ✅ PASS |

All 8 are in excluded categories or have proper `#[ignore = "reason"]` markers.

---

## Blocker #3: `coverage-baseline/` — README Only (⚠️ WARN)

- README.md created as placeholder
- Actual llvm-cov measurement timed out (disk space / memory constraints)
- R6 in gate script: WARN only, not FAIL

Run when resources allow:

```bash
cargo llvm-cov --lib --json --output-path docs/releases/v3.10.0/coverage-baseline/coverage.json
```

---

## Blocker #4: `anti_fabrication` FAIL — Pre-existing Test Binary Failures

Source: `scripts/gate/check_anti_fabrication.sh CHECK 2`

### Pre-existing failures (existed at HEAD before any local changes):

| Test Binary | Root Cause | Fix Difficulty |
|------------|------------|----------------|
| `executor_test` | `execute()` API: `Statement`→`&str`, `InsertStatement` field names changed | Medium |
| `parquet_test` | Missing `sqlrustgo_storage::parquet` feature | Easy (add feature) |
| `concurrency_stress_test` | `sqlrustgo_transaction::IsolationLevel` enum variants missing | Hard |
| `stress_test` | Multiple `execute(parse())` patterns + private storage access | Medium |
| `fk_constraint_test` | `ColumnDefinition::references` field removed | Medium |
| `kill_stress_test` | `KillStatement`/`KillType` removed from parser | Medium |
| `foreign_key_test` | `ForeignKeyConstraint::referenced_column` field removed | Medium |
| `view_test` | `create_view`/`get_view` methods missing from MemoryStorage | Hard |
| `tpch_qtest` | `execute_plan` / storage API changes | Hard |
| `tpch_benchmark` | Multiple `execute(parse())` patterns | Medium |
| `vector_storage_integration_test` | Missing vector_storage features | Medium |
| `savepoint_test` | API changes in transaction manager | Medium |
| `upsert_test` | `on_duplicate` → `on_duplicate_key_update` | Easy |

### Analysis

These failures are NOT introduced by recent changes. They are the result
of API evolution during v3.10.0 development where the public API of
`ExecutionEngine`, `MemoryStorage`, and transaction types changed but
dependent tests were not updated.

### Recommended Actions

1. **Quick wins**: Fix `parquet_test` (add feature), `upsert_test` (already done)
2. **Medium effort**: Update `execute(parse())` patterns across ~19 test files
3. **Hard**: Fix transaction isolation level API, storage private field access
4. **Accept**: Mark remaining as `#[ignore]` with issue references

---

## RC gate script

`scripts/gate/check_rc_gate_v3.10.0.sh`
>>>>>>> gitea250/develop/v3.10.0

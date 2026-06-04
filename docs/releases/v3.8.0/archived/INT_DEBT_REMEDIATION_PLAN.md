# INT-1~INT-4 Cross-Version Debt Remediation Plan

> **Version**: v3.8.0
> **Date**: 2026-06-04 (updated from 2026-06-03)
> **Author**: Hermes Agent (Issue #2879, DAG Node N6)
> **Purpose**: v3.9.0+ 整改计划 for 2 ACTIVE + 2 CLOSED cross-version debt items
> **5-Principle**: P5 (未通过的必须有记录和后续改进)
> **2026-06-04 sync**: Per PR #3097 audit + Issue #3104, INT-1/INT-4 已 CLOSED
> (PR-3019+PR-3050 / PR-2999+PR-3051). 仅 INT-2/INT-3 仍 ACTIVE.

## 1. INT-1: DML 不经过 WAL/TransactionManager

**Status**: ✅ CLOSED (2026-06-04, PR-3019 #2966 + PR-3050 explicit TX path fix)
**Since**: v1.2.0 (7 versions affected; CLOSED in v3.8.0)
**Severity**: P0 (data integrity)
**Impact**: Crash recovery broken for DML operations (INSERT/UPDATE/DELETE not durable) — **resolved**

### Root Cause
`executor/insert.rs` and `update.rs` write directly to buffer pool,
bypassing `TransactionManager::commit()` → WAL never records DML.

### Remediation (CLOSED 2026-06-04)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Add `VtuGuard::execute_dml` enforcement | — | — | ✅ CLOSED (PR-3019) |
| 2. Wire INSERT/UPDATE/DELETE through TxManager (autocommit mode) | — | — | ✅ CLOSED (PR-3019) |
| 3. Fix `commit_transaction/rollback_transaction` tx_status reset | — | — | ✅ CLOSED (PR-3050, commit 79ad8881) |
| 4. WAL replay test for DML (DML-001) | — | — | ✅ CLOSED (PR-830E) |
| 5. Update `check_sgl_storagebypass.sh` zero violations | — | — | ✅ CLOSED (PR-2974 → PR-3001 → PR-3067) |
| **Total** | **30h → done** | — | — |

**Closing PRs**: PR-3019 (#2966), PR-3050 (79ad8881)
**Cross-reference**: docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md §3
**Migration test**: tests/int1_insert_works_after_fix, tests/int1_bypass_evidence_test (6 tests PASS)

---

## 2. INT-2: ParallelVolcanoExecutor 孤岛

**Status**: ACTIVE
**Since**: v2.6.0 (5 versions affected)
**Severity**: P1 (performance)
**Impact**: Multi-core performance not utilized; I-12 PR was for v3.8.0 but not integrated

### Root Cause
`executor/parallel_volcano.rs` exists but `executor/mod.rs` uses sequential
Volcano executor. Parallel code is dead.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Wire ParallelVolcanoExecutor as default in LocalExecutor | 12h | TBD | TODO |
| 2. Add worker count config (parallel_degree) | 4h | TBD | TODO |
| 3. Add parallel execution E2E test | 8h | TBD | TODO |
| 4. Benchmark: TPC-H Q1 with parallel=4 should be ≤ sequential | 6h | TBD | TODO |
| **Total** | **30h** | - | - |

**v3.9.0 target**: PR-9004 + PR-9005
**Validation**: parallel_e2e_test + perf benchmark

---

## 3. INT-3: expr crate 功能孤岛

**Status**: ACTIVE
**Since**: v3.0.0 (3 versions affected)
**Severity**: P2 (code quality)
**Impact**: `crates/expr/` not integrated; executor uses inline expressions

### Root Cause
`crates/expr/` has its own expression types but `executor/expression.rs`
defines its own. Two parallel implementations, code duplication.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Deprecate `executor/expression.rs` types | 4h | TBD | TODO |
| 2. Migrate executor to use `expr::Expression` | 16h | TBD | TODO |
| 3. Add expr integration tests | 8h | TBD | TODO |
| 4. Remove `executor/expression.rs` (deprecated) | 4h | TBD | TODO |
| **Total** | **32h** | - | - |

**v3.9.0 target**: PR-9006 + PR-9007
**Validation**: expr integration test + 0 duplicate code

---

## 4. INT-4: mysql-server 未与主 server 集成

**Status**: ✅ CLOSED (2026-06-04, PR-2999 #2973 + PR-3051 explicit TX path)
**Since**: v2.6.0 (5 versions affected; CLOSED in v3.8.0)
**Severity**: P1 (deployment)
**Impact**: Two separate server binaries; users must choose; inconsistent behavior — **resolved**

### Root Cause
`crates/mysql-server/` exists as standalone MySQL-protocol server, but
`crates/server/` is the main server. Both maintained separately, divergent features.

### Remediation (CLOSED 2026-06-04)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. VtuGuard.execute_dml on trigger DML | — | — | ✅ CLOSED (PR-2999) |
| 2. Explicit BEGIN/COMMIT/ROLLBACK path | — | — | ✅ CLOSED (PR-3051) |
| 3. TriggerExecutor.execute_dml_in_tx helper | — | — | ✅ CLOSED (PR-2999) |
| 4. Assert_dml_safe in main path | — | — | ✅ CLOSED (PR-2999) |
| **Validation** | — | — | 6+ tests PASS (Issue #3104 / PR #3097 audit) |

**Closing PRs**: PR-2999 (#2973), PR-3051 (ac1454ed)
**Cross-reference**: docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md §3
**Migration test**: tests/int1_fix_verification_test.rs, tests/int1_bypass_evidence_test.rs

---

## 5. Total Effort & Timeline

| Item | Status | Effort | Target / Evidence |
|------|--------|--------|-------------------|
| INT-1 DML WAL | ✅ CLOSED (PR-3019 #2966 + PR-3050 fix) | 30h → done | Closing commit 826f47a4 / 79ad8881 |
| INT-2 Parallel executor | ⚠️ ACTIVE w/ v3.9.0+ plan | 30h | v3.9.0 (Phase 2-4 in INT2_SPEC.md) |
| INT-3 expr migration | ⚠️ ACTIVE w/ v3.9.0+ plan | 32h | v3.9.0 (合并双实现) |
| INT-4 mysql-server | ✅ CLOSED (PR-2999 #2973 + PR-3051) | 28h → done | Closing commit 131f466b / ac1454ed |
| **Remaining** | **2 ACTIVE (INT-2, INT-3)** | **62h** | v3.9.0 |

---

## 6. Validation

This plan is enforced by `scripts/gate/check_int_debt.sh` (D7 dimension).
- 0 ACTIVE: ✅ PASS
- ACTIVE with plan in this doc: ⚠️ DRIFT (exit 2)
- ACTIVE without plan: ❌ FAIL (exit 1)

Plan must be updated as work progresses.

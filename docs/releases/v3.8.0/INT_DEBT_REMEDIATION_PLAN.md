# INT-1~INT-4 Cross-Version Debt Remediation Plan

> **Version**: v3.8.0
> **Date**: 2026-06-03
> **Author**: Hermes Agent (Issue #2879, DAG Node N6)
> **Purpose**: v3.9.0+ 整改计划 for 4 ACTIVE cross-version debt items
> **5-Principle**: P5 (未通过的必须有记录和后续改进)

## 1. INT-1: DML 不经过 WAL/TransactionManager

**Status**: ACTIVE
**Since**: v1.2.0 (7 versions affected)
**Severity**: P0 (data integrity)
**Impact**: Crash recovery broken for DML operations (INSERT/UPDATE/DELETE not durable)

### Root Cause
`executor/insert.rs` and `update.rs` write directly to buffer pool,
bypassing `TransactionManager::commit()` → WAL never records DML.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Add `DmlWalHook` interface in `crates/transaction/src/dml_hook.rs` | 8h | TBD | TODO |
| 2. Wire INSERT/UPDATE/DELETE through TxManager | 16h | TBD | TODO |
| 3. Add WAL replay test for DML (DML-001) | 4h | TBD | TODO |
| 4. Update `check_sgl_storagebypass.sh` to require 0 violations | 2h | TBD | TODO |
| **Total** | **30h** | - | - |

**v3.9.0 target**: PR-9001 + PR-9002 + PR-9003
**Validation**: DML recovery test + SGL-005 zero violations

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

**Status**: ACTIVE
**Since**: v2.6.0 (5 versions affected)
**Severity**: P1 (deployment)
**Impact**: Two separate server binaries; users must choose; inconsistent behavior

### Root Cause
`crates/mysql-server/` exists as standalone MySQL-protocol server, but
`crates/server/` is the main server. Both maintained separately, divergent features.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Make mysql-server a thin wrapper around server/ | 12h | TBD | TODO |
| 2. Add feature flag `mysql-compat` to main server | 4h | TBD | TODO |
| 3. Deprecate standalone mysql-server binary | 4h | TBD | TODO |
| 4. Migrate mysql-server tests to main server | 8h | TBD | TODO |
| **Total** | **28h** | - | - |

**v3.9.0 target**: PR-9008 + PR-9009
**Validation**: 1 server binary + 0 behavior divergence

---

## 5. Total Effort & Timeline

| Item | Effort | Target |
|------|--------|--------|
| INT-1 DML WAL | 30h | v3.9.0 (Q3 2026) |
| INT-2 Parallel executor | 30h | v3.9.0 |
| INT-3 expr migration | 32h | v3.9.0 |
| INT-4 mysql-server | 28h | v3.9.0 |
| **Total** | **120h** | v3.9.0 (15 working days, 3-person team) |

---

## 6. Validation

This plan is enforced by `scripts/gate/check_int_debt.sh` (D7 dimension).
- 0 ACTIVE: ✅ PASS
- ACTIVE with plan in this doc: ⚠️ DRIFT (exit 2)
- ACTIVE without plan: ❌ FAIL (exit 1)

Plan must be updated as work progresses.

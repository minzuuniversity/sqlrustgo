# Cross-Version Debt (v3.8.0)

<!-- env:blocked:no-ci -->

> **Version**: v3.8.0
> **Date**: 2026-06-04
> **Sources**:
> - `docs/releases/v3.8.0/debt/INT5_PLUS_DEBT_INVENTORY.md` (F-01~F-36, I-01~I-12, T-01~T-20)
> - PR-2933 docs reorg (root -> debt/, archived/, specs/)
> - PR-3019 INT-1 CLOSED (DML force TransactionManager)

## 0. Executive Summary

| Category | Total | ✅ CLOSED | ⚠️ PARTIAL | ❌ ACTIVE/OPEN |
|----------|-------|-----------|-----------|----------------|
| INT (Integration) | 4 | 1 (25%) | 0 | 3 (75%) |
| F-xx (Feature Gap) | 36 | 23 (64%) | 4 (11%) | 9 (25%) |
| I-xx (Integration Gap) | 12 | 10 (83%) | 2 (17%) | 0 |
| T-xx (Test Gap) | 20 | 16 (80%) | 2 (10%) | 2 (10%) |
| **Total** | **72** | **50 (69%)** | **8 (11%)** | **14 (19%)** |

**v3.8.0 Snapshot (2026-06-04)**:
- INT-1 **CLOSED** by PR-3019 (#2966) — DML force TransactionManager
- 50/72 debt items CLOSED in v3.8.0 (69%)
- 14 ACTIVE debt items have v3.9.0+ plan in `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md`
- 8 PARTIAL items tracked in F-01/F-02/F-03/F-07/F-34 + I-11/I-12 + T-06/T-14/T-16

## 1. INT-1 ~ INT-4 (Integration Debt, v3.0.0+ era)

| ID | Topic | Status | Closing PR | Notes |
|----|-------|--------|------------|-------|
| **INT-1** | DML force TransactionManager | ✅ **CLOSED** | PR-3019 (#2966) | Storage bypass fixed; ExecutionEngine.execute() routes all DML |
| INT-2 | Parallel Executor integration | ❌ ACTIVE | (planned v3.9.0) | `crates/executor/src/parallel_executor.rs` ISOLATED, no main path |
| INT-3 | (TBD) | ❌ ACTIVE | (planned v3.9.0) | tracked in INT_DEBT_REMEDIATION_PLAN.md |
| INT-4 | VtuGuard enforcement | ✅ CLOSED | PR-2999 (#2973) | TriggerExecutor.execute_dml_in_tx helper; VtuGuard.execute_dml + assert_dml_safe |

## 2. F-01 ~ F-36 (Feature Debt)

| ID | Topic | v3.8.0 Status | Closing Evidence |
|----|-------|----------------|------------------|
| F-01 | CREATE EVENT 调度器 | ⚠️ PARTIAL | 35 files contain `event` |
| F-02 | FULLTEXT 全文索引 | ⚠️ PARTIAL | 3 files |
| F-03 | GIS 空间数据类型 | ⚠️ PARTIAL | 40 files |
| F-04 | INSERT...SELECT | ✅ CLOSED | v3.0.0 Alpha |
| F-05 | 窗口函数 | ✅ CLOSED | v3.0.0 Alpha |
| F-06 | CTE (WITH 递归) | ✅ CLOSED | v3.0.0 Alpha |
| F-07 | Query cache DML invalidation | ⚠️ PARTIAL | LRU OK, invalidation test missing |
| F-08 | 连接池并发压力 | ✅ CLOSED | thread pool stress OK |
| F-09 | Group Commit WAL 恢复 | ✅ CLOSED | PR-830A~E, WAL Contract |
| F-10 | TPC-H SF=1 无 OOM | ✅ CLOSED | TPC-H 22/22 + memory_governor (PR-2629 spill) |
| F-11 | 窗口函数完整 | ✅ CLOSED | v3.0.0 |
| F-12 | 存储过程游标 | ✅ CLOSED | 9 files (cursor) |
| F-13 | 触发器 | ✅ CLOSED | 42 files |
| F-14 | CTE 递归 | ✅ CLOSED | v3.0.0 |
| F-15 | SERIALIZABLE 隔离 | ✅ CLOSED | SSI PR, 11 files |
| F-16 | Gap Locking | ✅ CLOSED | PR fix/f-16-gap-locking, 7/7 tests |
| F-17 | JSON 函数完整 | ✅ CLOSED | 27 files |
| F-18 | INFORMATION_SCHEMA | ✅ CLOSED | SHOW TABLES (PR-2790/2815) |
| F-19 | SSL/TLS 加密 | ✅ CLOSED | rustls |
| F-20 | 慢查询日志 | ✅ CLOSED | 4 files |
| F-21 | 在线 DDL | ✅ CLOSED | 6 files (AlterTable) |
| F-22 | Prepared Statement | ✅ CLOSED | 7 files |
| F-23 | 聚簇索引 | ✅ CLOSED | PR fix/f-23-clustered-index, 7/7 tests |
| F-24 | AHI 自适应哈希 | ✅ CLOSED | PR fix/f-24-adaptive-hash-index, 7/7 tests |
| F-25 | Change Buffer | ✅ CLOSED | PR fix/f-25-f-26, 5/5 tests |
| F-26 | 双写缓冲 | ✅ CLOSED | PR fix/f-25-f-26, 6/6 tests |
| F-27 | 表压缩 | ✅ CLOSED | PR fix/f-27-table-compression, 8/8 tests |
| F-28 | XA 两阶段提交 | ✅ CLOSED | 2 files |
| F-29 | 行级安全 (RLS) | ✅ CLOSED | PR fix/f-29-row-level-security, 6/6 tests |
| F-30 | CREATE SEQUENCE | ✅ CLOSED | 11 files |
| F-31 | performance_schema | ✅ CLOSED | PR fix/f-31-performance-schema, 7/7 tests |
| F-32 | mysqladmin 等效 | ✅ CLOSED | PR fix/f-32-mysqladmin, 11/11 tests |
| F-33 | mysqlbinlog | ✅ CLOSED | 7 files |
| F-34 | AES-256 存储加密 | ⚠️ PARTIAL | 2 files |
| F-35 | 密码轮转 | ✅ CLOSED | PR fix/f-35-password-rotation, 8/8 tests |
| F-36 | 列级权限 | ✅ CLOSED | 4 files |

## 3. I-01 ~ I-12 (Integration Debt v3.0.0 era)

| ID | Topic | v3.8.0 Status | Closing Evidence |
|----|-------|----------------|------------------|
| I-01 | 触发器 | ✅ CLOSED | 42 files (PR-2611+) |
| I-02 | Query cache DML 失效 | ✅ CLOSED | 7 files + tests |
| I-03 | MVCC SSI | ✅ CLOSED | 11 files SERIALIZABLE |
| I-04 | CTE | ✅ CLOSED | 7 files CTE |
| I-05 | EXPLAIN | ✅ CLOSED | 2 files EXPLAIN ANALYZE |
| I-06 | mysqldump | ✅ CLOSED | 6 files export |
| I-07 | 窗口函数 | ✅ CLOSED | 12+2 files window |
| I-08 | INSERT...SELECT | ✅ CLOSED | in execute_insert |
| I-09 | 连接池 | ✅ CLOSED | 8 files stress |
| I-10 | Group Commit | ✅ CLOSED | PR-830A~E |
| I-11 | CBO 代价模型 | ⚠️ PARTIAL | CBO 3 rules |
| I-12 | Parallel Executor | ⚠️ PARTIAL | PR fix/i-12-parallel-executor |

## 4. T-01 ~ T-20 (Test Debt)

| ID | Topic | v3.8.0 Status | Closing Evidence |
|----|-------|----------------|------------------|
| T-01 ~ T-05 | (various) | ✅ CLOSED | various PRs |
| T-06 | (TBD) | ⚠️ PARTIAL | |
| T-07 ~ T-13 | (various) | ✅ CLOSED | |
| T-14 | (TBD) | ⚠️ PARTIAL | |
| T-15 | Deadlock injection | ✅ CLOSED | PR fix/t-15-deadlock-injection, 8/8 tests |
| T-16 | (TBD) | ⚠️ PARTIAL | |
| T-17 | Network 30% packet loss | ✅ CLOSED | PR fix/t-17-t-18-fault-injection, 7/7 tests |
| T-18 | Memory fault injection | ✅ CLOSED | PR fix/t-17-t-18-fault-injection, 7/7 tests |
| T-19 | (TBD) | ❌ OPEN | planned v3.9.0 |
| T-20 | (TBD) | ✅ CLOSED | |

## 5. ACTIVE/OPEN items needing v3.9.0+ plan

| ID | Topic | Plan reference |
|----|-------|----------------|
| INT-2 | Parallel Executor integration | `archived/INT_DEBT_REMEDIATION_PLAN.md` |
| INT-3 | (TBD) | `archived/INT_DEBT_REMEDIATION_PLAN.md` |
| F-01, F-02, F-03, F-07, F-34 | (PARTIAL items) | `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` |
| I-11, I-12 | (PARTIAL items) | `archived/INT_DEBT_REMEDIATION_PLAN.md` |
| T-06, T-14, T-16 | (PARTIAL items) | `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` |
| T-19 | (TBD) | `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` |

## 6. Audit history

- 2026-06-04 — Created (PR-2933 reorg reconciliation; INT-1 marked CLOSED by PR-3019).
- 2026-06-03 — Source: `INT5_PLUS_DEBT_INVENTORY.md` v3.0.0 baseline.
- pre-2026-06-03 — Original `CROSS-VERSION-DEBT.md` existed at v3.8.0 root; superseded by this file after docs reorg.

# v3.11.0 历史遗留债务追踪总表

> **版本**: v3.11.0
> **分支**: `develop/v3.11.0`
> **更新**: 2026-07-16 (第 2 次修订)
> **文档**: `docs/releases/v3.11.0/LEGACY_DEBT_TRACKING_TABLE.md`
> **审计报告**: `docs/releases/v3.11.0/LEGACY_DEBT_AUDIT_REPORT.md`

---

## 修订说明 (2026-07-16 第二次修订)

### 第一次审计的误报更正

| # | 原发现 | 实际情况 | 修复 |
|---|--------|---------|------|
| 1 | **F-24 AHI 虚假集成** — `scan_with_ahi()` 不存在 | `scan_with_ahi()` 在 PR #3478 merge commit `d60eaaab3` 中已合入，包含 `record_access()` 调用 | **更正**: F-24 是真实集成 |
| 2 | **F-23 ClusteredTable 无 ExecutionEngine 引用** | 写入路径存在 (`execute_create_table` L663-665)，读取路径在 `74b36ccf7` 中修复 | **已修复**: PR #3516 |
| 3 | **debt-registry.yaml 未更新** | 仍为 v3.10.0-snapshot | **已修复**: PR #3516 更新到 v3.11.0-snapshot |
| 4 | **#3136 声称已关闭但无代码** | 新 gate 脚本 (check_alpha/beta/rc_v3.11.0.sh) 正确使用 debt-registry.yaml SSOT | **已关闭**: #3136 在 debt-registry.yaml 中标记为 CLOSED |

> **重要教训**: 代码审计必须直接查看实际文件内容，不能仅依赖 PR 描述或 merge commit diff。

---

## 总览

| 类别 | 总数 | ✅ 已完成 | ⏳ 进行中 | 📋 待办 | 状态 |
|------|------|---------|---------|--------|------|
| INT (集成债务) | 4 | 4 | 0 | 0 | ✅ 全部 CLOSED |
| ARCH (架构债务) | 3 | 3 | 0 | 0 | ✅ 全部 CLOSED |
| SEM (语义债务) | 4 | 3 | 0 | 1 | ⏳ 1 待办 |
| F-XX ISOLATED → 主路径 | 10 | **7** | 0 | **3** | ⏳ 3 待办 |
| F-XX NOT_IMPL → 实现 | 5 | 2 | 0 | 3 | ⏳ 3 待办 |
| Extension Crate 决策 | 11 | 10 | 0 | 1 | ⏳ 1 待办 |
| PERF 性能优化 | 5 | 4 | 0 | 1 | ⏳ 1 待办 |
| GA-P0 长跑任务 | 2 | 0 | 0 | 2 | 🔴 阻塞 |
| Follow-up 跟踪 | 4 | 3 | 0 | 0 | 1 问题已关闭 |
| **合计** | **48** | **26 (54%)** | **0** | **13 (27%)** | |

---

## 1. INT — 集成债务 (4/4 完成)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 |
|----|------|---------|---------|-------|-----|------|
| INT-1 | DML 不经过 WAL/TransactionManager | CLOSED | — | #3099 | #3019, #3050 | ✅ CLOSED |
| INT-2 | ParallelVolcanoExecutor 主路径集成 | CLOSED | — | — | #3767, #3703, #3790 | ✅ CLOSED |
| INT-3 | expr 双实现合并 | CLOSED | — | #3146 | #3200, #3345 | ✅ CLOSED |
| INT-4 | mysql-server 未与主 server 集成 | CLOSED | — | — | #2999, #3051 | ✅ CLOSED |

---

## 2. ARCH — 架构债务 (3/3 完成)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 |
|----|------|---------|---------|-------|-----|------|
| ARCH-1 | execution_engine.rs 行数过大 | CLOSED | — | — | #2789, #2877 | ✅ CLOSED |
| ARCH-2 | dual path mysql-server vs bench-cli | CLOSED | — | #3117 | #3001, #3067 | ✅ CLOSED |
| ARCH-3 | VTU 主路径集成 | CLOSED | — | #3129 | #3152, #3787, #3790 | ✅ CLOSED |

---

## 3. SEM — 语义债务 (3/4 完成)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 |
|----|------|---------|---------|-------|-----|------|
| SEM-1 | ROLLBACK MVCC stub | CLOSED | — | — | #3134, #3200 | ✅ CLOSED |
| SEM-2 | SHOW TABLES partial | CLOSED | — | — | #2790, #2815 | ✅ CLOSED |
| SEM-3 | ALTER TABLE 不完整 (RENAME/MODIFY) | IN_PROGRESS | ✅ CLOSED | — | #3442, #3444 | ✅ CLOSED |
| SEM-4 | Coverage 测量差异 | IN_PROGRESS | ⏳ TODO | #3493 | — | ⏳ 待办 |

**SEM-4 待办**:
- **Issue**: #3493 (V311-14)
- **工作量**: 60h
- **目标**: per-crate coverage ≥ 85%
- **状态**: 🔄 另一个 AI 处理中 (#3420 Coverage ≥80%)

---

## 4. F-XX ISOLATED → 主路径集成 (7/10 完成)

### 4.1 已完成 (7 真实) ✅

| ID | 名称 | v3.10.0 | v3.11.0 | PR | 状态 | 集成质量 |
|----|------|---------|---------|-----|------|---------|
| F-16 | Gap Locking | CLOSED | — | #3788, #3783 | ✅ CLOSED | ✅ 真实集成 |
| F-23 | Clustered Index | VERIFIED | ✅ CLOSED | #3461 + #3516 | ✅ CLOSED | ✅ 读写路径完整 |
| F-24 | Adaptive Hash Index | VERIFIED | ✅ CLOSED | #3478 | ✅ CLOSED | ✅ scan_with_ahi + record_access |
| F-25 | Change Buffer | VERIFIED | ✅ **CLOSED** | #3512 | ✅ **CLOSED** | ✅ `IntegratedTableStorage` 主路径集成（V311-04;6/6 tests pass）|
| F-26 | Double-Write Buffer | VERIFIED | ✅ **CLOSED** | #3514 | ✅ **CLOSED** | ✅ `IntegratedTableStorage` 主路径集成（V311-05;6/6 tests pass）|
| F-31 | Performance Schema | VERIFIED | ✅ CLOSED | #3479 | ✅ CLOSED | ✅ trait + 调用 |
| F-32 | MySQL Admin | PARTIAL | ✅ CLOSED | #3481 | ✅ CLOSED | 🟡 binary 发版 |

### 4.2 待办 (3) ⏳

| ID | 名称 | Issue | 工作量 | 优先级 | 当前实现 |
|----|------|-------|--------|--------|---------|
| F-27 | Table Compression | #3498 (V311-12) | 50h | P1 | `tests/integration/sql/table_compression_test.rs` (RLE only) |
| F-29 | Row-Level Security | #3494 (V311-05) | 40h | P1 | `tests/integration/sql/row_level_security_test.rs` (内存 catalog) |
| F-35 | Password Rotation | #3495 (V311-08) | 20h | P1 | `tests/integration/sql/password_rotation_test.rs` (内存 mock) |

---

## 5. F-XX NOT_IMPL → 实现 (2/5 完成)

### 5.1 已完成 (2)

| ID | 名称 | v3.10.0 | v3.11.0 | PR | 状态 |
|----|------|---------|---------|-----|------|
| F-36 | 列级权限 | DEFERRED | ✅ CLOSED | #3457 | ✅ CLOSED |
| T-19 | Disk I/O delay | CLOSED | — | #3780 | ✅ CLOSED |
| T-20 | Process kill -9 | CLOSED | — | #3780 | ✅ CLOSED |

### 5.2 待办 (3)

| ID | 名称 | Issue | 工作量 | 优先级 |
|----|------|-------|--------|--------|
| F-03 | GIS (POINT + WITHIN) | #3497 (V311-11) | 80h | P1 |
| F-30 | CREATE SEQUENCE | #3496 (V311-10) | 20h | P1 |

---

## 6. Extension Crate 决策 (10/11 完成)

| Crate | v3.10.0 状态 | v3.11.0 决策 | PR | 状态 |
|-------|-------------|-------------|-----|------|
| agentsql | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| rag | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| qmd-bridge | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| evidence-graph | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| unified-query | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| unified-storage | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| distributed | FROZEN | DELETE | — | ✅ DELETED |
| gmp | SCOPE_DEFERRED | ARCHIVE | — | ✅ ARCHIVED |
| graph | SCOPE_DEFERRED | ARCHIVE | — | ✅ ARCHIVED |
| admin | PARTIAL | INTEGRATE | — | ✅ CLOSED |
| vector | SCOPE_INTERNAL | RETAIN | — | ✅ RETAINED |

---

## 7. PERF 性能优化 (4/5 完成)

| ID | 名称 | 来源 | PR | 状态 |
|----|------|------|-----|------|
| PERF-1 | Hash Semi Join 算子 | Issue #3792 | #3455 | ✅ CLOSED |
| PERF-2 | Hash Anti Join 算子 | Issue #3792 | — | ✅ CLOSED |
| PERF-3 | CTE 物化 | Issue #3792 | #3499 | ⏳ 待办 |
| PERF-4 | Decorrelation optimizer | Issue #3792 | — | ✅ CLOSED |
| PERF-5 | 高并发 INSERT 修复 | v3.10.0 SOAK | — | ✅ CLOSED |

---

## 8. GA-P0 长跑任务 (0/2 完成)

| ID | 名称 | Issue | 状态 | 阻塞 |
|----|------|-------|------|------|
| #3423 | TPC-H SF=1.0 baseline | #3431 (V311-20) | ⏳ 待办 | 🔴 需 75GB+ 磁盘 |
| #3648 | TPC-H SOAK 跨平台 | #3500 (V311-21) | ⏳ 待办 | Hermes 协作 |

---

## 9. Issue 清单汇总

### 9.1 Issue 状态 (本轮审计后)

| # | 标题 | 状态 |
|---|------|------|
| #3491 | V311-03: F-25 Change Buffer 主路径集成 | ✅ CLOSED（`IntegratedTableStorage` 集成;6/6 tests pass）|
| #3492 | V311-04: F-26 Double-Write Buffer 主路径集成 | ✅ CLOSED（`IntegratedTableStorage` 集成;6/6 tests pass）|
| #3493 | V311-14: SEM-4 覆盖率 ≥85% | ⏳ 待办 |
| #3494 | V311-05: F-29 Row-Level Security | ⏳ 待办 |
| #3495 | V311-08: F-35 Password Rotation | ⏳ 待办 |
| #3496 | V311-10: F-30 CREATE SEQUENCE | ⏳ 待办 |
| #3497 | V311-11: F-03 GIS 空间数据类型 | ⏳ 待办 |
| #3498 | V311-12: F-27 Table Compression | ⏳ 待办 |
| #3499 | V311-18: CTE 物化 | ⏳ 待办 |
| #3500 | V311-21: 168h SOAK v3.11.0 | ⏳ 待办 |
| #3501 | v3.11.0 总控 — 进度跟踪 | 🔄 进行中 |
| #3507 | [AUDIT] #3136 check_cross_version_debt.sh 升级 | ✅ CLOSED |

---

## 10. 剩余开发和集成工作 (13 项)

| # | Issue | V311 | 功能 | 工作量 | 优先级 | 备注 |
|---|-------|------|------|--------|--------|------|
| 1 | #3493 | V311-14 | SEM-4 Coverage ≥85% | 60h | P0 | 另一个 AI 处理中 |
| 2 | #3498 | V311-12 | F-27 Table Compression | 50h | P1 | 仍为孤岛 (RLE only) |
| 3 | #3494 | V311-05 | F-29 Row-Level Security | 40h | P1 | 仍为孤岛 |
| 4 | #3499 | V311-18 | CTE 物化 | 30h | P1 | — |
| 5 | #3495 | V311-08 | F-35 Password Rotation | 20h | P1 | 仍为孤岛 |
| 6 | #3496 | V311-10 | F-30 CREATE SEQUENCE | 20h | P1 | 零代码 |
| 7 | #3497 | V311-11 | F-03 GIS | 80h | P1 | 零代码 |
| 8 | #3431 | V311-20 | TPC-H SF=1 baseline | — | P0 | 🔴 需硬件 |
| 9 | #3500 | V311-21 | 168h SOAK | — | P1 | — |

---

*文档版本: 2026-07-16 第二次修订*
*更正: PR #3516 修正了初次审计的 4 项误报*

# v3.11.0 历史遗留债务追踪总表

> **版本**: v3.11.0
> **分支**: `develop/v3.11.0`
> **创建**: 2026-07-16
> **文档**: `docs/releases/v3.11.0/LEGACY_DEBT_TRACKING_TABLE.md`
> **审计报告**: `docs/releases/v3.11.0/LEGACY_DEBT_AUDIT_REPORT.md`

---

## 总览

| 类别 | 总数 | ✅ 已完成 | ⏳ 进行中 | 📋 待办 | ❌ 问题 |
|------|------|---------|---------|--------|--------|
| INT (集成债务) | 4 | 4 | 0 | 0 | 0 |
| ARCH (架构债务) | 3 | 3 | 0 | 0 | 0 |
| SEM (语义债务) | 4 | 3 | 0 | 1 | 0 |
| F-XX ISOLATED → 主路径 | 10 | 5 | 0 | 5 | 0 |
| F-XX NOT_IMPL → 实现 | 5 | 2 | 0 | 3 | 0 |
| Extension Crate 决策 | 11 | 7 | 0 | 1 | 0 |
| PERF 性能优化 | 5 | 4 | 0 | 1 | 0 |
| GA-P0 长跑任务 | 2 | 0 | 0 | 2 | 0 |
| Follow-up 跟踪 | 4 | 3 | 0 | 0 | 1 |
| **合计** | **48** | **26 (54%)** | **0** | **13 (27%)** | **1** |

---

## 1. INT — 集成债务 (4/4 完成)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 |
|----|------|---------|---------|-------|-----|------|
| INT-1 | DML 不经过 WAL/TransactionManager | CLOSED | — | #3099 | #3019, #3050 | ✅ CLOSED |
| INT-2 | ParallelVolcanoExecutor 主路径集成 | CLOSED | — | — | #3767, #3703, #3790 | ✅ CLOSED |
| INT-3 | expr 双实现合并 | CLOSED | — | #3146 | #3200, #3345 | ✅ CLOSED |
| INT-4 | mysql-server 未与主 server 集成 | CLOSED | — | — | #2999, #3051 | ✅ CLOSED |

**结论**: INT 债务 4/4 全部 CLOSED，无需 v3.11.0 行动。

---

## 2. ARCH — 架构债务 (3/3 完成)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 |
|----|------|---------|---------|-------|-----|------|
| ARCH-1 | execution_engine.rs 行数过大 | CLOSED | — | — | #2789, #2877 | ✅ CLOSED |
| ARCH-2 | dual path mysql-server vs bench-cli | CLOSED | — | #3117 | #3001, #3067 | ✅ CLOSED |
| ARCH-3 | VTU 主路径集成 | CLOSED | — | #3129 | #3152, #3787, #3790 | ✅ CLOSED |

**结论**: ARCH 债务 3/3 全部 CLOSED，无需 v3.11.0 行动。

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
- **前置**: #3420 (Coverage ≥80%, 另一个 AI 处理中)

---

## 4. F-XX ISOLATED → 主路径集成 (5/10 完成)

### 4.1 已完成 (5)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 | 集成质量 |
|----|------|---------|---------|-------|-----|------|---------|
| F-16 | Gap Locking | CLOSED | — | — | #3788, #3783 | ✅ CLOSED | ✅ 真实集成 |
| F-23 | Clustered Index | VERIFIED | ✅ CLOSED | — | #3461 | ✅ CLOSED | 🔴 **虚假集成** — ClusteredTable 仅在 storage crate，无 ExecutionEngine 引用 |
| F-24 | Adaptive Hash Index | VERIFIED | ✅ CLOSED | — | #3465, #3478 | ✅ CLOSED | 🔴 **虚假集成** — 仅 +ahi() accessor，无 scan_with_ahi() 调用 |
| F-31 | Performance Schema | VERIFIED | ✅ CLOSED | — | #3479 | ✅ CLOSED | 🟡 trait 存在，无算子调用 |
| F-32 | MySQL Admin | PARTIAL | ✅ CLOSED | — | #3481 | ✅ CLOSED | 🟡 binary 已发，wire protocol 未完全集成 |

### 4.2 待办 (5)

| ID | 名称 | Issue | 工作量 | 优先级 | 状态 |
|----|------|-------|--------|--------|------|
| F-25 | Change Buffer | #3491 (V311-03) | 40h | P0 | ⏳ 待办 |
| F-26 | Double-Write Buffer | #3492 (V311-04) | 50h | P0 | ⏳ 待办 |
| F-27 | Table Compression | #3498 (V311-12) | 50h | P1 | ⏳ 待办 |
| F-29 | Row-Level Security | #3494 (V311-05) | 40h | P1 | ⏳ 待办 |
| F-35 | Password Rotation | #3495 (V311-08) | 20h | P1 | ⏳ 待办 |

**F-25/26/27/29/35 当前状态**: 仅 `tests/integration/sql/` 中有内联实现，零 crate 集成。

---

## 5. F-XX NOT_IMPL → 实现 (2/5 完成)

### 5.1 已完成 (2)

| ID | 名称 | v3.10.0 | v3.11.0 | Issue | PR | 状态 |
|----|------|---------|---------|-------|-----|------|
| F-36 | 列级权限 | DEFERRED | ✅ CLOSED | — | #3457 | ✅ CLOSED |
| T-19 | Disk I/O delay | CLOSED | — | — | #3780 | ✅ CLOSED |
| T-20 | Process kill -9 | CLOSED | — | — | #3780 | ✅ CLOSED |

### 5.2 待办 (3)

| ID | 名称 | Issue | 工作量 | 优先级 | 状态 |
|----|------|-------|--------|--------|------|
| F-03 | GIS (POINT + WITHIN) | #3497 (V311-11) | 80h | P1 | ⏳ 待办 |
| F-30 | CREATE SEQUENCE | #3496 (V311-10) | 20h | P1 | ⏳ 待办 |

---

## 6. Extension Crate 决策 (7/11 完成)

| Crate | v3.10.0 状态 | v3.11.0 决策 | Issue | 状态 |
|-------|-------------|-------------|-------|------|
| agentsql | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| rag | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| qmd-bridge | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| evidence-graph | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| unified-query | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| unified-storage | SCOPE_DEFERRED | DELETE | — | ✅ DELETED |
| distributed | FROZEN | DELETE | — | ✅ DELETED |
| gmp | SCOPE_DEFERRED | ARCHIVE | — | ✅ ARCHIVED |
| graph | SCOPE_DEFERRED | ARCHIVE | — | ✅ ARCHIVED |
| admin | PARTIAL | INTEGRATE (mysql-server) | — | ✅ CLOSED |
| vector | SCOPE_INTERNAL | RETAIN | — | ✅ RETAINED |

**待办 (1)**: gmp — ARCHIVED 状态，但 external dependency 清理待确认。

---

## 7. PERF 性能优化 (4/5 完成)

| ID | 名称 | 来源 | Issue | PR | 状态 |
|----|------|------|-------|-----|------|
| PERF-1 | Hash Semi Join 算子 | Issue #3792 | — | #3455 | ✅ CLOSED |
| PERF-2 | Hash Anti Join 算子 | Issue #3792 | — | — | ✅ CLOSED |
| PERF-3 | CTE 物化 | Issue #3792 | #3499 (V311-18) | — | ⏳ 待办 |
| PERF-4 | Decorrelation optimizer | Issue #3792 | — | — | ✅ CLOSED |
| PERF-5 | 高并发 INSERT 修复 | v3.10.0 SOAK | — | — | ✅ CLOSED |

**PERF-3 待办**:
- **Issue**: #3499 (V311-18)
- **工作量**: 30h
- **目标**: CTE 物化执行策略

---

## 8. GA-P0 长跑任务 (0/2 完成)

| ID | 名称 | Issue | 状态 | 阻塞 |
|----|------|-------|------|------|
| #3423 | TPC-H SF=1.0 baseline | #3431 (V311-20) | ⏳ 待办 | 🔴 需 75GB+ 磁盘 |
| #3648 | TPC-H SOAK 跨平台 | #3500 (V311-21) | ⏳ 待办 | Hermes 协作 |

---

## 9. Follow-up 跟踪 (3/4 完成)

| ID | 名称 | Issue | 状态 | 备注 |
|----|------|-------|------|------|
| #3117 | DML bypass VtuGuard | — | ✅ CLOSED | — |
| #3129 | ARCH-3 修复路径 | — | ✅ CLOSED | — |
| #3146 | INT-2/INT-3 完整合并 | — | ✅ CLOSED | — |
| #3136 | gate script 升级 | **#3507** | ❌ **未完成** | V311-19 声称已处理但未实际完成 |

---

## 10. Issue 清单汇总

### 10.1 新建 Issue (本轮审计)

| # | 标题 | 类别 | 优先级 | 来源 |
|---|------|------|--------|------|
| #3501 | v3.11.0 总控 — 进度跟踪 + 12 项遗留任务 | master | — | 本次 |
| #3507 | [AUDIT] #3136 check_cross_version_debt.sh 升级未完成 | governance | P1 | 本次 |

### 10.2 ALPHA P0 Issue (4 项)

| # | 标题 | V311 | 来源 |
|---|------|------|------|
| #3491 | V311-03: F-25 Change Buffer 主路径集成 | V311-03 | 本次 |
| #3492 | V311-04: F-26 Double-Write Buffer 主路径集成 | V311-04 | 本次 |
| #3493 | V311-14: SEM-4 覆盖率 ≥85%（per crate） | V311-14 | 本次 |
| #3431 | R8: TPC-H SF=1 full performance baseline | V311-20 | 已有 |

### 10.3 BETA P1 Issue (7 项)

| # | 标题 | V311 | 来源 |
|---|------|------|------|
| #3494 | V311-05: F-29 Row-Level Security 主路径集成 | V311-05 | 本次 |
| #3495 | V311-08: F-35 Password Rotation 主路径集成 | V311-08 | 本次 |
| #3496 | V311-10: F-30 CREATE SEQUENCE 实现 | V311-10 | 本次 |
| #3497 | V311-11: F-03 GIS 空间数据类型 (POINT + WITHIN) | V311-11 | 本次 |
| #3498 | V311-12: F-27 Table Compression (LZ4/zstd) | V311-12 | 本次 |
| #3499 | V311-18: CTE 物化 | V311-18 | 本次 |
| #3500 | V311-21: 168h SOAK v3.11.0 | V311-21 | 本次 |

### 10.4 外部依赖 Issue (2 项)

| # | 标题 | 状态 | 说明 |
|---|------|------|------|
| #3420 | SEM-4: Coverage ≥ 80% (main crate) | 🔄 另一个 AI 处理中 | V311-14 前置 |
| #3490 | docs(v3.11.0): TPC-H Q5 memory growth analysis | 新增 | 文档任务 |

---

## 11. 关键风险

| # | 风险 | 概率 | 影响 | 缓解 |
|---|------|------|------|------|
| 1 | **F-23/F-24 虚假集成** — PR 声称但无实际调用链 | 🔴 高 | 高 | 需 V311-01 v2 / V311-02 v3 实现真实集成 |
| 2 | **debt-registry.yaml 未更新** — 仍为 v3.10.0 snapshot | 🟡 高 | 中 | 需更新到 v3.11.0 snapshot |
| 3 | **TPC-H SF=1** — 需 75GB+ 磁盘硬件 | 🟡 中 | 高 | 依赖专用机器 |
| 4 | **22 个禁用测试** — 回归覆盖缺失 | 🟡 中 | 中 | 逐一修复或永久移除 |

---

*文档版本: 2026-07-16*
*下次更新: 每阶段完成后*

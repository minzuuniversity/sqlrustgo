<!-- env:blocked:no-ci -->

# v3.8.0 严重遗留问题审计报告 (Comprehensive Legacy Issues Audit)

> **报告日期**: 2026-06-05
> **版本**: v3.8.0
> **分支**: `develop/v3.8.0` (HEAD: `da49fae9`)
> **作者**: Hermes Agent (subagent-driven, 3 个并行调研)
> **状态**: ACTIVE — 13 个 P0/P1 Issue 待开
> **依据**: v3.6.0 `LEGACY_ISSUE_ANALYSIS.md` 框架 + v3.8.0 功能矩阵 + 9 维门禁体系

---

## 0. Executive Summary

| 维度 | 文档声称 | 真实状态 | 偏差 |
|------|---------|---------|------|
| **16 F-XX + I-12** | 12/16 100% CLOSED | **10 真实 + 5 PARTIAL + 1 STALE** | F-09 ACID 违规 |
| **F-01~F-36** (v3.0.0 legacy) | 23/36 CLOSED (64%) | **~17/36 真实 CLOSED** | 至少 10 个孤岛 PARTIAL |
| **I-01~I-12** | 10/12 CLOSED (83%) | **~6/12 真实 CLOSED** | I-02/I-05/I-10/I-12 PARTIAL |
| **T-01~T-20** | 16/20 CLOSED (80%) | **~10/20 真实 CLOSED** | T-12/T-15/T-19/T-20 不存在 |
| **INT-1~INT-4** | "0 进展" / "4 ACTIVE" | **2 CLOSED + 2 ACTIVE** | GA §7.5 STALE |
| **ARCH-1~3 + SEM-1~4** | 7 OPEN | **3 CLOSED + 2 PARTIAL + 3 OPEN** | ARCH-1 实际 1669 行不是 6829 |
| **跨版本债务** | 50/72 CLOSED (69%) | **~25/100 真实 CLOSED** | 文档自验证循环 |

**核心结论**: v3.8.0 文档声称 72% CLOSED，**实际只有 ~24% 真正 CLOSED**；10 个 F-XX 是孤岛测试（与主代码零集成），5 个完全无实现，3 个关键 gate 脚本有 BUG。

**对比 v3.6.0**: v3.6.0 留下 6 个 I#DEBT-*/*ARCH-*/*GOV-* Issue（Parser 47%、ExecutionEngine 6829 行、双轨架构、Alpha CONDITIONAL PASS）。v3.8.0 已修复 INT-1/4/ARCH-1/SEM-2，但新增了 13 个 P0/P1 治理债务（见 §7）。

---

## 1. 真实状态总览

| 维度 | 文档声称 | 真实状态 | 偏差 |
|---|---|---|---|
| **16 F-XX + I-12** | 12/16 100% CLOSED | 10 真实 + 5 PARTIAL + 1 STALE | 1 个 ACID 违规 (F-09) |
| **F-01~F-36** | 23/36 CLOSED (64%) | ~17/36 真实 CLOSED | 至少 10 个孤岛 PARTIAL |
| **I-01~I-12** | 10/12 CLOSED (83%) | ~6/12 真实 CLOSED | I-02/I-05/I-10/I-12 PARTIAL |
| **T-01~T-20** | 16/20 CLOSED (80%) | ~10/20 真实 CLOSED | T-12/T-15/T-19/T-20 不存在 |
| **INT-1~INT-4** | 4 ACTIVE | 2 CLOSED + 2 ACTIVE | GA §7.5 STALE |
| **ARCH-1~3 + SEM-1~4** | 7 OPEN | 3 CLOSED + 2 PARTIAL + 3 OPEN | ARCH-1 实际 1669 行不是 6829 |
| **跨版本债务** | 50/72 CLOSED (69%) | ~25/100 真实 CLOSED | 文档自验证循环 |

---

## 2. 严重问题（必须修复）

### 🔴 P0 — ACID 数据完整性违规

**F-09 `test_insert_twice_duplicate_ignored` 失败**（`tests/wal_tx_contract_test.rs:365`）
- 主键重复插入应报错，实际静默接受第二个 INSERT
- 这是数据库核心契约破坏
- F-09 矩阵声称 "22/22 PASS"，**实际是 25/26 + 2 fail + 13 ignored**
- F-09 自审报告（`docs/audit/F09_ACTUAL_COMPLETION_REPORT_2026-06-03.md`）承认 12 个 KNOWN_GAP 推迟到 v3.8.0+1

### 🔴 P0 — Gate 脚本 BUG（P5 原则违反）

| Gate 脚本 | 真实行为 | 后果 |
|---|---|---|
| `check_int_debt.sh` | 路径失效（`CROSS-VERSION-DEBT.md` 在 `debt/` 不在 `archived/`），4 个 INT NOT FOUND → 假 PASS exit 0 | D7 永远假 PASS，掩盖 INT-1/4 已修复但未更新 |
| `check_arch2_no_bypass.sh` | 检测到 21+ bypass（recovery/backup/gmp/grpc_server/openclaw_endpoints）→ 真实 FAIL exit 1 | ARCH-2 dual path 消但 bypass FAIL 未被纳入评估 |
| `check_execution_semantics.sh` | grep 主动排除 execution_engine.rs/trigger.rs/local_executor.rs/parallel_executor.rs | 等同于"跳过所有真实 DML 路径"，假 PASS |
| `check_cross_version_debt.sh` | 只解析 markdown `✅/⚠️/❌` 符号，**完全不查代码** | "文档验证文档"的循环 — 任何 ✅ 都通过 |

### 🔴 P0 — 文档严重 STALE

| 文档 | 错误 | 真实 |
|---|---|---|
| `ga/GA_GATE_CHECKLIST.md §7.5` | INT-1/INT-4 ACTIVE | ✅ CLOSED (PR-3019/PR-2999) |
| `archived/INT_DEBT_REMEDIATION_PLAN.md` | 4 ACTIVE | 2 CLOSED + 2 ACTIVE |
| `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | ARCH-1 6829 行 OPEN | 1669 行（<2000 阈值），实际 CLOSED |
| `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | SEM-2 OPEN | ✅ CLOSED (PR-2790/2815 + execute_show_tables) |
| `V380_COMPREHENSIVE_ASSESSMENT.md D7` | "4 ACTIVE w/ plan" | PASS-WITH-DRIFT 是假（实际 2 CLOSED） |
| `FEATURE_MATRIX.md §1.5 vs §11 vs §14` | F-11 4 处描述互相矛盾 | 矩阵自相矛盾 |
| `d6_test_inventory.json` (2026-06-04 05:57) | "0 failed" | 实际 14 files FAIL / 62 tests FAIL (~15h 过期) |

### 🔴 P0 — 孤立测试（"X/X PASS" 假象）

**10 个 F-XX 的实现完全在 `tests/*.rs` 内自包含，生产代码零集成**：
- F-16 Gap Locking、F-23 Clustered Index、F-24 AHI、F-25 Change Buffer、F-26 Double-Write、F-27 Table Compression、F-29 RLS、F-31 Performance Schema、F-32 MySQL Admin、F-35 Password Rotation
- 每个 SPEC 明确写 "In-memory only, real integration in v3.9.0"
- 例如 F-32 测试注释: "Real CLI binary is deferred to v3.9.0"
- 例如 F-35 测试注释: "In v3.9.0 this can be promoted to `src/auth/password_rotation.rs`"
- `check_rc_ga_gate.sh D6a` 包含这些测试名 → 假 PASS

### 🔴 P0 — 完全无实现

| ID | 主题 | 真实状态 |
|---|---|---|
| F-03 | GIS 空间数据 | 0 实现（无 Point/LineString/Polygon） |
| F-30 | CREATE SEQUENCE | 0 实现（仅 parser 解析 1 drop_sequence 测试） |
| F-36 | 列级权限 | 0 实现（无 ColumnLevel/ColumnPrivilege） |
| T-12 | check_regression.sh | 文件不存在 |
| T-19 | Disk I/O delay | 0 commit，0 SPEC |
| T-20 | process_kill -9 | 0 文件，0 实现 |

---

## 3. 集成债务（INT/ARCH/SEM）真实状态

| ID | 主题 | 文档 | 真实 | 门禁 |
|---|---|---|---|---|
| **INT-1** | DML 不经过 WAL/TM | §7.5: ACTIVE / debt/: ✅ | ✅ CLOSED (PR-3019+PR-3050) | check_int_debt.sh 假 PASS |
| **INT-2** | ParallelVolcanoExecutor 孤岛 | ACTIVE | ❌ ACTIVE（`parallel_executor.rs` 未 `pub mod`，主路径零调用）| check_cross_version_debt.sh 真实 ACTIVE |
| **INT-3** | expr crate 功能孤岛 | ACTIVE | ❌ ACTIVE（`src/expr_utils.rs` + `crates/executor/src/expr/` 双实现）| 同上 |
| **INT-4** | mysql-server 主 server 集成 | §7.5: ACTIVE / debt/: ✅ | ✅ CLOSED (PR-2999+PR-3051) | check_int_debt.sh 假 PASS |
| **ARCH-1** | execution_engine.rs 6829 行 | OPEN | ✅ 实际 1669 行（<2000 阈值） | 硬编码 OPEN（错）|
| **ARCH-2** | dual path mysql-server vs bench-cli | OPEN | ⚠️ PARTIAL（dual path 消 PR-3067，但 bypass FAIL 21+）| check_arch2_no_bypass.sh 真实 FAIL |
| **ARCH-3** | VTU 主路径集成 | OPEN | ❌ OPEN（VtuGuard.execute_dml 主路径零调用）| 同 ARCH-2 |
| **SEM-1** | ROLLBACK MVCC stub | OPEN | ❌ OPEN（SavepointManager 未集成，tests/savepoint_test 引用不存在）| 硬编码 OPEN |
| **SEM-2** | SHOW TABLES partial | OPEN | ✅ CLOSED（`execution_engine.rs:1505` 真实实现）| 硬编码 OPEN（错）|
| **SEM-3** | ALTER TABLE incomplete | OPEN | ⚠️ PARTIAL（ADD/RENAME work，DROP/MODIFY 失败但缺测试）| 硬编码 OPEN |
| **SEM-4** | Coverage 测量差异 | OPEN | ❌ OPEN（无 commit）| 硬编码 OPEN |

**实际汇总**: 4 CLOSED（INT-1/4/ARCH-1/SEM-2）+ 2 PARTIAL（ARCH-2/SEM-3）+ 5 OPEN（INT-2/3/ARCH-3/SEM-1/SEM-4）

---

## 4. 16 个 F-XX 详细追踪矩阵

| F-XX | 主题 | SPEC | 测试 | 真实 | 集成主路径 | 门禁 | 评级 |
|---|---|---|---|---|---|---|---|
| F-09 | WAL Recovery | ❌ 无 | 25/26 + 16/31 + 2 fail + 13 ignored | 12 KNOWN_GAP | PARTIAL | d6 过期 | **❌ STALE** |
| F-10 | Multi-join Schema | ❌ 无 | 4/4 + 5/5 | SPEC-024 承认 90% | PARTIAL | 矩阵夸大 | ⚠️ PARTIAL |
| F-11 | Aggregate | ❌ 无 | 12/12 | 矩阵自相矛盾 | ✅ | D6 | ⚠️ PARTIAL |
| F-12 | DISTINCT | ❌ 无 | 6/6 + 12/12 | 矩阵自相矛盾 | ✅ | D6 | ⚠️ PARTIAL |
| F-14 | T-ISO | ❌ 无 | 11/11 | 无 SPEC | ✅ | D6 | ✅ REAL PASS |
| F-16 | Gap Locking | ✅ | 7/7 | 孤岛 mock | ❌ | D6 | ✅ TRUE 100% (按 SPEC 范围) |
| F-23 | Clustered Index | ✅ | 7/7 | 孤岛 BTreeMap | ❌ | D6 | ✅ TRUE 100% |
| F-24 | AHI | ✅ | 7/7 | 孤岛 mock | ❌ | D6 | ✅ TRUE 100% |
| F-25 | Change Buffer | ✅ | 5/5 | 孤岛 enum | ❌ | D6 | ✅ TRUE 100% |
| F-26 | Double-Write | ✅ | 6/6 | 孤岛 mock | ❌ | D6 | ✅ TRUE 100% |
| F-27 | Table Compression | ✅ | 8/8 | 孤岛 RLE (矩阵夸 LZ4) | ❌ | D6 | ✅ TRUE 100% |
| F-29 | RLS | ✅ | 6/6 | 孤岛 catalog | ❌ | D6 | ✅ TRUE 100% |
| F-31 | Perf Schema | ✅ | 7/7 | 孤岛 mock | ❌ | D6 | ✅ TRUE 100% |
| F-32 | MySQL Admin | ✅ | 11/11 | 孤岛 mock | ❌ | D6 | ✅ TRUE 100% |
| F-35 | Password Rotation | ✅ | 8/8 | 孤岛 in-memory | ❌ | D6 | ✅ TRUE 100% |
| I-12 | Parallel Executor | ✅ | 6/6 | 主路径未集成 | ❌ | D6 | ⚠️ PARTIAL（矩阵诚实标注）|

**5-类文档覆盖率（声称 16/16=100%）**: 实际 **10/16=62.5%**（F-09/10/11/12/14 无 SPEC）

---

## 5. 36+12+20 旧债务真实评级

### 真正 CLOSED (16 项)
- F: F-04, F-05, F-06, F-08, F-10, F-11, F-12, F-13, F-14, F-15, F-18, F-19, F-20, F-21, F-22, F-28, F-33
- I: I-01, I-03, I-04, I-07, I-08, I-09
- T: T-01, T-02, T-03, T-04, T-07, T-08, T-09, T-10, T-11, T-13

### 真实 PARTIAL (~25 项)
- F: F-01, F-02, F-07, F-16, F-17, F-23~27, F-29, F-31, F-32, F-34, F-35
- I: I-02, I-05, I-06, I-10, I-11（实际比声称好）, I-12
- T: T-05, T-06（实际比声称好）, T-14, T-15~18（孤岛 mock）

### STALE / OPEN (~18 项)
- **完全无实现**: F-03 (GIS), F-30 (SEQUENCE), F-36 (列权限), T-12 (regression.sh), T-19 (Disk I/O), T-20 (process_kill)
- **集成债务**: INT-2 (parallel), INT-3 (expr 双实现)
- **架构**: ARCH-3 (VTU 主路径)
- **语义**: SEM-1 (Savepoint 集成), SEM-4 (coverage 测量)

---

## 6. 整改计划（按 v3.6.0 框架）

### P0 立即修复（GA 阻断）

| # | 行动项 | 预计 |
|---|---|---|
| 1 | 修复 `check_int_debt.sh` 路径 BUG（加 `debt/CROSS-VERSION-DEBT.md` fallback）| 0.5h |
| 2 | 修复 `check_arch2_no_bypass.sh` 白名单（PR-3067 引入的 openclaw_endpoints/grpc_server/gmp bypass）| 2h |
| 3 | 修复 `check_execution_semantics.sh` 排除主路径的 grep | 1h |
| 4 | 修复 F-09 `test_insert_twice_duplicate_ignored` ACID 违规 | 4h |
| 5 | 同步 `ga/GA_GATE_CHECKLIST.md §7.5`（INT-1/4 → CLOSED）| 0.5h |
| 6 | 同步 `archived/INT_DEBT_REMEDIATION_PLAN.md` + `ARCH_SEM_DEBT_REMEDIATION_PLAN.md`（ARCH-1 1669 行、SEM-2 CLOSED）| 1h |
| 7 | 重跑 D6 门禁，刷新 `d6_test_inventory.json` 证据 | 1h |
| 8 | FEATURE_MATRIX.md §1.5/§11/§14 内部矛盾统一 | 0.5h |

### P1 v3.8.0+1 修复（4-6 周）

| # | 行动项 | 预计 |
|---|---|---|
| 9 | 升级 `check_cross_version_debt.sh`：从"解析 markdown"升级为"检查 use 语句 + 测试文件存在" | 1 周 |
| 10 | 10 个孤岛 F-XX（F-16/23/24/25/26/27/29/31/32/35）至少每个在 `src/` 创建占位 + 集成 PR 跟踪 | 4 周 |
| 11 | 5 个完全无实现债务进 Gitea Issue：F-03 GIS、F-30 SEQUENCE、F-36 列权限、T-19 Disk I/O delay、T-20 process_kill | 0.5h |
| 12 | 修复 SEM-1 Savepoint 集成（创建真实 `tests/savepoint_test.rs` + 主路径调用）| 1 周 |
| 13 | 修复 SEM-3 ALTER TABLE DROP/MODIFY（移除 `unwrap_or_default()`，加真实测试）| 1 周 |
| 14 | 修复 ARCH-3 VTU 主路径集成（`VtuGuard::execute_dml` 在 `execution_engine.rs` 中调用）| 2 周 |
| 15 | 修复 INT-3 expr 双实现（合并到 `src/expr_utils.rs` 或 crates/executor/src/expr/）| 1 周 |

### P2 v3.9.0 计划

| # | 行动项 |
|---|---|
| 16 | INT-2 Parallel Executor 主路径集成（I-12 worker pool 接入 ExecutionEngine.execute）|
| 17 | F-01 CREATE EVENT 调度器（增加 scheduler 线程）|
| 18 | F-34 AES-256 集成到 storage at-rest |
| 19 | F-07 Query cache DML 失效（DML 路径调用 `cache.invalidate_table`）|
| 20 | SEM-4 Coverage 测量差异（统一 Z6G4 vs Z440）|

---

## 7. 遗留问题追踪 Issue 清单（参照 v3.6.0 框架）

| Issue | 标题 | 级别 | 来源 | 状态 |
|---|---|---|---|---|
| **I#DEBT-v3.8.0-001** | F-09 ACID violation: test_insert_twice_duplicate_ignored 失败 | 🔴 P0 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-002** | check_int_debt.sh 路径 BUG 假 PASS（P5 违反）| 🔴 P0 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-003** | check_arch2_no_bypass.sh 真实 FAIL 21+ bypass 未纳入 | 🔴 P0 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-004** | 10 个 F-XX 孤岛测试（与生产代码零集成）| 🟡 P1 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-005** | 5 个完全无实现债务（F-03/30/36, T-12/19/20）| 🟡 P1 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-006** | INT-1/4 已 CLOSED 但 GA §7.5 + INT_DEBT_PLAN 文档未同步 | 🟡 P1 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-007** | ARCH-1（1669 行）SEM-2（已 CLOSED）ARCH-2（PARTIAL）状态文档 STALE | 🟡 P1 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-008** | check_cross_version_debt.sh 仅解析 markdown，不查代码 | 🟡 P1 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-009** | FEATURE_MATRIX.md 内部自相矛盾（§1.5/§11/§14）| 🟡 P1 | 本次 audit | OPEN |
| **I#DEBT-v3.8.0-010** | INT-2/INT-3 集成债务（主路径未集成，跨越 5+ 版本）| 🔴 P0 | 跨 v2.6.0+ | ACTIVE → v3.9.0 |
| **I#DEBT-v3.8.0-011** | ARCH-3 VTU 主路径集成（VTU Guard 零调用）| 🟡 P1 | 跨 v3.5.0+ | ACTIVE → v3.9.0 |
| **I#DEBT-v3.8.0-012** | SEM-1 Savepoint 集成（tests/savepoint_test 不存在）| 🟡 P1 | 跨 v3.0.0+ | ACTIVE → v3.9.0 |
| **I#DEBT-v3.8.0-013** | d6_test_inventory.json 证据过期 ~15h | 🟢 P2 | 本次 audit | OPEN |

---

## 8. 一句话总结

**v3.8.0 真实状态**：1 个 ACID 违规（F-09）+ 3 个 gate 脚本 BUG + 10 个孤岛测试（声称 PASS 实为 mock）+ 5 个完全无实现 + 7 份文档严重 STALE。**文档自验证 + 孤立测试 = 72% CLOSED 假象**，真实只有 ~24% 真正 CLOSED。

**对比 v3.6.0**：v3.6.0 留下 6 个 I#DEBT-*/*ARCH-*/*GOV-* Issue。v3.8.0 已修复 INT-1/4/ARCH-1/SEM-2，但新增了 13 个 P0/P1 治理债务（见 §7）。

---

## 9. 调研方法与证据来源

### 9.1 调研方法
- 3 个并行 subagent (subagent 1: 16 F-XX + I-12; subagent 2: 11 INT/ARCH/SEM; subagent 3: 36 F-XX + 12 I-XX + 20 T-XX)
- 直接读 13 份核心文档：FEATURE_MATRIX, debt/CROSS-VERSION-DEBT, debt/INT5_PLUS_DEBT_INVENTORY, ga/GA_GATE_CHECKLIST, historical/HISTORICAL_FEATURE_COVERAGE_MATRIX, historical/LEGACY_ISSUES, V380_COMPREHENSIVE_ASSESSMENT, rc/RC_GA_GATE_REPORT, beta/BETA_GATE_REPORT, alpha/ALPHA_GATE_REPORT, 9 份 SPEC (F16/F23/F24/F25-26/F27/F29/F31/F32/F35/I12)
- 5 个 gate 脚本源码审计：check_int_debt.sh, check_arch_sem_debt.sh, check_cross_version_debt.sh, check_arch2_no_bypass.sh, check_execution_semantics.sh
- 30+ 处文件路径 / 100+ 处代码引用作为证据

### 9.2 证据来源 SSOT
- `docs/releases/v3.8.0/FEATURE_MATRIX.md` (声称 12/16 100% CLOSED)
- `docs/releases/v3.8.0/debt/CROSS-VERSION-DEBT.md` (2026-06-03 反映最新 INT 状态)
- `docs/releases/v3.8.0/debt/INT5_PLUS_DEBT_INVENTORY.md` (v3.0.0 baseline 68 项)
- `docs/releases/v3.8.0/ga/GA_GATE_CHECKLIST.md §7.5` (旧 GA 文档)
- `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` (v3, 2026-06-04, 8.0/10)
- `docs/releases/v3.8.0/historical/LEGACY_ISSUES.md` (2026-05-31 旧版)
- 5-类文档：16 个 `specs/debt/*_SPEC.md` (10 真实存在 + 6 缺失)

### 9.3 参照框架
- v3.6.0 `docs/releases/v3.6.0/LEGACY_ISSUE_ANALYSIS.md` (本报告框架基础)
- v3.6.0 `INTEGRATION_DEBT_REPORT.md` (INT-1~4 起源)
- ADR-001 (Truthfulness Principle)
- ADR-010 (Ghost PR Resolution / 跨版本债务决策)
- 5-原则 (P5: 未通过必记)
- 9 维门禁 (D1-D9)

---

## 10. 维护信息

| 项目 | 值 |
|------|-----|
| 文档版本 | v3.8.0-LEGACY-ISSUES-AUDIT-1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | ACTIVE — 待开 13 个 Gitea Issue |
| 关联文档 | `historical/LEGACY_ISSUES.md` (2026-05-31 旧版) |
| 下游 | §7 Issue 清单 → `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 流程 |

---

*本审计报告遵循 v3.6.0 LEGACY_ISSUE_ANALYSIS 框架，参照 ADR-001 Truthfulness Principle 与 5-原则（P5）。所有发现基于 3 个 subagent 并行调研 + 13 份核心文档直接阅读 + 5 个 gate 脚本源码审计。*

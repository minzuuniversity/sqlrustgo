# ISOLATED_MODULES.md — SQLRustGo v3.10.0 (Post-GA)

> 反映 `develop/v3.10.0` @ `8056d5fb66` (2026-07-14, post-GA) 实时状态。
>
> **SSOT**: `docs/governance/debt/debt-registry.yaml` (v3.10.0-snapshot-2026-07-12) +
> `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` (PR #3794) +
> `docs/releases/v3.10.0/V310_TASK_CLOSURE_VERIFICATION.md` (PR #3836)。
>
> **生成日期**: 2026-07-14 (v3.10.0 GA Post-Release 更新)
>
> **范围**: 9 F-XX ISOLATED 项 + 11 extension crate 孤岛 + 其余非主路径组件。
>
> **v3.10.0 GA 状态**: 23/23 任务完成, 11/11 移交 v3.11.0 (V311-01 ~ V311-22)
> **v3.10.0 168h SOAK**: 🔄 IN PROGRESS (2026-07-14 启动, 预计 2026-07-21 完成)

---

## 0. 执行摘要

| 类别 | 总数 (v3.6 baseline) | v3.10.0 CLOSED/INTEGRATED | v3.10.0 仍 isolated/deferred |
|------|----------------------|---------------------------|-------------------------------|
| **F-XX ISOLATED** (功能孤岛) | 10 | **1** (F-16 GapLocking 主路径集成) + **1** PARTIAL (F-32 mysqladmin CLI binary) | 8 (F-23/24/25/26/27/29/31/35) |
| **F-XX NOT IMPLEMENTED** | 5 | **2** (T-19 Disk I/O delay, T-20 Process kill -9 — PR #3780) | 3 (F-03 GIS, F-30 SEQUENCE, F-36 列级权限) |
| **Extension crates** | 11 | **1** PARTIAL (admin — V310-14 PR #3795) + **1** FROZEN (distributed, frozen since v3.5) + **1** SCOPE_INTERNAL (vector) | 8 SCOPE_DEFERRED (gmp / agentsql / rag / graph / qmd-bridge / evidence-graph / unified-query / unified-storage) |

**关键判断**:
- **F-16 已彻底脱离孤岛**: Gap Locking 在 v3.10.0 通过 PR #3788 注入 `BTreeIndex` + `ExecutionEngine::commit/rollback` 主路径。
- **F-32 已半脱离孤岛**: `crates/admin/` real mysqladmin CLI binary via PR #3795 (V310-14),但 tests/mysqladmin_test.rs 仍使用 in-test mock。
- **8 个 F-XX ISOLATED + 8 个 extension crate + 3 个 F-XX NOT_IMPLEMENTED 全部明确推迟到 v3.11.0+**。这是显性的产品/工程权衡,不是 bug。

---

## 1. F-XX Functional Islands (9 项孤岛, 计划 v3.11.0+ 集成)

按 `ARCHITECTURE_DEBT_ANALYSIS.md §5.1` 规划,9 项 F-XX ISOLATED 的主路径集成需要约 960 行重构 + 多子系统协调 (BTree/索引/安全/性能),故显式推迟到 v3.11.0 Phase 1。

每个孤岛的具体状态:

| ID | 标题 | 测试文件 | 通过率 | v3.10.0 状态 | v3.11+ 计划 |
|----|------|----------|--------|--------------|-------------|
| **F-23** | Clustered Index | `tests/clustered_index_test.rs` | 7/7 | VERIFIED, BTreeMap 自包含 | Phase 1, BTree 改造 |
| **F-24** | Adaptive Hash Index | `tests/adaptive_hash_index_test.rs` | 7/7 | VERIFIED, in-memory mock | Phase 1, hot-page detection |
| **F-25** | Change Buffer | `tests/change_buffer_test.rs` | 5/5 | VERIFIED, ChangeOp enum 内联 | Phase 1, secondary index merge |
| **F-26** | Double-Write Buffer | `tests/double_write_buffer_test.rs` | 6/6 | VERIFIED, in-memory mock | Phase 1, WAL coordination |
| **F-27** | Table Compression | `tests/table_compression_test.rs` | 8/8 | VERIFIED, 仅 RLE | Phase 2, LZ4/zstd |
| **F-29** | Row-Level Security | `tests/row_level_security_test.rs` | 6/6 | VERIFIED, in-memory catalog | Phase 2, planner integration |
| **F-31** | Performance Schema | `tests/performance_schema_test.rs` | 7/7 | VERIFIED, 无 instrumentation hooks | Phase 2, telemetry hooks |
| **F-32** | MySQL Admin | `tests/mysqladmin_test.rs` | 11/11 | **PARTIAL** (binary shipped v3.10.0 PR #3795; tests still mock) | Phase 3, full CLI flag coverage |
| **F-35** | Password Rotation | `tests/password_rotation_test.rs` | 8/8 | VERIFIED, in-memory only | Phase 1, persistent credential store |

**判断**: F-23/24/25/26 为存储引擎核心扩展,**不建议在 v3.10.0 引入;** 否则存在不可控回归风险。v3.11.0 应作为 "孤岛消灭计划 Phase 1" 的主要载体。

---

## 2. F-XX NOT_IMPLEMENTED (3 项, 计划 v3.11+)

这些项目前完全是 0 代码 (无 SPEC 框架、无 commit):

| ID | 标题 | 推迟原因 | v3.11+ 计划 |
|----|------|----------|-------------|
| **F-03** | GIS 空间数据类型 | 需引入 Point/LineString/Polygon/WKT/WKB,与 NONE 类型无关 | Phase 1 |
| **F-30** | CREATE SEQUENCE | 与 MySQL 8.0 sequence 对象模型同步,需评估兼容性 | Phase 2 |
| **F-36** | 列级权限 | 与 F-29 RLS 协同,先集成 RLS 再列级 | Phase 2 |

**注意**: 历史上有夸大声称(如 "40 files" / "11 files" / "4 files"),实际 0 代码,见 `debt-registry.yaml` F-03/F-30/F-36 note 字段。

---

## 3. Extension Crates (11 项, v3.11+ 业务决策)

这些 crate **编译通过、单元测试可独立运行**,但**不在 sqlrustgo-mysql-server wire-protocol 调用链中**,亦不在 sqlrustgo-cli 的依赖图里。它们的存在是为了**保留 v3.6.x 时代的产品方向**,但当前没有任何生产调用方。

| Crate | v3.10.0 状态 | v3.11+ 处理思路 |
|-------|--------------|-----------------|
| `agentsql` | SCOPE_DEFERRED | AI agent SQL 生成器;评估是否纳入 v3.11+ CLI 工具集 |
| `gmp` | SCOPE_DEFERRED | 与外部 GMP-Platform 项目集成;平台决策未到 |
| `rag` | SCOPE_DEFERRED | RAG 流水线基础设施;无 mysql-server 依赖 |
| `distributed` | **FROZEN** | v3.5.0 起冻结;`docs/architecture/DISTRIBUTED_EXECUTION.md` 保留作设计档案 |
| `graph` | SCOPE_DEFERRED | 图查询支持;0 caller,评估是否 archive |
| `qmd-bridge` | SCOPE_DEFERRED | Markdown QMD query bridge;范围不明确 |
| `evidence-graph` | SCOPE_DEFERRED | Evidence-graph 存储;0 caller |
| `admin` | **PARTIAL** | V310-14 PR #3795 ships real mysqladmin CLI binary (F-32 shell); 半主路径 |
| `unified-query` | SCOPE_DEFERRED | 跨源查询 facade;0 caller |
| `unified-storage` | SCOPE_DEFERRED | 存储后端抽象;无消费方 |
| `vector` | SCOPE_INTERNAL | crate storage 内部使用,非独立主路径 |

**v3.11+ 推荐动作**: 建立正式的产品评审 (hermes + openclaw 协作) 把 8 个 SCOPE_DEFERRED 收敛为以下三选一:
1. **集成** (进入 mysql-server 或 cli 工具集)
2. **归档** (移出 workspace,保留历史 commit)
3. **保留** (继续作为长期 extension,延后判断)

详见 `docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md §5.2`。

---

## 4. 其他非主路径组件 (历史遗留)

| 组件 | 位置 | 状态 | v3.10.0 处理 |
|------|------|------|--------------|
| `crates/executor/src/local_executor.rs` | (3031 行,3 KB) | DEAD CODE | **v3.10.0 删除** (2026-07-12);文件从未进入 `lib.rs` mod 树 |
| `crates/executor/src/local_executor_dml.rs` | (275 行) | DEPRECATED since v3.9.0 | 保留以兼容历史测试;v3.11+ 评估删除 |
| `crates/server` (sqlrustgo-server) | (整个 crate) | **DEPRECATED** | 仅被 ~13 个集成测试引用;v3.11.0 应将测试切换到 `sqlrustgo-mysql-server` 路径 |
| `expression` (vs `expr`) | (expressional 重叠) | DEFERRED | v3.7.0 已知;`expr` 为主;`expression` 保留作为 fallback |

---

## 5. v3.10.0 GA 推荐的最终冲刺窗口

按 `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` 的判断,GA 窗口应聚焦:

1. ✅ **INT-2 (PR #3767/#3703/#3790)** — CLOSED
2. ✅ **ARCH-3 (PR #3787/#3790)** — CLOSED
3. ✅ **SEM-1 (rollback MVCC)** — CLOSED
4. ✅ **F-16 (GapLocking, PR #3788)** — CLOSED (主路径)
5. ✅ **T-19 / T-20 (PR #3780)** — CLOSED (C-5 crash recovery + I/O delay)
6. ✅ **F-32 (mysqladmin binary, PR #3795)** — PARTIAL
7. ✅ **`local_executor.rs` (3031 行死代码)** — DELETED (v3.10.0 prep)
8. ✅ **CI `local_executor_test` 引用** — REMOVED (stale ref)
9. ✅ **`debt-registry.yaml`** — UPDATED to v3.10.0 snapshot
10. ✅ **`ISOLATED_MODULES.md` (本文件)** — REWRITTEN for v3.10.0

**v3.10.0 GA 不应试图**:

- ❌ 在 GA 前完成 9 项 F-XX ISOLATED 的主路径集成 (~960 LOC 重构)
- ❌ 集成 8 个 extension crate (产品决策)
- ❌ 实现 F-03/F-30/F-36 (0 代码 → GA 是反常)
- ❌ 大范围重构 execution_engine.rs (已 1212 行,达标)

**v3.11.0 应承担**:

- "孤岛消灭计划 Phase 1" (9 项 F-XX 中 BTree/storage 子集)
- 8 个 extension crate 的归档/集成决策
- 覆盖率提升至 ≥80% (V310-10, 40h)
- TPC-H SF=1 完整 22/22 (V310-11a/b, 需 75GB 磁盘)
- 真实 24h SOAK on v3.10.0 RC (V310-05, 增强信心)

---

## 6. 引用

- SSOT: [`docs/governance/debt/debt-registry.yaml`](docs/governance/debt/debt-registry.yaml)
- 闭环追踪报告: [`docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md`](docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md)
- 架构债分析: [`docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md`](docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md)
- 模块生命周期: [`docs/MODULE_LIFECYCLE.md`](docs/MODULE_LIFECYCLE.md)
- 分布式执行设计: [`docs/architecture/DISTRIBUTED_EXECUTION.md`](docs/architecture/DISTRIBUTED_EXECUTION.md)

---

**版本**: v3.10.0 (2026-07-12)
**作者**: openclaw + hermes
**下次更新**: v3.11.0 启动窗口 (target 2026-09-01)

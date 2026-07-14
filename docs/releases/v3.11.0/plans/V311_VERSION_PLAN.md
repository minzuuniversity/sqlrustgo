# SQLRustGo v3.11.0 版本计划 — 债务清零 + 功能孤岛集成

> **版本**: v3.11.0
> **类型**: **债务清零 + 功能孤岛集成**
> **分支**: `develop/v3.11.0` (待从 `develop/v3.10.0` 创建)
> **当前阶段**: **DRAFT** (设计与规划)
> **创建日期**: 2026-07-13
> **目标**: 彻底解决 v3.6.0→v3.10.0 所有遗留债务，完成 9 项 F-XX 主路径集成，解决 v3.10.0 实测发现的性能瓶颈
> **前版本**: v3.10.0 (develop/v3.10.0, 已闭环 INT/ARCH 100%, SEM 50%)

---

## 1. 战略定位

### 1.1 核心问题

v3.10.0 解决了 INT/ARCH/SEM 主路径的集成问题，但仍有 23 项债务 OPEN：

1. **9 项 F-XX ISOLATED**：测试 PASS 但 0 主路径调用（F-23 Clustered Index 等）
2. **3 项 F-XX NOT IMPLEMENTED**：0 代码（F-03 GIS, F-30 SEQUENCE, F-36 列权限）
3. **2 项 SEM IN_PROGRESS**：SEM-3 ALTER TABLE RENAME/MODIFY, SEM-4 覆盖率
4. **9 项其他 OPEN/DEFERRED**：admin PARTIAL, 各种非主路径 crate
5. **11 个 extension crate**：产品决策未定

**v3.10.0 实测发现（Issue #3792）**：Q4 相关子查询占 96% 总执行时间，并行优化空间受限

### 1.2 v3.11.0 战略决策

**核心问题**：v3.10.0 已经做到"核心生产路径无债务"，但仍有大量功能代码和扩展 crate 与主路径脱节。v3.11.0 必须解决三个问题：

1. **功能孤岛整合**：9 项 F-XX 集成到主路径，让"测试 PASS 但用不上"变成"用户能用"
2. **遗留债清零**：SEM-3 ALTER 完整、覆盖率 ≥80%、3 个 NOT IMPL 落地
3. **性能瓶颈突破**：Q4 相关子查询优化、Hash Semi Join 实现

### 1.3 v3.11.0 vs v3.10.0 关系

| 维度 | v3.10.0 | v3.11.0 |
| --- | --- | --- |
| 主题 | MySQL 5.7 替代 (新功能) | **债务清零 + 功能集成** |
| 核心范围 | 主路径功能稳定 | **功能可用性 + 性能突破** |
| 主要工作 | 26 任务 / ~500h | **22 任务 / ~720h** |
| F-XX ISOLATED 处理 | 1/10 集成 | **10/10 集成（100%）** |
| F-XX NOT IMPL 处理 | 2/5 闭环 | **5/5 闭环（100%）** |
| Extension Crates | 11 项 SCOPE_DEFERRED | **产品决策落地** |
| 覆盖率目标 | ≥80% | **≥85%（更高目标）** |
| Q4 性能 | 14.5 分钟 (3M 行) | **目标 <5 分钟 (1.5x+ 加速)** |
| 文档架构 | 1 个 VERSION_PLAN + 5 个 plans | **1 个 VERSION_PLAN + 5 个 plans（一致）** |

### 1.4 v3.11.0 不做

明确边界（避免范围蔓延）：

- ❌ **新架构层**：分布式 / Cypher / SIMD 重写（属于 v3.12+）
- ❌ **MySQL 8.0 新特性**：窗口函数扩展、CTE 递归等已在 v3.10 范围外
- ❌ **重大 wire 协议变更**：保持 MySQL 5.7 wire protocol 兼容
- ❌ **新 SQL 方言**：仅 MySQL 5.7 兼容
- ❌ **物理备份重做**：基于现有 binlog + checkpoint（v3.10 已有）

---

## 2. v3.11.0 核心目标 (5 大维度)

### 2.1 维度 A: 历史债务清零 (P0)

| 类别 | 项 | 工作量 | 优先级 |
| --- | --- | --- | --- |
| **F-XX ISOLATED** | F-23 Clustered Index (主路径集成) | 80h | P0 |
| | F-24 Adaptive Hash Index (主路径集成) | 60h | P0 |
| | F-25 Change Buffer (主路径集成) | 40h | P0 |
| | F-26 Double-Write Buffer (主路径集成) | 50h | P0 |
| | F-27 Table Compression (LZ4/zstd) | 50h | P1 |
| | F-29 Row-Level Security (主路径集成) | 40h | P1 |
| | F-31 Performance Schema (instrumentation hooks) | 30h | P1 |
| | F-32 MySQL Admin (sqlrustgo-admin 与 mysql-server 集成) | 30h | P1 |
| | F-35 Password Rotation (主路径集成) | 20h | P1 |
| **F-XX NOT IMPL** | F-03 GIS 空间数据类型 | 80h | P1 |
| | F-30 CREATE SEQUENCE | 20h | P1 |
| | F-36 列级权限 | 40h | P0 |
| **SEM IN_PROGRESS** | SEM-3 ALTER TABLE RENAME/MODIFY 完成 | 20h | P0 |
| | SEM-4 覆盖率 ≥85% | 60h | P0 |
| **GA-P0 / GA-P1** | #3648 TPC-H 混合负载 SOAK (Hermes 协作) | 80h | P1 |
| | #3423 TPC-H SF=1.0 baseline | 80h | P0 |
| | #3136 check_cross_version_debt.sh 升级 | 8h | P2 |
| **小计** | 17 项 | **780h** | — |

### 2.2 维度 B: Extension Crate 产品决策 (P1)

| Crate | 状态 | v3.11.0 决策 | 工作量 |
| --- | --- | --- | --- |
| `agentsql` | SCOPE_DEFERRED | **决策: 删除** (无用户、无 MySQL 5.7 等价物) | 8h |
| `gmp` | SCOPE_DEFERRED | **决策: 归档** (移至 `archive/v3.11/` 不参与 build) | 4h |
| `rag` | SCOPE_DEFERRED | **决策: 删除** (依赖 gmp, 无 MySQL 等价物) | 4h |
| `distributed` | FROZEN | **决策: 删除** (8K LOC, 0 集成) | 16h |
| `graph` | SCOPE_DEFERRED | **决策: 归档** (Cypher 属 v3.12+) | 8h |
| `qmd-bridge` | SCOPE_DEFERRED | **决策: 删除** (0 调用方) | 2h |
| `evidence-graph` | SCOPE_DEFERRED | **决策: 删除** (gate tool 独立使用) | 2h |
| `admin` | PARTIAL | **决策: 集成** (mysql-server connect 实现) | 30h |
| `unified-query` | SCOPE_DEFERRED | **决策: 删除** (依赖 graph/vector) | 4h |
| `unified-storage` | SCOPE_DEFERRED | **决策: 删除** (依赖 graph, 0 文档) | 4h |
| `vector` | SCOPE_INTERNAL | **决策: 保留** (HNSW/IVF/PQ 是 storage 内部能力) | 0h |
| **小计** | 11 项 | 5 删 + 3 归档 + 1 集成 + 1 保留 | **84h** |

### 2.3 维度 C: 性能瓶颈突破 (P0)

基于 Issue #3792 实测发现的 v3.10.0 性能瓶颈：

| 项 | 内容 | 工作量 | 优先级 |
| --- | --- | --- | --- |
| **Q4 相关子查询** | Hash Semi Join 算子 (450K orders × 3M lineitem) | 80h | P0 |
| **子查询去相关** | decorrelation optimizer pass | 60h | P1 |
| **Hash Anti Join** | NOT EXISTS / NOT IN 优化 | 40h | P1 |
| **CTE 物化** | WITH ... AS (SELECT) 物化中间结果 | 30h | P1 |
| **小计** | 4 项 | **210h** | — |

### 2.4 维度 D: 文档架构整理 (P1)

v3.10.0 文档存在版本演进混乱（多个 VERSION_PLAN.md），需统一：

| 文档 | v3.10.0 现状 | v3.11.0 整改 |
| --- | --- | --- |
| `VERSION_PLAN.md` | v3.10.0 入口 | ✅ 保留 |
| `plans/V310_VERSION_PLAN.md` | 详细 | ❌ 删除（合并到 VERSION_PLAN.md） |
| `plans/V310_DEVELOPMENT_PLAN.md` | 26 任务 | ❌ 删除（合并到 V311_DEVELOPMENT_PLAN.md） |
| `plans/V310_ISSUES_PLAN.md` | 12 子 ISSUE | ❌ 删除（合并到 debt-registry.yaml） |
| `plans/V310_10_COVERAGE_PLAN.md` | 覆盖率 | ❌ 删除（合并到 V311_DEVELOPMENT_PLAN.md） |
| `plans/V310_CLI_BINARY_PLAN.md` | CLI | ❌ 删除（合并到 V311_DEVELOPMENT_PLAN.md） |
| `plans/V311_VERSION_PLAN.md` | — | ✅ 新建（详细） |
| `plans/V311_DEVELOPMENT_PLAN.md` | — | ✅ 新建（22 任务） |
| `plans/V311_DEBT_CLOSURE_PLAN.md` | — | ✅ 新建（基于 debt-registry） |
| `plans/V311_DOCS_RESTRUCTURE_PLAN.md` | — | ✅ 新建（本次整改） |

**工作量**: 4h (机械整理) + 8h (内容合并)

### 2.5 维度 E: 治理与基础设施 (P2)

| 项 | 内容 | 工作量 |
| --- | --- | --- |
| `debt-registry.yaml` | v3.11.0 snapshot 重生成 | 4h |
| `ISOLATED_MODULES.md` (root) | v3.11.0 重写 | 4h |
| `MAINLINE_COMPONENTS.md` (root) | v3.11.0 更新 | 2h |
| Gate scripts 更新 | 新增 F-XX 集成 gate | 8h |
| **小计** | 4 项 | **18h** |

---

## 3. v3.11.0 总工作量

| 维度 | 工作量 | 比例 |
| --- | --- | --- |
| A: 历史债务清零 | 780h | 62% |
| B: Extension 决策 | 84h | 7% |
| C: 性能瓶颈突破 | 210h | 17% |
| D: 文档架构整理 | 12h | 1% |
| E: 治理与基础设施 | 18h | 1% |
| **合计** | **1104h** | **88%** |

剩余 12% (132h) 用于 buffer（风险、Code Review、回归测试、PR 合并）。

**总预估**: ~1236h ≈ 6 人月 × 1 month（假设 1 个全职 + 2 个协作）

---

## 4. v3.11.0 阶段排期

### 4.1 DRAFT (2026-07-13 ~ 2026-07-20)

- ✅ 创建 v3.11.0 plans/ 5 个文档
- ⏳ 创建 Gitea issue tracker #V311-MASTER
- ⏳ 拆分 22 项为子 issue (#V311-01 ~ #V311-22)
- ⏳ Extension Crate 产品决策评审（5 删 3 归档 1 集成 1 保留）

### 4.2 ALPHA (2026-07-21 ~ 2026-08-10, ~3 周)

- 优先级 P0 项开工：F-23, F-36, SEM-3, Q4 Hash Semi Join
- Extension Crate 5 删 + 3 归档 (P1 任务)
- 文档架构整理 (D 维度)

### 4.3 BETA (2026-08-11 ~ 2026-09-10, ~4 周)

- 完成 P0 全部 9 项
- 完成 P1 全部 10 项
- 开始集成测试 + 回归测试
- GA-P0 硬件依赖项 (TPC-H SF=1, SOAK 168h) 启动

### 4.4 RC (2026-09-11 ~ 2026-09-30, ~3 周)

- 全部债务清零验证（debt-registry.yaml 0 OPEN）
- 覆盖率 ≥85% 验证
- TPC-H SF=1.0 baseline 完成
- Wired-SOAK 完整闭环

### 4.5 GA (2026-10-01)

- 发布 v3.11.0
- 评分目标：A (债务 100% 清零 + 功能 100% 集成 + 性能突破)

---

## 5. v3.11.0 验收标准

### 5.1 必达 (GO/NO-GO)

| Gate | v3.10.0 状态 | v3.11.0 目标 |
| --- | --- | --- |
| G1 TPC-H SF=0.1 22/22 | ✅ PASS | ✅ 保持不退化 |
| G3 覆盖率 ≥85% per crate | ❌ ~67% | ✅ 必达 (3 crate: executor/parser/storage) |
| G4 TPC-H SF=1 22/22 | ⚠️ V310-11c 部分 | ✅ 必达 (22/22) |
| G6 DML 完整性 ignore = 0 | ✅ 0 | ✅ 保持 |
| G7 ACID 5/5 | ✅ 5/5 | ✅ 保持 |
| G8 Crash Matrix (kill -9) | ✅ PASS | ✅ 保持 |
| G11 v3.11.0 债务清零 | ⚠️ 23 OPEN | ✅ **0 OPEN (100% 闭环)** |
| G12 v3.11.0 F-XX 集成 | ⚠️ 1/10 | ✅ **10/10 集成** |
| G13 v3.11.0 Q4 性能 | ⚠️ 14.5 min | ✅ **< 5 min** |

### 5.2 应达 (Soft Gates)

| Gate | v3.10.0 状态 | v3.11.0 目标 |
| --- | --- | --- |
| F-XX NOT IMPL 5/5 闭环 | ⚠️ 2/5 | ✅ **5/5 闭环** |
| Extension Crate 决策 | ⚠️ 11 SCOPE_DEFERRED | ✅ **0 SCOPE_DEFERRED** |
| 文档架构统一 | ❌ VERSION_PLAN 重复 | ✅ **1 个 VERSION_PLAN** |
| G9 24h SOAK | ✅ PASS | ✅ 保持 |

### 5.3 加分 (Optional)

| 项 | 说明 |
| --- | --- |
| F-27 Table Compression LZ4 | v3.10 仅 RLE |
| TPC-H SF=10 实测 | v3.10 已知 SF=10 cap bug 待 fix |
| Wired-SOAK sysbench prepare/run | V310-06/07/08/09 完整闭环 |

---

## 6. 风险与缓解

### 6.1 风险

| 风险 | 概率 | 影响 | 缓解 |
| --- | --- | --- | --- |
| **F-23 Clustered Index 集成复杂** | 高 | 高 | 80h 预算含 spike 验证；如不可行，延后到 v3.12 |
| **GIS (F-03) 实现深度** | 中 | 中 | 仅实现 POINT + WITHIN，不实现 POLYGON/GEOMETRY 等复杂类型 |
| **Extension 删除破坏现有测试** | 中 | 低 | 删除前先 grep 所有引用；archived 不删除，仅 exclude from workspace |
| **Q4 Hash Semi Join 性能不达预期** | 中 | 高 | 60h 用于 spike + 60h 用于 fallback (Decorrelation + Filter push-down) |
| **TPC-H SF=1 fixture 准备** | 中 | 中 | 与 #3423 协作，使用现有 dbgen 流程 |

### 6.2 决策回退

如果 ALPHA 阶段（2026-08-10）发现 F-23 / F-24 集成超预算，回退方案：

- **F-23 Clustered Index**: 标记 v3.12，保持 ISOLATED
- **F-24 Adaptive Hash Index**: 标记 v3.12，保持 ISOLATED
- **优先 F-36 列权限 + F-35 Password Rotation + SEM-3 + Q4 优化**（核心 P0）

---

## 7. v3.11.0 不做清单（再次确认）

明确不做（防止 scope creep）：

- ❌ Cypher / 图查询扩展（v3.12+）
- ❌ Vector SQL 增强（v3.12+）
- ❌ 分布式 / Raft / gRPC（v3.12+）
- ❌ MySQL 8.0 新特性（CTE 递归、窗口函数扩展）
- ❌ 物理备份重做（基于 v3.10 的 binlog + checkpoint）
- ❌ Wire 协议重大变更
- ❌ SIMD 重写执行引擎

---

## 8. 关联资源

- `plans/V311_DEVELOPMENT_PLAN.md` — 22 任务详细分解
- `plans/V311_DEBT_CLOSURE_PLAN.md` — 基于 debt-registry.yaml 的清零计划
- `docs/governance/debt/debt-registry.yaml` — SSOT（v3.11.0 必达 0 OPEN）
- `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` — 上版本债务基线
- `docs/releases/v3.10.0/PARALLEL_EXECUTOR_OPTIMIZATION.md` — 性能瓶颈背景
- `docs/releases/v3.10.0/perf/PERFORMANCE_BASELINE.md` — 实测 baseline

---

*Created: 2026-07-13 (DRAFT stage init)*
*Author: openclaw (基于 v3.10.0 评估报告 + Issue #3792 实测数据)*

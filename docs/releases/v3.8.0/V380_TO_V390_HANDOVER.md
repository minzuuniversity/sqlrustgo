# v3.8.0 遗留问题与 v3.9.0 整改计划

> **Date**: 2026-06-04
> **Author**: Hermes Agent
> **Audience**: v3.9.0 开发团队
> **Prior report**: `V380_COMPREHENSIVE_ASSESSMENT.md` (21.5K, 20 sections)

---

## 0. TL;DR

v3.8.0 进入 **Feature Freeze**. 9 open issues 移交 v3.9.0.
v3.9.0 = **Verification Release** (4 项 KPI, 无新 Feature).
**总 180h ≈ 4.5 周 × 1 人** → v3.9.0 = 第一个真正可讨论 RC 的版本.

---

## 1. v3.8.0 遗留问题 (9 Open)

### 1.1 P0 (0)
~~#2966 INT-1 DML Bypass~~ — **CLOSED (PR-3019)** ✅

### 1.2 P1 (5)

| Issue | 标题 | 详情 | 阶段 |
|-------|------|------|------|
| **#2977** | TPC-H 10/22 → 22/22 | Stage 4 (用户跳过, v3.9.0 必做) | **KPI-1** |
| **#2973** | INT-4 VtuGuard 强制 DML 经过 TM | explicit TX wrap 待补 (autocommit 已修) | **KPI-2** |
| **#2974** | ARCH-2 merge.rs 统一 DML 入口 | 标准化 DML 入口 | **KPI-2** |
| **#2975** | SEM-1 执行语义标准化 | NULL/比较/算术语义统一 | **KPI-2** |
| **#2702** | v3.8.0 历史遗留问题改进核实 | 评审请求 | v3.8.0 PR 验证 |

### 1.3 P2 (1)
- **Corpus 57 fail (MySQL 5.7 高级函数 parser)** — parser 增强, P1 后续

### 1.4 追踪 (3)
- #2763, #2743, 历史报告类

---

## 2. v3.8.0 已知不达 RC 门槛 (4 项)

| 项 | 内容 | 状态 |
|----|------|------|
| 1 | Transaction/WAL 主路径统一 | ✅ INT-1 修 |
| 2 | TPC-H 10/22 → 22/22 | ❌ 待 v3.9.0 KPI-1 |
| 3 | Corpus Failures 分类清零 | ⚠️ 57 fail 全是 MySQL 5.7 函数 parser |
| 4 | 系统级压力测试 | ❌ 待 v3.9.0 KPI-3 |
| 5 | 长时间稳定性 (24h-168h) | ❌ 待 v3.9.0 KPI-4 |

---

## 3. v3.9.0 整改计划 (4 项 KPI)

### 3.1 KPI-1: TPC-H 22/22 (60h)

**目标**: TPC-H Q1-Q22 全部跑通 (结果正确, 不要求极致性能).

| Task | 详情 | 工作量 | 状态 |
|------|------|--------|------|
| T1.1 Q1-Q8 基础聚合 | COUNT/SUM/AVG/GROUP BY/HAVING | 10h | v3.8.0 GROUP BY 核心 100% 已具备 |
| T1.2 Q9-Q13 JOIN | INNER/LEFT/multi-join | 15h | v3.8.0 JOIN 核心 100% 已具备 |
| T1.3 Q14-Q17 表达式 | CASE WHEN/CAST | 10h | 部分已具备, 需补全 |
| T1.4 Q18-Q22 子查询+窗口 | Subquery/CTE/Window | 25h | **主要瓶颈** |

**TPC-H 路线**:
1. 第 1 周: Q1-Q13 (基础聚合 + JOIN) — 25h
2. 第 2 周: Q14-Q17 (表达式) — 10h  
3. 第 3 周: Q18-Q22 (子查询+窗口) — 25h

**验证**:
- 用 `tpc-h/` 测试套件 (PR-2902 已集成)
- 每个 Q 跑 100 次, 验证结果稳定性
- 与 MySQL 5.7 结果对比 (值正确性, Phase 2d TPC-H value-correctness gate)

### 3.2 KPI-2: 架构债务收口 (50h)

| Task | 详情 | 工作量 | Issue |
|------|------|--------|-------|
| T2.1 INT-4 VtuGuard 强制 | explicit TX wrap | 15h | #2973 |
| T2.2 ARCH-2 merge.rs 统一 DML | 标准化入口 | 15h | #2974 |
| T2.3 SEM-1 执行语义 | NULL/比较/算术统一 | 20h | #2975 |

**关键路径**:
- INT-4 修法: 给 VtuGuard 加 explicit_tx_mode flag, 让 BEGIN/COMMIT 路径也走 VtuGuard
- ARCH-2 修法: 重构 `execution_engine.rs`, 把 INSERT/UPDATE/DELETE 入口统一到 `merge_dml.rs`
- SEM-1 修法: 写 `SEMANTICS.md` 标准文档, 列出所有 SQL 操作的语义约定

### 3.3 KPI-3: 压力测试 (40h)

| Task | 详情 | 工作量 |
|------|------|--------|
| T3.1 1M SQL 自动执行 | 持续 INSERT/UPDATE/DELETE/SELECT 混合 | 15h |
| T3.2 10M SQL 极限测试 | 24h 跑完 10M SQL | 15h |
| T3.3 崩溃恢复压力 | 每 100K SQL 强制 kill -9 验证 WAL recovery | 10h |

**工具链**:
- 写 `tools/stress_runner/` (基于现有 wire protocol)
- 自动生成 SQL 模板 (1K 个)
- CI 集成: 每日 nightly + 每周 full stress

### 3.4 KPI-4: 长稳测试 (30h active)

| Task | 详情 | 工作量 |
|------|------|--------|
| T4.1 24h 持续运行 | CI nightly 跑 24h 无 crash | 5h |
| T4.2 72h 持续运行 | 模拟 3 天业务负载 | 10h |
| T4.3 168h 持续运行 | 模拟 1 周业务负载 | 15h |

**注**: 168h 实际等待 1 周, 工作量是"维护脚本 + 异常分析".
**主要验证**:
- 无 Crash
- 无 Data Loss (WAL recovery 验证)
- 无 Deadlock
- 无 Corruption
- 内存泄漏 (heaptrack 监控)
- 磁盘 I/O 异常

---

## 4. v3.9.0 时间节点 (估算)

| Week | 阶段 | 内容 | 工作量 |
|------|------|------|--------|
| W1 | KPI-1 T1.1-1.2 | TPC-H Q1-Q13 | 25h |
| W2 | KPI-1 T1.3-1.4 + KPI-2 T2.1 | TPC-H Q14-Q22 + INT-4 | 35h |
| W3 | KPI-2 T2.2-2.3 | ARCH-2 + SEM-1 | 35h |
| W4 | KPI-3 T3.1-3.3 | 压力测试 | 40h |
| W5 | KPI-4 T4.1-4.3 | 长稳测试 (active work) | 30h |
| W5-W12 | KPI-4 168h | 长稳测试 (wait) | 0h active |
| **总计** | | | **165h active + 168h wait** |

**预计日期**:
- v3.9.0-rc1: 2026-07-09 (5 周 active)
- v3.9.0-ga: 2026-07-23 (5 周 active + 2 周 wait)

---

## 5. v3.9.0 不允许的 Feature

按 ChatGPT 路线图, v3.9.0 是 **Verification Release**, 不接受新 Feature:

- ❌ SIMD 集成 (v3.10.0+ 考虑)
- ❌ Vector 集成
- ❌ 新 SQL 语法 (CTE/MySQL 8.0 等)
- ❌ 新索引 (B+ tree/AHI/Change Buffer 改进)
- ❌ 新优化器特性
- ❌ 任何"看起来有吸引力"但会拖慢 KPI 验证的 PR

**所有 PR 必须在标题加 `[v390]` 标记 + 关联 4 项 KPI 之一**.

---

## 6. v3.8.0 Feature Freeze 持续

v3.8.0-beta 进入 **Feature Freeze**. 只接受:

- ✅ P0/P1 Bug 修复 (Crash, Data Loss, Deadlock, Corruption)
- ✅ 文档完善 (DOC 5 步流程)
- ✅ 9 维门禁的 bug fix

---

## 7. 关键文档入口

- 综合评估: `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` (21.5K, 20 sections)
- 发布说明: `docs/releases/v3.8.0/RELEASE_NOTES.md` (v3.2)
- Beta 发布报告: `docs/releases/v3.8.0/V380_BETA_RELEASE_REPORT.md` (11K)
- 9 维门禁脚本: `scripts/gate/check_*.sh` (9 个)
- TPC-H 套件: `tpc-h/` + `crates/sql-corpus`
- 压力测试工具 (待写): `tools/stress_runner/`
- 长稳测试脚本 (待写): `tools/long_haul/`

---

## 8. 致 v3.9.0 团队

**核心原则 (按 ChatGPT 评估)**:
1. **Feature Test Pass ≠ System Integration Pass**: 关键路径闭环, 而非数量堆
2. **TPC-H 是组合压力测试**: Join + Aggregate + Subquery + Sort + Expression
3. **1M SQL 跑过 ≠ 稳定**: 1000 次正常, 1001 次可能炸掉
4. **8.0/10 ≠ GA Ready**: 8.0/10 仍只是 Strong Beta

**不要做的事**:
- 不要在 v3.9.0 周期内追"更多功能"
- 不要把 v3.8.0 拉长成无限膨胀的发布周期
- 不要在压力测试中"跳过 1001 次"

**做 4 件事**:
- TPC-H 22/22 (KPI-1)
- 架构债务收口 (KPI-2)
- 压力测试 (KPI-3)
- 长稳测试 (KPI-4)

完成后, v3.9.0 = **第一个真正有资格讨论 RC 的版本**.

---

**v3.9.0 = Verification Release, 180h 距 RC, 4 项 KPI 决定一切.**

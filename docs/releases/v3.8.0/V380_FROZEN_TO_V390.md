# v3.9.0 Frozen Items (从 v3.8.0 Beta 阶段冻结)

> **Date**: 2026-06-04 (rewritten after Route B decision)
> **Author**: Hermes Agent
> **Audience**: v3.9.0+ 后续版本开发团队
> **Status**: v3.8.0 走完整生命周期到 GA, **不创建 v3.9.0**.
>            本文档列出从 v3.8.0 冻结到后续版本的工作项.

---

## 0. 路线决策 (2026-06-04, user directive)

**原路线**: v3.8.0 Beta → v3.9.0 Alpha → v3.9.0 Beta (双重版本碎片化)

**新路线 (Route B)**:
```
v3.8.0-beta  →  v3.8.0-rc1  →  v3.8.0-rc2  →  v3.8.0-ga
```

**理由**:
- 当前工作属于"补完与收敛",不是"新增能力"
- SIMD / Parallel / Vector / 新优化器 等"全新架构增强"明确冻结到 v3.9.0+ 后续
- 用户难以理解连续多个"半成品版本"

**本文档用途**: 列出从 v3.8.0 冻结到 v3.9.0 的工作项, 作为后续版本 backlog.

---

## 1. 冻结清单 (FROZEN to v3.9.0+)

### 1.1 性能工程 (Performance Engineering)

| 模块 | 说明 | 规格 |
|------|------|------|
| SIMD Executor | 向量化执行 | INT-2 Phase 2+ |
| Parallel Executor 主路径 | 集成到 ExecutionEngine | INT-2 Phase 3-4 (SPEC #3014) |
| sysbench OLTP_READ_WRITE | MySQL 5.7 比较 | PERF-01 Phase 2-3 (SPEC #3015) |
| 索引统计 | #2295 | backlog |

### 1.2 新增能力 (New Capabilities)

| 模块 | 说明 | 规格 |
|------|------|------|
| Vector SQL 表面 | CREATE VECTOR INDEX + ORDER BY vector_distance | VEC-01 Phase 2-5 (SPEC #3016) |
| CTE Recursive | 递归 CTE 解析器 | #2987 |
| MySQL 5.7 高级函数 (续) | ROLLUP / CUBE / INSERT / REPLACE / RANK 等 | #2988 follow-up |
| 新优化器 | CBO 重构 / Histogram-based | TBD |
| Aggregates 扩展 | STDDEV / VARIANCE / MEDIAN / GROUP_CONCAT (mod-tree 死代码) | EXEC-03 (#2969) — 5 个基础 aggregates (Count/Sum/Avg/Min/Max) 已工作，4 个高级冻结 |

### 1.3 集成债务 (Integration Debt)

| 模块 | 说明 |
|------|------|
| I-12 Parallel Executor 完整集成 | 与 ExecutionEngine::execute 路由 (SPEC #3014) |
| wal-verification 工具 | 当前废弃, 待重新设计 |

### 1.4 暂不实施 (Parked)

| 项目 | 原因 |
|------|------|
| F-32 mysqladmin | 工具完整度低, 后置 (#2937) |
| Graph Database (Cypher) | 暂未列入主线 |

---

## 2. v3.9.0 (后续) 入口标准

v3.9.0 启动条件:
1. v3.8.0-ga tag 已打
2. CHANGELOG.md v3.8.0-ga 段已发布
3. main 分支已同步 v3.8.0-ga (需 PM 授权)
4. v3.8.0-ga 后 24h 灰度期无 critical issue

---

## 3. 旧版 HANDOVER 文档

旧 `V380_TO_V390_HANDOVER.md` (PR-3029) 内容已重写为本文件.
原始版本在 git history 中可追溯 (`git log --all -- docs/releases/v3.8.0/V380_TO_V390_HANDOVER.md`).

---

## 4. References

- `docs/releases/v3.8.0/RELEASE_NOTES.md` — v3.8.0 主发布说明
- `docs/releases/v3.8.0/verify/SERVER_01_ALPHA_ACCEPTANCE.md` — Server 验收
- `docs/governance/EXECUTION_SEMANTICS.md` — SEM-1 contract
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md` — Issue 关闭规则


---

## 0. 核心判断 (ChatGPT 最终)

**问题**: 3.9.0 能彻底解决所有历史遗留问题，提供一个可以初步用于**简单生产环境**的单机版吗？

**答案**:
- ❌ **不能彻底解决所有历史遗留问题** (数据库项目不会在一个版本里把所有债务归零)
- ✅ **有机会做到"简单生产环境可用的单机版"** (前提: 严格 Feature Freeze + 5 项条件)

---

## 1. "简单生产环境"定义

### 1.1 ✅ 适用 (3.9.0 目标可达)

| 类别 | 例子 |
|------|------|
| 内部业务 | 内部业务系统, 中小后台管理, 配置中心, CI/CD 元数据 |
| 协作类 | 工单系统, 监控系统, 实验室系统 |
| 教学类 | 教学平台, 企业内部工具 |

**规模约束**:
- 并发: 10~100
- 数据: < 100GB
- 部署: 单机
- 负载: 每天几万~几十万 SQL

### 1.2 ❌ 不适用 (4.x/5.x 才可能)

- 替代 MySQL
- SaaS 核心库
- 金融交易, 电商订单, 银行系统, ERP 核心库

---

## 2. 数据库生产可用的三层标准

### 2.1 第一层: 正确性 (v3.8.0 已接近完成)

```
✅ Parser
✅ JOIN
✅ GROUP BY
✅ DISTINCT
✅ Aggregate
✅ NULL
✅ WAL 主路径 (INT-1 修)
```

**这是 Beta 阶段的核心**.

### 2.2 第二层: 可靠性 (v3.9.0 核心目标)

```
❌ 24h 持续运行
❌ 72h 持续运行
❌ 168h 持续运行
```

**这是 v3.9.0 重点**.

**典型长稳问题** (单元测试抓不到):
- 内存泄漏
- 锁泄漏
- WAL 增长失控
- Snapshot 积压
- 文件句柄泄漏
- 死锁

### 2.3 第三层: 恢复能力 (v3.9.0 关键 KPI)

**ChatGPT 最关注这层**:

```text
生产环境真正考验:
不是 "是否会崩"
而是 "崩了以后能否回来"
```

**Crash Test Matrix** (v3.9.0 必须建立):
```
执行 SQL
  ↓
随机 kill -9
  ↓
重启
  ↓
校验数据
  ↓
循环 1000/10000 轮
```

---

## 3. v3.9.0 错误 vs 正确路线

### 3.1 ❌ 错误路线 (禁止)

继续做 SIMD / Vector / 新 SQL 函数 / 更多索引, 不会提高生产可用性.

### 3.2 ✅ 正确路线 (Feature Freeze + 5 项 KPI)

| # | KPI | 内容 | 工作量 |
|---|-----|------|--------|
| **1** | TPC-H 22/22 | 执行器毕业考试 | 60h |
| **2** | Crash Test Matrix | 1000/10000 轮 kill -9 验证 | 50h |
| **3** | 长稳测试 | 24h/72h/168h Nightly | 30h active |
| **4** | 并发压力 | 64 线程 INSERT/UPDATE/DELETE/SELECT | 25h |
| **5** | WAL 一致性 | LSN/Checkpoint/Recovery 边界 | 15h |
| **总计** | | | **180h** |

**6 严禁**:
- ❌ SIMD
- ❌ Vector
- ❌ 新 SQL 语法
- ❌ 新索引
- ❌ 新优化器
- ❌ 任何 Feature 提交

---

## 4. 5 项 KPI 详细

### 4.1 KPI-1: TPC-H 22/22 (60h)

| 阶段 | 内容 | 工作量 |
|------|------|--------|
| Q1-Q8 基础聚合 | COUNT/SUM/AVG/GROUP BY/HAVING | 10h |
| Q9-Q13 JOIN | INNER/LEFT/multi-join | 15h |
| Q14-Q17 表达式 | CASE WHEN/CAST | 10h |
| Q18-Q22 子查询+窗口 | Subquery/CTE/Window | 25h |

**v3.8.0 已具备**: GROUP BY 81/81 + JOIN 核心 100% (基础聚合+JOIN 不再是瓶颈).

**v3.9.0 主攻**: Q18-Q22 (子查询+窗口) — **主要瓶颈**.

### 4.2 KPI-2: Crash Test Matrix (50h, ChatGPT 最关注)

| Task | 详情 | 工作量 |
|------|------|--------|
| Crash Harness 工具 | `tools/crash_harness/` 自动 kill -9 + 重启 + 校验 | 20h |
| 1000 轮循环 | 持续 INSERT/UPDATE/DELETE/SELECT 混合 + 随机 kill | 15h |
| 10000 轮极限 | 7×24 跑完 10K 轮 | 15h |

**核心验证**:
```rust
// 伪代码
for i in 0..10000 {
    // 1. 执行一批 SQL
    exec_batch(&random_sqls(10))?;
    
    // 2. 随机 kill -9 (30% 概率)
    if rand::random::<f32>() < 0.3 {
        kill_process();
    }
    
    // 3. 重启
    restart_engine()?;
    
    // 4. 校验数据一致性
    assert_data_consistent()?;
}
```

**关键不变量**:
- 提交的事务不丢失 (WAL fsync 验证)
- 未提交的事务回滚 (Undo log 验证)
- Page checksum 一致
- 索引与表数据一致

### 4.3 KPI-3: 长稳测试 (30h active, 168h wait)

| Task | 详情 | 工作量 |
|------|------|--------|
| 24h Nightly | CI 每日跑 24h 无 crash | 5h |
| 72h Weekly | 模拟 3 天业务负载 | 10h |
| 168h Monthly | 模拟 1 周业务负载 | 15h |

**监控指标**:
- Heap 内存增长 (heaptrack)
- File descriptor 数量
- 活跃事务数
- WAL 文件大小
- Checkpoint 频率
- Deadlock 计数

### 4.4 KPI-4: 并发压力 (25h)

| Task | 详情 | 工作量 |
|------|------|--------|
| 64 线程混合负载 | 持续 INSERT/UPDATE/DELETE/SELECT | 15h |
| 锁竞争测试 | 高并发同一行更新 | 5h |
| MVCC 隔离验证 | 读写并发下 snapshot 一致性 | 5h |

### 4.5 KPI-5: WAL 一致性 (15h)

| Task | 详情 | 工作量 |
|------|------|--------|
| LSN 单调性 | 所有 LSN 严格递增 | 3h |
| Checkpoint 边界 | 强制 checkpoint 时不丢数据 | 5h |
| Recovery 全场景 | partial write / torn page / OOM | 7h |

---

## 5. v3.8.0 → v3.9.0 预期改善

| 项目 | v3.8.0 | **v3.9.0 目标** | 提升 |
|------|--------|-----------------|------|
| SQL Executor | 6.5 | **8** | +1.5 |
| TPC-H | 10/22 | **22/22** | +12 |
| DML/WAL | 8 | **9** | +1 |
| **稳定性** | **3** | **7** | **+4** ⭐ |
| **恢复能力** | **5** | **8** | **+3** ⭐ |
| **压力测试** | **2** | **7** | **+5** ⭐ |
| **单机生产能力** | **4** | **7** | **+3** ⭐ |

**核心改善**: 不是 SQL 功能, 而是**稳定性 + 恢复 + 压力 + 单机生产能力** (4 项 +15).

---

## 6. 9 Open Issues 移交

### 6.1 P1 (5, 全部移交 v3.9.0)

| Issue | 标题 | 关联 KPI |
|-------|------|----------|
| **#2977** | TPC-H 10/22 → 22/22 | **KPI-1** |
| **#2973** | INT-4 VtuGuard 强制 (explicit TX) | **KPI-5 (WAL 一致性)** |
| **#2974** | ARCH-2 merge.rs 统一 DML 入口 | **KPI-4 (并发)** |
| **#2975** | SEM-1 执行语义标准化 | **KPI-1 (TPC-H 子查询)** |
| **#2702** | v3.8.0 历史遗留评审 | 文档 |

### 6.2 P2 (1)
- Corpus 57 MySQL 5.7 函数 parser (P1 后续, v3.9.x 考虑)

### 6.3 追踪 (3)
- #2763, #2743, 历史报告类 (随 v3.9.0 收口)

---

## 7. 历史问题清零的真相 (按 ChatGPT)

> **不能清零**.

原因:
- 真正到生产阶段会出现 **INT-5, INT-6, ARCH-7, SEM-9** (新问题)
- 数据库永远不会出现"历史问题 = 0"的状态
- 更现实目标: **P0 = 0, P1 = 0, 允许存在少量 P2/P3**

**v3.9.0 目标**:
- ✅ P0 保持 0 (新出问题及时修)
- ✅ P1 → 0 (KPI 完成时同步关闭)
- ⚠️ P2 保持少量 (parser 增强)
- ⚠️ P3 允许存在 (边角)

---

## 8. 时间节点 (估算)

| 版本 | 状态 | 预计日期 |
|------|------|----------|
| **v3.8.0-beta** | Strong Beta (现) | 2026-06-04 |
| **v3.8.0-beta+** | Strong Beta + 1-2 P1 | 2026-06-18 (2 周) |
| **v3.9.0-rc1** | 5/5 KPI PASS | 2026-07-23 (7 周) |
| **v3.9.0-ga** | 简单生产可用 | 2026-08-13 (10 周) |

**总工作流**:
- W1-2: KPI-1 TPC-H (Q1-Q13)
- W3: KPI-1 TPC-H (Q14-Q22) + KPI-2 Crash Harness 工具
- W4: KPI-2 1000 轮 + KPI-3 24h + KPI-5 WAL
- W5: KPI-4 并发 + KPI-2 10000 轮 + KPI-3 72h
- W6-7: KPI-3 168h (1 周) + 异常分析
- W8: 收口 + 文档 + 标签

---

## 9. v3.9.0 禁止 Feature (持续 Feature Freeze)

- ❌ SIMD
- ❌ Vector
- ❌ 新 SQL 语法
- ❌ 新索引
- ❌ 新优化器
- ❌ 任何"看起来有吸引力"但会拖慢 KPI 的 PR

**所有 PR 标题必须加 `[v390]` + 关联 5 项 KPI 之一**.

---

## 10. 关键原则 (ChatGPT 第二轮)

### 10.1 不要混淆
- **功能完成 ≠ 生产完成**
- **单元测试 PASS ≠ 1000 轮 kill -9 后数据一致**
- **8.0/10 ≠ 简单生产可用**

### 10.2 三层标准
1. **正确性** (v3.8.0 已接近完成)
2. **可靠性** (v3.9.0 核心)
3. **恢复能力** (v3.9.0 关键 KPI)

### 10.3 长稳问题模式
"第 1 天没事 / 第 3 天没事 / **第 7 天崩**" — 内存泄漏, 锁泄漏, WAL 增长, Snapshot 积压, FD 泄漏, 死锁

### 10.4 生产环境真正考验
"不是 **是否会崩**, 而是 **崩了以后能否回来**"

---

## 11. 致 v3.9.0 团队

如果严格执行 Feature Freeze, 只做 5 项 KPI:

- ✅ TPC-H 22/22
- ✅ Crash Test Matrix (1000/10000 轮)
- ✅ 长稳 24h/72h/168h
- ✅ 并发压力 64 线程
- ✅ WAL 一致性

那么 SQLRustGo 的定位会从:

```text
Beta 数据库原型
```

提升到:

```text
可用于简单生产环境的单机数据库系统
```

**这比增加 SIMD/Vector/新 SQL 语法都有价值**.

---

## 12. 文档链接

- 综合评估: `V380_COMPREHENSIVE_ASSESSMENT.md` (21.5K)
- 发布说明: `RELEASE_NOTES.md` (v3.2)
- 9 维门禁: `scripts/gate/check_*.sh`
- TPC-H 套件: `tpc-h/` + `crates/sql-corpus`
- **Crash Harness 工具 (待写)**: `tools/crash_harness/`
- **长稳测试脚本 (待写)**: `tools/long_haul/`
- **并发压力工具 (待写)**: `tools/stress/`
- **WAL 一致性测试 (待写)**: `tools/wal_consistency/`

---

**v3.9.0 = Verification Release, 5 项 KPI, 180h 距"简单生产可用", 数据库永远不会历史清零, 但可以做到 4 项 P0/P1 全部 0.**

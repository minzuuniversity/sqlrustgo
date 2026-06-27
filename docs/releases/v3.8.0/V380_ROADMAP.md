# v3.8.0 路线图 (长期收敛版本, 不开 3.9.0)

> **Date**: 2026-06-04
> **Author**: Hermes Agent
> **Audience**: v3.8.0-rc1/rc2/ga 开发团队
> **基于**: ChatGPT 第三轮评估 (2026-06-04)
> **Prior**: V380_TO_V390_HANDOVER.md (已废弃), 本文档为正式长期路线图
> **核心理念**: v3.8.0 = 长期收敛版本 = `beta → rc1 → rc2 → ga` (不开 3.9.0)

---

## 0. 决策背景 (按 ChatGPT 第三轮)

> **不要创建 3.9.0**. 把 v3.8.0 作为长期收敛版本.

**判断标准**:
- 剩余工作属于"补完和收敛", 而非"新增能力" → 走 v3.8.0 收敛路线
- 剩余工作属于"全新性能/架构/能力" → 才开 3.9.0

**任务分类**:
| 任务 | 属于 v3.8.0? |
|------|----------|
| INT-1 DML→TM→WAL | ✅ 架构闭环 |
| EXEC-01 GROUP BY | ✅ Executor 完整化 |
| EXEC-02 JOIN | ✅ Executor 完整化 |
| ARCH-2 merge.rs | ✅ 架构 |
| SEM-1 语义标准化 | ✅ 语义 |
| CLI-01 Client CLI | ✅ 可用性 |
| SERVER-01 Alpha Server | ✅ Server 成立 |
| TPC-H 22/22 | ✅ 验证 |
| SIMD Executor | ❌ 全新性能工程 |
| Parallel 主路径 | ❌ 架构增强 |
| Vector SQL | ❌ 新能力 |
| 新优化器 | ❌ 新能力 |
| MySQL 函数扩展 | ❌ 新能力 |

---

## 1. v3.8.0 Release Scope (最终)

### 1.1 目标

```text
单机数据库
Server + CLI
事务
WAL
恢复
JOIN
GROUP BY
TPC-H
```

全部成立.

### 1.2 必须完成 (7 项)

```
INT-1
EXEC-01
EXEC-02
SEM-1
ARCH-2
CLI-01
SERVER-01
```

### 1.3 必须验证 (5 项)

```
TPC-H 22/22
Corpus >95%
Recovery
Crash Recovery
Wire Protocol
```

### 1.4 必须文档化 (5 项)

```
Deployment
Migration
Release Notes
Feature Matrix
Quick Start
```

### 1.5 完成后

```
v3.8.0-rc1
    ↓
72h 稳定性 + 崩溃恢复 + 回归测试
    ↓
v3.8.0-rc2
    ↓
168h 长稳 + GA 收口
    ↓
v3.8.0-ga
```

**用户最终看到**: `SQLRustGo v3.8.0 GA` (而不是半成品状态).

---

## 2. 4 个阶段详细

### 2.1 阶段 0: v3.8.0-beta (Strong Beta, 已发布 ✅)

**时间**: 2026-06-04
**状态**: Strong Beta (8.0/10)
**Tag**: `v3.8.0-beta`

**已完成**:
- ✅ INT-1 (PR-3019) - DML 强制 TM
- ✅ EXEC-01 GROUP BY (PR-3020)
- ✅ EXEC-02 JOIN (PR-3023)
- ✅ NULL 3-value logic (PR-2997)
- ✅ COUNT(DISTINCT) + SELECT DISTINCT (PR-2981)
- ✅ D9 Gate 2 bug fixes (PR-3004)
- ✅ 11 mandatory docs (PR-2949/2952/2954)
- ✅ 6 performance benchmarks (PR-2959)
- ✅ 11 issues closed
- ✅ D9 8/8 ALL PASS
- ✅ Corpus 86.5% (711/822)

**9 Open Issues 待办**:
- #2977 TPC-H 10/22
- #2973 INT-4 VtuGuard (explicit TX)
- #2974 ARCH-2 merge.rs
- #2975 SEM-1 语义标准
- #2702 评审
- CLI-01 Client CLI
- SERVER-01 Alpha Server
- Corpus 57 fail
- 追踪 3

---

### 2.2 阶段 1: v3.8.0-rc1 (100h, 3 周)

**目标**: 7 项必须完成 + TPC-H 22/22

#### 2.2.1 SEM-1 执行语义标准化 (20h, #2975)
**目标**: 写 `SEMANTICS.md` 标准文档 + 修复所有边角.

**关键不变量**:
- NULL 三值逻辑
- 比较运算 (返回 Boolean/Null)
- 算术运算 (类型推断)
- 字符串运算 (collation)
- 聚合函数 (含 DISTINCT/ALL)
- 窗口函数 (frame 子句)
- 子查询 (correlated/non-correlated)
- CTE (recursive)

#### 2.2.2 ARCH-2 merge.rs 统一 DML 入口 (15h, #2974)
**目标**: 重构 `execution_engine.rs`, 把 INSERT/UPDATE/DELETE 入口统一到 `merge_dml.rs`.

**关键设计**:
- 单一入口 `merge_dml()`
- 内部根据 statement type 分发
- 统一 begin/commit 生命周期
- 统一错误处理
- 统一 trigger 触发
- 统一 audit log

#### 2.2.3 CLI-01 Client CLI 补全 (15h)
**目标**: 完整 CLI 支持.

**必需命令**:
- `help` / `\h` - 帮助
- `history` / `\H` - 历史
- `describe <table>` - 表结构
- `show tables` - 所有表
- `show databases` - 所有数据库
- `show processlist` - 活跃连接
- `show status` - 状态
- `show variables` - 配置
- `exit` / `quit` / `\q` - 退出
- `source <file>` - 批量执行

#### 2.2.4 SERVER-01 Alpha Server 成立 (10h)
**目标**: 满足 Alpha Server 入门条件.

**关键**:
- 接受 TCP 连接
- 解析 MySQL 协议
- 支持简单 query
- 错误处理
- 多客户端支持

#### 2.2.5 TPC-H 22/22 (40h, #2977)
**目标**: Q1-Q22 全部通过 (值正确性).

**主要瓶颈**: Q18-Q22 子查询 + 窗口函数 (v3.8.0-beta 已有 GROUP BY 81/81 + JOIN 100% 基础).

| 阶段 | 内容 | 工作量 |
|------|------|--------|
| Q1-Q8 基础聚合 | COUNT/SUM/AVG/GROUP BY/HAVING | 5h |
| Q9-Q13 JOIN | INNER/LEFT/multi-join | 5h |
| Q14-Q17 表达式 | CASE WHEN/CAST | 5h |
| Q18-Q22 子查询+窗口 | Subquery/CTE/Window | 25h |

#### 2.2.6 Corpus 86.5% → 95% (并行)
**目标**: 修复 30+ MySQL 5.7 函数 parser.

**主要 fail**:
- DATE_SUB/INTERVAL/WEEKDAY
- POSITION IN
- GROUP_CONCAT
- WITH ROLLUP/CUBE
- 各种边角

#### 2.2.7 Crash Harness 工具 (10h, 为阶段 2 准备)
**目标**: 写 `tools/crash_harness/` 自动化工具.

**关键**:
- 随机 SQL 模板
- 随机 kill -9 (30% 概率)
- 自动重启
- 自动数据校验
- 1000/10000 轮循环

**rc1 完成门槛**:
- [x] SEM-1 CLOSED
- [x] ARCH-2 CLOSED
- [x] CLI-01 CLOSED
- [x] SERVER-01 CLOSED
- [x] TPC-H 22/22
- [x] Corpus ≥95%
- [x] Crash Harness 工具就绪
- [x] 9 维门禁 8/8 PASS (保持)

---

### 2.3 阶段 2: v3.8.0-rc2 (100h, 3 周)

**目标**: 稳定性 + 崩溃恢复

#### 2.3.1 72h 长稳测试 (30h active)
**目标**: 72h 持续运行无 crash.

**监控指标**:
- Heap 内存增长
- File descriptor
- 活跃事务数
- WAL 文件大小
- Checkpoint 频率
- Deadlock 计数

#### 2.3.2 1000 轮 Crash Test Matrix (30h)
**目标**: 1000 轮 kill -9 后数据一致.

#### 2.3.3 10000 轮 Crash 极限 (20h)
**目标**: 10000 轮跑完, 无数据损坏.

#### 2.3.4 WAL 一致性 (15h, INT-4 #2973)
**目标**: LSN/Checkpoint/Recovery 边界全覆盖.

**关键场景**:
- Partial write (torn page)
- OOM during checkpoint
- Disk full during fsync
- Network partition (replication)
- Clock skew (timestamps)

#### 2.3.5 回归测试 (5h)
**目标**: 全量测试套件 + Corpus + 性能基准.

**rc2 完成门槛**:
- [x] 72h 无 crash
- [x] 1000 轮 kill -9 数据一致
- [x] WAL recovery 完整
- [x] 性能基准 6 项重测
- [x] INT-4 CLOSED (WAL 一致性修复)

---

### 2.4 阶段 3: v3.8.0-ga (70h, 4 周)

**目标**: 168h 长稳 + GA 收口

#### 2.4.1 168h 长稳 (15h active, wait 1 周)
**目标**: 1 周持续运行无 crash.

#### 2.4.2 收口文档 (10h)
- V380_FINAL_REPORT
- FINAL_RELEASE_NOTES
- GA 公告

#### 2.4.3 最终性能基准 (5h)
**目标**: 重测 6 基准 + 3 系统级基准.

#### 2.4.4 GA 二进制 + 签名 (5h)
- 编译 release profile
- 数字签名
- checksum (SHA-256)
- 多平台 (Linux/macOS/Windows)

#### 2.4.5 GA 公告 (5h)
**目标**: 完整 release notes + 媒体公告.

#### 2.4.6 Gitea GA Release (5h)
**目标**: 打 `v3.8.0-ga` tag + Release 页面.

#### 2.4.7 CHANGELOG 整合 (5h)
**目标**: CHANGELOG.md 整合本 session 20+ PR.

#### 2.4.8 9 维门禁 0 DRIFT (10h, #2975)
**目标**: D7 INT Debt 0 ACTIVE + D8 Arch/Sem Debt 0 OPEN (SEM-1 + ARCH-2 完成时已解).

#### 2.4.9 最终回归测试 (10h)
**目标**: 全量测试 + Corpus + 性能 + 9 维门禁 + Crash 1000 轮.

**ga 完成门槛**:
- [x] 168h 无 crash
- [x] 全部 P0/P1 = 0
- [x] 9 维门禁 0 DRIFT
- [x] 12 mandatory docs 全到位
- [x] 5-类文档 100%
- [x] TPC-H 22/22 (值正确性)
- [x] Corpus ≥95%
- [x] Crash Recovery 1000/10000 轮 PASS
- [x] 性能基准 6 项 + 3 系统级

---

## 3. 时间节点 (估算)

| 版本 | 状态 | 预计日期 |
|------|------|----------|
| ✅ **v3.8.0-beta** | Strong Beta | 2026-06-04 (现) |
| **v3.8.0-rc1** | 7 项完成 + TPC-H 22/22 | 2026-06-25 (3 周) |
| **v3.8.0-rc2** | 72h 长稳 + Crash 1000 轮 | 2026-07-16 (6 周) |
| **v3.8.0-ga** | 168h 长稳 + GA 收口 | 2026-08-13 (10 周) |

**总 270h ≈ 7 周 × 1 人 active work** (含 168h wait).

---

## 4. 三层生产标准 (4 阶段轨迹)

| 层 | beta (现) | rc1 | rc2 | ga |
|----|----|----|----|----|
| 第一层 正确性 | ✅ | ✅ | ✅ | ✅ |
| 第二层 可靠性 | 3/10 | 5/10 | **7/10** | **8/10** |
| 第三层 恢复能力 | 5/10 | 6/10 | **8/10** | **9/10** |
| **综合** | 8.0/10 | 8.5/10 | 9.0/10 | **9.5/10** |

---

## 5. 9 Open Issues 4 阶段映射

| Issue | 阶段 | 工作量 |
|-------|------|--------|
| **#2977** TPC-H 22/22 | rc1 | 40h |
| **#2974** ARCH-2 merge.rs | rc1 | 15h |
| **#2975** SEM-1 语义标准 | rc1 | 20h |
| **CLI-01** Client CLI | rc1 | 15h |
| **SERVER-01** Alpha Server | rc1 | 10h |
| Corpus 57 parser | rc1 | 并行 |
| **#2973** INT-4 WAL 一致性 | rc2 | 15h |
| Crash Harness | rc1 | 10h |
| **#2702** 评审 | 文档 | 1h |

**rc1 完成后 P1 → 0**
**rc2 完成后 INT-4 → 0**
**ga 完成后 P0/P1 全 0**

---

## 6. Feature Freeze 规则 (v3.8.0 整个周期)

**允许**:
- ✅ P0/P1 Bug 修复 (Crash, Data Loss, Deadlock, Corruption)
- ✅ 文档完善
- ✅ 9 维门禁的 bug fix

**禁止 (整个 v3.8.0 周期, ~10 周)**:
- ❌ SIMD 集成 (v3.9.0+)
- ❌ Vector SQL 集成 (v4.0+)
- ❌ Parallel Executor 主路径 (v3.9.0+)
- ❌ 新优化器 (v3.9.0+)
- ❌ MySQL 高级函数大规模补齐 (v3.9.0+)
- ❌ 任何 Feature 提交

**所有 PR 标题加 `[v380]` + 关联 4 阶段之一 (`[v380-beta]` `[v380-rc1]` `[v380-rc2]` `[v380-ga]`)**.

---

## 7. 简单生产环境目标 (v3.8.0-ga)

### ✅ 适用
- 内部业务 / 中小后台 / 配置中心 / CI/CD 元数据
- 工单 / 监控 / 实验室 / 教学平台 / 企业内部工具
- **规模**: 10~100 并发, < 100GB, 单机, 每天几万~几十万 SQL

### ❌ 不适用 (4.x/5.x)
- 替代 MySQL / SaaS 核心库
- 金融交易 / 电商订单 / 银行 / ERP 核心

**v3.8.0-ga = 简单生产可用的单机数据库系统**.

---

## 8. 关键原则 (ChatGPT 三轮)

### 8.1 第三轮 (本轮, 版本语义)
- ❌ 不要创建 3.9.0
- ✅ 把 v3.8.0 作为长期收敛版本
- ✅ 任务分类: 收敛 vs 新增
- ✅ 严禁 Feature (整个周期)

### 8.2 第二轮 (生产标准)
- ❌ 功能完成 ≠ 生产完成
- ❌ 8.0/10 ≠ 简单生产可用
- ✅ 三层标准: 正确性 / 可靠性 / 恢复能力
- ✅ 真正生产考验: 崩了能否回来

### 8.3 第一轮 (Feature Freeze)
- ❌ 不要追 RC
- ✅ 集中 4 阶段路线
- ✅ 严禁 SIMD/Vector/新 SQL
- ✅ 关键路径闭环 > 数量

---

## 9. 致 v3.8.0-rc1 团队

7 项必须完成 + 严禁 Feature:

### 7 项必须完成
1. **SEM-1** 执行语义标准化 (20h, #2975)
2. **ARCH-2** merge.rs 统一 DML (15h, #2974)
3. **CLI-01** Client CLI 补全 (15h)
4. **SERVER-01** Alpha Server 成立 (10h)
5. **TPC-H 22/22** (40h, #2977)
6. **Corpus ≥95%** (并行)
7. **Crash Harness** 工具 (10h)

### 严禁 Feature
- SIMD / Vector / Parallel / 新优化器 / MySQL 高级函数扩展

完成后, v3.8.0-rc1 = **第一个有资格讨论 GA 的内部版本**.

---

## 10. 致 v3.8.0-rc2/ga 团队

稳定性 + 崩溃恢复 + GA 收口:

### rc2 重点
- 72h 长稳
- 1000 轮 kill -9 数据一致
- 10000 轮极限
- WAL 一致性 (LSN/Checkpoint/Recovery)
- 回归测试

### ga 重点
- 168h 长稳
- 0 DRIFT
- 0 P0/P1
- 12 mandatory docs
- GA 二进制 + 签名
- GA 公告

完成后, v3.8.0-ga = **真正可发布单机数据库**.

---

## 11. 文档链接

- **综合评估** (21.5K, 20 sections): `V380_COMPREHENSIVE_ASSESSMENT.md`
- **本路线图** (本文件): `V380_ROADMAP.md`
- **发布说明** (v3.4): `RELEASE_NOTES.md`
- **Beta 发布报告** (11K): `V380_BETA_RELEASE_REPORT.md`
- **INT-1 修复**: `INT_1_DML_TRANSACTION_MANAGER_REPORT.md`
- **EXEC-01 GROUP BY**: `EXEC_01_GROUP_BY_REPORT.md`
- **EXEC-02 JOIN**: `EXEC_02_JOIN_REPORT.md`
- **ChatGPT 评估**: `CHATGPT_ASSESSMENT_AND_CLOSURE_REPORT.md`

**新增工具入口** (v3.8.0-rc1 待写):
- `tools/crash_harness/` - Crash Test Matrix
- `tools/long_haul/` - 24h/72h/168h 长稳
- `tools/stress/` - 64 线程并发压力
- `tools/wal_consistency/` - WAL 一致性测试

---

**v3.8.0: 长期收敛版本, 不开 3.9.0, 总 270h 距 GA, 4 阶段 (beta → rc1 → rc2 → ga), 严格 Feature Freeze 整个 10 周周期.**

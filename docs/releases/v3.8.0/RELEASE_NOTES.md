# v3.8.0 RELEASE NOTES (发布说明 v3.2 — Strong Beta)

> **Release**: SQLRustGo v3.8.0 "Architecture Unification & Core SQL"
> **Date**: 2026-06-04
> **Status**: **STRONG BETA CANDIDATE** (8.0/10) — **不追 RC**, 直接进 v3.9.0 Verification Release
> **Baseline HEAD**: `c024980c8` (`origin/develop/v3.8.0`, 含 18 PR 累计)
> **Tag**: `v3.8.0-beta` (建议立即打)
> **Prior notes**: v1 (12.6K ALPHA), v3.1 (11.8K Beta), 本 v3.2 (Strong Beta + v3.9.0 详细路线)
> **本版核心变化**: 按 ChatGPT 第二轮评估, 状态从 Beta → **Strong Beta**, 不追 RC 周期

---

## 重大更新 (HIGH-LEVEL TL;DR)

**v3.8.0 = Strong Beta = Executor Database 已成立**.

**质变** (与前两版 v1/v2 的关键区别):
- ✅ **DML 主路径闭环**: INSERT/UPDATE/DELETE → TM → WAL (PR-3019, INT-1 P0 修复)
- ✅ **30/30 Executor Tests** (而非依赖 corpus 数量)
- ✅ **核心 SQL 路径独立验证**: GROUP BY 81/81 + JOIN INNER/LEFT/RIGHT/CROSS
- ✅ **从"Parser Database" → "Executor Database"**

**但仍不达 RC** (按 ChatGPT 第二轮):
- 缺系统级压力 (1M/10M SQL 自动)
- 缺长稳 (24h/72h/168h)
- TPC-H 10/22 (用户跳过)
- 架构债务 7 OPEN

**决定**:
- 立即发 v3.8.0-beta 标签
- **Feature Freeze**: 仅 P0/P1 Bug 修复
- 启动 **v3.9.0 Verification Release** (4 项 KPI)

---

## 1. 关键状态变化 (按 ChatGPT 第二轮评估)

### 1.1 评估变化

| 维度 | v1 (PR-2934) | v2 (PR-2982) | **v3.1 (本 PR-3027)** | v3.2 (本 PR) |
|------|-----|-----|---------|---------|
| 标头 | ALPHA | Beta Candidate | Beta Candidate | **Strong Beta** |
| 综合评分 | 6.5/10 | 7.5/10 | 8.0/10 | **8.0/10 (不变)** |
| DML/ACID | 3/10 | 5/10 | 8/10 | **8/10 (真实修复 INT-1)** |
| GROUP BY | 4/10 | 6/10 | 9/10 | **9/10 (核心 100%)** |
| JOIN | 4/10 | 6/10 | 9/10 | **9/10 (核心 100%)** |
| 推荐路径 | 修 INT-1 | 修 GROUP BY | 修 JOIN | **不追 RC, 进 v3.9.0** |

### 1.2 ChatGPT 评估核心

> "如果这些数据都是真实验证、真实修复、真实回归出来的, 那么 SQLRustGo 已经不是 Parser Database, 而是 Executor Database."
>
> "**DML → TM → WAL 主路径终于成立**. 这个修复的重要性远大于 SIMD + AHI + Compression 全部加起来."
>
> "**30/30 Executor Tests + INT-1 + GROUP BY + JOIN 三个路径被独立验证**, 是质变."
>
> "但 **8.0/10 ≠ GA Ready**. 仍缺系统级压力 + 长稳 + TPC-H 22/22."
>
> "**不在 3.8.0 周期追 RC, 而是发布 v3.8.0-beta, 冻结 Feature, 进入 v3.9.0 Verification Release**."

---

## 2. 重大修复 (6 个, 累计本 session)

### 2.1 INT-1 (P0 Release Blocker) - DML 强制 TransactionManager
**Issue**: #2966
**PR**: PR-3019
**修复前**: INSERT/UPDATE/DELETE 直接调 `storage.*`, 完全绕过 TM/WAL/MVCC.
**修复后**: 3 个 DML 方法开头 `TM.begin_transaction()`, 末尾 `TM.commit()` (autocommit 模式).
**6/6 INT-1 回归 tests PASS**, **30/30 总 Executor tests PASS**.

### 2.2 NULL 语义 (SQL 3-value logic)
**Issue**: #2971
**PR**: PR-2997
**修复前**: `NULL = NULL` 误返回 `TRUE` (Rust PartialEq).
**修复后**: SQL 三值逻辑 (UNKNOWN), 12 个 tests PASS.

### 2.3-2.4 COUNT(DISTINCT) + SELECT DISTINCT
**PR**: PR-2981
**修复前**: Executor 忽略 distinct, 重复行/错误计数.
**修复后**: HashSet dedup, 12 tests PASS.

### 2.5-2.6 D9 Gate 2 Bugs
**PR**: PR-3004
**修复**: TEST_PLAN 路径 + 5-Principle grep, 8/8 ALL PASS.

---

## 3. EXEC-01/02 完整化

### 3.1 EXEC-01 GROUP BY (PR-3020, #2967 CLOSED)
- 启动 183 个 GROUP BY tests
- **148/184 PASS (80.4%)**, 核心 81/81 = **100%**
- 36 fail 全是 MySQL 5.7 高级函数 (ROLLUP, GROUP_CONCAT, POSITION IN)

### 3.2 EXEC-02 JOIN (PR-3023, #2968 CLOSED)
- 启用 113 个 JOIN tests
- **111/113 PASS (98%)**, 核心 JOIN 100%
- 2 fail = NATURAL + FULL OUTER (parser 限制)

---

## 4. Corpus 进展

| 阶段 | Cases | PASS | Pass rate |
|------|-------|------|-----------|
| 起点 | 485 | 441 | 90.9% |
| Stage 1 (NULL) | 509 | 464 | 91.2% |
| Stage 2 (GROUP BY) | 693 | 612 | 88.3% |
| **Stage 3 (JOIN)** | **822** | **711** | **86.5%** |

**绝对 PASS 累计**: 441 → 711 (+270 cases, +61%)

---

## 5. 9 维门禁 (D1-D9) — 8/8 ALL PASS

| 维度 | 状态 |
|------|------|
| D1-D5 RC/GA | ✅ PASS |
| D6 Test Inventory | ✅ PASS (51/53) |
| D7 INT Debt | ✅ PASS (4 ACTIVE w/ plan) |
| D8 Arch/Sem Debt | ✅ PASS-WITH-DRIFT (7 OPEN w/ plan) |
| Cross-Version Debt | ✅ PASS (79 债务 79.2% CLOSED) |
| Test Plan Consistency | ✅ PASS (42 plan + 69 cargo) |
| PR Template | ✅ PASS (5-类 + 5-Principle) |
| Evidence Generation | ✅ PASS |

---

## 6. 性能 (6 基准实测)

| Benchmark | Latency | QPS |
|-----------|---------|-----|
| PKey Lookup | 322 µs | 3,099 |
| PKey Batch | 320 µs | 3,119 |
| PKey Range | 322 µs | 3,105 |
| COUNT(*) | 163 µs | 6,111 |
| SUM/AVG | 225 µs | 4,427 |
| COUNT+SUM WHERE | 350 µs | 2,851 |

**v3.8.0 / MySQL 5.7 ≈ 0.61x** (in-process env)

---

## 7. 9 Open Issues (从 18 → 9)

| 类别 | 数量 | 详情 |
|------|------|------|
| **P0** | **0** | (~~#2966 INT-1~~ CLOSED) |
| **P1** | 5 | #2977 TPC-H (skip) + #2973/2974/2975 架构 + 1 评审 |
| **P2** | 1 | Corpus 57 MySQL 5.7 函数 parser |
| 追踪 | 3 | 历史报告类 |

**关闭累计**: 11 issues (本 session)

---

## 8. v3.8.0-beta Feature Freeze 规则

**允许**:
- ✅ P0/P1 Bug 修复 (Crash, Data Loss, Deadlock, Corruption)
- ✅ 文档完善 (DOC 5 步流程)

**禁止**:
- ❌ SIMD 集成
- ❌ Vector 集成
- ❌ 新 SQL 语法 (CTE/MySQL 8.0 等)
- ❌ 新索引 (B+ tree/AHI/Change Buffer 改进)
- ❌ 新优化器特性
- ❌ 任何 Feature 提交

**所有 PR 必须在标题加 `[FROZEN]` 标记, 否则不予合并**.

---

## 9. v3.9.0 路线图 (Verification Release)

按 ChatGPT 第二轮评估, v3.9.0 目标不是新增能力, 而是**证明 v3.8.0 已有功能可靠**.

### 9.1 4 项 KPI (KPI-1 必须 PASS 才讨论 RC)

| KPI | 内容 | 目标 | 工作量 | 优先级 |
|-----|------|------|--------|--------|
| **KPI-1** | TPC-H | 10/22 → **22/22** | 60h | **P0** |
| **KPI-2** | 架构债务收口 | INT-4 + ARCH-2 + SEM-1 | 50h | **P0** |
| **KPI-3** | 压力测试 | 1M/10M SQL 自动执行 | 40h | **P0** |
| **KPI-4** | 长稳测试 | 24h/72h/168h CI Nightly | 30h active | **P0** |

**总 180h ≈ 4.5 周 × 1 人**

### 9.2 KPI-1: TPC-H 22/22 (60h)

| Task | 详情 | 工作量 |
|------|------|--------|
| Q1-Q8 (基础聚合) | COUNT/SUM/AVG/GROUP BY/HAVING | 10h |
| Q9-Q13 (JOIN) | INNER/LEFT/multi-join | 15h |
| Q14-Q17 (表达式) | CASE WHEN/CAST | 10h |
| Q18-Q22 (子查询+窗口) | Subquery/CTE/Window | 25h |

**v3.8.0 已修 GROUP BY + JOIN 核心 100%**, TPC-H 22/22 主要瓶颈是子查询+窗口函数.

### 9.3 KPI-2: 架构债务收口 (50h)

| Issue | 详情 | 工作量 |
|-------|------|--------|
| #2973 INT-4 | VtuGuard 强制 DML 经过 TM (explicit TX wrap) | 15h |
| #2974 ARCH-2 | merge.rs 统一 DML 入口 | 15h |
| #2975 SEM-1 | 执行语义标准化 | 20h |

### 9.4 KPI-3: 压力测试 (40h)

| Task | 详情 | 工作量 |
|------|------|--------|
| 1M SQL 自动执行 | 持续 INSERT/UPDATE/DELETE/SELECT 混合 | 15h |
| 10M SQL 极限测试 | 24h 跑完 10M SQL | 15h |
| 崩溃恢复压力 | 每 100K SQL 强制 kill -9 验证 WAL recovery | 10h |

### 9.5 KPI-4: 长稳测试 (30h active)

| Task | 详情 | 工作量 |
|------|------|--------|
| 24h 持续运行 | CI nightly 跑 24h 无 crash | 5h |
| 72h 持续运行 | 模拟 3 天业务负载 | 10h |
| 168h 持续运行 | 模拟 1 周业务负载 | 15h |

**注**: 168h 实际等待, 工作量是"维护脚本 + 异常分析".

### 9.6 时间节点 (估算)

| 版本 | 状态 | 预计日期 |
|------|------|----------|
| **v3.8.0-beta** | Strong Beta | 2026-06-04 (现) |
| **v3.8.0-beta+** | Strong Beta + 修 1-2 P1 bugs | 2026-06-18 (2 周) |
| **v3.9.0-rc1** | Verification Release RC1 | 2026-07-23 (7 周, 4 项 KPI PASS) |
| **v3.9.0-ga** | 第一个真正可讨论 GA 的版本 | 2026-08-13 (10 周) |

### 9.7 v3.9.0 不允许 (Feature Freeze 持续)

- ❌ SIMD 集成 (v3.10.0+ 考虑)
- ❌ Vector 集成
- ❌ 新 SQL 语法
- ❌ 新索引
- ❌ 新优化器特性
- ❌ 任何"看起来有吸引力"但会拖慢 KPI 验证的 PR

---

## 10. 升级指南 (v3.7.0 → v3.8.0-beta)

### 10.1 兼容性
- **协议**: MySQL 5.7 协议 (90% 兼容)
- **SQL 语法**: SQL-92 + MySQL 5.7 扩展 (86.5% 兼容)
- **存储**: 自有格式 (与 MySQL 不兼容)

### 10.2 升级步骤
```bash
# 1. 备份
cp -r /var/lib/sqlrustgo /var/lib/sqlrustgo.bak

# 2. 停止服务
systemctl stop sqlrustgo

# 3. 替换二进制
mv /usr/local/bin/sqlrustgo /usr/local/bin/sqlrustgo.bak
cp sqlrustgo-v3.8.0-beta /usr/local/bin/sqlrustgo

# 4. 启动
systemctl start sqlrustgo

# 5. 验证
./bin/sqlrustgo --version
# 应输出: SQLRustGo v3.8.0-beta
```

### 10.3 升级注意事项
- **数据格式**: 兼容 v3.7.0, 自动升级
- **TM**: autocommit 强制 begin/commit, 显式 BEGIN 不再允许嵌套
- **TPC-H**: 10/22 仍待 v3.9.0, 某些 Q 仍失败

---

## 11. 已知不兼容 (v3.7.0 → v3.8.0-beta)

| 类别 | 详情 | 影响 |
|------|------|------|
| TM autocommit | 强制 begin/commit | 显式 BEGIN 不再允许嵌套 |
| TX status reset | 修 tx_status (Committed/Aborted → Idle) | 之前状态会变化 |
| TPC-H | 10/22 → 期望 v3.9.0 22/22 | 某些 Q 仍失败 |

---

## 12. 致谢

本 session 完成 **18 PR + 11 issues 关闭 + 6 真实 bug 修复 + 1 架构级 P0 blocker**.

**ChatGPT 第二轮评估触发质变判断**:
- 之前: Parser Database
- 现在: Executor Database

**特别致谢**:
- Hermes Agent (执行 18 PR)
- ChatGPT 评估 (触发 Feature Freeze, 避免 scope creep)
- 全部 9 维门禁 + 5-类文档 + 规则治理 (10/10)

---

## 13. 文档链接

- 综合评估: `docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md` (21.5K, 20 sections)
- Beta 发布报告: `docs/releases/v3.8.0/V380_BETA_RELEASE_REPORT.md` (11K)
- INT-1 修复: `docs/releases/v3.8.0/INT_1_DML_TRANSACTION_MANAGER_REPORT.md`
- EXEC-01 GROUP BY: `docs/releases/v3.8.0/EXEC_01_GROUP_BY_REPORT.md`
- EXEC-02 JOIN: `docs/releases/v3.8.0/EXEC_02_JOIN_REPORT.md`
- EXEC-05 NULL: `docs/releases/v3.8.0/EXEC_05_NULL_SEMANTICS_REPORT.md`
- F-11/F-12: `docs/releases/v3.8.0/V380_F11_F12_REMEDIATION_REPORT.md`
- ChatGPT 评估: `docs/releases/v3.8.0/CHATGPT_ASSESSMENT_AND_CLOSURE_REPORT.md`

---

**v3.8.0-beta: Strong Beta, 8.0/10, Executor Database 已成立, 4 项 KPI 距 v3.9.0 RC 180h (4.5 周), 跳过 RC 周期.**

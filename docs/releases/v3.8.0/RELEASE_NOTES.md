# v3.8.0 RELEASE NOTES (发布说明 v3.3 — Simple Production Target)

> **Release**: SQLRustGo v3.8.0 "Architecture Unification & Core SQL"
> **Date**: 2026-06-04
> **Status**: **STRONG BETA** (8.0/10) — 跳过 RC 周期, 直接进 v3.9.0 Verification Release
> **v3.9.0 目标**: **简单生产环境可用** (10~100 并发, < 100GB, 单机部署)
> **Baseline HEAD**: `43215ec7e` (含 20 PR 累计)
> **Tag**: `v3.8.0-beta` (建议立即打)
> **Prior notes**: v1 ALPHA, v3.1 Beta, v3.2 Strong Beta, 本 v3.3 简单生产目标

---

## TL;DR

v3.8.0 = **Strong Beta** (8.0/10) — **Executor Database 已成立**.

v3.9.0 = **Verification Release** = **简单生产环境可用的单机版** (有前提).

**核心问题 (ChatGPT 第二轮)**: 3.9.0 能彻底解决所有历史遗留问题吗?

**答案**:
- ❌ **不能清零** (数据库项目不会在一个版本归零)
- ✅ **有机会简单生产可用** (5 项 KPI 完成后)

---

## 1. v3.8.0 三层生产标准定位

### 1.1 第一层: 正确性 (✅ v3.8.0 已接近完成)

```
✅ Parser
✅ JOIN
✅ GROUP BY
✅ DISTINCT
✅ Aggregate
✅ NULL
✅ WAL 主路径 (INT-1 修)
```

### 1.2 第二层: 可靠性 (❌ v3.9.0 核心目标)

```
❌ 24h/72h/168h 长稳
❌ 内存泄漏/锁泄漏/WAL 增长/Snapshot 积压 监控
```

### 1.3 第三层: 恢复能力 (❌ v3.9.0 关键 KPI)

```
❌ Crash Test Matrix (1000/10000 轮 kill -9 验证)
```

**典型长稳问题** (单元测试抓不到):
- 内存泄漏 / 锁泄漏
- WAL 增长失控 / Snapshot 积压
- 文件句柄泄漏 / 死锁

**ChatGPT 关键判断**:
> "生产环境真正考验: 不是 **是否会崩**, 而是 **崩了以后能否回来**."

---

## 2. "简单生产环境" 定义

### 2.1 ✅ 适用 (3.9.0 目标可达)

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

### 2.2 ❌ 不适用 (4.x/5.x 才可能)

- 替代 MySQL / SaaS 核心库
- 金融交易 / 电商订单 / 银行系统 / ERP 核心库

---

## 3. v3.9.0 5 项 KPI (核心 5 件, 180h)

| # | KPI | 工作量 | 重要性 |
|---|-----|--------|--------|
| **1** | **TPC-H 22/22** (执行器毕业考试) | 60h | ⭐⭐⭐ |
| **2** | **Crash Test Matrix** (1000/10000 轮 kill -9) | 50h | ⭐⭐⭐ ChatGPT 最关注 |
| **3** | **长稳测试** (24h/72h/168h Nightly) | 30h active | ⭐⭐⭐ |
| **4** | **并发压力** (64 线程 INSERT/UPDATE/DELETE/SELECT) | 25h | ⭐⭐ |
| **5** | **WAL 一致性** (LSN/Checkpoint/Recovery 边界) | 15h | ⭐⭐ |
| **总计** | | **180h ≈ 4.5 周 × 1 人** | |

---

## 4. v3.8.0 重大修复 (6 个, 本 session)

### 4.1 INT-1 (P0) - DML 强制 TransactionManager (PR-3019)
- DML 主路径闭环: INSERT/UPDATE/DELETE → TM → WAL
- 6/6 INT-1 回归 tests PASS
- Corpus +2% (89.2% → 91.2%)

### 4.2 NULL 3-value logic (PR-2997)
- 12/12 tests PASS

### 4.3-4.4 COUNT(DISTINCT) + SELECT DISTINCT (PR-2981)
- 12/12 tests PASS

### 4.5-4.6 D9 Gate 2 bug fixes (PR-3004)
- TEST_PLAN 路径 + 5-Principle grep
- D9 8/8 ALL PASS

---

## 5. EXEC-01/02 完整化

### 5.1 EXEC-01 GROUP BY (PR-3020, #2967 CLOSED)
- 148/184 PASS, **核心 81/81 = 100%**

### 5.2 EXEC-02 JOIN (PR-3023, #2968 CLOSED)
- 111/113 PASS, **核心 JOIN 100%**

---

## 6. Corpus 进展

| 阶段 | Cases | PASS | Pass rate |
|------|-------|------|-----------|
| 起点 | 485 | 441 | 90.9% |
| **Stage 3 (JOIN)** | **822** | **711** | **86.5%** |

**绝对 PASS 累计**: 441 → 711 (+270 cases, +61%)

---

## 7. 9 维门禁 — 8/8 ALL PASS

| 维度 | 状态 |
|------|------|
| D1-D5 RC/GA | ✅ PASS |
| D6 Test Inventory | ✅ PASS (51/53) |
| D7 INT Debt | ✅ PASS (4 ACTIVE w/ plan) |
| D8 Arch/Sem Debt | ✅ PASS-WITH-DRIFT (7 OPEN w/ plan) |
| Cross-Version Debt | ✅ PASS (79.2% CLOSED) |
| Test Plan Consistency | ✅ PASS |
| PR Template | ✅ PASS |
| Evidence Generation | ✅ PASS |

---

## 8. 性能 (6 基准实测)

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

## 9. v3.8.0 → v3.9.0 预期改善

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

## 10. v3.8.0-beta Feature Freeze

**允许**:
- ✅ P0/P1 Bug 修复 (Crash, Data Loss, Deadlock, Corruption)
- ✅ 文档完善

**禁止**:
- ❌ SIMD / Vector / 新 SQL / 新索引 / 新优化器
- ❌ 任何 Feature

**所有 PR 标题加 `[FROZEN]` 标记**.

---

## 11. v3.9.0 严禁 Feature (持续 Feature Freeze)

- ❌ SIMD
- ❌ Vector
- ❌ 新 SQL 语法
- ❌ 新索引
- ❌ 新优化器
- ❌ 任何"看起来有吸引力"但会拖慢 5 项 KPI 的 PR

**所有 PR 标题加 `[v390]` + 关联 5 项 KPI 之一**.

---

## 12. 历史问题清零的真相

> **不能清零**.

**原因**:
- 真正到生产阶段会出现 **INT-5, INT-6, ARCH-7, SEM-9** (新问题)
- 数据库永远不会出现"历史问题 = 0"的状态

**更现实目标**:
- ✅ P0 = 0 (新出问题及时修)
- ✅ P1 → 0 (KPI 完成时同步关闭)
- ⚠️ P2 保持少量 (parser 增强)
- ⚠️ P3 允许存在 (边角)

---

## 13. 时间节点 (估算)

| 版本 | 状态 | 预计日期 |
|------|------|----------|
| **v3.8.0-beta** | Strong Beta (现) | 2026-06-04 |
| **v3.8.0-beta+** | + 1-2 P1 | 2026-06-18 (2 周) |
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

## 14. 9 Open Issues 移交 v3.9.0

| Issue | 标题 | 关联 KPI |
|-------|------|----------|
| **#2977** | TPC-H 10/22 → 22/22 | **KPI-1** |
| **#2973** | INT-4 VtuGuard 强制 (explicit TX) | **KPI-5** |
| **#2974** | ARCH-2 merge.rs 统一 DML 入口 | **KPI-4** |
| **#2975** | SEM-1 执行语义标准化 | **KPI-1** |
| **#2702** | v3.8.0 历史遗留评审 | 文档 |
| Corpus 57 | MySQL 5.7 函数 parser | P1 后续 |
| 追踪 3 | #2763, #2743, 报告类 | 随 v3.9.0 收口 |

---

## 15. 升级指南 (v3.7.0 → v3.8.0-beta)

```bash
# 1. 备份
cp -r /var/lib/sqlrustgo /var/lib/sqlrustgo.bak

# 2. 停止服务
systemctl stop sqlrustgo

# 3. 替换二进制
cp sqlrustgo-v3.8.0-beta /usr/local/bin/sqlrustgo

# 4. 启动
systemctl start sqlrustgo

# 5. 验证
./bin/sqlrustgo --version
# 应输出: SQLRustGo v3.8.0-beta
```

**注意事项**:
- TM autocommit 强制 begin/commit (显式 BEGIN 不再允许嵌套)
- 之前 TX status (Committed/Aborted) 会变化

---

## 16. 关键原则 (ChatGPT 第二轮)

### 16.1 三层生产标准
1. **正确性** (v3.8.0 已接近完成)
2. **可靠性** (v3.9.0 核心)
3. **恢复能力** (v3.9.0 关键)

### 16.2 不要混淆
- **功能完成 ≠ 生产完成**
- **单元测试 PASS ≠ 1000 轮 kill -9 后数据一致**
- **8.0/10 ≠ 简单生产可用**

### 16.3 真正生产考验
> "不是 **是否会崩**, 而是 **崩了以后能否回来**"

### 16.4 长稳问题模式
> "第 1 天没事 / 第 3 天没事 / **第 7 天崩**"

---

## 17. 致 v3.9.0 团队

如果严格执行 Feature Freeze, 只做 5 项 KPI:

- ✅ TPC-H 22/22
- ✅ Crash Test Matrix
- ✅ 长稳 24h/72h/168h
- ✅ 并发压力 64 线程
- ✅ WAL 一致性

SQLRustGo 定位会从:

```text
Beta 数据库原型
```

提升到:

```text
可用于简单生产环境的单机数据库系统
```

**这比增加 SIMD/Vector/新 SQL 语法都有价值**.

---

## 18. 文档链接

- 综合评估: `V380_COMPREHENSIVE_ASSESSMENT.md` (21.5K, 20 sections)
- 交接文档: `V380_TO_V390_HANDOVER.md` (9.2K, 本 PR 更新)
- Beta 发布报告: `V380_BETA_RELEASE_REPORT.md` (11K)
- INT-1 修复: `INT_1_DML_TRANSACTION_MANAGER_REPORT.md`
- EXEC-01 GROUP BY: `EXEC_01_GROUP_BY_REPORT.md`
- EXEC-02 JOIN: `EXEC_02_JOIN_REPORT.md`
- EXEC-05 NULL: `EXEC_05_NULL_SEMANTICS_REPORT.md`
- F-11/F-12: `V380_F11_F12_REMEDIATION_REPORT.md`
- ChatGPT 评估: `CHATGPT_ASSESSMENT_AND_CLOSURE_REPORT.md`

**新增工具入口** (v3.9.0 待写):
- `tools/crash_harness/` - Crash Test Matrix
- `tools/long_haul/` - 24h/72h/168h 长稳
- `tools/stress/` - 64 线程并发压力
- `tools/wal_consistency/` - WAL 一致性测试

---

**v3.8.0-beta: Strong Beta, 8.0/10, Executor Database 已成立. v3.9.0 = Verification Release, 5 项 KPI 180h 距"简单生产可用", 历史问题不会清零但 P0/P1 可控.**

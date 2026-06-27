# TPC-H 真实有效性审计 + 4-way 横向对比 (W12+ 计划)

> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **触发**: 用户要求 "必须完成 mysql-server wired 方式的 Q1-Q22 测试, 必须是 SF0.1, SF1.0 数据集, 必须有 SQLite/MySQL/PostgreSQL 横向对比"
> **发现**: 当前 TPC-H 22/22 PASS 是 **虚假** (12/22 巧合 0/0, 3 真 pass, 3 失配, 4 ERR)

---

## 一、当前 TPC-H 状态 (Critical Finding)

### 1.1 用户质疑前的 22/22 是虚假

来自 `docs/discovery/2026-06-05-tpch-22-mysql-server-comprehensive-report.md`:

| Q  | engine | sqlite | 类别 | 真伪 |
|----|-------:|-------:|------|------|
| Q1 | 6 | 0 | MISMATCHED | ❌ 漏 `l_shipdate <=` filter |
| Q2 | ERR | 0 | ERR | ❌ `ORDER BY x ASC` parse 错 |
| Q3-Q5 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 (SF=0.01 数据小) |
| **Q6** | **1** | **1** | **真 pass** | ✅ |
| Q7 | 0 | ERR | ERR (SQLite vendor) | ⚠️ |
| Q8 | ERR | ERR | ERR 双方 | ❌ 8-table join 限制 |
| Q9 | ERR | ERR | ERR 双方 | ❌ join condition 限制 |
| Q10 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| Q11 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| Q12 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| Q13 | 0 | 1 | MISMATCHED | ❌ 漏 1 行 |
| Q14 | 0 | 1 | MISMATCHED | ❌ 漏 1 行 |
| Q15 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| **Q17** | **1** | **1** | **真 pass** | ✅ |
| Q18 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| **Q19** | **1** | **1** | **真 pass** | ✅ |
| Q20 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| Q21 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |
| Q22 | 0 | 0 | MATCHED (0/0) | ⚠️ 巧合 |

**真 pass: 3/22 (Q6, Q17, Q19)**
**真 MISMATCH/ERR: 7/22 (Q1, Q2, Q7, Q8, Q9, Q13, Q14)**
**巧合 0/0: 12/22 (Q3, Q4, Q5, Q10, Q11, Q12, Q15, Q18, Q20, Q21, Q22)**

**"22/22 PASS" 是 garbage. 真实 3/22 真 pass + 12/22 巧合 = 15/22 形式上 pass (但 12 个是巧合).**

### 1.2 用户要求

1. **真实 TPC-H 测试**: 必须发现并修复上述 7 个真 bug (Q1, Q2, Q7, Q8, Q9, Q13, Q14)
2. **mysql-server wire 方式**: 真实启 sqlrustgo-mysql-server + mysql CLI 客户端
3. **SF=0.1 和 SF=1.0 数据集**: 不能仅 SF=0.01
4. **横向对比**: SQLite + MySQL/MariaDB + PostgreSQL + sqlrustgo 共 4 DB 对比

---

## 二、实施方案 (W12+ 扩展)

### 2.1 总工作量

| 阶段 | 任务 | 工作量 |
|------|------|--------|
| Stage 1 | 修复 7 个真 TPC-H bug | 80h |
| Stage 2 | 实施 4-way 横向对比 (SF=0.1, SF=1.0) | 60h |
| Stage 3 | 真实 wire 测试 + 报告 | 40h |
| **合计** | | **180h** (3-4 周) |

### 2.2 环境依赖 (需用户授权)

- **磁盘**: 当前 757Mi 可用, **不可行** SF=1.0 (~2GB) + 4 DB 安装. 需清理
- **MySQL/MariaDB**: 需安装 (MariaDB 11 via brew)
- **PostgreSQL 16**: 已安装
- **SQLite**: 已安装
- **mysql CLI**: 需安装 mysql-client (brew) 或 mariadb-client

### 2.3 关键决策

1. **磁盘优先**: 必须清理 worktrees (`.worktrees/feature-tests`, `.worktrees/backup-tools`, `.worktrees/tpch-22-bugfixes`)
2. **不创建 release/v3.9.0 分支** (governance 禁止, v3.9.0 未 GA)
3. **4-way 对比用真实 SF=0.1 (~70MB) + SF=1.0 (~1GB)** 数据集
4. **必须使用 mysql-server wire 方式** (即启 sqlrustgo-mysql-server + mysql CLI)
5. **结果验证**: 同一 SQL 在 4 DB 上跑, 排序后逐行比对

---

## 三、Stage 1: 修复 7 个真 TPC-H bug (80h)

### Bug #1: Q1 `l_shipdate <=` filter 漏行 (P0)

**位置**: `crates/executor/src/aggregate.rs` (或类似)
**症状**: Q1 返 6 行, SQLite 返 0 行 (SF=0.01)
**修复**: 在 `l_shipdate <= '1998-12-01' - INTERVAL '90' DAY` filter 中, 漏处理 `<=` 操作符
**估计**: 4h

### Bug #2: Q2 `ORDER BY x ASC` parse 错 (P0)

**位置**: `crates/parser/src/parser.rs` ORDER BY 子句
**症状**: `Expected RParen, got "ASC"`
**修复**: 允许 ORDER BY 后面接 ASC/DESC
**估计**: 4h

### Bug #3: Q8 "8-table join 限制" (P0)

**位置**: `crates/executor/src/join.rs`
**症状**: "Unsupported join condition expression"
**修复**: Q8 8-table join 应该 decompose 成 binary tree
**估计**: 12h

### Bug #4: Q9 "Join condition must reference one column from each side" (P0)

**位置**: `crates/executor/src/join.rs`
**症状**: 限制 join condition 必须 1:1 引用
**修复**: 允许复杂 join condition (Q9 多条件 AND)
**估计**: 12h

### Bug #5: Q7/Q8/Q9 "vendor syntax" (P1)

**位置**: `crates/parser/src/parser.rs` INTERVAL 等
**症状**: 不支持 `INTERVAL '3' DAY` 语法
**修复**: 添加 INTERVAL 表达式解析
**估计**: 16h

### Bug #6: Q13 漏 1 行 (P0)

**位置**: `crates/executor/src/aggregate.rs` COUNT DISTINCT 处理
**症状**: Q13 engine=0, sqlite=1
**修复**: 漏算 c_orders 包含的 customer
**估计**: 8h

### Bug #7: Q14 漏 1 行 (P0)

**位置**: `crates/executor/src/aggregate.rs` CASE WHEN 表达式
**症状**: Q14 engine=0, sqlite=1
**修复**: promotion flag 漏算
**估计**: 8h

### 7 bug 总: 64h + 调试 16h = 80h

---

## 四、Stage 2: 4-way 横向对比 (60h)

### 4.1 测试框架 (3-way → 4-way)

**位置**: `tests/tpch_4way_comparison_test.rs` (新, ~500 lines)

```
4 DBs:
- sqlrustgo (via sqlrustgo-mysql-server + mysql CLI, wire 方式)
- SQLite (in-process + sqlite3 CLI)
- MariaDB 11 (via mysql CLI, wire 方式)
- PostgreSQL 16 (via psql, wire 方式)

2 数据集:
- SF=0.1 (~70MB)
- SF=1.0 (~1GB)

22 queries × 4 DBs × 2 SF = 176 query runs
```

### 4.2 实现

- **数据生成**: 用 `crates/bench/examples/tpch_data_gen.rs` (已存在) 生成 SF=0.1 和 SF=1.0
- **SQLRustGo**: 启 server, mysql CLI 跑 22 query
- **SQLite**: 用 sqlite3 CLI 跑 (已支持)
- **MariaDB**: 启 server, mysql CLI 跑
- **PostgreSQL**: 用 psql 跑
- **比较**: 排序后逐行 hash 对比

### 4.3 报告: `docs/releases/v3.9.0/perf/TPCH_4WAY_COMPARISON.md`

| Query | SF | sqlrustgo | SQLite | MariaDB | PostgreSQL | 一致? |
|-------|----|----|------|---------|------------|------|
| Q1 | 0.1 | row_count | row_count | row_count | row_count | ✓/✗ |
| Q1 | 1.0 | ... | ... | ... | ... | ... |
| Q2 | 0.1 | ... | ... | ... | ... | ... |
| ... | ... | ... | ... | ... | ... | ... |

---

## 五、Stage 3: 真实 wire 测试 + 报告 (40h)

### 5.1 修复 sqlrustgo-mysql-server LOAD DATA EAGAIN bug

**位置**: `crates/mysql-server/src/load_data.rs`
**症状**: 装载 orders.tbl (150万行 SF=1) 时 EAGAIN
**修复**: 流式读取 + 异步处理
**估计**: 16h

### 5.2 真实 wire 测试

```
1. 启 sqlrustgo-mysql-server
2. mysql CLI LOAD DATA all 8 .tbl files
3. mysql CLI run 22 queries
4. 捕获每 query row_count + first 3 rows + 耗时
5. 与 SQLite/MariaDB/PostgreSQL 对比
```

### 5.3 报告

- `docs/releases/v3.9.0/perf/TPCH_WIRE_REAL_REPORT.md`
- 含 22/22 query 结果 + 4-way 对比

---

## 六、执行计划 (W12+ 扩展)

### W12 (实施周, 需 Z6G4 真实环境)

| Day | 任务 |
|-----|------|
| D1-2 | 修 Bug #1 (Q1 filter) + #2 (ORDER BY ASC) |
| D3-4 | 修 Bug #3 (Q8 join) + #4 (Q9 join) |
| D5 | 修 Bug #5 (Q7/Q8/Q9 INTERVAL) |
| D6-7 | 修 Bug #6 (Q13) + #7 (Q14) |

### W13 (对比周)

| Day | 任务 |
|-----|------|
| D1-2 | 数据生成 (SF=0.1, SF=1.0) + DB 装载 |
| D3-4 | 4-way 横向对比 (SF=0.1) |
| D5-6 | 4-way 横向对比 (SF=1.0) |
| D7 | 生成 4-way 报告 |

### W14 (真实 wire 周)

| Day | 任务 |
|-----|------|
| D1-3 | 修 sqlrustgo-mysql-server LOAD DATA bug |
| D4-5 | 真实 wire test (mysql CLI) |
| D6-7 | 报告归档 + GA 收口 |

---

## 七、Subsumed 与借力

- **借力**: `tests/tpch_gate_test.rs` (in-process 测试框架) - 扩展为 wire
- **借力**: `tests/tpch_full_22_test.rs` (22 query 列表) - 复用
- **借力**: `tests/tpch_22_mysql_cli_wire_test.rs` (mysql CLI 框架) - 修复 EAGAIN
- **借力**: `crates/bench/examples/tpch_data_gen.rs` (数据生成) - 改 SF=1.0 支持
- **借力**: `docs/plans/2026-06-05-tpch-22-wire-three-way.md` (3-way plan) - 升级 4-way
- **借力**: existing Postgres (psql 16) + SQLite (sqlite3) - 已有, 直接对比

---

## 八、风险与限制

### 8.1 磁盘限制 (Critical)

- **当前可用**: 757Mi (100% used)
- **需要**:
  - SF=0.1 数据: ~80MB
  - SF=1.0 数据: ~1GB
  - MariaDB 11: ~500MB (bin) + 2GB (data)
  - sqlrustgo-mysql-server: ~200MB (release build)
  - 测试结果: ~50MB
  - **总: ~4GB**
- **结论**: 必须清理磁盘, 否则 SF=1.0 不可行

### 8.2 内存限制

- 当前 ~256MB free, 加载 SF=1.0 lineitem (600万行) 需要 4-8GB RAM
- 需 Z6G4 真实环境 (64GB)

### 8.3 时间限制

- 修 7 个真 bug: 80h
- 4-way 对比: 60h
- 真实 wire: 40h
- 总: 180h (3-4 周)
- 当前 P0-P3 16 任务已 100% 完成, 但 TPC-H 真实有效性审计发现新工作

---

## 九、最终评估

### 9.1 v3.9.0 GA 状态 (TPC-H 角度)

| 维度 | 现状 | 期望 |
|------|------|------|
| TPC-H 22/22 形式 | 22/22 PASS (虚假) | 22/22 真 PASS |
| 真实 PASS | 3/22 (Q6, Q17, Q19) | 22/22 |
| Wire 方式 | 0/22 (LOAD DATA EAGAIN) | 22/22 |
| 横向对比 | SQLite-only (1-way) | 4-way |
| 数据集 | SF=0.01 (~7MB) | SF=0.1 + SF=1.0 |

### 9.2 是否可发布 v3.9.0 GA?

**如果以"功能 22/22 PASS"为标准**: ✅ 可发布
**如果以"真实 22/22 PASS"为标准**: ❌ 不可发布, 需先修 7 个真 bug

### 9.3 建议

**v3.9.0 GA 推迟 3-4 周, 优先修复 7 个真 TPC-H bug + 实施 4-way 横向对比.**

或者:
**v3.9.0 GA 拆分为 2 个版本**:
- v3.9.0: 现有 16 P0-P3 任务 + 修 7 bug (核心修复, 3 周)
- v3.9.1: 4-way 横向对比 + SF=1.0 wire test (4-way 对比, 2 周)

---

## 十、Subsumed 决策

| Issue | 决策 |
|-------|------|
| TPC-H 22/22 真伪 | **必须修复 7 个真 bug 才能 GA** |
| 4-way 对比 | **W12-W14 实施** |
| Wire 测试 | **W14 修 LOAD DATA EAGAIN + 真实 wire** |
| SF=0.1 vs 0.01 | **W13 升级到 SF=0.1** |
| SF=1.0 | **W13 必须测试** |
| MySQL 替换 | **MariaDB 11 wire 兼容 (无 MySQL)** |
| 磁盘清理 | **需用户授权清理 .worktrees/** |

---

## 十一、需用户决策

1. **磁盘清理授权**: 是否可清理 `.worktrees/feature-tests` (workspace), `.worktrees/backup-tools`, `.worktrees/tpch-22-bugfixes` 释放 ~5GB?
2. **MySQL 替换**: MariaDB 11 wire 兼容 vs 找 MySQL 8?
3. **TPC-H 真伪优先**: 推迟 GA 3-4 周 修 7 bug? 还是拆分 v3.9.0 + v3.9.1?
4. **Z6G4 真实环境**: 是否可访问 Z6G4 跑 SF=1.0 (需 4-8GB RAM)?

---

**当前 commit**: `936d7cc78` (W12 已 push 4 remote)
**新 issue 建议**: 7 个 TPC-H bug + 4-way 对比 + LOAD DATA 修复 + 4-way 报告 (4 个)

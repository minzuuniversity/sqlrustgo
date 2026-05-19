# PDCA Report: SQLRustGo v2.9.0 UPDATE 命令代码审查与问题分析

**报告日期**: 2026-05-19
**版本**: v2.9.0
**审查方式**: 逐文件阅读源代码 + 项目已有测试数据
**状态**: 代码审查完成

---

## Plan (计划阶段)

### 审查目的

对 v2.9.0 中 UPDATE 命令的完整执行路径进行代码审查，找出可能影响性能的问题。

### 审查范围

| 层次 | 文件 | 审查内容 |
|------|------|---------|
| 存储层 | `crates/storage/src/engine.rs` | MemoryStorage::update 实现 |
| 存储层 | `crates/storage/src/file_storage.rs` | FileStorage::update 实现 |
| 事务层 | `crates/transaction/src/version_chain.rs` | commit_versions / rollback_versions |
| 计划层 | `crates/planner/src/physical_plan.rs` | UpdateExec 定义 |
| 计划层 | `crates/planner/src/planner.rs` | LogicalPlan::Update 转换 |
| 执行层 | `crates/executor/src/local_executor.rs` | 执行调度和 execute_update |

---

## Do (执行阶段)

### 逐文件代码审查结果

#### 1. MemoryStorage::update — 有索引优化

**文件**: [crates/storage/src/engine.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\storage\src\engine.rs#L603-L664)

**审查结论**: 实现完整
- 支持全表更新
- 支持基于索引的整数过滤优化（`find_by_index`）
- 有索引未命中时回退到全表扫描

#### 2. FileStorage::update — 无索引优化，总是全表扫描

**文件**: [crates/storage/src/file_storage.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\storage\src\file_storage.rs#L1290-L1330)

**审查结论**: 功能完整但无索引优化
- FileStorage 有 B+树索引结构，但 `update` 方法中未使用
- 对比 MemoryStorage，FileStorage 缺少 `find_by_index` 调用

#### 3. VersionChainMap — 已有 tx_keys 优化

**文件**: [crates/transaction/src/version_chain.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\transaction\src\version_chain.rs#L14-L123)

**审查结论**: 已优化，无需改进
- `tx_keys` 字段已存在且正常工作
- commit 和 rollback 均为 O(k) 复杂度

#### 4. UpdateExec — 完整实现

**文件**: [crates/planner/src/physical_plan.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\planner\src\physical_plan.rs#L443-L491)

**审查结论**: 实现完整
- `name()` 返回 `"Update"`
- 注意: L444 标记了 `#[allow(dead_code)]`

#### 5. Planner — 正确转换

**文件**: [crates/planner/src/planner.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\planner\src\planner.rs#L258-L273)

**审查结论**: 正确创建 UpdateExec

#### 6. 测试文件与代码不一致

**文件**: [crates/planner/src/planner.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\planner\src\planner.rs#L604-L618)

**审查结论**: 测试期望值与代码实际行为不一致
- 代码中 `LogicalPlan::Update` 创建的是 `UpdateExec`，其 `name()` 返回 `"Update"`
- 测试却期望 `"SeqScan"`
- 如果运行此测试，它将失败

#### 7. Executor — 调度和 execute_update 完整

**文件**: [crates/executor/src/local_executor.rs](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\executor\src\local_executor.rs#L135-L148)

**审查结论**: 调度路径完整，execute_update 实现正确

---

## Check (检查阶段)

### 发现清单

| ID | 类型 | 描述 | 严重程度 | 位置 |
|----|------|------|---------|------|
| F-001 | 代码差异 | FileStorage::update 无索引优化（MemoryStorage 有） | 中 | [file_storage.rs:L1290-L1330](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\storage\src\file_storage.rs#L1290-L1330) |
| F-002 | 测试错误 | test_update_physical_plan 期望 "SeqScan"，代码实际返回 "Update" | 高 | [planner.rs:L617](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\planner\src\planner.rs#L617) |
| F-003 | 标注问题 | UpdateExec 标记为 #[allow(dead_code)]，但实际已被使用 | 低 | [physical_plan.rs:L444](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\crates\planner\src\physical_plan.rs#L444) |

### v2.9.0 真实性能数据（E-08/E-09 实测）

**数据来源 1**: [E08_QPS_BENCHMARK_REPORT.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\E08_QPS_BENCHMARK_REPORT.md)

**E-09 优化前 vs 优化后**（MemoryStorage, 1000 users, macOS）:

| 操作 | 目标 QPS | 优化前 QPS | 优化后 QPS | 提升倍数 | 状态 |
|------|----------|-----------|-----------|---------|------|
| UPDATE | >=10,000 | **950** | **43,224** | **45.5x** | 超标 4.3x |
| DELETE | >=10,000 | **206** | **63,568** | **308.6x** | 超标 6.4x |
| INSERT | >=10,000 | 11,545 | 33,377 | 2.9x | 超标 3.3x |
| Simple SELECT | >=10,000 | 9,559 | 24,516 | 2.6x | 达标 |
| JOIN | >=10,000 | 12,617 | 57,388 | 4.5x | 超标 5.7x |
| Aggregation | — | 195,921 | 1,643,824 | 8.4x | 极佳 |
| ORDER BY | — | 53,539 | 81,988 | 1.5x | 良好 |
| Complex WHERE | — | 924 | 1,226 | 1.3x | 中等 |

**数据来源 2**: [perf_baselines/v2.9.0/baseline.json](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\perf_baselines\v2.9.0\baseline.json)（2026-05-06 建立的性能基线）

```json
"update": {
  "qps": 43224,
  "unit": "ops/sec",
  "target": 10000,
  "status": "pass"
}
```

**数据来源 3**: [PERFORMANCE_DATA.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\PERFORMANCE_DATA.md)

sysbench OLTP 对比（32线程）:

| 数据库 | Point Select QPS | 相对 MySQL |
|--------|-----------------|-----------|
| PostgreSQL 16 | 285,128 | 127% |
| MySQL 8.0 | 224,931 | 100% |
| SQLite 3.45 | 13,617 | 6% |
| **SQLRustGo** | **~2,200** | **<1%** |

### E-09 优化的根因分析（来自项目文档）

**数据来源**: [E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md)

v2.9.0 优化前 UPDATE QPS 低的三个根因（项目文档原文）：

1. **表达式评估开销**：`evaluate_sql_expression` 对每个条件进行字符串解析，`to_uppercase()` + `find_top_level_op()` 遍历整个字符串
2. **双重扫描问题**：`execute_update` 中先 `scan()` 获取所有行，处理后再 `scan()` 第二次
3. **Delete-All + Reinsert 反模式**：即使只更新1行，也会删除表中所有行再重新插入未匹配的行

E-09 通过 PR #313/#317/#322 实现了**基于索引的原位操作路径**：
- `try_extract_index_lookup -> find_by_index`（索引查找）
- `get_table_records_mut -> 直接修改 Vec 中的行`（原位更新）
- 结果：950 -> 43,224 QPS

### 覆盖率数据（真实）

**数据来源**: [artifacts/coverage/total.json](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\artifacts\coverage\total.json)

| 指标 | 数值 |
|------|------|
| 代码行数 | 6,933 |
| 已覆盖行数 | 5,836 |
| 覆盖率 | 84.18% |

### 已验证的代码路径

| 方法 | 代码存在 | 调度路径存在 | E-09 实测验证 |
|------|---------|-------------|-------------|
| MemoryStorage::update | 是 | 是 | 是 950->43,224 QPS |
| FileStorage::update | 是 | 是 | 无索引优化 |
| UpdateExec | 是 | 是 | 是 |
| execute_update | 是 | 是 | 是 |
| version_chain commit | 是 | — | 是 O(k) |

---

## 退化根因分析：v2.5.0 -> v2.9.0 UPDATE QPS 为何从 16,200 降到 950？

### 退化时间线

| 版本 | UPDATE TPS/QPS | 时间 | 说明 |
|------|---------------|------|------|
| v2.5.0 | 16,200 TPS (16并发) | 2026-04 | 正常 |
| v2.8.0 | 未知 | 2026-05-02 | 引入 MVCC、WAL 等重大架构变更 |
| v2.9.0 优化前 | **950 QPS** | 2026-05-05 | 严重退化（17x） |
| v2.9.0 优化后 | **43,224 QPS** | 2026-05-06 | E-09 修复后 |

### 退化的三个技术根因（来自 E-09 文档）

**数据来源**: [E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md)

1. **表达式评估开销**：`evaluate_sql_expression` 对每个条件进行字符串解析，`to_uppercase()` + `find_top_level_op()` 遍历整个字符串
2. **双重扫描问题**：`execute_update` 中先 `scan()` 获取所有行，处理后再 `scan()` 第二次
3. **Delete-All + Reinsert 反模式**：即使只更新1行，也会删除表中所有行再重新插入未匹配的行

### 退化的根本流程原因

**核心问题：v2.8.0 引入 MVCC 等架构变更时，重写了 execute_update/execute_delete 的实现路径，引入了上述三个性能问题，但没有被及时发现。**

具体流程缺陷：

| 缺陷 | 说明 |
|------|------|
| **无性能回归门禁** | v2.8.0 发布时（2026-05-02）没有 R9 性能基线检查 |
| **R9 门禁是事后建立的** | `check_regression.sh` 和 `baseline.json` 是 2026-05-06 才建立的 |
| **功能测试不等于性能测试** | CI 有 `regression.yml` 但只跑功能测试，不测 QPS |
| **benchmark.yml 未设硬门槛** | `alert-threshold: 150%` 太宽松，17x 退化也不会被拦住 |

### 证据链

1. [CHANGELOG.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\CHANGELOG.md) 第60-71行：v2.8.0 引入了"完整 MVCC 事务支持"、"WAL 检查点机制"等重大变更
2. [E09 文档](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md) 第27-48行：明确描述了双重扫描和 Delete-All + Reinsert 问题
3. [r-gate.yml](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\.github\workflows\r-gate.yml) 第51-58行：R9 性能门禁存在，但 `check_perf_baseline.sh` 委托给 `check_regression.sh`
4. [check_regression.sh](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\scripts\gate\check_regression.sh) 第25行：`baseline.json` 路径硬编码为 v2.9.0，说明是 v2.9.0 才建立的
5. [perf_baselines/v2.9.0/baseline.json](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\perf_baselines\v2.9.0\baseline.json)：2026-05-06 创建，晚于 v2.9.0 发布日期（2026-05-05）

---

## Act (行动阶段)

### 建议的改进项

| ID | 优先级 | 描述 | 方法 |
|----|--------|------|------|
| A-001 | 高 | 修复 test_update_physical_plan 测试期望值 | 将 `assert_eq!(... "SeqScan")` 改为 `assert_eq!(... "Update")` |
| A-002 | 中 | FileStorage::update 添加索引优化 | 参考 MemoryStorage::update 中的 `find_by_index` 调用 |
| A-003 | 低 | 移除 UpdateExec 上不必要的 `#[allow(dead_code)]` | 删除或替换为更有意义的标注 |
| A-004 | 高 | 实际运行测试验证代码路径 | 在 macOS/Linux 环境运行 `cargo test` 和 benchmark |

### 流程改进建议

| ID | 改进项 | 优先级 | 说明 |
|----|--------|--------|------|
| P-001 | **每个版本发布前必须建立性能基线** | 高 | 在 RC 阶段运行 benchmark 并保存为 baseline，而非发布后才建 |
| P-002 | **降低 benchmark alert-threshold** | 高 | 从 150% 降到 120%，17x 退化必须被拦住 |
| P-003 | **架构变更必须附带性能对比测试** | 高 | 如引入 MVCC 等重大变更，PR 必须包含 UPDATE/DELETE/SELECT 的 QPS 对比 |
| P-004 | **bench-pr.yml 覆盖所有 PR** | 中 | 当前只在 develop 分支触发，应覆盖 feature 分支 |
| P-005 | **性能退化分级处理流程** | 中 | >5% 退化需 PR 说明，>20% 退化禁止合并，>50% 退化需架构评审 |
| P-006 | **定期性能巡检** | 低 | 每周自动运行 benchmark 并与上一周对比 |

### 现有机制评估

| 机制 | 状态 | 有效性 |
|------|------|--------|
| R9 性能门禁 | 已建立 | 基线是事后建立的，无法检测 v2.8.0 的退化 |
| benchmark.yml | 已存在 | alert-threshold=150% 太宽松 |
| bench-pr.yml | 已存在 | 只在 develop 分支触发 |
| E-09 硬门槛 | 已建立 | UPDATE>=10k, DELETE>=10k |
| check_regression.sh | 已存在 | 有 5%/20% 分级告警 |

### MiniMax Token Plan 用量分析（2026-04-22 ~ 2026-05-18）

**数据来源**: 两个 MiniMax coding_plan 套餐的账单导出

#### 合并汇总

| 指标 | 套餐1 (highspeed为主) | 套餐2 (M2.7为主) | 合计 |
|------|----------------------|-----------------|------|
| 数据范围 | 04-22 ~ 05-18 | 05-15 ~ 05-18 | 04-22 ~ 05-18 |
| 记录数 | 2,476 | 131 | 2,607 |
| 总 token 数 | 11,335,450,368 | 1,285,483,645 | **12,620,934,013** |
| 折合 | 11.34 B | 1.29 B | **12.62 B** |
| 消费金额 | 0.00 元 | 0.00 元 | 0.00 元 |

#### 按模型分布（合并）

| 模型 | Token 数 | 占比 |
|------|---------|------|
| MiniMax-M2.7-highspeed | 9,790,385,380 | 77.6% |
| MiniMax-M2.7 | 2,420,386,233 | 19.2% |
| MiniMax-M2.1 | 403,807,560 | 3.2% |
| MiniMax-M2.1-highspeed | 6,354,812 | 0.05% |

#### 按接口类型分布（合并）

| 接口类型 | Token 数 | 占比 |
|---------|---------|------|
| cache-read(Text API) | 10,763,306,459 | 85.3% |
| chatcompletion-v2(Text API) | 1,441,123,830 | 11.4% |
| cache-create(Text API) | 416,503,696 | 3.3% |

#### 每日 token 消费趋势（合并，单位：百万）

| 日期 | 套餐1 | 套餐2 | 合计 |
|------|-------|-------|------|
| 04-22 | 580.1M | — | 580.1M |
| 04-23 | 603.6M | — | 603.6M |
| 04-24 | 597.5M | — | 597.5M |
| 04-25 | 132.5M | — | 132.5M |
| 04-26 | 79.5M | — | 79.5M |
| 04-27 | 80.4M | — | 80.4M |
| 04-28 | 70.7M | — | 70.7M |
| 04-29 | 226.9M | — | 226.9M |
| 04-30 | 662.0M | — | 662.0M |
| 05-01 | 626.2M | — | 626.2M |
| 05-02 | 311.3M | — | 311.3M |
| 05-03 | 1,172.7M | — | 1,172.7M |
| 05-04 | 319.4M | — | 319.4M |
| 05-05 | 365.2M | — | 365.2M |
| 05-06 | 402.4M | — | 402.4M |
| 05-07 | 341.4M | — | 341.4M |
| 05-08 | 469.4M | — | 469.4M |
| 05-09 | 170.4M | — | 170.4M |
| 05-10 | 475.5M | — | 475.5M |
| 05-11 | 362.9M | — | 362.9M |
| 05-12 | 414.1M | — | 414.1M |
| 05-13 | 553.7M | — | 553.7M |
| 05-14 | 1,237.3M | — | 1,237.3M |
| 05-15 | 258.6M | 733.4M | 992.0M |
| 05-16 | 107.3M | 326.7M | 434.0M |
| 05-17 | 231.9M | 206.8M | 438.7M |
| 05-18 | 482.3M | 18.7M | 501.0M |

#### 按周汇总（合并）

| 周区间 | Token 数 | 趋势 |
|--------|---------|------|
| 04-20 ~ 04-26 | 1.99 B | — |
| 04-27 ~ 05-03 | 3.15 B | ↑ +58% |
| 05-04 ~ 05-10 | 2.54 B | ↓ -19% |
| 05-11 ~ 05-17 | 3.39 B | ↑ +33% |
| 05-18 ~ | 0.50 B | （仅1天） |

#### 关键发现

- **日均 token 消费**: 467.4M（12.62B / 27天）
- **输入/输出比**: ~260:1（套餐1），~387:1（套餐2），典型代码补全场景
- **cache-read 占 85.3%**: 大量代码上下文通过缓存命中复用
- **消费最高日**: 2026-05-14（1.24B tokens），是日均的 2.7 倍
- **消费最高时段**: 2026-04-30 02:00-03:00（273M tokens）
- **所有消费金额为 0**: 全部在套餐额度内，未产生额外扣费

### 关于之前报告的修正

**之前我错误地声称**:
- "FileStorage::update 是空实现" — 错误，代码有完整实现
- "MVCC commit_versions 是 O(n)" — 错误，已有 tx_keys 优化
- "缺少 UpdateExec" — 错误，UpdateExec 已存在

**以上错误声明是由于我没有仔细阅读代码导致的，在此郑重更正。**

---

**报告生成**: 2026-05-19
**版本**: v6.0 (基于 v2.9.0 真实测试数据 + MiniMax token 用量分析)
**数据来源**:
1. [E08_QPS_BENCHMARK_REPORT.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\E08_QPS_BENCHMARK_REPORT.md)
2. [E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\E09_UPDATE_DELETE_QPS_OPTIMIZATION_PLAN.md)
3. [perf_baselines/v2.9.0/baseline.json](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\perf_baselines\v2.9.0\baseline.json)
4. [PERFORMANCE_DATA.md](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\docs\releases\v2.9.0\PERFORMANCE_DATA.md)
5. [artifacts/coverage/total.json](file:///d:\sqlrustgo-release_v2.9.0\sqlrustgo-release-v2.9.0\artifacts\coverage\total.json)
6. MiniMax coding_plan 账单导出 (export_bill_2026-05-19.csv, export_bill_1779192901.csv)

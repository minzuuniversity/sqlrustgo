# B2「不在生产执行路径」排除清单

> **基线**：`develop/v4.1.0` @ `988ef451ad3b52a9eb69122d629d444d3a394a6e`
> 性质：**排除登记，不是缺陷登记**。这些 binary 今天全部通过。
> 排除它们的理由不是「慢」或「失败」，而是**它们通过与否与用户可观测行为无关**。
>
> - source_agent: mcode (MiniMax-M3.1-Flash-Preview)
> - source_run: iaia410-5193-optionc-20261009
> - 关联: #5193（IAIA-410 F3，路线裁决 = 选项 C）
> - SSOT: `scripts/gate/run_b2_per_binary.py` 的 `OFF_PRODUCTION_PATH_BINARIES`

## 1. 为什么需要这一类

B2 此前只有两类排除，二者都不覆盖本情形：

| 类别 | 语义 | 本情形是否适用 |
|---|---|---|
| `DISABLED_BINARIES` | 测试**失败**，登记原因待修 | ❌ 这些测试是通过的 |
| `PERF_ONLY_BINARIES` | 测试**慢**，另走 perf 门禁 | ❌ 这些测试很快 |

「测试通过，但跑的是生产不可达的代码」是第三种情形。IAIA-410 归为 **F3 假集成**：
能力被宣称（架构文档写 `Parser -> Planner -> Optimizer -> Executor`），
既未完成语义（`PhysicalPlan` 无 `execute()`），也未在运行时拒绝。

把它留在门禁里的具体危害：planner 套件全绿时，`cargo test` 的整体 PASS
会被读成「查询路径已验证」，而真实执行者
（`src/engine_select.rs` + `src/engine_dml.rs`）当时可以同时存在 P0 缺陷
（参见 #5167 / #5168 / #5177 / #5181）。

## 2. 排除项与证据

### 2.1 先说清机制：它们本来就不在门禁里

实测（`cargo metadata` + 根 manifest 扫描）：

```
B2 枚举来源: 根 Cargo.toml [[test]] = 434 + mysql-server = 41 = 475

sqlrustgo-planner: 3 个 test target, 其中进入 B2 的 = 0 []
sqlrustgo-executor: 68 个 test target, 其中进入 B2 的 = 2
                    ['distinct_test', 'multi_join_test']

planner_chain_tests      在 B2 枚举中: False
coverage_v3_12           在 B2 枚举中: False
statement_tests          在 B2 枚举中: False
```

`list_test_binaries()`（`run_b2_per_binary.py:259`）只读根 manifest 的 `[[test]]`
条目，加上 `EXTRA_TEST_PACKAGES = ["sqlrustgo-mysql-server"]`。
**`sqlrustgo-planner` 不在其中**，所以这三个 binary 从来没有为 B2 贡献过任何通过率。

这意味着本清单**不是**「把已在门禁里的东西摘出去」，而是**把一个跨 manifest 的
不变量变成受检事实**：`sqlrustgo-planner` 一旦被加进 `EXTRA_TEST_PACKAGES`，
门禁就会开始把不可达代码计为覆盖。R4a 会在那一刻 FAIL，要求显式决策，
而不是让覆盖率静默膨胀。

### 2.2 判据：导入面检查

排除判据是**可复现的导入面检查**：binary 的测试文件不得引用生产执行引擎
`sqlrustgo::ExecutionEngine`。

| binary | 导入面 | 结论 |
|---|---|---|
| `planner_chain_tests` | `sqlrustgo_planner::{...}`、`sqlrustgo_types::Value` | 零 `ExecutionEngine` |
| `coverage_v3_12` | `sqlrustgo_planner::physical_plan::{...}` | 零 `ExecutionEngine` |
| `statement_tests` | `sqlrustgo_planner::{Expr, MergeClause, MergeStatement}` | 零 `ExecutionEngine` |

复现命令：

```bash
grep -rn 'ExecutionEngine\|sqlrustgo::' crates/planner/tests/
# 仅命中 sqlrustgo_planner / sqlrustgo_types，无执行引擎
```

### 2.3 明确**不**排除的部分：`crates/executor`

`crates/executor` 的 68 个 test target 中有 2 个（`distinct_test`、
`multi_join_test`）与根 manifest 同名，因而**确实进入 B2 门禁并真实执行**。
它们 import `sqlrustgo::ExecutionEngine` 并驱动真实解释器
（如 `crates/executor/tests/multi_join_test.rs:11`），跑的是生产执行路径，
绿灯是真证据。

同理，`crates/optimizer` 不整体排除：`decorrelate` / `stats` /
`unified_cost` 在生产路径被真调用，`rules` 只借类型。仅借类型的模块由
`CORE_PATH_INVENTORY.md` 记录，不在此清单。

按 crate 整体排除会误伤真实覆盖 —— 这正是 #5193 选项 B 的删除风险，
也是本清单范围必须窄到 binary 级的原因。

## 3. 失效条件（防腐化）

本仓库已有反面教训：B2 禁用清单曾累计 89 项，其中 `mysqladmin_e2e_test`
的登记归因（「Investigate mysqladmin e2e flow」）是**错的** —— 缺陷实际在
共用客户端行解析器，修复后 8/8 通过并移出清单（#5186）。
**排除而不设复审条件，五年后就是下一个它。**

因此每一项必须满足以下全部条件才继续排除：

| # | 条件 | 判定方式 |
|---|---|---|
| R1 | `PhysicalPlan` trait 仍无 `execute()`（注意：只看 trait 定义块，`SeqScanExec` 的固有 `execute()` 不算） | `check_off_production_path_registry.sh` R1 |
| R2 | 生产侧仍从任一入口不 import `sqlrustgo_planner` | 同上 R2 |
| R3 | 未发生路线变更（#5193 裁决改选 A 或 B） | 同上 R5 |
| R4a | 排除项仍**不在** B2 枚举集合内 | 同上 R4a |
| R4b | 排除项仍零 `ExecutionEngine` 引用 | 同上 R4b |

**复审周期：每 90 天，或 #5193 状态变化时，以先到者为准。**
超期未复审时，`check_off_production_path_registry.sh` 报 FAIL 并阻断合并
（`stale_after_days = 90`）。

任一条件不再成立时的处置：**移出清单并恢复进门禁**，而不是改判据。

## 4. 绕过与重跑

```bash
# 强制重跑（排除项仍必须能跑通，否则是另一类缺陷）
python3 scripts/gate/run_b2_per_binary.py --include-off-production-path

# 只跑这三项
python3 scripts/gate/run_b2_per_binary.py \
  --include-off-production-path \
  --bin planner_chain_tests --bin coverage_v3_12 --bin statement_tests
```

汇总输出新增 `SKIP_OFF_PROD` 计数与 `skipped_off_production_path` JSON 字段，
使排除在报告中**可见**（不可静默）。

## 5. 关联

- Issue #5193 —— F3 定性与 A/B/C 成本量化
- `docs/releases/v4.1.0/ARCHITECTURE_ROUTE_DECISION_5193.md` —— 裁决材料与防腐化要求
- `docs/releases/v4.1.0/ARCHITECTURE.md` §2.2 —— 架构文档中的未接入标注
- `docs/releases/v4.1.0/CORE_PATH_INVENTORY.md` —— 生产可达路径清单
- `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md` §13.2 —— AUTH-V410-002
- `docs/releases/v3.12.0/b2-disabled-test-binary-registry.md` —— 失败类排除清单（89 项）
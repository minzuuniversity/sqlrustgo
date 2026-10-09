# #5193 架构路线裁决材料（A / B / C 三案实测成本）

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: iaia410-5193-decision-20261009
- 基线: `427a446278`
- 性质: **决策支持材料，不代替裁决**
- Refs #5193, #5168, #5177, #5116

---

## 0. 为什么这份材料现在才写

#5193 提出于 2026-10-09，此后一直在等裁决。它不是形式问题：

- **#5168**（单列主键点查全表扫描）与 **#5177**（UPDATE/DELETE WHERE 全表扫描）修的都是
  `engine_select.rs` / `engine_dml.rs` 里的扫描路径
- 选 A → 这两个文件被重写，已做的优化作废
- 选 B → 性能路线需在存量手写解释器上重画
- 选 C → 可立即推进，不阻塞

**两条 P0 性能工作目前都压在这个未决问题上。**

---

## 1. 核心事实（全部经 grep 交叉核对）

### 1.1 物理计划层的真实状态

```
$ grep -n "pub trait PhysicalPlan" -A12 crates/planner/src/physical_plan.rs | grep -c "fn execute"
0
```

`PhysicalPlan` trait **没有 `execute()` 方法**。`SeqScanExec::execute` 硬编码
`Ok(vec![])`；`IndexScanExec` 根本没有 `execute`。

构造点分布：

| 文件 | 构造点 | `#[cfg(test)]` 起于 | 测试模块之前 |
|---|---:|---:|---:|
| `planner/src/physical_plan.rs` | 15 | 494 | **0** |
| `planner/src/planner.rs` | 9 | 358 | **9** |
| `executor/src/explain.rs` | 6 | 459 | **0** |

`planner.rs` 的 9 个位于 `impl Planner for NoOpPlanner`（:343）→ `DefaultPlanner::new()`
（:348/:353）—— 生产源码内，但 `mysql-server` 从不 import planner，`DefaultPlanner` 的
全部外部构造点在 `crates/planner/tests/`。

**准确表述**：生产源码中存在构造代码，但无生产调用方可达。

### 1.2 迁移面（决定选项 A 的成本）

```
engine_select.rs   9450 行
engine_dml.rs      2478 行
────────────────────────────
合计             11928 行
```

选项 A 要把这 11928 行手写解释器迁到物理计划执行。

### 1.3 优化器的真实接入面（决定选项 B 的风险）

生产路径对 `crates/optimizer` 的引用：

| 模块 | 生产引用 | 性质 |
|---|---:|---|
| `decorrelate` | 3 | **真调用**（`try_decorrelate` 有诊断埋点计数） |
| `stats` | 3 | **真调用**（`build_histogram_from_values`，见 `cbo_estimator.rs:209`） |
| `unified_cost` | 2 | **真调用**（`engine_builder.rs` 七处构造 `UnifiedCostModel`） |
| `rules` | 2 | ⚠️ **只借类型，未调用任何优化规则** |
| `unified_plan` | 1 | 待确认 |
| `analyze_predicate_for_index` | 1 | 真调用 |

`rules` 的两处引用都是 `use sqlrustgo_optimizer::rules::{BinaryOperator, Expr}` —— 借用
`Expr` 类型做表达式转换，**不是调用优化规则**。

**这是选项 B 最容易出错的地方**：若按「有引用=在用」判断，会把 `rules` 一起保留；若按
crate 整体删除，会误删真在用的 `decorrelate` / `stats` / `unified_cost`。

---

## 2. 三案实测成本

| | **A 接入** | **B 废弃** | **C 标注** |
|---|---|---|---|
| 工作量 | 补 `execute()` + 实现各算子 + 迁移 11928 行 | 删物理计划层 + 未接入的 Volcano 执行器 | 保留代码 + 文档标注 + B2 枚举排除 |
| 触及生产代码 | **是**，核心执行器重写 | 否 | 否 |
| 对 #5168/#5177 | **作废**（重写） | 重画路线 | **不阻塞** |
| 主要风险 | 迁移期两套执行器并存，所有正确性缺陷可能翻倍暴露 | **误删在用代码**（§1.3） | 「B2 排除」退化为永久豁免 |
| 前置条件 | 无 | **须先完成「在用代码保留清单」** | 须防排除项腐化 |

### 2.1 选项 B 的保留清单（实测得出）

若选 B，**必须保留**：

```
crates/optimizer/src/decorrelate*      # try_decorrelate 有生产埋点
crates/optimizer/src/stats*            # cbo_estimator.rs:209 真调用
crates/optimizer/src/unified_cost*     # engine_builder.rs 七处构造
```

**须逐项确认**：`rules`（只借类型）、`unified_plan`。

**可删候选**：`crates/planner/src/physical_plan.rs`（706 行）、`SeqScanExec` /
`IndexScanExec` 及其测试。`crates/executor` 49 个文件中 20 个含执行器特征，需逐个判定 ——
**不能按 crate 整体删**。

### 2.2 选项 C 的防腐化要求

C 是成本最低且不阻塞的，但「B2 枚举排除」必须绑定复审，否则会变成永久豁免。本仓库已有
反面教训：B2 禁用清单累计 89 项，其中 `mysqladmin_e2e_test` 的登记归因（「Investigate
mysqladmin e2e flow」）是**错的** —— 缺陷实际在共用的客户端行解析器，修复后 8/8 通过并
已移出清单（#5186）。

建议：排除项必须写明失效条件（如「#5193 裁决后 N 天内复核」），否则重蹈覆辙。

---

## 3. 与 IAIA-410 的关系

无论选哪个，`crates/planner/tests/` 的通过率**在任何情况下都不构成生产正确性证据** ——
它跑的是生产不可达的代码路径。本文件的数据进一步支持该结论。

审计的一票否决条款不因本裁决而改变：若因路线切换引入新的静默数据丢失，仍阻断 GA。

---

## 4. 裁决所需信息（已备齐）

- [x] 物理计划层是否可执行 —— 不能
- [x] 生产可达性 —— 无
- [x] 选项 A 迁移面 —— 11928 行
- [x] 选项 B 保留清单 —— decorrelate / stats / unified_cost
- [x] 选项 C 防腐化要求 —— 需绑定复审
- [ ] **项目层面裁决：选 A / B / C**

---

## 5. 建议（供参考，非结论）

**若项目已决定向火山模型演进**，应尽早选 A 并接受 #5168/#5177 返工 —— 越晚切换，
重复劳动越多。

**若尚未决定投入**，选 C 是成本最低的：不阻塞 P0 性能工作，同时把「planner 是死代码」
这一事实写进架构文档，消除 F3 认知偏差。F3 的危害不是那 706 行本身，而是**后续会话
可能据此认为 planner 已集成**。

**选 B 前必须先完成保留清单**。§2.1 给出了实测的三项，但 `crates/executor` 的 20 个
候选文件仍需逐个判定 —— 在没有这份清单的情况下删除，会误伤生产路径。

---

## 6. 引用

- `docs/releases/v4.1.0/CORE_PATH_INVENTORY.md` —— 核心路径清单（人工复核版）
- `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md` —— §9 工具边界、§2.1 缺口 A
- `docs/releases/v3.12.0/b2-disabled-test-binary-registry.md` —— 89 项禁用登记
- `crates/planner/src/physical_plan.rs`、`planner.rs`
- `src/engine_select.rs`、`src/engine_dml.rs`
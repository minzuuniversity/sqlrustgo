# 独立契约测试运行域概念设计

## 决策

采用独立 Cargo package，路径拟定为 `tools/contract-test-domain/`，并从 workspace `members` 与 `default-members` 排除。它只通过显式 `--manifest-path tools/contract-test-domain/Cargo.toml` 执行。根包 `cargo test --all-features` 和 `cargo test --workspace --exclude sqlrustgo` 均不得自动选择它。

不采用仅 feature 隔离：现有根包 CI 使用 `--all-features`，无法保证隔离。不采用批量 `#[ignore]`：这会消耗 G19 额度并混淆 NOT_RUN 与执行结果。不采用“测试与修复同批才合入”作为唯一机制：它不能持续保存修复前的可复现实证。

## 组件

- `contract-test-domain`：编译和运行契约测试，输出逐 case 原始事件。
- Test Factory 分类器：读取计划 case 与原始事件，产出双轴结果。
- 正式 DAG/Issue/测试源码：唯一权威输入；本域只保存固定引用和运行收据。
- 常规 CI：只验证既有回归，不消费 EXPECTED_RED。
- 契约域 CI：显式运行；任何漏测、签名漂移、意外转绿或基础设施失败均非成功。
- 发布门禁：读取适用 Issue/契约结果；存在未获出口批准的开放 RED 时拒绝发布。

## 数据流

`正式 DAG + 固定 Git SHA + 测试源码` → 计划清单 → 显式 Cargo 运行 → 原始事件 → 双轴分类 → 机器收据 → 独立复验/发布判定。

## 关键隔离条件

1. 根 workspace metadata 不把独立包算作成员。
2. 常规根包和 workspace 命令不执行契约二进制。
3. 契约入口必须核对计划 case 数与实际 case 身份集合完全相等。
4. 独立包可依赖根仓库 crate，但不得复制产品实现。
5. 发布门禁只能消费签名和来源都验证过的收据。

## 角色分离

Codex 是设计作者，不是最终独立验收签署者。DSH-B 可协调实施但不得自签验收；DSH-A 调度的独立 Reviewer 或 Owner 指定 Reviewer 完成最终复验。

Reviewer 必须与实现者不同，并从最终 PR HEAD 的干净 worktree 执行；报告包含 source_agent、source_run、timestamp、evidence_hash、conflict_resolution。仅检查 mergeable、draft、分支保护或 CI 状态的自动巡检属于 `FORMAL_CHECK_PASSED`，不得写入 `TECHNICAL_REVIEW_APPROVED`，不得解除 HUMAN_GATE。

形式审批与技术审批是两个正交状态。任何一方缺失时，工作包不能关闭、PR 不能合并、发布门禁不能通过。

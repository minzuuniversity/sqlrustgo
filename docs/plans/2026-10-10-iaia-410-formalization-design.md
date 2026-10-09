# IAIA-410 正式化设计

> **文档性质**：设计决策，不是审计结论或门禁结果。
> **制定日期**：2026-10-10
> **目标分支**：`develop/v4.1.0`
> **source_agent**：codex
> **source_run**：iaia-410-formalization-20261010
> **assessed_commit**：`f1b77bf32cd38c95105de3c6191d75b8bb3687f0`

## 1. 背景

现有 `IAIA_410_AUDIT_PLAN.md` 同时承载计划、阶段执行结果、整改说明和 Issue 状态，导致原始审计协议会随修复过程变化，也使手工维护的状态与 Gitea 发生漂移。IAIA-410 需要一个冻结边界清晰、可机械验证且不把文档声明当作实跑证据的正式入口。

## 2. 选定方案

采用“一个稳定 SSOT + 三类派生材料 + Gitea Issue DAG”的结构：

1. `IAIA_410_SSOT.md` 只定义范围、术语、阶段、角色、证据契约、判定规则和关闭协议。
2. 每次执行使用不可变 `run_id`，证据写入 `audit/v4.1.0/<run_id>/`，由 `manifest.yaml` 绑定 commit、环境、命令、退出码和哈希。
3. finding 使用机械可读记录，独立携带 ADR-014 的来源字段；结果报告只能从 manifest 和 finding 派生。
4. Gitea 主 Issue 管理阶段依赖，子 Issue 管理可独立验收的工作包；缺陷 Issue 作为 finding 的整改下游，不替代审计工作包。
5. 现有 `IAIA_410_AUDIT_PLAN.md` 保留为历史材料，不再作为状态或审计协议的唯一来源。

## 3. SSOT 边界

SSOT 回答“如何执行和判定”，不写任何未经当前 run 实跑支持的 PASS/FAIL，也不复制会快速漂移的 Issue 状态。它必须覆盖：

- 审计对象和明确的 in-scope/out-of-scope/deferred 能力；
- 生产入口到声明能力的追溯矩阵；
- 静态、动态、差分、故障注入、测试反审计和跨版本六类工作流；
- provenance、环境 manifest、finding、独立复核和冲突裁决协议；
- 一票否决和阶段门禁的衔接，但不得自行宣布发布阶段 PASS；
- Master Issue 与子 Issue 的关闭条件。

## 4. Issue DAG

主 Issue 下建立十二个工作包：

- A01 基线冻结与证据 manifest；
- A02 对外声明、生产入口与范围矩阵；
- A03 静态调用链真实性审计；
- A04 SQL 语义差分；
- A05 索引与存储真实性；
- A06 事务、WAL、恢复和故障注入；
- A07 测试系统反向审计和 mutation REVIEW；
- A08 disabled binary 复核；
- A09 跨版本回归；
- A10 安全、协议和隔离边界；
- A11 Independent Judge 复核；
- A12 最终报告、治理审批与阶段衔接。

每个子 Issue 必须声明依赖、交付物、验收命令、证据字段和关闭条件。A11 依赖 A01–A10；A12 依赖 A11。任何发现产生独立 defect Issue，并从工作包双向链接。

## 5. 状态模型

工作包状态使用 `not_started / running / evidence_ready / independently_verified / blocked`。finding 使用 `suspected / confirmed / refuted / unverified / remediated / regression_verified`。Issue 的 open/closed 由 Gitea 提供，SSOT 不复制实时状态。

只有独立复核完成且证据链闭合后，工作包才能进入 `independently_verified`。修复 PR 合并不自动等于 finding 已经 `regression_verified`。

## 6. 错误处理与证据规则

- 命令非零退出、证据缺失、哈希不匹配或环境污染必须 fail closed。
- 参考 DBMS 对某项语法不兼容时，记录 `not_comparable`，不得强行判等。
- 无法复现只产生 `unverified`；只有固定环境下的反证、完整尝试记录和独立复核才能给出 `refuted`。
- Agent 间结论冲突必须保存双方证据，并填写 `conflict_resolution`；人类架构师拥有最终裁决权。
- 任何 PASS/FAIL 必须绑定实际命令、退出码、日志文件和 SHA-256，不得引用本文档代替运行证据。

## 7. 验证策略

本次正式化仅创建治理文档和 Issue，不宣称 IAIA-410 审计本身已执行。文档变更至少验证：

1. 新增文档链接有效；
2. Issue 编号和 URL 可查询；
3. SSOT 不包含手写实时状态；
4. finding 和 manifest schema 含完整 provenance 字段；
5. 文档 gate 按仓库规则实跑并记录退出码；
6. 由未参与编写的 reviewer 对 SSOT 与 Issue DAG 做独立审查。


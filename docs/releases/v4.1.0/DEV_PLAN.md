# v4.1.0 开发计划

> **更新日期**: 2026-10-08
> **状态**: ACTIVE PLAN；`STAGE.yaml` 当前阶段仍为 `ALPHA`。
> **审计基线**: `gitea252/develop/v4.1.0@d312683244`（2026-10-08 rebase 后快照）
> **已合并代码进展**: 见 `ISSUES_PLAN.md` §5.1；#5025/#5057/#5099/#5103/#5117
> 已有部分 PR 合入 develop，但所有 P0/P1 issue 在 Gitea 上仍为 OPEN。

## 1. 当前判断

v4.1.0 已完成一批事务、SessionContext 和多数据库隔离修复，但这些合并记录只证明
修复已进入开发分支，不证明 Alpha→Beta 门禁通过。BustubX-EDU B 轨暴露的问题说明，
项目需要从“按模块测试”升级为“连接级状态 + 真实协议 + 参考模型”的验证方式。

本阶段不直接推进 Beta 标签。先完成 #5117 管理的 P0/P1 整改，再在单一冻结提交上
执行晋级审核。

## 2. 工作流与优先级

### 2.1 P0：阻断 Alpha→Beta

| 工作包 | Issue | 交付物 | 完成标准 |
|---|---|---|---|
| 事务/会话上下文 | #5112 | 显式连接/事务所有权、并发回归、mutation probe | 事务与多连接矩阵全部通过；依赖 #5099/#5057/#5025 关闭 |
| CI 与门禁可信度 | #5113 | 可解析 workflow、fail-closed 脚本、P11-P16 自测 | required checks 全绿；无 invalid/skipped/missing |

P0 未清零时禁止发起阶段晋级 PR。

### 2.2 P1：Beta 可验证性

| 工作包 | Issue | 交付物 | 完成标准 |
|---|---|---|---|
| BustubX-EDU 回归 | #5114 | B 轨 manifest、oracle、SQL corpus 分类 | 必测 100%，corpus `>=80%`，无 P0 正确性失败 |
| SOAK/恢复 | #5115 | 1h/24h/168h runner、heartbeat、故障注入 | Alpha 1h 成功；Beta 24h 可执行；证据 fail-closed |
| 性能基线 | #5116 | 固定环境基线、比较器、PR/nightly 分层 | 基线可复现；明显吞吐/尾延迟回退可阻断 |

### 2.3 既有阻断依赖

- #5099：事务上下文三层根因与重构验收。
- #5057：连接级 SessionContext 和多库隔离。
- #5025：多数据库表命名空间与 `SHOW TABLES FROM`。
- #5102：SOAK 中止、证据真实性及阶段文档漂移。
- #5103：executor `call_body_raw_sql_runs_through_dispatcher` 回归。

不得新建重复 Issue 替代这些依赖；新工作包只负责统一验收和补齐测试系统。

## 3. 实施阶段

### Phase A：测试系统止血

1. 修复 CI workflow 解析和退出码传播。
2. 让 P11-P16、anti-ignore、覆盖率和安全门禁具备自测与 mutation probe。
3. 建立统一证据 manifest，禁止跨提交复用 PASS。

退出条件：#5113 关闭，基础 lint/build/test 在同一提交成功。

### Phase B：正确性收口

1. 明确连接、SessionContext、TransactionContext 和 undo 的所有权边界。
2. 先以失败回归固定 #5099/#5057/#5025，再修复实现。
3. 运行确定性调度、真实 wire 交错和 8000 事务参考模型测试。

退出条件：#5112 关闭，正确性矩阵无间歇失败。

### Phase C：外部工作负载与恢复

1. 固化 BustubX-EDU B 轨 workload 和结果 oracle。
2. 执行 SQL corpus，并按根因分桶而非只统计总通过率。
3. 完成 1h SOAK；验证 24h/168h runner、heartbeat 与故障注入能力。

退出条件：#5114、#5115 关闭，#5102/#5103 已处置。

### Phase D：性能基线与晋级审核

1. 建立功能等价的 Criterion/sysbench A/B 基线。
2. 为 PR 提供快速性能检查，为 nightly 提供完整矩阵。
3. 冻结一个 commit，执行 AB-01..AB-10，生成带哈希的审核证据。
4. 经 codeowner 与 governance 独立批准后，以单独 PR 修改 `STAGE.yaml`。

退出条件：#5116 和 #5117 关闭，才允许创建 Beta 标签。

## 4. 开发和关闭规则

1. 从最新 `develop/v4.1.0` 创建短生命周期分支；不得直接修改保护分支。
2. 每个缺陷先提交可复现的失败测试，再提交实现修复。
3. 涉及符号修改前执行 GitNexus impact；提交前执行 detect changes。若索引不可用，
   必须在 PR 证据中记录限制，并以调用点审计和针对性测试补偿。
4. 文档和代码 PR 均执行对应门禁，日志必须含命令、退出码和 commit。
5. Issue 只能由已合并 PR 关闭；评论需附 source agent、source run、时间和证据哈希。
6. P0/P1 发布阻断项不得以 DEFERRED 绕过；变更范围必须由 governance 明确裁决。

## 5. 分支与合并

| 分支 | 用途 |
|---|---|
| `develop/v4.1.0` | 活跃开发和 Alpha/Beta 集成 |
| `feature/*` / `fix/*` / `codex/*` | 单一 Issue 的实现与文档工作 |
| `beta/v4.1.0` | 通过 Alpha→Beta 终验后建立 |
| `release/v4.1.0` | RC/GA 候选，当前不得提前同步 |

PR 至少需要代码所有者和治理审核；以服务器实际保护规则为准。多远端同步只能发生在
252 主开发服合并后，并逐个校验目标 SHA。

## 6. 里程碑

| 里程碑 | 必须完成 | 不包含 |
|---|---|---|
| Alpha 修复完成 | #5112、#5113 和既有 P0 阻断项 | Beta 阶段声明 |
| Beta 晋级候选 | #5114、#5115、#5116；AB-01..AB-10 同提交复核 | RC/GA 结论 |
| Beta 正式进入 | #5117 终验、人工批准、独立 STAGE PR | 168h 已通过声明 |
| RC/GA 准备 | 24h/168h、85% GA 覆盖率、性能与安全长期证据 | 本计划当前执行范围 |

## 7. 引用

- `docs/releases/v4.1.0/TEST_PLAN.md`
- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md`
- `docs/releases/v4.1.0/ISSUES_PLAN.md`
- `docs/releases/v4.1.0/STAGE.yaml`
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md`
- `docs/governance/DOC_CHECK_CORRECTION_RULES.md`
- `docs/governance/adr/ADR-014-multi-ai-coordination.md`

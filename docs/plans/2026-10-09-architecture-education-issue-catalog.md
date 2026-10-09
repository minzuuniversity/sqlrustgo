# SQLRustGo 架构与教学文档 Issue 目录

> **状态**：PROPOSED，尚未在 Gitea 创建
> **基线提交**：`345ee9539cc98adc92e2a4cd27c8c4b1aa6cdebf`
> **总计划**：[`2026-10-09-architecture-education-documentation-implementation.md`](2026-10-09-architecture-education-documentation-implementation.md)

本文件提供可以直接转换为 Gitea Issue 的工作包。文中的 `DOC-*`、`EDU-*`、`GATE-*` 是稳定逻辑 ID，不是假定的远程 Issue 编号；创建远程 Issue 后，应增加编号映射，而不是覆盖逻辑 ID。

## 1. 并行执行 DAG

```text
EPIC-DOC-001
    |
    +--> DOC-001 inventory --------+
    |                              |
    +--> DOC-002 canonical schema -+--> DOC-003 current entrypoint
                                   |
                                   +--> DOC-101 ... DOC-108 (8 layers, parallel)
                                   |         |
                                   |         +--> DOC-201 execution flows
                                   |         +--> DOC-202 testing/evidence
                                   |         +--> DOC-203 capability matrix
                                   |
                                   +--> EDU-001 lab framework
                                             |
                                             +--> EDU-101 ... EDU-105 (parallel)

DOC-101..108 + DOC-201..203 + EDU-101..105
    --> GATE-001 gate scripts and self-tests
    --> DOC-301-* migration/archive shards
DOC-301-*
    --> MAP-001 canonical single-writer integration
MAP-001 + GATE-001
    --> GATE-002 CI activation
GATE-002
    --> DOC-999 same-commit final audit
```

## 2. 通用 Issue 合同

所有子 Issue 都必须包含并遵守：

```yaml
coordination:
  source_agent: <agent-id>
  source_run: <run-id>
  timestamp: <ISO8601>
  evidence_hash: sha256:<hash>
  conflict_resolution: <none or resolution note>
ownership:
  worktree: .worktrees/<agent-id>-<issue-id>
  branch: <agent-prefix>/<issue-id>-<slug>
  owned_paths:
    - <paths>
closure:
  requires_merged_pr: true
  target_branch: develop/v4.1.0
```

通用关闭条件：

- 只修改 `owned_paths`；公共索引变更交给集成 Issue。
- PR 描述列出实际命令、exit code、commit、timestamp 和 artifact/hash。
- `bash scripts/gate/check_docs_links.sh` 和 `git diff --check` 实跑无失败。
- PR 不得新增全量断链；`--all` 的 14 个基线旧债在 DOC-001 中逐项登记，由 DOC-301 分片处理。在旧债清零前，不得把 `--all` 非零归因于当前子 Issue，也不得声称全量链接门禁通过。
- 代码或 gate 有变更时，执行对应测试与 P11–P16；文档 Claim 不替代执行证据。
- PR 合并后才可关闭；部分完成保持 Open。

## 3. Epic 与基础 Issue

### EPIC-DOC-001：八层架构、教学轨与唯一文档入口

**目标**：协调全部子 Issue，并在同一最终提交上完成文档真实性审计。

**Owned paths**：仅远程 Epic 内容；不直接修改模块文件。

**依赖**：本文所有 Issue。

**关闭条件**：

- DOC-001、DOC-002、DOC-003、DOC-101–108、DOC-201–203、EDU-001、EDU-101–105、MAP-001、GATE-001、GATE-002、全部 DOC-301 分片和 DOC-999 均已关闭。
- 最终 CANONICAL-MAP、代码、测试和证据均绑定同一 commit。
- 没有未解决的事实冲突或两个 current owner。
- Epic 是追踪 Issue，不单独要求代码 PR；它以 DOC-999 最终审计 PR 和所有子 Issue 的合并状态作为关闭证据。

### DOC-001：当前文档资产盘点与漂移分类

**Priority**：P0
**可并行**：可与 DOC-002 并行。
**Owned paths**：`docs/audit/documentation/architecture-education-inventory.md`

**工作内容**：

- 清点 current、release、governance、teaching、archive 候选文档。
- 对重复教学目录和入口漂移逐项记录 Keep、Redirect、Historical、Archive 建议。
- 每项记录路径、最后修改提交、事实 owner 和冲突说明。

**验收**：清单至少覆盖根 `README.md`、`docs/README.md`、`docs/architecture.md`、`docs/EXECUTION_PATH.md`、`docs/tutorials/`、`docs/教学实践/`、`docs/教学计划/`、`docs/teaching-labs/`、L0–L7 候选文档和 B 轨素材；登记当前 14 个全量断链；不移动任何文件。

### DOC-002：CANONICAL-MAP schema 与文档生命周期规则

**Priority**：P0
**可并行**：可与 DOC-001 并行。
**Owned paths**：`docs/current/CANONICAL-MAP.md`、`docs/current/DOCUMENT-LIFECYCLE.md`

**工作内容**：定义字段、状态枚举、版本锚点、freshness、owner 和迁移规则；先登记 L0–L7 与 B00–B18 为 Planned，禁止编造完成状态。

**验收**：schema 能区分 Verified、Partial、Planned、Historical；明确唯一 owner 规则和 stale 处理。

### DOC-003：建立 current 总入口

**Priority**：P0
**依赖**：DOC-001、DOC-002。
**Owned paths**：`docs/current/README.md`、`docs/current/navigation-check.tsv`、`docs/README.md`

**工作内容**：建立面向开发者、维护者、学生和教师的四条导航；将旧 `docs/README.md` 收敛为稳定入口，不删除历史记录。

**验收**：生成 `docs/current/navigation-check.tsv`，逐行记录 `source -> hop1 -> target`；任一 L0–L7、B00–B18、测试策略和版本历史从 `docs/README.md` 最多经过两个中间 Markdown 链接可达，并由链接检查脚本验证目标存在。

## 4. 八层模块 Issue

下列八项在 DOC-002 合并后并行执行。每项只拥有自己的目录，不直接编辑 CANONICAL-MAP。

### DOC-101：L0 系统入口与 Session

**Owned paths**：`docs/modules/00-system-entry/**`
**范围**：CLI、mysql-server、server、network、连接、Session、请求生命周期。
**最低执行流**：客户端连接到 SQL 执行入口。
**验收重点**：区分进程、连接、Session、事务上下文，明确 CLI 与 MySQL 协议入口差异。

### DOC-102：L1 SQL 前端

**Owned paths**：`docs/modules/01-sql-frontend/**`
**范围**：types、parser、catalog，以及当前真实存在的绑定/校验职责。
**最低执行流**：SQL 文本到 AST，再到解析后的名称/类型。
**验收重点**：不能把尚未独立存在的 Binder 层写成已实现能力。

### DOC-103：L2 Planner 与 Optimizer

**Owned paths**：`docs/modules/02-planner-optimizer/**`
**范围**：planner、optimizer、计划数据结构、规则与代价边界。
**最低执行流**：SELECT AST 到可执行计划。
**验收重点**：给出优化前后语义等价测试，不以计划变化代替结果正确性。

### DOC-104：L3 Executor

**Owned paths**：`docs/modules/03-executor/**`
**范围**：executor、根 ExecutionEngine、算子、DML/DDL、ExecutionContext。
**最低执行流**：SeqScan → Filter → Projection，以及一条写入路径。
**验收重点**：明确当前是否存在多条执行路径和兼容外观，不将目标架构描述为现状。

### DOC-105：L4 Storage

**Owned paths**：`docs/modules/04-storage/**`
**范围**：storage、cache、spill、Tuple/Page、Buffer、索引、文件与列存。
**最低执行流**：逻辑读写到 Page I/O。
**验收重点**：记录 pin/dirty/flush、RID/slot、索引一致性和崩溃边界。

### DOC-106：L5 Transaction 与 Recovery

**Owned paths**：`docs/modules/05-transaction-recovery/**`
**范围**：transaction、wal-verification、隔离、锁/MVCC、WAL、恢复。
**最低执行流**：BEGIN → DML → COMMIT 和崩溃重启。
**验收重点**：只陈述代码与实测支持的 ACID 语义；列出未验证边界。

### DOC-107：L6 Server Operations

**Owned paths**：`docs/modules/06-server-operations/**`
**范围**：security、telemetry、admin、information-schema、query-stats。
**最低执行流**：请求认证/授权、执行、观测和管理查询。
**验收重点**：区分配置存在、运行启用和安全效果已验证。

### DOC-108：L7 Extensions

**Owned paths**：`docs/modules/07-extensions/**`
**范围**：distributed、vector、graph、gis、rag、shard-router。
**最低执行流**：至少一条扩展查询或适配路径。
**验收重点**：说明与核心 SQL 路径的依赖方向、成熟度和非目标。

## 5. 横向文档 Issue

### DOC-201：五条端到端执行流教程

**Priority**：P1
**依赖**：DOC-101–108 至少提供已复核代码锚点。
**Owned paths**：`docs/current/EXECUTION-FLOWS.md`、`docs/tutorials/README.md`、`docs/tutorials/flows/**`

**交付**：SELECT、写事务、崩溃恢复、MySQL Session、扩展查询五条流程。

**验收**：每条流程含输入、hop、数据形态、失败出口、源码链接、测试和可执行命令；禁止复制模块正文。

### DOC-202：测试策略、测试地图与证据政策

**Priority**：P0
**可并行**：可与 DOC-101–108 并行。
**Owned paths**：`docs/testing/**`

**交付**：`TEST-STRATEGY.md`、`TEST-MAP.md`、`EVIDENCE-POLICY.md`。

**验收**：L0–L7 都有测试 owner 和命令；明确 ignored、filtered、oracle、fixture、SOAK 和 performance 口径；缺证据标 Gap。

### DOC-203：当前能力矩阵

**Priority**：P0
**可并行**：可与 DOC-101–108 并行。
**Owned paths**：`docs/current/CAPABILITY-MATRIX.md`

**交付**：按 L0–L7 建立能力、状态、代码锚点、测试、commit、freshness 与证据映射。

**验收**：每层至少一项由独立 reviewer 复核；没有证据的能力标为 Partial 或 Planned；向 MAP-001 提交结构化映射输入。

## 6. 教学轨 Issue

### EDU-001：B 轨课程 DAG、模板与评分合同

**Priority**：P0
**依赖**：DOC-002。
**Owned paths**：`docs/labs/README.md`、`docs/labs/LAB-TEMPLATE.md`

**交付**：B00–B18 前置 DAG、难度、预计时间、必做/提高分类和固定模板。

**验收**：模板包含目标、前置红灯、任务、不变量、验收命令、禁止事项、口头题、提交证据和 AI 使用声明。

### EDU-101：B00–B04 工具链与 SQL 前端

**Owned paths**：`docs/labs/B00-*/**` 至 `docs/labs/B04-*/**`
**依赖**：EDU-001、DOC-101、DOC-102。
**内容**：环境、仓库地图、类型、Parser、Catalog/绑定。

**验收**：学生视角 dry-run；每项在自身 Lab 目录写入 `evidence/dry-run.md`，包含 commit、环境、逐条命令、exit code、实际/预期差异、agent/run/hash；绑定精确代码锚点和独立测试，不能依赖过期开发分支。

### EDU-102：B05–B08 查询规划与执行

**Owned paths**：`docs/labs/B05-*/**` 至 `docs/labs/B08-*/**`
**依赖**：EDU-001、DOC-103、DOC-104。
**内容**：Logical Plan、优化、基础算子、Join/Aggregate。

**验收**：每个 Lab 提交 `evidence/dry-run.md`；至少一个实验以具体测试命令验证“优化前后结果等价”，至少一个以具体测试命令验证组合算子边界。

### EDU-103：B09–B11 存储与索引

**Owned paths**：`docs/labs/B09-*/**` 至 `docs/labs/B11-*/**`
**依赖**：EDU-001、DOC-105。
**内容**：Tuple/Page、Disk/Buffer、B+ Tree。

**验收**：每个 Lab 提交 `evidence/dry-run.md`；实验用具名测试覆盖 round-trip、槽位稳定、脏页写回、pin 不淘汰、索引结构和范围语义。

### EDU-104：B12–B14 事务、恢复与协议

**Owned paths**：`docs/labs/B12-*/**` 至 `docs/labs/B14-*/**`
**依赖**：EDU-001、DOC-101、DOC-106。
**内容**：事务隔离、WAL/恢复、MySQL Session。

**验收**：每个 Lab 提交 `evidence/dry-run.md`；包含确定性并发命令、故障注入步骤和协议级命令；不得以单元测试代替 wire 行为声明。

### EDU-105：B15–B18 运维、测试、扩展与综合答辩

**Owned paths**：`docs/labs/B15-*/**` 至 `docs/labs/B18-*/**`
**依赖**：EDU-001、DOC-107、DOC-108、DOC-202。
**内容**：安全/观测、测试工程、扩展域、综合项目。

**验收**：每个 Lab 提交 `evidence/dry-run.md`；B17 可拆子轨；B18 要求提交架构解释、运行证据、失败复盘和能力边界，不以演示视频替代测试。

## 7. 门禁、迁移和审计 Issue

### MAP-001：CANONICAL-MAP 单写者集成

**Priority**：P0
**依赖**：DOC-101–108、DOC-201–203、EDU-101–105、全部 DOC-301 分片。
**Owned paths**：`docs/current/CANONICAL-MAP.md`

**工作内容**：收集各 Issue 的结构化映射输入，统一回填代码锚点、测试、状态、版本、freshness、evidence 和 owner；解决冲突并记录裁决。

**关闭条件**：CANONICAL-MAP 不存在重复 current owner；每行均能追溯到已合并 PR；独立 reviewer 抽查 L0–L7 和 B00–B18。

### GATE-001：CANONICAL-MAP 与代码锚点门禁脚本及自测

**Priority**：P0
**依赖**：DOC-002、DOC-202。
**Owned paths**：`scripts/gate/check_canonical_docs.sh`、`scripts/gate/check_doc_code_anchors.sh`、`tests/baseline/canonical_docs.json`、对应脚本自测。

**工作内容**：

- 校验 canonical 文件存在、唯一 owner、状态枚举和版本锚点。
- 校验 crate、文件、测试 target 和可可靠解析的代码符号。
- 为缺文件、重复 owner、缺符号、缺测试、stale version 注入故障并确认非零退出。
- GitNexus 不可用或索引过期时禁止静默给出 PASS。

**关闭条件**：脚本自测及本地实际门禁有独立实跑证据；本 Issue 不修改 CI workflow，也不声称 CI 已启用。

### DOC-301-*：旧文档迁移、重定向与归档分片

**Priority**：P1
**依赖**：DOC-001、DOC-003、DOC-101–108、EDU-101–105。
**Owned paths**：由 DOC-001 清单生成多个子 Issue；每个子 Issue 在认领前冻结互不重叠的精确路径，禁止使用整个 `docs/**` 通配所有权。共享入口和 CANONICAL-MAP 均不属于迁移分片，由 DOC-003 和后置 MAP-001 分别维护。

**工作内容**：按 Keep、Redirect、Historical、Archive 逐批处理重复教学目录和旧架构入口。

**关闭条件**：每个分片单独 PR；移动前后快速链接检查通过且不新增全量断链；Git 历史可追溯；没有删除发布证据；路径冻结表无重叠。

### GATE-002：文档门禁 CI 启用

**Priority**：P0
**依赖**：MAP-001、GATE-001、全部 DOC-301 分片。
**Owned paths**：`.gitea/workflows/ci.yml` 及创建远程 Issue 前解析并冻结的单一门禁入口文件；若入口不是 `ci.yml`，必须先更新 Issue 路径而不是运行时扩权。

**工作内容**：在迁移和 canonical 集成稳定后，将 GATE-001 脚本接入唯一 CI 入口；验证缺文件、重复 owner、缺符号、缺测试和 stale version 在 CI 中均 fail closed。

**关闭条件**：workflow 实际运行证据、P11–P16、故障注入与恢复后绿灯证据完整；不得仅凭 YAML 审查声称 CI 启用成功。

### DOC-999：同提交最终真实性审计

**Priority**：P0
**依赖**：除 Epic 外所有 Issue。
**Owned paths**：`docs/audit/documentation/architecture-education-final-audit.md`

**工作内容**：

- 在同一 commit 上复核 L0–L7、五条流程、B00–B18、测试地图和门禁。
- 重新运行链接、canonical、anchor、P11–P16 及计划中登记的验证命令。
- 抽样执行至少一个前端、执行、存储、事务、协议和扩展 Lab。
- 汇总未关闭 Gap，不把 Partial 写成 Verified。

**关闭条件**：报告绑定 commit、命令、exit code、计数、timestamp、source agent/run 和 evidence hash；Epic 仅在本 PR 合并后关闭。

## 8. 推荐并行批次

| Wave | 并行 Issue | 集成点 |
|---|---|---|
| W0 | DOC-001、DOC-002 | schema 与资产清单评审 |
| W1 | DOC-003、DOC-101–108、DOC-202、DOC-203、EDU-001 | current 入口和模块锚点冻结 |
| W2 | DOC-201、EDU-101–105 | 教程与 Lab 学生视角复核 |
| W3 | GATE-001、DOC-301 各路径分片 | 门禁自测和互斥迁移 |
| W4 | MAP-001 | 迁移后单写者最终集成 |
| W5 | GATE-002 | CI 启用及故障注入验证 |
| W6 | DOC-999 | 同提交最终审计 |

理论最大并行度不是执行目标。实际调度应限制同时修改公共索引的 Agent 数量为 1，并优先保证每个模块有独立 reviewer。

## 9. 远程 Issue 创建顺序

1. 查询现有 Gitea Issue，避免重复创建。
2. 先创建全部子 Issue，记录真实编号。
3. 创建 Epic 并链接子 Issue。
4. 回填“逻辑 ID → Gitea 编号”映射。
5. 每个评论使用 `[agent: <id>]` 前缀。
6. 本文件本身不授权自动创建、认领或关闭远程 Issue；执行创建时需按用户明确要求操作。

# SQLRustGo Architecture and Education Documentation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 建立“八层工程架构 + B00–B18 教学轨 + CANONICAL-MAP 唯一入口”的 SQLRustGo 文档体系，使设计、实现、测试、教程和练习能够从同一代码与证据源逐层追溯。

**Architecture:** 文档采用“双入口、单事实源”：工程人员从八层架构与模块页进入，学习者从教程与 B 轨进入；两条路径共同引用 `docs/current/CANONICAL-MAP.md` 中登记的代码符号、测试和版本锚点。发布文档继续保留为历史证据，但不再作为当前架构与教学状态的权威入口。

**Tech Stack:** Markdown、Mermaid、Rust/Cargo、Bash、GitNexus、文档链接检查脚本、Gitea Issue/PR。

---

## 1. 计划边界与基线

### 1.1 本计划交付什么

1. 当前文档唯一入口与文档生命周期规则。
2. 八层工程架构总览和八个模块包。
3. 可复现的端到端执行流教程。
4. 不限定为十讲的 B00–B18 教学实验轨。
5. 设计、代码符号、测试、证据之间的映射。
6. 支持多个 AI Agent 并行认领的 Issue DAG。
7. 防止链接、路径、符号、测试目标和版本口径漂移的文档门禁。

本计划不声称任何待建文档、练习或门禁已经完成，也不改变当前发布阶段。

### 1.2 取证基线

| 项目 | 基线值 | 证据 |
|---|---:|---|
| SQLRustGo 基线提交 | `345ee9539cc98adc92e2a4cd27c8c4b1aa6cdebf` | `git rev-parse HEAD` |
| 当前分支 | `feature/docs-architecture-education-plan` | `git branch --show-current` |
| `docs/**/*.md` | 2206 | 基线提交中的 tracked snapshot，不含本计划新增文件；`find docs -type f -name '*.md' \| wc -l` |
| `docs/releases/**/*.md` | 1555 | `find docs/releases -type f -name '*.md' \| wc -l` |
| 一级 crate 目录 | 43 | `find crates -mindepth 1 -maxdepth 1 -type d \| wc -l` |
| 文档链接基线 | exit 0 | `bash scripts/gate/check_docs_links.sh`，2026-10-09 实跑 |
| 全量文档链接基线 | exit 1，14 个既有断链 | `bash scripts/gate/check_docs_links.sh --all`，2026-10-09 实跑；由独立债务清单管理 |

这些数字只描述上述提交，不得复制为未来版本的当前状态。

### 1.3 已确认问题

| ID | 问题 | 当前证据 | 处理原则 |
|---|---|---|---|
| P-01 | 当前入口漂移 | `docs/README.md` 仍以 v3.10.0 为当前版本 | 新建 current SSOT，再将旧入口降为兼容跳转页 |
| P-02 | 架构页早于当前 workspace | `docs/architecture.md` 未覆盖 planner、optimizer、catalog、server 与扩展域 | 以八层模型重建，不在旧页继续堆叠 |
| P-03 | 教学目录分叉 | `docs/tutorials/教学实践` 与 `docs/教学实践` 等目录并存 | 先建立清单和 canonical owner，再迁移或归档 |
| P-04 | 发布报告压过设计文档 | release Markdown 占当前 docs Markdown 的多数 | 历史发布材料保留，但移出 current 导航主线 |
| P-05 | 设计与测试未系统绑定 | 多数模块说明没有符号、测试和 freshness 字段 | 模块模板强制包含实现导航与测试矩阵 |
| P-06 | 教学只有零散 Lab | `docs/teaching-labs/` 尚无完整实现轨 | 建设 B00–B18，并使用独立教育适配层 |

## 2. 目标信息架构

```text
docs/
├── current/
│   ├── README.md
│   ├── CANONICAL-MAP.md
│   ├── ARCHITECTURE.md
│   ├── EXECUTION-FLOWS.md
│   ├── CAPABILITY-MATRIX.md
│   └── DOCUMENT-LIFECYCLE.md
├── modules/
│   ├── 00-system-entry/
│   ├── 01-sql-frontend/
│   ├── 02-planner-optimizer/
│   ├── 03-executor/
│   ├── 04-storage/
│   ├── 05-transaction-recovery/
│   ├── 06-server-operations/
│   └── 07-extensions/
├── tutorials/
│   ├── README.md
│   └── flows/
├── labs/
│   ├── README.md
│   ├── LAB-TEMPLATE.md
│   └── B00 ... B18
├── testing/
│   ├── TEST-STRATEGY.md
│   ├── TEST-MAP.md
│   └── EVIDENCE-POLICY.md
└── archive/
```

### 2.1 八层工程架构

| 层 | 名称 | 代表 crate/入口 | 说明重点 |
|---|---|---|---|
| L0 | 系统入口 | `sqlrustgo-cli`、`mysql-server`、`server`、`network` | 进程、连接、Session、协议边界 |
| L1 | SQL 前端 | `types`、`parser`、`catalog` | 类型、AST、名称解析、Schema |
| L2 | 查询规划 | `planner`、`optimizer` | Logical/Physical Plan、规则与代价 |
| L3 | 执行引擎 | `executor`、根 `ExecutionEngine` | 算子、表达式、DML/DDL、执行上下文 |
| L4 | 存储系统 | `storage`、`cache`、`spill` | Tuple/Page、Buffer、索引、持久化 |
| L5 | 事务恢复 | `transaction`、`wal-verification` | 并发控制、WAL、恢复、不变量 |
| L6 | 服务与治理 | `security`、`telemetry`、`admin`、`information-schema` | 安全、观测、管理、运行边界 |
| L7 | 扩展能力 | `distributed`、`vector`、`graph`、`gis`、`rag` | 扩展接口、数据模型、非核心依赖 |

模块作者必须先验证真实依赖和执行流；GitNexus 索引过期或查询不完整时，文档须声明边界并以当前源码补证。

### 2.2 模块页固定结构

每个 `docs/modules/<layer>/README.md` 必须依次包含：

1. 定位、读者与非目标。
2. 上下游边界和禁止依赖。
3. 核心数据结构。
4. 至少一条真实执行流。
5. 关键不变量。
6. 实现导航（crate、文件、符号）。
7. 错误、失败和恢复路径。
8. 并发、持久化或资源语义（适用时）。
9. 测试矩阵和精确命令。
10. 对应教程和 B 轨实验。
11. 已知限制及非能力声明。
12. provenance、commit、freshness 和 evidence hash。

## 3. B00–B18 教学轨

课程不按固定讲数裁剪，以知识依赖和可验证能力为准。

| Lab | 主题 | 核心产出 | 主要架构层 |
|---|---|---|---|
| B00 | 环境、构建与第一条 SQL | 可复现开发环境和命令证据 | L0 |
| B01 | 仓库地图与 SQL 全链路 | 从入口追踪一条 SELECT | L0–L5 |
| B02 | Value、Schema 与三值逻辑 | 类型与 NULL 不变量 | L1 |
| B03 | Lexer、Parser 与 AST | 语法扩展及错误定位 | L1 |
| B04 | Catalog、名称绑定与类型检查 | 将名称解析为稳定内部引用 | L1 |
| B05 | Logical Plan | AST 到计划树 | L2 |
| B06 | 规则与代价优化 | 等价重写和计划对比 | L2 |
| B07 | 基础执行器 | SeqScan、Filter、Projection | L3 |
| B08 | Join、Aggregate 与排序 | 多算子组合及边界条件 | L3 |
| B09 | Tuple、Page 与编码 | round-trip、槽位稳定性 | L4 |
| B10 | Disk 与 Buffer Pool | pin、dirty、flush、evict | L4 |
| B11 | B+ Tree 与索引扫描 | 结构不变量和范围查询 | L4 |
| B12 | 事务状态与隔离 | 并发异常和事务生命周期 | L5 |
| B13 | WAL 与崩溃恢复 | write-ahead、redo/undo、幂等性 | L5 |
| B14 | MySQL 协议、连接与 Session | 线协议到执行上下文 | L0/L6 |
| B15 | 安全、可观测性与管理面 | 权限、日志、指标、管理接口 | L6 |
| B16 | 测试工程 | 单元、集成、差分、模糊与门禁 | 全层 |
| B17 | 向量、图、GIS、RAG 与分布式扩展 | 理解扩展边界，不强求全部实现 | L7 |
| B18 | 综合项目与答辩 | 全链路演示、故障分析、证据包 | 全层 |

### 3.1 教学实现约束

- 不在生产主路径植入教学用 `todo!()`。
- 不把 gate-referenced test 标记为 `#[ignore]`。
- 优先通过 `crates/sqlrustgo-edu/`、独立 starter 分支或补丁包提供练习接缝。
- 公开测试、教师测试和 gate test 必须标明归属，不能混用统计口径。
- 每个实验必须有红灯、绿灯、关键不变量、验收命令、口头解释题和证据清单。
- B17 属于拓展轨，可按主题拆为 B17-V、B17-G、B17-GIS、B17-D；编号不是课程讲数上限。

## 4. CANONICAL-MAP 合同

`docs/current/CANONICAL-MAP.md` 每一行至少包含：

| 字段 | 含义 |
|---|---|
| Unit | 架构层、教程或 Lab ID |
| Canonical document | 当前唯一权威文档 |
| Code anchors | crate、文件和符号 |
| Tests | 测试目标和精确命令 |
| Capability status | Verified、Partial、Planned、Historical |
| Version anchor | commit 或 tag |
| Freshness | 生成/复核时间 |
| Evidence | 原始命令输出或持久 artifact |
| Owner | 维护 Issue 或责任域 |

同主题旧文档只有三种处理方式：重定向到 canonical 文档、标记 Historical、移动到 archive。禁止保留两个未声明关系的 current 文档。

## 5. 多 Agent 协作模型

详细 Issue 内容见 [`2026-10-09-architecture-education-issue-catalog.md`](2026-10-09-architecture-education-issue-catalog.md)。并行执行必须遵守以下规则：

1. 每个 Issue 使用独立 worktree 和独立分支。
2. 每个 Agent 只修改 Issue 的 owned paths；公共索引由集成 Issue 统一修改。
3. Claim 必须携带 `source_agent`、`source_run`、`timestamp`、`evidence_hash`、`conflict_resolution`。
4. Agent 不得引用另一个 Agent 的“完成”声明代替本地验证。
5. 每个 Issue 由独立 PR 合并，禁止手动关闭无 PR 的 Issue。
6. 发生路径或事实冲突时，代码与同提交实跑证据优先；冲突记录写入 Issue。

## 6. 实施任务

### Task 1: 建立 current SSOT 与文档生命周期

**Files:**
- Create: `docs/current/README.md`
- Create: `docs/current/CANONICAL-MAP.md`
- Create: `docs/current/DOCUMENT-LIFECYCLE.md`
- Modify: `docs/README.md`

**Step 1:** 编写失败清单，列出旧索引无法定位当前版本、八层架构和教学轨的证据。

**Step 2:** 创建最小 CANONICAL-MAP schema，先登记 L0–L7、B00–B18，未验证项标为 `Planned`。

**Step 3:** 定义 Current、Historical、Release Evidence、Teaching 四类文档及迁移规则。

**Step 4:** 将 `docs/README.md` 改为稳定入口页，只引用 current、release history、governance 和 archive。

**Step 5:** 运行 `bash scripts/gate/check_docs_links.sh`，并对比 DOC-001 的全量断链基线确认本 PR 未新增断链；只有 14 个既有旧债全部修复后，才要求 `bash scripts/gate/check_docs_links.sh --all` exit 0。

**Step 6:** 提交：`docs: establish canonical documentation entrypoint`。

### Task 2: 建立架构总览和八层模块骨架

**Files:**
- Create: `docs/current/ARCHITECTURE.md`
- Create: `docs/modules/00-system-entry/README.md`
- Create: `docs/modules/01-sql-frontend/README.md`
- Create: `docs/modules/02-planner-optimizer/README.md`
- Create: `docs/modules/03-executor/README.md`
- Create: `docs/modules/04-storage/README.md`
- Create: `docs/modules/05-transaction-recovery/README.md`
- Create: `docs/modules/06-server-operations/README.md`
- Create: `docs/modules/07-extensions/README.md`

**Step 1:** 用当前 Git HEAD、Cargo workspace 和 GitNexus context 生成候选映射。

**Step 2:** 对每层抽查入口符号、调用边界和测试路径；过期索引不得作为唯一证据。

**Step 3:** 按 DOC-101–DOC-108 八个独立 Issue 分别填写模块固定结构，公共文件只由 MAP-001 集成 Agent 修改。

**Step 4:** 独立复核跨层依赖、重复事实和能力越界声明。

**Step 5:** 运行模块中登记的窄测试，以及文档链接检查。

**Step 6:** 每个模块单独 PR；合并后由 MAP-001 单写者 Issue 更新 CANONICAL-MAP。

### Task 3: 建立执行流教程

**Files:**
- Create: `docs/current/EXECUTION-FLOWS.md`
- Create: `docs/tutorials/README.md`
- Create: `docs/tutorials/flows/select-query.md`
- Create: `docs/tutorials/flows/write-transaction.md`
- Create: `docs/tutorials/flows/crash-recovery.md`
- Create: `docs/tutorials/flows/mysql-session.md`
- Create: `docs/tutorials/flows/extension-query.md`

**Step 1:** 为每条流记录入口、关键 hop、数据形态、失败出口和测试。

**Step 2:** 使用 GitNexus trace/context 加源码核验每个 hop。

**Step 3:** 为流程添加可复制命令和预期观察点，不复制模块设计正文。

**Step 4:** 运行对应窄测试；记录实际 exit code、测试计数和 commit。

**Step 5:** 向 MAP-001 提交结构化映射输入，不直接修改 CANONICAL-MAP，并运行链接检查。

### Task 4: 建立测试和证据体系

**Files:**
- Create: `docs/testing/TEST-STRATEGY.md`
- Create: `docs/testing/TEST-MAP.md`
- Create: `docs/testing/EVIDENCE-POLICY.md`

**Step 1:** 区分 unit、contract、integration、wire/E2E、differential、fuzz、SOAK、performance 和 release gate。

**Step 2:** 为 L0–L7 登记测试 owner、命令、fixture、oracle、ignored/filtered 口径。

**Step 3:** 定义机器可读证据 schema 和 freshness 规则。

**Step 4:** 校验所有测试 target 存在；缺失项标为 Gap，不创建虚假 PASS。

**Step 5:** 提交：`docs: map architecture layers to test evidence`。

### Task 4A: 建立当前能力矩阵

**Files:**
- Create: `docs/current/CAPABILITY-MATRIX.md`

**Step 1:** 定义 Verified、Partial、Planned、Historical 四种状态及证据要求。

**Step 2:** 按 L0–L7 登记能力、代码锚点、测试、commit 和 freshness；缺证据的能力只能标记 Partial 或 Planned。

**Step 3:** 由独立 reviewer 抽查至少每层一项能力，并把映射输入交给 MAP-001。

### Task 5: 建立 B 轨框架和 B00–B18

**Files:**
- Create: `docs/labs/README.md`
- Create: `docs/labs/LAB-TEMPLATE.md`
- Create: `docs/labs/B00-*/README.md` through `docs/labs/B18-*/README.md`
- Optional create: `crates/sqlrustgo-edu/`（须另开实现 Issue）

**Step 1:** 先确定每个 Lab 的前置关系、预计工作量和评分维度。

**Step 2:** B00–B18 分成五个互不重叠的文档包并行编写。

**Step 3:** 每个 Lab 绑定模块页、代码锚点、测试和证据模板。

**Step 4:** 由未参与编写的 Agent 执行学生视角 dry-run；无法复现的步骤不得标记 Ready。

**Step 5:** 需要代码接缝的 Lab 只创建后续实现 Issue，不在文档 PR 中修改生产逻辑。

**Step 6:** 各 EDU Issue 提交结构化映射输入，由 MAP-001 单写者更新课程 DAG 和 CANONICAL-MAP。

### Task 6: 增加文档防漂移门禁

**Files:**
- Create: `scripts/gate/check_canonical_docs.sh`
- Create: `scripts/gate/check_doc_code_anchors.sh`
- Create: `tests/baseline/canonical_docs.json`
- Modify: applicable Gitea workflow only after script self-tests pass

**Step 1:** 先写脚本自测，覆盖缺文件、重复 current owner、缺符号、缺测试目标和 stale version 五类失败。

**Step 2:** 运行自测并确认每个故障注入都以非零退出。

**Step 3:** 实现最小脚本，使测试转绿。

**Step 4:** 运行 P11–P16 及新门禁；任一失败则不声称门禁完成。

**Step 5:** GitNexus 只用于增强符号检查；索引不可用时必须 fail closed 或明确进入人工复核，不能静默 PASS。

**Step 6:** 门禁脚本和自测由 GATE-001 交付；只有迁移及 MAP-001 集成完成后，GATE-002 才能把门禁接入 CI。

### Task 7: 迁移、归档与最终审计

**Files:**
- Move or annotate: DOC-301 分片中冻结的、互不重叠的 duplicate teaching/current paths
- Exclude: `docs/README.md`（DOC-003 owner）
- Exclude: `docs/current/CANONICAL-MAP.md`（MAP-001 owner）

**Step 1:** 为每份候选文档决定 Keep、Redirect、Historical 或 Archive。

**Step 2:** 逐批迁移，每批单独 diff 和链接验证，禁止一次性大规模无审查移动。

**Step 3:** 所有迁移分片合并后，由 MAP-001 统一回填新路径，再执行 canonical、anchor 和适用 governance gate。

**Step 4:** 独立 Agent 对照当前 commit 复核 L0–L7、B00–B18 和测试映射。

**Step 5:** 生成最终审计报告；只有合并 PR 与实跑证据齐全后才能关闭 Epic。

## 7. 验收标准

- `docs/current/CANONICAL-MAP.md` 是所有当前设计、教程、Lab 和测试的唯一地图。
- L0–L7 各有一份符合固定模板的权威模块页。
- 至少五条端到端执行流可按文档复现。
- B00–B18 均有目标、前置、任务、不变量、测试、禁止事项和证据清单。
- 任一 current Claim 可追溯到同 commit 的代码或执行证据。
- 历史 release 文档不再被当前入口误标为现行事实。
- canonical 与 code-anchor 门禁能够对故障注入 fail closed，且由独立启用 Issue 接入 CI。
- 所有实施子 Issue 通过合并 PR 关闭；Epic 作为追踪 Issue，以 DOC-999 最终审计 PR 和所有子 Issue 的合并状态关闭。

## 8. 本计划自身验证

```bash
bash scripts/gate/check_docs_links.sh
git diff --check
```

预期：上述两个命令 exit 0，且新增文件不出现在 `--all` 的断链输出中。当前基线的 `--all` 已知为 exit 1、14 个既有断链；只有 DOC-301 旧债清零后才把 `--all` exit 0 作为最终要求。

## 9. Provenance

```yaml
provenance:
  generated_by: codex
  source_agent: codex-root
  source_run: interactive-2026-10-09-architecture-education-plan
  generated_at: 2026-10-09T18:20:59+08:00
  input_refs:
    - type: commit
      value: 345ee9539cc98adc92e2a4cd27c8c4b1aa6cdebf
    - type: repository
      value: /Users/liying/dev/sqlrustgo
    - type: reference_commit
      value: /Users/liying/dev/BustubX-EDU@aa36c0bb3
    - type: reference_file
      value: /Users/liying/dev/BustubX-EDU/docs/course/CANONICAL-MAP.md
    - type: reference_file
      value: /Users/liying/dev/BustubX-EDU/docs/course/b-track-rust-starter-plan.md
  evidence_hashes:
    sqlrustgo_agents: sha256:4687416a99c253b078a60da5243a8b92e52c4efae2904dbcdc4019eedcf5c68d
    sqlrustgo_docs_index: sha256:2fc51d70c3b64b6cfb6fa40ce4964c7f358f9d28871c59a0a37a8a317a6eaff8
    sqlrustgo_architecture: sha256:018679cb80657288f7ce36fa88543591bc312c89254edfac69e8bdc1d98ff84f
    bustubx_canonical_map: sha256:198d171edf7b086b9c956da9b4a4eba58f4dbd178b16fea9f8e2e83c6b32de8b
  conflict_resolution: current SQLRustGo source and same-commit execution evidence override stale indexes and historical documents
```

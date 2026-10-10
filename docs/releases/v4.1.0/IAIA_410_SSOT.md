# IAIA-410 正式审计 SSOT

> **文档性质**：正式审计协议与执行治理 SSOT，不是审计结果或阶段门禁报告。
> **适用版本**：SQLRustGo v4.1.0
> **Master Issue**：[#5235](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5235)
> **制定日期**：2026-10-10
> **目标分支**：`develop/v4.1.0`
> **authoring_source_commit**：`f1b77bf32cd38c95105de3c6191d75b8bb3687f0`
> **source_agent**：codex
> **source_run**：`iaia-410-formalization-20261010`
> **timestamp**：`2026-10-10T00:27:00+08:00`
> **evidence_ref**：`docs/plans/2026-10-10-iaia-410-formalization-design.md` 的创建时内容快照，由下列 SHA-256 寻址
> **evidence_hash**：`sha256:97490c7ed1c08c2ba46a193860a71c1d363ec5af2474c884e6dd82879b676dc7`
> **conflict_resolution**：none
> **evidence_status**：protocol-only；本文不得作为任何审计 PASS/FAIL 的运行证据

`authoring_source_commit` 只是本文编写时的来源版本，不是 IAIA-410 正式被审计版本；
正式 `assessed_commit` 只能由 A01 的 `manifest.yaml` 冻结。

---

## 0. 权威边界

IAIA-410 使用分层 SSOT，避免一份 Markdown 同时承担计划、运行结果和实时状态：

| 信息 | 唯一权威来源 |
|---|---|
| 审计范围、方法、证据契约、判定规则 | 本文 |
| 工作包、依赖、负责人、open/closed 状态 | Gitea [Master #5235](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5235) 及子 Issue |
| 某次运行的 commit、环境、命令、退出码、日志哈希 | `audit/v4.1.0/<run_id>/manifest.yaml` |
| 单条发现及其证据与复核 | `audit/v4.1.0/<run_id>/findings/<finding_id>.yaml` |
| 修复状态 | finding 关联的 defect Issue、PR 与 regression run |
| v4.1.0 发布阶段 | [`STAGE.yaml`](STAGE.yaml) |
| Alpha → Beta 晋级条件 | [`ALPHA_TO_BETA_GATE_PLAN.md`](ALPHA_TO_BETA_GATE_PLAN.md) |

冲突时按上表选择权威来源。本文不复制 Gitea 实时状态，也不把历史报告中的 PASS 当成当前实跑证据。

原 [`IAIA_410_AUDIT_PLAN.md`](IAIA_410_AUDIT_PLAN.md) 是 2026-10-09 形成的计划、阶段记录和方法论材料；它不是本协议生效后的实时状态 SSOT。

---

## 1. 审计目标

IAIA-410 要回答：

> 从真实生产入口发起一项对外声明的数据库能力时，能否证明请求经过声明所需的核心模块，得到正确且可观测的结果，并在并发、失败、重启和版本变化下保持承诺语义？

审计不是代码量、模块数量、测试数量或编译成功的汇总。上述信号只能提供线索，不能独立证明能力真实存在。

### 1.1 成功条件

正式审计完成必须同时满足：

1. 对外 claim 与生产入口、必经模块、可观测语义和审计 case 完整映射；
2. 每个 in-scope claim 有冻结运行上的 verdict；
3. P0/P1 和一票否决 finding 经 Independent Judge 独立复核；
4. 所有 verdict 可从 manifest、原始日志和 evidence hash 重放；
5. 未验证项保持 `unverified` 或 `blocked`，不得通过文字解释变成 PASS；
6. 最终报告从证据记录派生，不手工维护运行数字或 Issue 状态。

### 1.2 非目标

- 不以代码风格或命名偏好作为真实性 finding；
- 不把明确公开的教学限制自动判为缺陷；
- 不在审计阶段顺手修改被测实现；
- 不让 GitNexus、AOCI、覆盖率或单元测试独立充当正确性 oracle；
- 不取代 `STAGE.yaml` 或既有发布 Hard Gate。

---

## 2. 基本术语和状态

### 2.1 Claim

Claim 是项目对外或对内用于发布判断的能力声明。来源包括用户文档、架构文档、CLI help、release plan、测试名称、模块契约和阶段清单。代码或 crate 的存在本身不是 claim 完成证据。

### 2.2 工作包状态

| 状态 | 含义 |
|---|---|
| `not_started` | 尚无冻结运行证据 |
| `running` | 已建立 run，执行未结束 |
| `evidence_ready` | 原始证据完整，等待独立复核 |
| `independently_verified` | Judge 已复核所需样本和关键 finding |
| `blocked` | 缺少外部条件或前置依赖，原因已记录 |

### 2.3 Finding 状态

| 状态 | 含义 |
|---|---|
| `suspected` | 静态或单一信号发现风险，尚未满足定性证据 |
| `confirmed` | 静态、动态和可复现证据支持缺陷成立 |
| `refuted` | 固定环境下完成充分反证并经独立复核 |
| `unverified` | 证据不足、环境不一致或无法可靠复现 |
| `remediated` | 修复 PR 已合并，但尚未完成独立回归 |
| `regression_verified` | 在修复后的冻结 commit 上完成独立回归 |

`PR merged` 只能支持 `remediated`，不能自动产生 `regression_verified`。

### 2.4 缺陷分类

沿用 F1–F10：空实现、伪实现、假集成、假优化、静默语义退化、测试伪通过、降级伪装、版本退化、持久性假象、观测误导。每个 finding 必须给出分类理由，不允许仅凭关键词扫描定性。

---

## 3. 审计范围模型

具体 capability inventory 由 [A02 #5224](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5224) 在冻结 commit 上生成。每项只能使用以下范围状态：

| 状态 | 要求 |
|---|---|
| `in_scope` | 有 production entry、可观察语义和执行工作包 |
| `out_of_scope` | 明确说明为什么不属于 v4.1.0 IAIA，并引用权威依据 |
| `deferred` | 有 owner、目标版本、独立 Issue 和重新纳入条件 |

### 3.1 必须盘点的生产入口

- MySQL Server 非 TLS 与 TLS；
- 真实 MySQL 兼容客户端和 prepared statement；
- REPL；
- CLI/admin 工具；
- 启动、恢复和关闭路径；
- 后台维护、checkpoint、backup/restore 等被声明入口。

### 3.2 必须决策范围的能力域

| 能力域 | 最低检查内容 |
|---|---|
| SQL 语义 | parser、binder/名称解析、执行、列元数据、错误语义 |
| DDL/DML | 表、索引、约束、重复键、UPDATE/DELETE |
| 优化与计划 | 声明的规则是否生产可达，计划是否可观察 |
| 索引与存储 | PK、二级索引、范围、Buffer Pool、FileStorage |
| 事务与 WAL | begin/commit/rollback、隔离、失败传播、恢复 |
| 协议 | 握手、选库、错误包、列类型、TLS 对等性 |
| 安全与隔离 | 认证、权限、跨库、跨连接、多实例、敏感日志 |
| 测试系统 | ignored、disabled binary、mutation、生产入口覆盖 |
| 跨版本 | 正确性、兼容性、性能、资源和首次退化范围 |
| 运维与持久化 | data-dir、崩溃恢复、备份恢复、格式兼容、资源上限 |

范围矩阵不得因为能力当前失败而删除该能力；失败应产生 finding。

---

## 4. Issue DAG 与职责

### 4.1 工作包

| ID | Issue | 产物 |
|---|---|---|
| A01 | [#5223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5223) | 冻结基线、manifest、重放契约 |
| A02 | [#5224](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5224) | claim/entry/scope 矩阵 |
| A03 | [#5225](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5225) | 生产调用链和 suspected findings |
| A04 | [#5226](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5226) | SQL 与协议差分矩阵 |
| A05 | [#5227](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5227) | 索引、Buffer Pool、存储证据 |
| A06 | [#5228](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5228) | 事务、WAL、checkpoint、backup/restore、格式兼容、恢复与故障注入证据 |
| A07 | [#5229](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5229) | 测试反审计、mutation REVIEW |
| A08 | [#5230](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5230) | disabled binaries 全量复核 |
| A09 | [#5231](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5231) | 跨版本矩阵 |
| A10 | [#5232](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5232) | 认证、协议、会话和隔离审计 |
| A11 | [#5233](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5233) | 独立复核和冲突裁决 |
| A12 | [#5234](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5234) | 最终报告、治理审批和阶段衔接 |

### 4.2 依赖

```text
A01 ──> A02
 │       ├──> A03 ──> A05 ──┐
 │       ├──> A04 ──────────┤
 │       ├──> A06 ──────────┼──> A09
 │       ├──> A07            │
 │       ├──> A08            │
 │       └──> A10            │
 └───────────────────────────┘

A01-A10 evidence_ready（保持 open） ──> A11 verdict
A11 verdict ──> 按关闭验证逐项关闭 A01-A10 ──> A12 ──> Master closure
```

A03、A04、A06、A07、A08、A10 可以在 A01/A02 冻结后并行；A05 等待 A03，
A09 等待 A04/A05/A06 的稳定 corpus 和证据。修改同一文件或共享测试环境时仍须按
ADR-014 协调。

### 4.3 角色隔离

| 角色 | 权限与限制 |
|---|---|
| Audit Owner | 管理 Master、冻结协议和运行边界，不得单独批准自己的结论 |
| Auditor | 执行指定工作包，提交原始证据和 provisional verdict |
| Remediator | 在独立 Issue/PR 修复 confirmed finding，不改写原始 run |
| Independent Judge | 未参与该 finding 的发现和修复，负责重放与 verdict review |
| Human Architect | 审批范围、治理标准和冲突最终裁决 |
| CI/Hard Gate | 机器执行，输出运行事实；AI 只能解释，不能伪造或替代 |

---

## 5. 冻结运行协议

每次运行使用：

```text
run_id = iaia410-<work-package>-<UTC timestamp>-<short commit>
```

运行目录至少包含：

```text
audit/v4.1.0/<run_id>/
├── manifest.yaml
├── commands/
│   ├── 001.stdout.log
│   ├── 001.stderr.log
│   └── 001.meta.yaml
├── datasets/
├── findings/
├── traces/
└── SHA256SUMS
```

### 5.1 Manifest 最低 schema

```yaml
schema_version: iaia410-manifest/v1
run_id: iaia410-a04-20261010T000000Z-deadbeef
work_package: A04

source_agent: <agent-id>
source_run: <agent-run-id>
timestamp_started: <ISO-8601>
timestamp_finished: <ISO-8601-or-null>
conflict_resolution: none

repository:
  canonical_remote: <remote-url>
  commit: <40-char-sha>
  branch_or_detached: <value>
  status_porcelain_sha256: <sha256>
  cargo_lock_sha256: <sha256>

environment:
  os: <value>
  arch: <value>
  cpu: <value>
  memory_bytes: <value>
  filesystem: <value>
  rustc: <value>
  cargo: <value>
  features: [<feature>]
  profile: <debug-or-release>
  reference_dbms: [{name: <name>, version: <version>}]
  process_pollution_check: <evidence-path>

datasets:
  - id: <dataset-id>
    generator: <command-or-file>
    seed: <value>
    sha256: <sha256>

commands:
  - id: 001
    argv: [<argv-elements>]
    cwd: <repository-relative-path>
    started_at: <ISO-8601>
    finished_at: <ISO-8601>
    exit_code: <integer>
    stdout: commands/001.stdout.log
    stderr: commands/001.stderr.log
    stdout_sha256: <sha256>
    stderr_sha256: <sha256>

manifest_sha256: <sha256-over-canonical-form>
```

禁止在 argv 中保存 token、密码、私钥或明文凭据。需要凭据的运行只记录变量名和凭据来源类别。

### 5.2 环境污染检查

运行前后至少记录：

- SQLRustGo 相关进程和监听端口；
- data-dir 是否新建、是否为空以及清理结果；
- 临时目录和文件所有权；
- 可影响行为的环境变量名；
- CPU/内存压力和并发任务；
- reference DB 数据库是否为同一 dataset hash。

污染无法排除时，本次性能或稳定性结论为 `unverified`。

---

## 6. Finding 契约

每条 finding 必须是独立文件，不能只存在于报告正文或 Issue 评论。

```yaml
schema_version: iaia410-finding/v1
id: AUTH-V410-001
run_id: <run-id>
work_package: A06
module: transaction/wal
category: F9-durability-illusion
severity: P0
status: suspected

source_agent: <agent-id>
source_run: <agent-run-id>
timestamp: <ISO-8601>
evidence_hash: <sha256>
conflict_resolution: none

claim:
  text: <被验证的能力声明>
  source_type: <SSOT|user-doc|CLI|architecture|test-contract>
  source_ref: <path-or-url>
  freshness: <date-or-version>

production_entry: <entrypoint>
assessed_commit: <40-char-sha>
source_locations: [<file:line>]

expected: <expected-observable-behavior>
actual: <actual-observed-behavior>
evidence:
  static: [<path>]
  dynamic: [<path>]
  differential: [<path>]
  reproduction_command_ids: [001]
  counterexample_attempts: [<path>]

linked_issue: null
linked_prs: []
review:
  reviewer: null
  source_run: null
  timestamp: null
  evidence_hash: null
  verdict: null
```

### 6.1 定性阈值

- `confirmed`：至少包含生产入口动态证据、可复现命令和支持根因或影响的静态证据；涉及语义时应有独立 oracle。
- `refuted`：记录原复现条件、反证矩阵、为什么先前证据不成立以及 Judge verdict。
- `unverified`：列出缺失证据和下一步，不得使用类似“基本正常”的模糊措辞。
- 性能 finding：至少三档规模、重复测量、分位数、噪声界限和正确性前置条件。
- 持久性 finding：明确进程崩溃与断电持久性的边界，不互相替代。

---

## 7. 六类审计工作流

### 7.1 静态生产路径审计

从生产入口反向建立调用图，区分 production、test-only、feature-gated、dead 和 fallback 路径。GitNexus/AOCI 可用于定位，但索引版本、commit 和 freshness 必须记录；图谱结论要由源码或运行时 trace 交叉验证。

### 7.2 动态与差分审计

使用真实客户端穿过 wire protocol，比较：

- 列名、列类型、行数和逐值；
- NULL 位置和三值逻辑；
- 有 ORDER BY 时的顺序；无 ORDER BY 时使用集合或多重集语义；
- 错误类别、错误码和是否有副作用；
- reference DBMS 不可比时的差异说明。

### 7.3 模块破坏与故障注入

在隔离测试构建中使 WAL append、页读、索引读、事务提交等返回受控错误。探针必须覆盖声明所需的合法实现；否则“未受影响”只能生成 suspected finding。

A06 必须为 checkpoint、backup/restore、磁盘格式兼容与升级/降级分别定义动态 case、
独立 oracle 和重启后校验。若 capability inventory 证明某项未被 v4.1.0 声明，则 A02
只能将其标为 `out_of_scope` 或带独立 Issue 的 `deferred`，不得让该项无人负责。

### 7.4 测试系统反向审计

核实测试是否经过生产入口、真实存储、真实事务和真实协议；检查 ignored、disabled、只断言 `is_ok()`、模拟组件和 stale binary。mutation surviving/review 项逐条进入 A07。

### 7.5 跨版本审计

在同一环境、同一 dataset hash、等价配置和同一客户端下运行 v3.12、v4.0、v4.1。正确性不成立的结果不得进入性能排名；首次退化只定位到证据支持的版本或 commit 区间。

### 7.6 安全与隔离审计

覆盖 TLS/非 TLS、认证、权限拒绝、握手选库、会话隔离、跨库、多实例、错误包、资源限制和敏感日志。安全结论缺少实跑或权威扫描证据时必须为 `unverified`。

---

## 8. 判定与发布关系

### 8.1 一票否决候选

以下 finding 经 Independent Judge `confirmed` 后，IAIA-410 必须向阶段 SSOT 提交阻断变更提案：

1. 声称支持的核心事务语义无法实现；
2. COMMIT 成功后发生不可接受的数据丢失；
3. 常用或复杂 SQL 静默返回错误结果；
4. 生产链路绕过必要权限、事务或持久化机制；
5. CI/gate 失败被隐藏，或报告声称通过但证据不可复现。

本文不能直接修改 `STAGE.yaml` 的状态。阻断关系只有在相应 PR 合并后才成为阶段治理事实。

### 8.2 真实性评分

100 分制属于拟议治理模型。在 A12 完成 governance 审批前，它只能标记为 `[SOFT GATE]`，不得据此宣布 Alpha/Beta/RC/GA PASS。

### 8.3 Hard Gate 与审计 verdict

- Hard Gate 的 PASS/FAIL 只能来自实际 gate 输出；
- IAIA finding 可以说明 gate 是否缺少真实性覆盖，但不能伪造 gate 状态；
- gate 全绿也不能自动反证 confirmed authenticity finding；
- 两者冲突时保留双方证据并交由 A11/A12 处理。

---

## 9. Independent Judge 协议

Judge 必须满足：

1. 未参与该 finding 的原始发现和修复；
2. 从 clean checkout 读取 manifest 并校验 SHA256SUMS；
3. 对 P0/一票否决 finding 重跑核心复现，而不是只审阅文字；
4. 对 `refuted` finding 复核原条件、反证条件和环境差异；
5. 记录自己的 `source_agent/source_run/timestamp/evidence_hash`；
6. 发生冲突时填写双方证据和 `conflict_resolution`，不覆盖原记录。

Judge 可以给出 `accepted / rejected / needs_more_evidence`。最后一种必须使 finding 保持 `unverified` 或工作包保持 `blocked`。

---

## 10. Issue 和整改协议

### 10.1 Finding → Issue

`confirmed` finding 必须创建独立 defect Issue，并包含：

- finding ID、run_id 和 assessed commit；
- 最小复现及 evidence hash；
- 影响范围和 severity；
- 修复验收，不预设实现方案；
- regression run 要求；
- 工作包和 Master 的反向链接。

### 10.2 Issue 关闭

遵守 [`ISSUE_CLOSING_VERIFICATION.md`](../../governance/ISSUE_CLOSING_VERIFICATION.md)：

1. 有关联且已合并到目标分支的 PR；
2. 修复代码已集成；
3. 相关测试有运行证据；
4. 必要文档已合并；
5. finding 已通过独立 regression run；
6. 子 Issue 关闭后才可关闭依赖它的聚合 Issue；Master 最后关闭。

取消或错误报告不得静默删除；应保留 `refuted`/`cancelled with reason` 记录和证据。

### 10.3 已知 finding 的迁移入口

以下 Issue 是历史计划产生的 finding/remediation 入口；状态以 Gitea 查询为准：

- [#5167](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5167)
- [#5191](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5191)
- [#5192](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5192)
- [#5193](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5193)
- [#5209](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5209)
- [#5217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5217)

本列表只表示关联，不声明这些 Issue 已修复、已验证或可关闭。

---

## 11. 交付物

| 交付物 | 生成者 | 验证者 |
|---|---|---|
| capability/entry/scope matrix | A02 | A11 |
| production call-chain matrix | A03 | A11 |
| SQL/protocol differential matrix | A04 | A11 |
| index/storage evidence | A05 | A11 |
| transaction/WAL/recovery evidence | A06 | A11 |
| mutation/test-effectiveness report | A07 | A11 |
| disabled binary revalidation | A08 | A11 |
| cross-version matrix | A09 | A11 |
| security/isolation matrix | A10 | A11 |
| independent review matrix | A11 | Human Architect |
| final report and stage proposal | A12 | Governance + Human Architect |

所有报告必须引用 run_id 和 finding ID，不允许复制无法追溯的数字。

---

## 12. 执行顺序

1. A01 产出 manifest schema、目录和重放规则，进入 `evidence_ready` 并保持 open；
2. A02 产出 claim、入口和范围矩阵，进入 `evidence_ready` 并保持 open；
3. 为 A03–A10 分配独立 worktree、run_id 和数据/端口范围；
4. 并行执行 A03–A10，finding 只写入各自 run；
5. confirmed finding 创建 defect Issue，修复在独立分支进行；
6. A01–A10 全部进入 `evidence_ready` 且保持 open，A11 再校验哈希、重放 P0/P1 和抽样 P2；
7. A11 verdict 后按 Issue closing verification 逐项关闭 A01–A10；
8. A12 从已验证记录生成最终报告及治理提案；
9. 关闭 A12，最后处理 Master。

---

## 13. 本协议的变更控制

修改本文必须：

1. 关联 Master 或专门 governance Issue；
2. 说明变更是否影响已有 run 的解释；
3. 不回写或改写已经完成的 run manifest/finding；
4. 通过文档链接和一致性检查；
5. 经 reviewer 审查；
6. 通过 PR 合并到 `develop/v4.1.0` 后生效。

若协议发生不兼容变更，提升 `schema_version` 并保留旧 schema 的解释规则。

---

## 14. 治理引用

- [`ADR-001-truthfulness-framework.md`](../../governance/adr/ADR-001-truthfulness-framework.md)
- [`ANTI_FABRICATION_POLICY.md`](../../governance/ANTI_FABRICATION_POLICY.md)
- [`ISSUE_CLOSING_VERIFICATION.md`](../../governance/ISSUE_CLOSING_VERIFICATION.md)
- [`DOC_CHECK_CORRECTION_RULES.md`](../../governance/DOC_CHECK_CORRECTION_RULES.md)
- [`AI_COLLABORATION.md`](../../governance/AI_COLLABORATION.md)
- [`GATE_CONDITIONS.md`](../../governance/GATE_CONDITIONS.md)
- [`ADR-008-test-claim-transparency.md`](../../governance/adr/ADR-008-test-claim-transparency.md)
- [`ADR-014-multi-ai-coordination.md`](../../governance/adr/ADR-014-multi-ai-coordination.md)

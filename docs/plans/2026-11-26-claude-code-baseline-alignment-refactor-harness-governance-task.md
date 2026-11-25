# 任务文档（交付 Claude Code / GitHub 侧执行）：SQLRustGo v4.1.0 基线对齐、重构与 Harness 治理启动方案

> **文档类型**: 报告 + 完整实施方案（**不包含开发实现代码落地**，仅规划与验收口径）  
> **生成时间（本地）**: 2026-11-26（Asia/Shanghai 语境）  
> **生成环境**: OpenCode 工作树 `calm-otter`（detached HEAD，见下文“证据快照”）  
> **目标读者**: 在 **可访问 GitHub（`minzuuniversity/sqlrustgo`）且可使用 Claude Code CLI** 的远端服务器上执行分析/评估/后续执行的工程师或自动化代理  
> **强约束**: 本文只给出“如何做”的分析与方案；**不得**在未获明确授权前擅自修改受保护分支、门禁阈值、STAGE、或进行任何生产部署/合并

---

## 1. 执行摘要（Executive Summary）

本任务要求 Claude Code 在 GitHub 侧仓库上下文中，对 SQLRustGo **v4.1.0 开发线**完成：

1. **基线对齐（Baseline Alignment）**：以仓库内**权威事实源**对齐“代码/文档/远端/门禁/发布阶段”的单一真相，消除多远端与多文档之间的漂移认知。
2. **重构启动（Refactor Kickoff）**：在不破坏既有门禁与治理约束前提下，产出可执行的重构路线图（优先级、切片、验证矩阵、回滚策略）。
3. **Harness 治理启动（Harness Governance Bootstrapping）**：将 Agent Harness（AGENTS、GitNexus、AOCI、Gates、证据链、Anti-Fabrication）固化为可审计流程。

**重要事实（必须先承认）**：当前仓库存在“文档口径”与“运行时/发布口径”的历史张力（例如 Cargo 版本字段与发布叙事、v4.0.0 GA 治理缺口遗留到 v4.1.0 等）。任何“已对齐/已完成”结论，**必须附带可复现证据**（命令、退出码、关键输出摘要、commit SHA、必要时哈希）。

---

## 2. 证据快照（本任务生成时刻，只读观测）

> 以下为生成本文时在工作树内执行的只读检查结果，用于让 Claude Code 先校准上下文；**Claude Code 应在 GitHub 仓库检出后重新核验**（不得复用本节数字作为 GitHub 侧真相）。

- 工作目录（OpenCode worktree）: `/Users/liying/.local/share/opencode/worktree/c089c3/calm-otter`
- Git 状态: `git status --short` **无输出**（工作区干净）
- HEAD（detached）: `cd8cefe623e148964fd0c4ce0ccfcc56eff762df`
- Remotes（节选，来自 `git remote -v`）:
  - `origin` / `gitea252`: `http://192.168.0.252:3000/openclaw/sqlrustgo.git`
  - `mz`: `https://github.com/minzuuniversity/sqlrustgo.git`（**本文所称 GitHub 目标远端**）
  - `github`: `git@github.com:yinglichina8848/sqlrustgo-public.git`（镜像，非本任务目标）
  - `gitee`, `gitcode`, `gitea250` 等为只读/辅助镜像语境（以仓库 `docs/gitea-only.md` 及项目规则为准）

**Claude Code 必做的首轮核验命令（在 GitHub 检出目录执行）**:

```bash
git rev-parse HEAD
git branch --show-current
git remote -v
git status --short
cat docs/releases/v4.1.0/STAGE.yaml | sed -n '1,120p'
sed -n '1,120p' README.md
```

并记录：`source_agent / source_run / timestamp / evidence_hash(可对关键文件做 sha256) / conflict_resolution`（与仓库 Anti-Fabrication 要求一致）。

---

## 3. 权威事实源与冲突处理规则（必须遵守）

### 3.1 权威顺序（建议写入团队约定）

1. **Git 事实**：`git rev-parse`、分支/标签实际指向、远端 `refs/*`（以你当次执行环境能访问的远端为准）。
2. **发布阶段 SSOT**：`docs/releases/v4.1.0/STAGE.yaml`（及 `docs/governance/STAGE_CONFIG.yaml`、`scripts/gate/check_stage.sh` 所描述的约束链）【以仓库当前文件为准】。
3. **治理与流程文档**：`AGENTS.md`、`docs/governance/*`、`docs/agent-harness/*`。
4. **README / 其他叙事文档**：可能滞后，需与 1–3 交叉验证后再采信。

### 3.2 已知的“口径张力点”（Claude Code 必须显式处理）

- **版本号叙事**：`AGENTS.md` 曾提及 `Cargo.toml version = "3.12.0-fix-zombie"` 为有意保留发布口径（README §0 说明）——Claude Code 需在 GitHub 侧实际打开 `Cargo.toml` 与 `README.md` 对应章节核对，并在报告中写明“现状 vs 文档描述”。
- **v4.0.0 遗留治理缺口**进入 v4.1.0：见 `STAGE.yaml` 中 `inherited_from_v4000_revocation`、`false_done_wp_a`、`legacy_chain_audit_2026_09_30` 等段落（本任务**不得**擅自改写这些历史记录，只能基于现状补充证据与计划）。
- **多远端**：Gitea 为主线协作语境（项目规则强调 Gitea-only 流程），GitHub `minzuuniversity/sqlrustgo` 可能是镜像或协作目标之一——**基线对齐必须先定义“对齐到谁”**（见第 4 节）。

### 3.3 绝对禁止（摘录自仓库规则，执行时逐条自检）

- 不得将未验证结论写成 PASS/完成；不得用“测试通过”代替“已运行测试+证据”。
- 不得削弱测试（删断言、改期望以过绿）、不得把 `--no-run` 当通过。
- 不得在未授权下修改门禁脚本语义、阈值、STAGE、分支保护策略。
- 不得使用 `gh` CLI 对 GitHub 做仓库管理操作（若 Claude Code 运行环境策略允许 git push 到已配置 remote，需区分“git push”与“gh 命令”；以仓库 `CLAUDE.md` / `docs/gitea-only.md` 当前表述为准，冲突时在报告中标出并请求人工裁决）。
- 任何声称“可发布/可合并/已修复”的结论，必须带 5 字段证据链与可复核产物路径。

---

## 4. 基线对齐（Baseline Alignment）——分析框架与输出要求

### 4.1 先定义“基线”（必须由 Claude Code 在报告中明确写出二选一或混合策略）

在动手前，Claude Code 必须在报告《基线对齐决策》中明确采用下列哪种（或组合），并说明理由与风险：

- **策略 A（Gitea 主线基线）**：以 `http://192.168.0.252:3000/openclaw/sqlrustgo.git` 上约定分支（例如 `develop/v4.1.0`）的 `git rev-parse` 为开发事实基线；GitHub `minzuuniversity/sqlrustgo` 作为**镜像同步目标**，需对齐提交图/标签/受保护分支策略（仅在授权下）。
- **策略 B（GitHub 目标基线）**：以 GitHub `minzuuniversity/sqlrustgo` 的默认分支与指定开发分支为基线，反向审计 Gitea 是否落后；适用于“Claude Code 只能改 GitHub、人工再同步回 Gitea”的过渡期（需明确标注为临时策略与债务）。
- **策略 C（双基线差异报告）**：不强行合并，先产出差异清单（commit 差集、文件漂移、CI/门禁文件差异、文档冲突点），再由 Owner 选择收敛方向。

**推荐默认**：除非 Owner 另有书面指令，否则默认按 **策略 A + C**（先对齐认知与差异清单，再决定是否向 GitHub 推送/镜像），因为项目规则长期强调 Gitea-only 与镜像只读语义。

### 4.2 基线对齐检查清单（Claude Code 必跑，结果落表）

对每个你纳入比对的 remote/ref，记录：`fetch 时间`、`rev`、`tag`（若相关）、`是否包含 STAGE.yaml 所述继承项/已知缺口文件`。

建议比对维度（最小集）：

1. **分支拓扑**：`main`、`develop/v4.1.0`（或当前活跃开发分支）、最近治理/文档分支命名模式（仅观察，不操作）。
2. **发布与阶段**：`docs/releases/v4.1.0/STAGE.yaml`、`docs/releases/v4.0.0/*` 中与“撤销/遗留”相关的关键文件是否存在且内容一致（哈希或摘要）。
3. **门禁与脚本**：`scripts/gate/check_*.sh`、`.gitea/workflows/*`（若 GitHub Actions 不同需单列）。
4. **Harness 文档**：`AGENTS.md`、`docs/agent-harness/INDEX.md`、`docs/agent-harness/REPOSITORY_MAP.md`、`docs/governance/ANTI_FABRICATION_POLICY.md`、`docs/governance/adr/*` 关键 ADR 是否齐全。
5. **已知技术债入口**：`STAGE.yaml` 中列出的 `COV-01/02/DONE`、`REF-01..03`、`false_done_wp_a` 状态与仓库中 `ISSUES_PLAN.md`/相关 evidence 目录是否可交叉验证（只读）。

### 4.3 “基线对齐”完成的判定（Definition of Done for Baseline Alignment Report）

Claude Code 的报告中必须包含：

- 一张 **Remote/Ref 对比表**（至少包含 Gitea 主线候选 ref 与 GitHub `minzuuniversity/sqlrustgo` 的对应 ref）。
- 一张 **差异分类表**：`一致 / 仅文档差异 / 代码差异 / 门禁/CI 差异 / 未知（需人工）`，每项带证据指针（路径+commit+命令摘要）。
- 明确的 **“对齐目标声明”**（对应 4.1 的策略选择）与 **风险清单**（含不可逆操作警告）。
- **不确定项清单**：凡无法从仓库内文件与可访问远端事实推出的，一律标 `UNKNOWN`，不得猜测。

---

## 5. 重构启动（Refactor Kickoff）——方案要求

### 5.1 重构原则（硬性）

- **小步可回滚**：每个重构切片应能独立编译、独立跑相关测试或门禁子集（在授权的本地/CI 环境中）。
- **不以“结构优化”为名削弱语义**：涉及 SQL 语义、事务/WAL、权限等模块时，必须先写/更新测试意图说明与风险评估。
- **优先处理与门禁/稳定性直接相关的项**：例如 `STAGE.yaml` 中提到的 `REF-01..03`（expr/stored_proc/trigger 大文件拆分）及其与 `C-ARCH-05` 的关系（见 `gate_policy` 注释）。
- **生产路径优先**：修改前确认生产调用链（对照 `AGENTS.md` 第 5 条示例：FileStorage 生产实现位置等），避免只改测试夹具路径造成虚假“修复”。

### 5.2 建议的重构路线图结构（Claude Code 输出）

按阶段输出（示例结构，Claude Code 可调整但需理由）：

- **Phase R0（只读盘点，0 代码改动）**：列出目标模块的依赖图摘要、测试覆盖热点、最近变更作者/时间（从 git 历史只读）、与 `C-ARCH-05`/架构门禁的耦合点。
- **Phase R1（文档与契约先行）**：为每个拟拆分的大文件产出“拆分设计笔记”（接口不变/可变清单、公共类型归属、迁移顺序）。
- **Phase R2（机械性拆分切片）**：按文件/子模块切片，每片含：改动范围、验证命令、预期失败模式、回滚方式。
- **Phase R3（加固与门禁对齐）**：在拆分稳定后再评估是否扩展 `C-ARCH-05` 覆盖范围（严格遵循 `STAGE.yaml` 中 “Extending C-ARCH-05 should happen AFTER REF-01..03 land” 的策略意图）。

### 5.3 每个重构任务必须包含的验收卡片（模板，Claude Code 填写）

对每个任务 T-k 输出：

- 目标与非目标
- 影响面（文件/模块/公开 API）
- 风险等级（Low/Med/High）与回滚
- 验证计划（至少包含：`cargo fmt --check --all`、`cargo clippy --all-features -- -D warnings`、与变更相关的 `cargo test ...`；若涉及门禁脚本需列出具体 `scripts/gate/*.sh`）
- 证据留存路径建议（例如 `docs/releases/v4.1.0/evidence/...` 或团队约定目录）

---

## 6. Harness 治理启动（Harness Governance Bootstrapping）——方案要求

### 6.1 目标

使“Agent/人类协作”可审计、可复现、可追责：任何声称结果都能回溯到命令与产物，任何规则变更都有 ADR/治理文档链。

### 6.2 必须产出的治理工件（Claude Code 在 GitHub 侧建议创建的清单，**是否创建需经仓库维护者确认**）

> 注意：本节是**建议清单**。若涉及新增/修改 `docs/governance/*`，必须遵守仓库中 `DOC_CHECK_CORRECTION_RULES.md` 等流程（Claude Code 报告中应引用并提醒）。

建议至少包含（按优先级）：

1. **Harness 启动检查单（可执行版）**：一页纸清单，对应 `AGENTS.md` Phase 0 + 必跑门禁的最小子集，标注“本地可跑 vs CI-only”。
2. **证据包规范（Evidence Bundle Spec）**：统一目录命名、文件命名、必须包含的 5 字段、哈希方法（`shasum -a 256` 等）、与 `docs/releases/v4.1.0/evidence/` 的衔接方式。
3. **基线对齐周报/里程碑模板**：用于多远端、多 Agent 并行时的状态同步（避免口头结论）。
4. **GitNexus/AOCI 使用合规检查点**：在 PR/重大改动前的强制步骤（与 `AGENTS.md` 中 MUST 条目一致，形成 checklist）。

### 6.3 与现有文档的衔接（Claude Code 必做）

在报告中建立映射表：`本方案条款 -> 已有文档路径`，对“缺失的衔接点”单独列出并提出最小补丁建议（不直接改仓库，除非被明确授权）。

---

## 7. Claude Code 在 GitHub 侧的推荐执行顺序（Step-by-Step）

> 以下为建议顺序，Claude Code 可调整，但必须在报告中说明偏差原因。

1. **环境与访问确认**（只读优先）
   - 确认 `git remote -v` 包含 `mz`（`minzuuniversity/sqlrustgo`）或用户指定的 GitHub remote 名称；确认当前分支与是否允许 push（需用户明确指令）。
2. **拉取与对照**
   - `git fetch --all --prune`（网络允许时），记录各 remote HEAD。
3. **生成《基线对齐报告》**（产出 Markdown，建议路径：`docs/releases/v4.1.0/evidence/BASELINE_ALIGNMENT_<YYYY-MM-DD>.md` 或团队约定；若仓库无约定，需在报告中提出建议路径并说明未落盘原因）
4. **生成《重构路线图 v0.1》**（对应第 5 节结构）
5. **生成《Harness 治理启动包 v0.1》**（对应第 6 节结构）
6. **汇总《风险与待 Owner 裁决清单》**（例如：是否以 GitHub 为临时基线、是否允许创建镜像同步分支、是否修改 governance 文档等）
7. **（仅当用户明确授权）**提交文档到 Git 仓库并 push 到**用户指定**的 remote/branch；否则仅输出补丁/diff 建议与本地提交说明。

---

## 8. 交付物清单（Claude Code 必须产出）

| 交付物 | 说明 | 建议格式 |
|--------|------|----------|
| 基线对齐报告 | 含策略选择、remote/ref 表、差异表、UNKNOWN 列表 | Markdown |
| 重构路线图 | Phase R0–R3 + 任务验收卡片 | Markdown（可拆分多个文件） |
| Harness 治理启动包 | 检查单、证据包规范、模板、映射表 | Markdown |
| 风险与裁决清单 | 需 Owner/架构师决策的条目（编号、影响、建议） | Markdown |
| 证据索引 | 本次分析涉及的命令与关键输出的摘要索引 | 附在报告末尾 |

**禁止交付**：未经授权的代码大改、门禁放宽、将推测写成事实的“结论”。

---

## 9. 给提交到 GitHub 后的“交接提示词”（可直接复制给 Claude Code）

以下为建议粘贴给 Claude Code 的提示词（用户可按需修改）：

```text
你现在位于 GitHub 仓库 minzuuniversity/sqlrustgo 的一个检出目录中（请先用 git remote -v 与 git rev-parse HEAD 确认）。  
请阅读仓库内文件：AGENTS.md、docs/releases/v4.1.0/STAGE.yaml、docs/agent-harness/INDEX.md、docs/governance/ANTI_FABRICATION_POLICY.md 以及本任务文档所在路径（若已合并进仓库，请以仓库内路径为准）。

任务（仅分析与方案，不做生产开发落地，除非我后续逐条授权）：
1) 基线对齐：按本文第4节产出《基线对齐报告》，先声明采用策略 A/B/C（推荐 A+C），列出 Gitea 主线候选 ref（若你无法直连 Gitea，仅能访问 GitHub，则在报告中标注 UNKNOWN 并说明阻塞）。
2) 重构启动：按本文第5节产出《重构路线图 v0.1》，聚焦 STAGE.yaml 中 REF-01..03 与 C-ARCH-05 策略意图，并给出任务验收卡片。
3) Harness 治理启动：按本文第6节产出《Harness 治理启动包 v0.1》，包含检查单与证据包规范草案。

硬约束：
- 不得使用 gh CLI 执行会改变 GitHub 仓库状态的管理操作，除非我明确要求且你说明风险。
- 任何“完成/通过/对齐成功”的表述必须附命令与可复核证据；无法验证的标 UNKNOWN。
- 不得修改 STAGE.yaml、门禁脚本语义、分支保护相关配置，除非我单独授权每一项。
- 输出语言中文，技术标识符保留英文。

请先输出“Step 0 证据快照”（remote、HEAD、当前分支、STAGE.yaml 关键行摘要），再进入报告正文。
```

---

## 10. 附录：本工作树内与任务直接相关的路径索引（供 Claude Code 交叉核对）

- `AGENTS.md`
- `docs/releases/v4.1.0/STAGE.yaml`
- `docs/governance/ANTI_FABRICATION_POLICY.md`
- `docs/governance/adr/ADR-001-truthfulness-framework.md`
- `docs/agent-harness/INDEX.md`
- `docs/agent-harness/REPOSITORY_MAP.md`
- `docs/gitea-only.md`
- `CLAUDE.md`（仓库根，含 GitHub 同步例外说明）

---

*文档结束。*

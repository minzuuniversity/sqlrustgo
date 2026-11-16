# STAGE_GOVERNANCE_RC_TO_GA.md — v4.1.0 的 RC→GA 阶段治理

> 本文件是 `docs/releases/v4.1.0/STAGE.yaml` 的 `doc_artifacts_required.stage_GA`
> 所要求的产物之一（另一份是 `GA_GATE_REPORT.md`，本文件是阶段治理口径）。
> **本文件不含任何未经实跑的结论**：每一项状态都标注了它的证据来源，
> 读不到证据的一律写「未验证 / INCOMPLETE」。

- 版本：v4.1.0
- 当前阶段（2026-11-16 实测）：**ALPHA**
- 权威判据源：`docs/governance/STAGE_CONFIG.yaml` 的 `stages.RC` / `stages.GA`
- 阶段跃迁规则：`docs/governance/STAGE_CONFIG.yaml` 的 `transitions`

---

## 1. RC→GA 的判据（原文，不改写）

`STAGE_CONFIG.yaml` → `stages.RC.exit_criteria`：

| # | 判据 | 执行者 | 本轮状态（2026-11-16） |
|---|---|---|---|
| 1 | All RC{N} release-blocking bugs closed | **人工**（仓内无执行点） | 未验证 —— 本轮未做 RC 候选轮次 |
| 2 | All 5 dimensions (D1-D5) PASS | `check_full_gate_verification.sh` + `check_rc_ga_gate.sh` | 实跑中（run `20261116-ga-gates-002`，冻结提交 `7e26c2b7c6`） |
| 3 | Tag v{VER}-rcN cut, no P0 bugs since | **人工**（仓内无执行点） | 未验证 —— 无 rc tag，且 P0 计数无门禁覆盖 |
| 4 | GA promotion approved by human architect | **人工**（`requires_human_approval: true`） | 未取得 —— 属主尚未署名 |
| 5 | Coverage >= 80% | `check_coverage_v312.sh` | 实跑中（同上 run） |

`STAGE_CONFIG.yaml` → `stages.RC.manual_only_criteria` 对第 1/3/4 条的原文说明：

> 「All RC{N} release-blocking bugs closed：没有门禁覆盖 blocker 是否关闭；
> 候选 `check_v312_01_blocker_closed.sh` 不在任何阶段的门禁清单里，也没有活执行点。」
>
> 「Tag v{VER}-rcN cut, no P0 bugs since：两半都没有执行点：没有门禁要求 rcN tag 存在，
> 也没有门禁统计 open P0。」
>
> 「GA promotion approved by human architect：人的批准
> （`transitions.RC_to_GA.requires_human_approval: true`）。
> **批准记录写进本文件的变更记录，不能由 Agent 生成。**」

⇒ 第 4 条是**硬边界**：阶段跃迁签批行必须由人类架构师署名，Agent 不得代签、
不得以「条件通过」形式变通。

## 2. RC 阶段的承载与清单

- 承载分支（`stages.RC.branch_pattern`）：`develop/v*` / `release/v*` / `rc/v*`，`carrier_enforced: true`。
- RC 承重清单：26 项（22 个脚本门禁 + 4 条 cargo 命令），来自 `stages.RC.required_gates`。
- v4.1.0 的 `STAGE.yaml` 已把这 26 项同步为镜像（不再是「Deferred」占位）。

## 3. GA 阶段的承载与清单

- 承载分支（`stages.GA.branch_pattern`）：**`main` / `release/v*` / `hotfix/v*`**，`carrier_enforced: true`
  ⇒ **develop/v4.1.0 上不得宣称 GA**（`current_state.ga_carrier_note` 同义）。
- GA 承重清单：**35 项 = 31 个脚本门禁 + 4 条 cargo 命令**（`stages.GA.required_gates`）。
- GA 所需文件（`stages.GA.required_files`）：`STAGE.yaml` / `RELEASE_NOTES.md` /
  `GA_GATE_REPORT.md` / `GA_RELEASE_TIMELINE.md` / `CHANGELOG.md` —— 五份**当前均在位**。
- GA 文档产物（`STAGE.yaml` 的 `doc_artifacts_required.stage_GA`）：
  `GA_GATE_REPORT.md` / `GA_RELEASE_REPORT.md` / `STAGE_GOVERNANCE_RC_TO_GA.md` /
  `CLAIM_DOWNGRADE_MANIFEST.md` —— **本文件与 `GA_RELEASE_REPORT.md` 此前缺失，
  本轮补齐**。

## 4. 证据格式（阶段跃迁必须齐备的五件）

依 `STAGE.yaml` 的 `stage_decision_2026-11-08.evidence_discipline`：

1. 归档 run id；
2. `evidence_hash`（sha256）；
3. 门禁 rc；
4. **干净检出复现命令**；
5. **独立 Reviewer 回执**。

补充（本轮实测确认）：Hub `ai-project-hub` #133 的独立 Reviewer 派单权在 Controller
答复中转给 Owner，**至今未回** ⇒ 第 5 环缺失，阶段跃迁证据链不完整。

## 5. 2026-11-16 的 RC→GA 阻断台账（四条硬约束，AI 不可自行消除）

| # | 约束 | 事实 | 消除条件 | 责任方 |
|---|---|---|---|---|
| 1 | **SOAK-168H** | `exit_criteria.rc_to_ga` 要求「168h multi-model SOAK PASS」；SOAK 物理需 7 天独占窗口 | 2026-11-16 启动的 run 满 168h 且两臂产物齐 | 执行（本轮已交付运行器并启动） |
| 2 | **HUMAN-SIGNOFF** | `transitions.RC_to_GA.requires_human_approval: true`；签批「不能由 Agent 生成」 | 人类架构师在本文件/GA_GATE_REPORT 上署名 | **属主** |
| 3 | **CARRIER** | GA 须 `main` / `release/v*` 承载；`release/v4.1.0` 尚不存在 | 建分支并把 RC 内容落到其上 | 执行 + 属主授权 |
| 4 | **BASELINE-GAP** | 168 道门禁重盘：REQUIRED 134，其中 **49 道「应执行但无执行点」**（37 NONE_PENDING + 12 NONE_BLOCKED_EXTERNAL）；owner 2026-11-08 裁决要恢复的 23 道**至今未落地**（`gov_probe` 实测 `dead` 仍 = 61） | 逐道恢复执行点或从声称面撤下 | 执行 + 属主裁定 |

## 6. 不得采用的做法（明确写下来，避免后来者走回头路）

- **不得复制 v4.0.0 的「GA (CONDITIONAL PASS)」模式** —— 该判定已于 2026-09-30 被撤销
  （`docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict = REVOKED — v4.0.0 never legitimately reached GA`），
  撤销原因正是「覆盖率 78.28% < 门槛 + 3 个继承 ALPHA 门禁失败被写成已解决而实测未解决」。
- **不得用「CI 绿灯」替代本地实跑**：CI 执行面已死（#5597 / #5551 / #5569 / #5573），
  本地归档 run 是唯一证据源，并须逐 PR 声明该偏差。
- **不得用文件存在代替证据**：SOAK 判据明写「file existence alone is NOT evidence」。
- **不得把专家意见当门禁依据**：`ANTI_FABRICATION_POLICY` §5.1/5.3 —— 模型意见仅作参考。

## 7. 变更记录（签批行）

> 本节由**人类架构师**填写。Agent 不得代填、不得代签。
>
> - 签批人（姓名 / 角色）：
> - 签批日期：
> - 签批结论（批准 RC→GA / 不批准 / 有条件批准并列出条件）：
> - 依据的 run id 与 evidence_hash：

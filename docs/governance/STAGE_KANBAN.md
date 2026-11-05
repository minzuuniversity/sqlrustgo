# 阶段看板制度（STAGE_KANBAN）

> 状态：强制制度。适用于所有版本的所有阶段。
> 建立原因：2026-11-04 owner 指示——"当前项目的进展非常不清晰"。
> 本文件不复制任何既有 SSOT 的内容：阶段状态读 `docs/releases/vX.Y.Z/STAGE.yaml`，
> 门禁清单读 `gates/`，标准读 `STAGE_CONFIG.yaml`。看板只回答四个问题：
> **现在在哪、还剩什么、卡在哪、下一步做什么**。

---

## 1. 粒度：版本级 + 阶段级

| 层级 | 什么时候建 | 文档 | ISSUE 标题 |
|---|---|---|---|
| 版本级 | 每个版本进入 DRAFT 时建 1 张 | `docs/releases/vX.Y.Z/KANBAN.md` | `[看板] vX.Y.Z 版本总览（YYYY-MM-DD）` |
| 阶段级 | 版本内容多（任务 > 12 或跨 2 个以上能力域）时，每个阶段各 1 张 | `docs/releases/vX.Y.Z/KANBAN_<STAGE>.md` | `[看板] vX.Y.Z · <STAGE> 阶段总览（YYYY-MM-DD）` |

阶段取值与 `docs/governance/STAGE_CONFIG.yaml` 一致：`DRAFT / ALPHA / BETA / RC / GA`。
一个版本在同一时刻**只有一张 open 的阶段看板**（当前阶段那张）；其余阶段的看板要么未建，
要么已按 §5 关闭。

## 2. 看板五节（缺一即视为看板不合格）

1. **阶段目标**：这个阶段结束时，世界和现在有什么不同。必须是可判定的陈述，
   不允许"完善质量""推进整改"这类无法证伪的措辞。
2. **子任务台账**：每行一个任务，字段固定：
   `ID | 任务 | 状态 | 验收判据（可证伪） | 证据 | 阻塞方`
   - 状态词汇只有五种：`未开始 / 进行中 / 待复核 / 完成 / 阻塞`。
   - **门禁三态不许借用看板状态**：看板里的"完成"指任务做完并归档证据，
     不等于任何门禁 `PASS`；引用门禁结论时照写 `PASS / FAIL / INCOMPLETE`。
   - `证据` 列必须是 run id + 路径（`audit/v4.1.0/runs/<run_id>/...`、Gitea run/PR/commit），
     空值 ⇒ 该行自动降级为 `待复核`（AFP §1：无证据的完成声明不成立）。
3. **问题汇总**：当前已知问题，每条给"现象 / 实测根因 / 影响 / 谁能解"。
   Agent 无法自行解除的（凭据、登录、分支保护、发布审批）必须显式写 `owner`。
4. **下一步计划**：有序列表，第一项必须是"下一个能产生新证据的动作"，
   不是"继续分析"。
5. **证据索引**：本看板引用的每个数字，指向一个可校验的 run
   （`audit/v4.1.0/runs/<run_id>/manifest.yaml` + `logs/NNN-<id>.stdout`，
   由 `audit/v4.1.0/audit_run.py verify --run-id <id>` 机器核对）。
   没有可校验归档的数字一律标注"文档声称"。
   **注意文件类型约束**：`check_no_log_tbl_json.sh` 与 `.gitignore` 禁止 `*.json`/`*.log` 入仓，
   所以数字类证据只能是 `.stdout`/`.stderr`/`.jsonl`/`.yaml`。
   **数字/日志类 run 证据一律进 `audit/v4.1.0/runs/`**（否则落在 `check_audit_manifest.sh`
   的核对范围之外）；`evidence/` 只用于叙述性记录（仓内既有约定，如 `evidence/v312-58/*.md`），
   不得用它承载看板引用的数字。

## 3. 台账必须有机读副本

看板同目录放 `KANBAN_<STAGE>.jsonl`：每行一个任务对象
（`id/task/status/acceptance/evidence/blocked_by/domain/priority`）。
Markdown 给人看，JSONL 给机器核对（防"看板好看、任务对不上"）。
两者不一致时，以 JSONL 为准并当场修 Markdown。

**生成关系（2026-11-04 起为强制）**：Markdown 里的台账表与"台账计数"行是
`scripts/gate/kanban_render.py` 从 JSONL **生成**的，被
`<!-- LEDGER:BEGIN ... -->` / `<!-- LEDGER:END -->` 标记包住，**不得手改这一段**。
核对与重生成：

```bash
python3 scripts/gate/kanban_render.py check  docs/releases/vX.Y.Z/KANBAN_<STAGE>.md   # 漂移 -> 退出码 1
python3 scripts/gate/kanban_render.py render docs/releases/vX.Y.Z/KANBAN_<STAGE>.md   # 重新生成该段
```

`check` 的退出码是两态的（0=一致，1=漂移或输入不合法），**不会把"没能核对"返回成 0**。
夹具：`scripts/gate/test_kanban_render.py`（含反向变异：从 JSONL 删一行必须红，恢复必须绿）。
Markdown 的其余小节（阶段目标、问题汇总、下一步、证据索引）仍是人写的，生成关系只覆盖台账表。

## 4. ISSUE 与文档的关系

- ISSUE 正文 = 看板摘要（阶段目标 + 台账表 + 问题 + 下一步），
  并在首行写明对应文档路径与最后更新的 commit。
- 台账的**权威副本是文档**；ISSUE 是它的展示与讨论面。ISSUE 评论里产生的新任务，
  必须当天回写文档台账，否则不算登记。
- 看板 ISSUE 由 Agent 创建时按 ADR-014 D-01 标注身份（`[agent: mcode-macmini]`）。
- **看板 ISSUE 不得用来销账**：关闭它只允许发生在阶段跃迁获批之后（§5），
  任务本身的关闭通过台账状态体现。这与"关闭无 PR 关联的 Issue 属违规"（AGENTS §8）一致。

## 5. 关闭与迁移规则（这是本制度唯一有牙齿的地方）

阶段跃迁（例如 `ALPHA → BETA`）在 `STAGE.yaml` 记录获批后：

1. 对当前阶段看板的**每一条**任务，台账必须补齐"完成情况说明 + 证据"两列内容；
2. 未完成/阻塞的任务**必须显式迁移**：在下一阶段看板里以新行出现，
   `来源` 列写 `承接 <上一阶段看板 ID>`；
3. 然后才允许关闭上一阶段看板 ISSUE，并在该 ISSUE 的关闭评论里列出
   "完成 X 条 / 迁移 Y 条 / 放弃 Z 条（附理由）"；
4. 任何任务不得"静默消失"：从 open 看板里删除一行而既没有 `完成` 也没有
   `迁移`/`放弃(理由)` 记录，视为治理违规，按 incident 处理。

跃迁本身需要人工批准（AGENTS §2.7 同源纪律，本仓 STAGE.yaml 的 `last_transition` 记录），
看板制度不授予任何跃迁权威。

## 6. 待建的机器核对（登记为任务，尚未实现）

| 判据 | 内容 | 反向变异 |
|---|---|---|
| `check_kanban`（**核对逻辑已实现，尚未接入执行点**） | 每个 open 版本必须有且仅有一张 open 阶段看板；文档台账与 JSONL 的 ID 集合一致；每行有 `证据` 或状态为 `未开始/进行中` | 从 JSONL 删一行 ⇒ 必须红 |
| `check_kanban_closeout`（未建） | 已关闭的阶段看板，其任务全部有完成情况说明；`迁移` 行在下一阶段看板里存在 | 抹掉一条说明 ⇒ 必须红 |

按"新增判据一律挂靠既有门禁、不新增分母"的纪律，这两条在实现前**只是台账任务**，
不进入门禁清单；若将来要进清单，必须由 owner 在独立 PR 里做。
接线工作登记为看板任务 KR-2。

## 7. 变更记录

| 日期 | 变更 | 依据 |
|---|---|---|
| 2026-11-04 | 首版：粒度、五节结构、ISSUE 关系、关闭与迁移规则、待建判据 | owner 指示"进展非常不清晰，要写工作看板并发布为文档和 ISSUE"；`mcode-macmini` 起草 |
| 2026-11-04 | 台账生成关系强制化：`kanban_render.py` 渲染 + `check` 判漂移；实测发现看板 MD 与 JSONL 已漂移（IL-1 文本 105/106、§6 引用不存在的证据文件名），当场按"以 JSONL 为准"修回 | 实测：`check` 在修前 rc=1、修后 rc=0；夹具 `test_kanban_render.py` exit 0（run `20261104-kanban-001`） |
| 2026-11-04 | 证据列与证据索引改指 `audit/v4.1.0/runs/` 的 run 布局；禁止另立 `evidence/` 布局 | 实测：`check_audit_manifest.sh` 只扫描 `audit/v4.1.0/{,runs/}*/manifest.yaml`；`check_no_log_tbl_json.sh` 拒绝 `*.json`/`*.log` 入仓 |

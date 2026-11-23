# TF-01 执行前检查报告与授权缺口（DSH-B）

provenance: source_agent=DSH-B; source_run=TF-PREFLIGHT-20261121-R1; timestamp=2026-11-21T04:35:00+08:00; evidence_hash=见 §8; conflict_resolution=设计包哈希全部一致；未发现第三方修改；唯一缺口为运行态实施授权未持久化。

## 0. 结论（先说）

- 设计包 SHA256SUMS **15/15 校验通过（rc=0）**，基线锚点全部命中。
- DSH-A 运行态 **不存在 TF-01～TF-07 任何任务、预算释放或 allowed_paths 持久化**；设计包自身 `authority.implementation=false`、DAG 全部 `token_budget=OWNER_TO_AUTHORIZE`、`human_gate=true`。
- 依指令 §二/§十三：**TF-01 未获完整授权，本轮不写任何测试/产品文件、不派发 Worker**；本报告即 §十三.4 要求的执行前检查产物。
- 同时以真实命令重放并**扩大确认了 DSH-A R1 缺陷（VERIFIED_DEFECT / observed）**：缺陷工件不止 #5920/#5921，`c01-09`/`c01-14`（PR #5919/#5922）同样携带恒真版 `ca01_commit_frontier.rs`。整改属于 TF-01 范围，等待授权。

## 1. TF-01 开工记录模板（§三 格式，状态：未开工）

```text
task_id            = TF-01
base_sha           = e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0  (已验证存在，且是设计 worktree HEAD 的祖先=其本身)
branch             = (未创建；拟 dsh-b/tf01-s1-classification)
worktree           = (未创建；拟 .worktrees/dsh-b-tf01)
source_agent       = DSH-B
source_run         = TF-PREFLIGHT-20261121-R1
timestamp          = 2026-11-21T04:35:00+08:00
allowed_paths      = tests/integration/test_factory/ ; tools/contract-test-domain/ ;
                     docs/releases/v4.1.0/refactoring/evidence/test-factory/TF-01/   (来自 09_DAG)
forbidden_paths    = src/ ; crates/*/src/ ; .gitea/ ; scripts/gate/ ; STAGE.yaml
budget_authorization_ref = MISSING —— DAG 为 OWNER_TO_AUTHORIZE；运行态 budgets 仅有
                     DSH-B-SRG-CALPHA01 batch1(400k, released 2026-11-21T02:34) 属 C-ALPHA-01 工作包范围，
                     未对 TF-01 释放
human_gate_ref     = MISSING —— 09_DAG human_gate=true，无运行态批准记录
dependencies       = 无（TF-01 为根）；但 S1 三例先前归 Codex 裁决，现设计包 04/14 已给出正式
                     分类（GAP_PROBE/GAP_PROBE/INEFFECTIVE_TEST），与 Owner 指令一致，可作 TF-01 输入
```

## 2. 设计包只读核验（§一）

| 项 | 结果 |
|---|---|
| 位置 | `.worktrees/codex-test-factory-design-20261121/`（分支 `codex/test-factory-design-20261121`，HEAD=`e9ff21fca3`，恰为 product_base） |
| SHA256SUMS | **15/15 OK，rc=0**（00～14 全部；SHA256SUMS 自身 sha256=`b08b0641…294058`） |
| 计划文档 | `docs/plans/2026-11-21-test-factory-implementation.md` 存在（自引用 DAG，声明未获实施授权） |
| 基线锚点 | `45da6968ae`/`b681b74ca3` 均为本地对象存在；`e9ff21fca3` = worktree HEAD |
| 14_HANDOFF_MANIFEST | `status=DESIGN_COMPLETE_IMPLEMENTATION_NOT_AUTHORIZED`；`authority`: design=true，implementation/worker_dispatch/ci_or_gate_change/manifest_resign/pr_merge/stage_change 全 **false**；`files_digest=PENDING`；`verified_red_count=null`（原因：本轮未独立重跑 17 候选） |

⚠️ 风险登记：设计包全部文件目前 **未 commit、未 push**（`git status` 显示 `??`），仅存在于 Codex 本地 worktree。Codex 须提交至 `codex/test-factory-design-20261121` 并推 `gitea252`，`files_digest` 才能脱离 PENDING；在此之前该包存在漂移风险，DSH-B 不搬运、不代提交。

## 3. 运行态授权查询（§二.1 / §十三.2）

- `scheduler.db` tasks：`TF` 关键词 **0 命中**；triage 无 TF 条目。
- `delegations`：`DSH-B-SRG-CALPHA01` 有效（batch1 400k 已释放），但授权动作集合无 TF 包语义，预算未按 TF 包划分。
- `09_IMPLEMENTATION_TASK_DAG.jsonl`：TF-01～TF-07 全部 `token_budget=OWNER_TO_AUTHORIZE` 且 `human_gate=true`（13 号文档第 9 条：未授权不得自行填数或派发）。

**缺 §二 条件 1 与 3 → 只能准备材料。**

## 4. R1 缺陷重放（DSH-A `DSH-A-IREVERIFY-20261121-001` 交办，DSH-B 独立复现）

claim 复现（全部实测，非引用）：

- `git branch --contains 334ccf423a` → 仅 `c01-09`、`c01-14`（+对应 gitea252 远程）。`c01-05/07/10/11` 均不含。
- `e5cc6b56d2` 同样仅含于 `c01-09`/`c01-14`；`d0b1157b62` 仅含于 `c01-07`。

工件版本漂移矩阵（sha256 前 16，完整值见 §8）：

| 分支（=PR head） | ca01_commit_frontier.rs | ca01_backend_caps.rs |
|---|---|---|
| c01-05 (f48073a027) | 文件不存在（无缺陷面） | 不存在 |
| c01-07 (d0b1157b62, PR#5918) | **72f7afda** = 静态契约修订版 | 不存在 |
| c01-09 (334ccf423a, PR#5919) | **c688a673 = 恒真版** ⚠️ | 8d1b9939 = 修订版 |
| c01-10 (3949d6c1be, PR#5920) | **c688a673 = 恒真版** ⚠️ | **ca856f25 = 空函数版** ⚠️ |
| c01-11 (bfafe98d8b, PR#5921) | **c688a673 = 恒真版** ⚠️ | **ca856f25 = 空函数版** ⚠️ |
| c01-14 (ec1bb97cb3, PR#5922) | **c688a673 = 恒真版** ⚠️ | 8d1b9939 = 修订版 |

- 恒真版特征：`c01_07c_durable_frontier_concept_absence_documented` 内 `let debug = format!("{:?}", type_name::<FileStorage>())` 恒真断言（c01-14/09/10/11 实测 grep 命中）。
- 空函数版特征：`c01_09a_*_absence_documented` / `c01_09c_*_honesty_observed` 空函数体仅注释（c01-10/11 实测命中）。
- 06 映射钉住的是修订版 `72f7afda…`（TF-T4/CA01-07 行）与 `8d1b9939…`（CA01-09 行）→ **开放 PR #5919/#5920/#5921/#5922 head 上的工件与映射钉住版本不一致**，属 TF-04 预警的漂移陷阱，实例级证据已固化。

verdict=**VERIFIED_DEFECT**；evidence_class=**observed**；invariant_id=SRG-RED-ARTIFACT-PROPAGATION（DSH-A 原 id：SRG-RED-ARTIFACT-NOT-SHIPPED，本重放确认并扩大范围）；base_sha=e9ff21fca3。
整改（cherry-pick/重建修订工件到 4 个分支并刷新 PR head）属于 **TF-01 范围**，未获 §二 授权前不执行。
注：`c01_09c_wrapper_capability_attestation_must_be_observable` 在 8d1b9939 修订版中仍为 `None` 恒真探针——这是 Owner/设计包认定的 INEFFECTIVE_TEST（TF-S1-003），按先前属主裁决保持原样等待 Codex/TF-01 处置，不计新缺陷。

## 5. 本轮实际完成的修改（低风险格式化，属既有 C-ALPHA-01 协调范围）

- **PR #5922 body 补五锚点**（SPEC/TEST_PLAN/TEST_DESIGN/REVIEW/ACCEPTANCE + 指向 `docs/releases/v4.1.0/refactoring/evidence/ca01/C01-14/PR-C01-14_*.md`，徽章保持末尾）。Gitea API PATCH 成功，body 1324→1966 字符。
- 门禁复跑：`bash scripts/gate/check_pr_template_sections.sh --pr-body /tmp/prb4_5922.md` → `present: 5 / missing: 0 / ✅ PASS`。
- 未触碰任何产品/测试源码、CI、门禁、基线、STAGE、Issue、分支保护。

## 6. 未闭项与风险

1. TF-01～TF-07 运行态授权（预算、allowed_paths、HUMAN_GATE 记录）未持久化 → **BLOCKED_AUTH**。
2. 设计包未 commit/push，`files_digest=PENDING` → 需 Codex 提交并推 gitea252。
3. R1 工件漂移：4 个开放 PR head 携带旧缺陷工件 → 待 TF-01 授权后整改；在此之前 Reviewer 审 #5919-#5922 时应知悉。
4. `BLK-a4906467d1`（双控制面 Hub 并发写，human_gate=1）仍 OPEN → bus 消息一律走 `hub` CLI。
5. G5 网络层 FAIL、SRG-D004-01 维持 BLOCKED；06 映射 6 行 `UNVERIFIED_HISTORY`（TF-T4-001..004、PILOT-T5-101、D-002B）与 source_sha=None，属 TF-04 待验输入，不得当作事实。

## 7. 复现命令

```bash
cd /Volumes/workspace/dev/sqlrustgo
git branch --contains 334ccf423a
git show dsh-b/c01-11-ca01-bufferpool:tests/integration/test_factory/ca01_commit_frontier.rs \
  | grep -n -A8 'fn c01_07c_durable_frontier_concept_absence_documented'
git cat-file blob dsh-b/c01-10-ca01-complex-dml:tests/integration/test_factory/ca01_backend_caps.rs | shasum -a 256
cd .worktrees/codex-test-factory-design-20261121/docs/releases/v4.1.0/refactoring/test-factory-design
shasum -a 256 -c SHA256SUMS
bash scripts/gate/check_pr_template_sections.sh --pr-body /tmp/prb4_5922.md
```

## 8. 证据哈希

- frontier 恒真版 `c688a673aaefda51b5023f0f3ad3d3a6846e3cee84a784820ed875252c2f1525`
- frontier 修订版 `72f7afda397e08c565c5d89928dc5bd629d46adbe4a27880b6e7c958dac1a16d`
- caps 空函数版 `ca856f255bd36da825a1c1dd67a0c2a67c115263db7f6fecbde37e1405f234d9`
- caps 修订版 `8d1b99397a589e5d588dd3fcf9a1e197b82aa14ce7f140bb855d0b4d88c0d6da`
- SHA256SUMS 文件 `b08b064164256c84408d0ab12ea1f65ed91b9e36dde32c20df5b58051b294058`
- 14_MANIFEST `bb638c051773d019fd61167a210ccf6e919477ba7a71ba9bf25c5869eabe455c`
- 09_DAG `518292b361161089f516619e3cd8a77c4c019bea408c0790d9ba15d1b2cad60d`
- 本报告自身 sha256 于 bus 消息内给出（避免自引用悖论）。

## 9. 下一步与所需授权

1. **DSH-A/Owner**：在运行态持久化 TF-01 授权（token 预算按 §二.1；引用 `DSH-B-SRG-CALPHA01` 或新授权；记录 allowed_paths 与 human_gate 批准）。
2. 授权后 DSH-B：从 `e9ff21fca3` 建 `.worktrees/dsh-b-tf01` + 分支 `dsh-b/tf01-s1-classification`；按 04 号规格先写 mutation 反例（名称注入/虚报能力/固定 None 必须非绿），再写行为契约与 Gap Probe `NOT_IMPLEMENTED` 出口；含 R1 工件传播整改；证据落 `evidence/test-factory/TF-01/`；单 commit（中文 conventional）；是否 push/PR 按该包运行态授权执行。
3. **Codex**：提交并推送设计包（终结 files_digest=PENDING）；确认 06 映射钉住版本与分支漂移的处置口径。
4. **Reviewer**（独立 Reviewer，非 252/8021 同族；候选 250:8011 异构但 GMP 预留，须 DSH-A 调度决定）复验 TF-01 后，TF-02 才具备启动条件。

DSH-B 不自签任何验收；未获运行态授权前不写测试/产品文件、不派发 Worker。

## 10. 补记（06:45 复查）：自动巡检审批缺口（observed）

- `GET /pulls/{n}/reviews` 实测：#5918×3、#5919×3、#5920/#5921/#5922 各 1 条 **APPROVED**，全部由账号 `claude-macmini` 于 2026-11-21T02:18–03:54+08:00 提交，正文同文：`[巡检自动审阅] CI=none；审阅判据全绿（mergeable/非 draft/非本人 authored/无 REQUEST_CHANGES/分支保护已满足）`。
- 判定：这是**形式判据的自动批准**，不含任何测试内容审阅；发生在 R1 暴露（03:55）之前。按 ADR-014 角色分离与属主"独立 Reviewer 复验"要求，这些 APPROVED **不得**被当作技术评审完成，也不应作为解除 HUMAN_GATE 的依据。
- 影响：巡检 bot 使 4 个携带缺陷工件的 PR（#5919/#5920/#5921/#5922）显示"已批准"，存在被误合并风险。DSH-B 已在受影响 PR 发布 R1 警示评论（06:50，见各 PR 时间线），并建议 DSH-A：① 将巡检自动 APPROVE 降级为 FORMAL_CHECK_PASSED 性质或删除其权威态；② OBL-REVIEWER-DISPATCH 的技术评审必须走异构 Reviewer 实审。
- #5918 工件核验：`ca01_commit_frontier.rs@head=72f7afda`（修订版）、`file_undo/snapshot_reopen` 与映射一致，**无缺陷工件**；其合入仍受 ADR 绑定空引用门禁与 HUMAN_GATE 约束。
- V78-TF-R1（04:34 发出）状态仍 `pending`，DSH-A 未消费；设计包仍未 commit/push。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

# v4.1.0 重构就绪整备 —— 当前状态与续跑指引

> **本文件是本计划的记忆锚点。** 新会话（无论是重启后恢复，还是别的 agent 接手）
> **先读本文件**，再读同目录的 `V410-READINESS-LEDGER.md`（逐条裁决台账）。
> 设计论证在 `V410-REFACTOR-READINESS-DESIGN.md`，实施计划在
> `V410-READINESS-IMPLEMENTATION-PLAN.md`。
>
> 最后更新：2026-11-25

---

## 0. 一句话状态

**9 个计划任务已完成 8.5 个**（Task 1–8 + Task 10 全部通过独立评审；Task 9「放行判据汇总 +
R2 外审包 + 属主放行裁决」**尚未开始**）。当前**不能**宣布放行，也不该宣布「整备失败」——
整备把材料层修好了，但**证据层**和**签署层**仍有缺口，缺口已逐条量化并登记。

---

## 1. 事实锚点（不要重新推导，直接采信）

| 项 | 值 |
|---|---|
| 承载分支 | `base/v4.1.0-frozen` |
| 隔离 worktree | `/Volumes/workspace/dev/wt-v410-base` |
| **BASE（内容基线提交）** | `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` |
| BASE 的 tree | `72d378e8e58797eae18502ab474325015b8993d4` |
| BASE 的 parent（develop tip） | `8cbde5b6d33c876fa1197474c46d50142a9823bd` |
| 冻结记录（唯一基线身份） | `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` |
| handoff manifest | `docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json` |

> **`base_sha` 一律填 BASE，不随提交前移。** 这是 R-BASE，Task 2 之后所有前向绑定都钉死在这里。

---

## 2. 已完成的任务与各自的证据锚点

| 任务 | 做了什么 | 证据锚点 | 状态 |
|---|---|---|---|
| **Task 1** | 唯一基线 + 123 个材料文件导入 + 冻结记录 | `BASELINE_FREEZE.json`、`tools/verify_baseline.py` | ✅ 评审通过 |
| **Task 2** | 前向/历史 SHA 分界重指 | `tools/verify_sha_scope.py`（S1–S6） | ✅ 评审通过 |
| **Task 3** | `verify_handoff.py` 断言闭包（ADR 档位 / manifest / 变异臂） | `tools/verify_handoff.py`、`tools/test_verify_handoff.py` | ✅ 评审通过 |
| **Task 4** | 11 份 ADR 的机器字段落地（档位记入正文） | `takeover/dsh-b/ADR-DECISION-RECORD.md` | ✅ 评审通过 |
| **Task 5** | 75 场景矩阵修复 + 矩阵驱动层 + 状态词汇 | `TEST_COVERAGE_MATRIX.json`、`tools/matrix_driver.py`、`tools/verify_matrix.py` | ✅ 评审通过 |
| **Task 6** | L1 切片实跑（44 条）+ 双向差异表 + C5 绑定 | `refactoring/evidence/L1-RERUN-*.json` / `L1-DIFF-*.md` / `L1-BINDING-*.json` | ✅ 评审通过 |
| **Task 7** | Oracle 逐条判定（**判定完成，签署 BLOCKED**） | `takeover/ORACLE-REVIEW-LOG-T1.md` | ✅ 实质交付通过 |
| **Task 8** | ALPHA 阶段执行点 + 7 道负向夹具 | `.gitea/workflows/ci-alpha-stage.yml`、`audit/v4.1.0/neg/fx*.sh` | ✅ 修订 2 后通过 |
| **Task 10** | 逐用例重做 invariant 映射 + 修复 38 条不可判定断言 | `TRANSACTION_INVARIANTS.md:30`、`ORACLE-REVIEW-LOG-T1.md:505-511` | ✅ 评审通过 |
| **Task 9** | 12 条判据汇总 + R2 外审包 + 属主放行裁决 | — | ❌ **未开始** |

---

## 3. 唯一剩余任务：Task 9

Task 9 产出三件东西：

1. `docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json` —— 12 条判据逐条的
   `id` / `criterion` / `command` / `observed` / `verdict`；
2. `docs/refactor/v4.1.0-c/takeover/R2-RELEASE-PACKAGE.md` —— R2 外审包生成说明；
3. `docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` —— 属主放行/不放行的书面裁决
   + 豁免记录（`waiver_id` / `waived_criterion` / `reason` / `scope` /
   `compensating_control` / `expires_at` / `approved_by`）。

12 条判据的清单与命令见 `V410-READINESS-IMPLEMENTATION-PLAN.md` 的 Task 9 Step 1 表格。
**其中判据 7 的检索范围已由控制者扩到 `.gitea/workflows/*.yml`**（原因见台账 R52；
判据强度不变：仍要求真实执行、非 `--dry-run`、退出码可传播、触发条件可指认）。

R2 外审包在 `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0/` 下生成；**R1 的 `package/` 一个字节都不许动**。

---

## 4. 已定且**不可被后续会话重新推翻**的结论

> 下面每一条都经过独立评审，并且有可复算的证据。**续跑时不要重新论证它们**；
> 要改必须带新证据。

- **R47**：`GATE_CATALOG.jsonl` 的 `neg_fixture` 原义（门禁**自带** selftest）不变，
  「有外部夹具」是**另一个**字段 `external_fixture`；基准是台账
  `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` + `audit/v4.1.0/neg/fx*.sh` 实物。
- **R48**：7 道 ALPHA 门禁里**只有 1 道**（`check_anti_ignore_gate.sh`）原本就有夹具；
  新造的是 `fx15`–`fx20`（+ 表征夹具 `fx21`/`fx22`）。
- **R49**：每道夹具的缺陷规格来自台账的 `target` 字段，不得自创。
- **R50**：**超时（124）/信号不算「门禁变红」**，只能报 INCONCLUSIVE；宁可 6/7 也不凑 7/7。
- **R51**：CI 执行点用 `check_stage.sh`（两态）；三态 runner 的**执行**接线属 kanban IL-7。
  代价：ALPHA 执行面**丢失 INCOMPLETE 语义** ⇒ Task 9 必须登记为残余项。
- **R52**：ALPHA 执行点**不由 push 触发**，落在独立 workflow
  `.gitea/workflows/ci-alpha-stage.yml`（`workflow_dispatch` + 独立 concurrency 组 +
  `cancel-in-progress: false`）。**不能放进 `ci.yml`**：它的 concurrency 组带
  `cancel-in-progress: true`，任何非 push 触发器都会取消别人正在跑的 run。
- **R53**：ALPHA 执行点**只允许有一处**。
- **R54**：两层 RED 必须来自**同一次** `run_gates.py execute` 的**同一份** `results.jsonl`；
  汇总层必须是 `gate_report.py` 的**独立重解析**；目标门禁必须在汇总里被标为 FAIL（因果闭合）。
- **R55**：新夹具落 `audit/v4.1.0/neg/fxNN-<gate>-<缺陷类型>.sh`，并**必须**在台账登记。
- **R56**：本计划**不新增门禁、不改 `STAGE_CONFIG.yaml`、不改 168 分母**。
- **R57**：证据 run 的 manifest 必须叫 `manifest.yaml`；新增的 run 路径在冻结记录里
  **显式登记**（`historical_set_additions`），S5 的期望集 = 冻结集 ∪ 登记增补集。
- **R58**：机器可读的声明（台账行、`effect.catches`）**只允许写夹具真正证明的性质**；
  证明反面时写成 **known hole**。
- **R59**：台账是**生成物**——`audit/v4.1.0/gen_neg_ledger.py` 必须能逐字节复现已提交的台账。
- **R60**：端到端跑过一次 ALPHA（27m06s / 退出码 1 / 13 PASS 4 FAIL），**不得用 dry-run 冒充**。
- **R61**：夹具泳道的第四种期望值 `inconclusive` **被接受**（它无法冒充红）。
- **R40–R42**：Oracle 签署人必须是**属主授权的独立 Reviewer**；实施者不得自签；
  不可独立判定的断言**不得**为凑数而签署。
- **R43–R46**（Task 10）：`invariant_ids` 必须**逐用例**从**测试源码**派生；
  宁可留 13 条零覆盖，也不保留无法复核的覆盖。

---

## 5. 必须由**属主**裁决的事项（Task 9 汇总时逐条列出）

| # | 事项 | 阻塞什么 |
|---|---|---|
| O1/O3/O4/O5/O10/O11/O12/O14/O15/O17 | spec §6.2 的待裁决项 | 各自对应判据 |
| **N1** | 在基线里注册 `[[test]] tf_t1_undo_invariants`，vs 把 `PILOT-T1-101/102` 记为不可认证 | L1 切片能否作验收凭据 |
| **N2** | `assembly=UNVERIFIED` 的场景可否记 PASS —— 两种读法并列（DSH-B 字面读法下「42 PASS」不可作验收凭据，而该切片不含任何 RED） | L1 切片的证据效力 |
| **N3** | ledger #10/#11 的 `source_sha256` 滞后一次改写 | 台账自洽 |
| **N4** | ledger 实为 18 行而其正文自称 17 | 台账自洽 |
| **R-T7-1** | Oracle 副签人身份 —— 目前**无可用 Reviewer** ⇒ Oracle 步骤 **BLOCKED** | 判据 6 |
| **R-T7-2** | 第 2 段签署是在错误指示下进行的（3 条已撤回） | 判据 6 |
| **R-T7-3** | `ORACLE-REVIEW-LOG-T1.md` 在 manifest 哈希范围之外 | 判据 5/6 |
| **GAP-13** | 两条 R3 测试所需能力在冻结基线上不存在 | 判据 6 |
| **GAP-14** | 3 条用例的原映射为标签派生，已撤下 | 判据 6 |
| — | **13 条不变量零覆盖**（INV-01/02/04/07/14 之外的） | 「可开始重构」的实质差距 |
| — | 3 条断言仍不可独立判定（T1-009 / T3-002 / T3-003） | 判据 6 |

**R60 的冷 target 数据缺口**：端到端 ALPHA 在**热 target** 下 23m58s 跑完，但冷 target
（CI 的真实情形）下 composite 的 release 构建要重跑，90 分钟预算两次撞墙，
**180 分钟（workflow 的 `timeout-minutes`）才跑完**。冷 cache 下 180 分钟仍可能紧张 ——
Task 9 必须写进裁决记录。

---

## 6. 续跑指令（新会话照此接手）

1. `cd /Volumes/workspace/dev/wt-v410-base`，确认分支 `base/v4.1.0-frozen` 与 tip。
2. 读本文件 + `V410-READINESS-LEDGER.md`（**不要**读实施计划全文，除非要做 Task 9 的判据表）。
3. 起一个 Task 9 的实施者（subagent），把
   `V410-READINESS-IMPLEMENTATION-PLAN.md` 的 Task 9 段落 + 本文件的 §3/§4/§5 交给它。
4. Task 9 的产物要**过一遍独立评审**（与前 8 个任务同一标准）。
5. Task 9 完成后：把 `V410-READINESS-VERDICT.json` 的 12 条 `verdict` 与
   `V410-OWNER-RULING.md` 一起交给**属主**，由属主裁决放行/不放行。
6. 若中途再次 compact：**以本文件与 git log 为准**，不要凭记忆。

### 过程脚本（不在 git 里，随手可重建）

| 路径 | 作用 |
|---|---|
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/progress.md` | 原始 ledger（含全部裁决行与评审记录） |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-*-brief.md` | 各任务的计划正文切片 |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-*-report.md` | 各任务的实施者报告 + 修订记录 |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/review-*.diff` | 评审包（**可删**，可由 git 重新生成） |

`.superpowers/sdd/` 是 git-ignored 的临时区，**不是记忆**；记忆在仓库里（本文件 + 台账副本）。

---

## 7. 记忆边界

- 本文件记录**结论**，不记录**过程流水**；过程在 ledger 与 task 报告里。
- 凡是需要重新跑一遍才能确认的数字（覆盖率、通过率、耗时），本文件只给**当时的实测值**
  与**当时树的身份**，不声称它们现在仍然成立。
- 任何与本文件矛盾的**新**结论，必须附带**可复算的证据**并更新本文件 —— 否则以本文件为准。

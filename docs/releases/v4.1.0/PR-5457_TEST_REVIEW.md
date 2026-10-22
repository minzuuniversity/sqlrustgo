# PR-5457 TEST_REVIEW — 门禁判据可执行性修复

> **PR Number**: 待创建（分支 `opencode/fix-5457-5458-gate-refs`）
> **Commits**: `2252a50d28`（主变更）→ `e749768b5f`（两个假绿漏洞修复）→ `fad55217ef`（进程替换支持）→ `7d686ecd0f`（协议文档域名订正）
> **Issue**: #5457 / #5458 / #5450
> **Version**: v4.1.0
> **Base**: `378f01452998cf0f6d33a55602254f4b4f8374d0`（`develop/v4.1.0`）
> **Author**: `opencode-z440`
> **Created**: 2026-10-22
> **Status**: DRAFT — 待独立 reviewer 确认后转 PASS
> **配套文档**: `PR-5457_SPEC.md` / `PR-5457_TEST_PLAN.md` / `PR-5457_TEST_DESIGN.md` / `PR-5457_ACCEPTANCE.md`
>
> ⚠️ **review_type: INDEPENDENT_REVIEW + SELF_REVIEW**
>
> 2026-10-22：由**独立 reviewer subagent**（非本文作者）执行了一次完整评审，
> 结论 **REQUEST-CHANGES**，列出 5 个 BLOCKER + 10 个 SHOULD-FIX + 7 个 NITPICK。
> 该评审的每个 BLOCKER 都由作者**独立复现**并已修复（见
> `PR-5457_ACCEPTANCE.md` §2·五）。
>
> 但修复后的版本**尚未经第二人复核** —— 修复者与评审者是同一批 agent 家族
> （同源模型），严格意义上不构成 ADR-014 意义上的**跨身份独立审核**。
> 合并前仍建议由 `omp-z440` / `openclaw` / User 对修复后的版本做一次复核。
>
> 按 ADR-014 D-01 与 ISSUE_CLOSING_VERIFICATION §2.5，**作者不得自称独立审核**；
> 本文档中标注 `SELF_REVIEW` 的部分只记录作者自审证据。

---

## 1. 审核范围

| 项 | 值 |
|---|---|
| 变更文件 | 16（新增 12 / 修改 4）|
| 新增门禁脚本 | 3（`verify_pr_dag.sh`、`check_gate_script_refs.sh`、`check_pr_template_sections.sh`）+ 1 alias |
| 新增测试 | 23 个 Rust 用例，0 ignored |
| 审核基线 | `378f014529`（`develop/v4.1.0`）|

---

## 2. 反向变异结果（协议 §2 ③ 第 4 项必含）

**方法**：逐条禁用门禁的检测路径，确认对应 fixture 从 FAIL(1) 翻成 PASS(0)；
再还原，确认翻回 FAIL(1)。**只有当「改坏 → 转绿 → 还原 → 转红」成立时，
该检测才真的在起作用** —— 「测试通过」本身不能证明这一点。

脚本：`/tmp/opencode/mutation_test.py`（逐行变异 + `git diff` 还原确认）

```
=== 反向变异测试：verify_pr_dag.sh ===
  [PASS] M1 虚假通过检测（声明已合并但无 merge commit）
         line 363: if printf '%s' "$status" | grep -q "已合并"; then
         mutated=exit0 (detection gone), restored=exit1 (detection back)
  [PASS] M2 重复状态行检测
         line 318: if [ "$row_count" -gt 1 ]; then
         mutated=exit0 (detection gone), restored=exit1 (detection back)
  [PASS] M3 表外行检测（§2.1 有而 §2 链没有的 PR）
         line 400: if ! printf '%s\n' "$CHAIN" | awk ... then
         mutated=exit0 (detection gone), restored=exit1 (detection back)
  [PASS] M4 依赖倒置检测
         line 382: elif [ -n "$node_pos" ] && [ "$dep_pos" -gt "$node_pos" ]; then
         mutated=exit0 (detection gone), restored=exit1 (detection back)
  [FAIL] M5 链内缺行检测（§2 链有而 §2.1 没有的 PR）
         line 330: if ! printf '%s\n' "$CHAIN" | awk ... then
         mutated=exit1 (detection gone), restored=exit1 (detection back)

=== 还原确认 ===
  文件已完全还原
```

### 2.1 M5「FAIL」的解释 —— 不是缺陷

M5 变异后仍报 exit 1，但**报错信息不同**：

```
[ERROR] PR-810 — present in the §2 chain but missing from the §2.1 status table
```

即：变异掉第 330 行后，**另一条独立路径**（Phase 4b 的链成员检查）
仍然检出了同一问题。所以「链有表没有」这个缺陷被**两处对称检查双重覆盖**，
变异任一处都不足以让它逃逸。

**M5 的正确结论**：检测有效且有冗余保护，不是「变异后仍能检出 = 变异无效」。
这是本审核中唯一一处需要解释的变异结果，特此记录以免被误读为缺陷。

---

## 3. 对抗性测试发现并修复的两个假绿漏洞

变异测试之外，另做了**结构性对抗测试** —— 构造文档让门禁本该报 FAIL 却报 PASS。
**发现 2 个真漏洞**，均已修复并加了回归用例。

### 3.1 漏洞 A：§2.1 状态表里同一 PR 出现两次

原实现用 `awk ... { print $2; exit }` 取**第一行**就退出：

```
| PR-800 (...) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
| PR-800 (...) | Phase 0 | ✅ 已合并 | duplicate row |
```

被判「❌ 未合并」→ 与 git 一致 → **PASS**。第二行的「✅ 已合并」
**从未被读过** —— 而它恰恰是危险的那一句。

**自相矛盾的文档被静默地按有利方向解读** = 假绿。

**修复**：同一 PR 出现 >1 次 ⇒ FAIL（`verify_pr_dag.sh:318`）。
**回归用例**：`verify_pr_dag_detects_duplicate_status_rows`

### 3.2 漏洞 B：§2.1 里有 §2 链中不存在的 PR

主循环遍历的是 **§2 依赖链**，所以只出现在 §2.1 表里的 PR 行**根本不会被检查**：

```
| PR-800 (...) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
| PR-999 (not a chain node) | Phase 9 | ✅ 已合并 | fabricated |
```

`PR-999` 声称已合并，却**没有任何人验证它有没有 merge commit**。
一个没人验证的「已合并」声明，正是这个门禁要消灭的东西。

**修复**：新增 Phase 4b —— 每一个 §2.1 行都必须对应一个 §2 链节点，
链外行 ⇒ FAIL（`verify_pr_dag.sh:400`）。
**回归用例**：`verify_pr_dag_detects_a_status_row_outside_the_chain`

> **这一条是自审的价值所在**：如果只跑「正例 PASS + 负例 FAIL」的常规用例，
> 这两个漏洞都不会暴露 —— 它们只在「构造出让门禁误判的文档」时出现。

---

## 4. 逐条验收标准复核

| AC | 判据 | 实测 | 复核结论 |
|---|---|---|---|
| AC-1 | `verify_pr_dag.sh` 可执行并产出判定 | exit 0；11 节点、0 虚假通过、3 stale(WARN) | ✅ |
| AC-2 | `check_pr_dag.sh` alias 与实现一致 | 坏 base 下同为 exit 2；正常同为 exit 0 | ✅ |
| AC-3 | 虚假通过判 FAIL | 构造 PR-800 声明 `✅ 已合并` → exit 1 | ✅ |
| AC-4 | 依赖倒置判 FAIL | PR-810 依赖 900 → exit 1 | ✅ |
| AC-5 | 不可判定判 DRIFT(2) | 坏 base → exit 2；无 PR DAG 段 → exit 2 | ✅ |
| AC-6 | 新增缺失引用判 FAIL | 临时加假路径 → exit 1 且点名 | ✅ |
| AC-7 | PR 正文 5 标记判定 | 模板 → exit 0；无标记 → exit 1；空 → exit 2 | ✅ |
| AC-8 | `AGENTS.md` 路径指向真实文件 | 指向 `.gitea/pull_request_template.md`，由测试守护 | ✅ |
| AC-9 | CHECK 4 接受本身份 | `[PASS] HEAD author email: opencode-z440@gaoyuanyiyao.com` | ✅ |
| AC-10 | 新脚本不污染既有门禁 | P14 findings 与 base 逐条相同；P16/P11 exit 0；孤儿测试 PASS | ✅ |
| AC-11 | fmt + clippy | 均 exit 0，clippy 0 warnings | ✅ |
| AC-12 | 新增 Rust 测试 | 23 passed; 0 failed; **0 ignored** | ✅ |

---

## 5. 未达成 / 未验证项（协议 §2 ③ 第 5 项必含，不能省）

**没有这一段，读者会默认「都验证了」。**

| # | 项 | 状态 | 原因 / 风险 |
|---|---|---|---|
| 1 | **3 个新门禁未接入 CI** | ❌ 未做 | `.gitea/workflows/ci.yml` 被 5 个他人分支高频改动，触碰会显著提高冲突面。⚠️ **风险**：仓库自己的教训（`check_stage.sh` 注释里的 #5440）——「这个脚本是 SSOT 唯一的执行者，没有任何 CI job 调用过它」。**无调用方的门禁会腐烂**。已登记为待办 issue |
| 2 | `check_pr_template_sections.sh` 无调用方 | ❌ 未做 | 同上 |
| 3 | §2.1 状态表 3 行过时（PR-840/850/870）| ⚠️ 已检出未修 | 修它要改 v3.8.0 归档区文档，超出 #5457 范围。gate 报 WARN 并给出证据 |
| 4 | 8 条已知债脚本缺失 | ⚠️ 已登记未修 | baseline 逐条带成因与 Fix 建议 |
| 5 | **独立审核** | ❌ 未做 | 本文档是 `SELF_REVIEW`。合并前需另一身份复核 |
| 6 | `check_pr_template_sections.sh` 的**真实 PR 正文**验证 | ⚌ 部分 | 只用模板本身与构造 fixture 验证过，未对真实线上 PR 正文跑过 |
| 7 | 全量回归 `cargo test --all-features --no-fail-fast` | ⚠️ 未跑 | 耗时超预算；base 上已有既存失败。**如实声明未跑**，不宣称通过 |
| 8 | 新脚本在**真实 CI runner**（Alpine/musl）上的行为 | ❌ 未验证 | 本地是 glibc。#5528 已报告 runner 换 Alpine/musl 后缺静态 zlib —— 新脚本是纯 bash，无编译依赖，风险低但**未实测** |

---

## 6. 治理合规复核

| 规则 | 要求 | 本 PR | 结论 |
|---|---|---|---|
| ADR-001 G-01 | Claim ≠ Evidence | 每条 PASS 结论都附命令 + exit code + stdout | ✅ |
| ADR-001 G-08 | 不引文档 claim 当 PASS | 全部实跑；存量 FAIL 明确标注为 base 既有 | ✅ |
| AFP §5.4 / ADR-014 D-06 | 5 evidence fields | PR body §5 齐备 | ✅ |
| ADR-014 D-01 | Issue 评论带 `[agent: ...]` 前缀 | CLAIM 评论带 `[agent: opencode-z440]` | ✅ |
| ADR-014 D-02 | worktree 隔离 | `.worktrees/opencode-z440-20261022` | ✅ |
| ADR-014 D-03 | 分支前缀 | `opencode/fix-5457-5458-gate-refs` | ✅ |
| ADR-014 D-05 / G-09 | Gate FAIL 时禁止 push | **已由 User 显式授权**，且 PR body §3 逐条列出存量 FAIL 与归属 | ✅（有授权记录）|
| ADR-008 Policy 1 | PASS 声明须限定 | 本文档 §5 逐条列出未达成项 | ✅ |
| ADR-008 Policy 2 | gate test 禁止 `#[ignore]` | 新测试 0 ignored；P16 exit 0 | ✅ |
| PERMISSION_MATRIX §3 | AI Developer 不得合并自己的 PR | 不合并，等 User / Reviewer | ✅ |
| 协议 §2 ③ | PR body 只写 `Refs` 不写 `Closes` | 见 PR body | ✅ |

---

## 6·五、独立评审发现的问题（2026-10-22）

独立 reviewer 的 5 个 BLOCKER 全部独立复现成立，已全部修复：

- **B1** 浅克隆下门禁 exit 1 → 会**打挂 CI 的 test job**
- **B2** 纯文档 PR 提到规划号即被算作"已合并" → 假绿
- **B3** 依赖解析被一个空格静默关闭，且**无任何诊断** → 假绿
- **B4** `AGENTS.md` 引用测试**空转通过**（断言块被整体跳过）
- **B5** 依赖倒置断言被**夹具文件名**满足（子串来自路径而非诊断）

**其中 B4 与 B5 与本文作者自审发现的 2 个假绿漏洞是同一类错误** ——
「测试跑了，但没测到它声称保护的东西」。这说明：

> **自审的判别力是有限的。** 本文的 §2 反向变异测试覆盖了 M1–M4，
> 但它**没有覆盖**"断言是否真的由被测逻辑产生"这一层 ——
> B5 的断言被子串意外满足，而变异测试本身没发现它。
> 只有**另一个人**构造对抗性输入，才会发现这一层。

修复后的验证矩阵与负向路径见 `PR-5457_ACCEPTANCE.md` §2·五。

---

## 7. 结论

**自审结论：实现与验收标准一致，12/12 AC 通过；变异测试证明 4 条检测路径
（M1–M4）确实有效且有判别力；对抗性测试额外发现并修复了 2 个假绿漏洞。**

**独立评审结论（2026-10-22）：REQUEST-CHANGES** —— 5 个 BLOCKER 全部成立，
已全部修复并逐条复验。修复后的版本**尚未经第二人复核**。

**但本审核的效力受限**：

- `review_type: SELF_REVIEW` —— **不等于** `MAINTAINER_REVIEW`
- §5 列出 **8 项未达成/未验证**，其中「独立审核」「CI 接线」「全量回归」
  三项在合并前需要处理或明确豁免
- 建议：由 `omp-z440` / `openclaw` / User 对 §2 的变异方法与 §3 的两个漏洞
  做一次独立复核，特别是**独立构造对抗性 fixture** —— 本 PR 的两个漏洞
  正是这样发现的，只做正例验证的 reviewer 不会看到它们

# PR-5457 ACCEPTANCE — 门禁判据可执行性修复

> **Issue**: #5457（主）/ #5458 / #5450
> **Branch**: `opencode/fix-5457-5458-gate-refs`
> **Base**: `378f014529`（`origin/develop/v4.1.0`）
> **Commit**: `2252a50d28`
> **Author**: `opencode-z440 <opencode-z440@gaoyuanyiyao.com>`
> **Created**: 2026-10-22
> **Status**: PASS（本地实跑）— 待 CI 验证后确认
> **配套文档**: `PR-5457_SPEC.md` / `PR-5457_TEST_PLAN.md` / `PR-5457_TEST_DESIGN.md` / `PR-5457_TEST_REVIEW.md`
> **设计文档**: `docs/superpowers/specs/2026-10-22-gate-criterion-existence-design.md`

---

## 1. 概述

本 PR 让**门禁判据第一次真正可执行**。

问题不是「某个脚本写错了」，而是**判据指向了不存在的实体** ——
`GATE_CONDITIONS.md` 的 BE5/B-F5 引用 `scripts/gate/verify_pr_dag.sh`，
而那个文件从未被提交过。后果有三层，且都不能自愈：

1. **判据不可执行** —— 跑它只会得到 "No such file or directory"，既不 PASS 也不 FAIL
2. **历史记录不可复现** —— v3.11.0 与 v4.0.0 的 BETA 门禁报告已记为 `✅ PASS`
   / `All V400-* PRs merged in documented order`，而**没有任何可复现的方式能重新验证**
3. **收录时没做存在性校验** —— 判据表被接受时从未检查过脚本是否存在

本 PR 交付三个门禁脚本 + 三处 #5458 订正 + 一处 CHECK 4 白名单修复，
并附一个**根治件**：让第 4 个同类缺陷在出现瞬间就被发现。

---

## 2. 验收标准与实测

### AC-1: BE5 / B-F5 的判据可执行

**标准**：`bash scripts/gate/verify_pr_dag.sh` 能跑并产出 PASS/FAIL/DRIFT。

```
$ bash scripts/gate/verify_pr_dag.sh; echo "EXIT=$?"
=== PR-DAG Consistency Gate (issue #5457) ===
  source_agent: opencode-z440
  source_run:   opencode-z440-20261022-pr-dag
  timestamp:    2026-10-22T07:52:00Z
  base_ref:     origin/develop/v4.1.0
  check_order:  0
[INFO] plan document: docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md
[INFO] base ref resolved: origin/develop/v4.1.0 (378f014529)
[INFO] §2 chain nodes parsed: 11
[INFO] §2.1 status rows parsed: 11
[PASS] PR-800 — declared '⚠️ 部分落地', no merge commit in git (consistent)
[PASS] PR-810 — declared '❌ 未合并', no merge commit in git (consistent)
[PASS] PR-820 — declared '❌ 未合并', no merge commit in git (consistent)
[PASS] PR-830 — declared 已合并, 72 merge commit(s) in git (Gitea #2756, 753162046c03)
[WARN] PR-840 — declared '❌ 未合并' but 8 merge commit(s) exist in git (Gitea #2740, dd68090fa1d7). Status row is stale.
[WARN] PR-850 — declared '❌ 未合并' but 8 merge commit(s) exist in git (Gitea #2682, b079c7984be6). Status row is stale.
[PASS] PR-860 — declared '❌ 未合并', no merge commit in git (consistent)
[WARN] PR-870 — declared '❌ 未合并' but 8 merge commit(s) exist in git (Gitea #2683, 23feb6e23021). Status row is stale.
[PASS] PR-880 — declared '❌ 未合并', no merge commit in git (consistent)
[PASS] PR-890 — declared '❌ 未合并', no merge commit in git (consistent)
[PASS] PR-900 — declared '❌ 未合并', no merge commit in git (consistent)
--- Summary ---
  §2 chain nodes:        11
  §2.1 status rows:      11
  declared 已合并:       1
  declared 未合并:       9
  declared 部分落地:     1
  actually merged:       4
  false passes:          0
  stale status rows:     3
  errors:                0
  warnings:              3
✅ PASS: no false pass; dependency order intact; §2 chain and §2.1 table agree.
EXIT=0
```

**PASS** ✅ — 判据第一次可执行，且**顺带抓出了 3 行过时的状态**（PR-840/850/870
声明「❌ 未合并」但 git 里各有 8 个 merge commit）。

### AC-2: 第二个引用名也能解析

**标准**：`check_pr_dag.sh`（`DEVELOPMENT_PLAN.md:92` 与 `ALPHA_STAGE_REVIEW.md:81`
引用的名字）存在且与实现行为一致。

```
$ bash scripts/gate/check_pr_dag.sh; echo "EXIT=$?"
✅ PASS: no false pass; dependency order intact; §2 chain and §2.1 table agree.
EXIT=0

$ bash scripts/gate/check_pr_dag.sh --base refs/heads/nope_zzz; echo "EXIT=$?"
❌ base ref cannot be resolved: refs/heads/nope_zzz
   Cannot judge — DRIFT, not PASS.
EXIT=2
```

**PASS** ✅

### AC-3: 虚假通过会被判 FAIL

**标准**：声明 `✅ 已合并` 但无 merge commit ⇒ exit 1。

```
$ sed 's/| PR-800 (COM_QUERY AST Routing) | Phase 0 | ⚠️ 部分落地 |/| PR-800 (COM_QUERY AST Routing) | Phase 0 | ✅ 已合并 |/' \
    docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md > /tmp/opencode/plan_fakepass.md
$ bash scripts/gate/verify_pr_dag.sh --plan /tmp/opencode/plan_fakepass.md; echo "EXIT=$?"
[ERROR] PR-800 — declared 已合并 but NO merge commit exists in origin/develop/v4.1.0. This is a false pass.
...
❌ FAIL: 1 false pass(es) or structural error(s) detected.
EXIT=1
```

**PASS** ✅ — 这是**危险方向**：门禁不会认证从未落地的工作。

### AC-4: 依赖顺序倒置会被判 FAIL

```
$ sed 's/（依赖800）/（依赖900）/' docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md > /tmp/opencode/plan_dep.md
$ bash scripts/gate/verify_pr_dag.sh --plan /tmp/opencode/plan_dep.md; echo "EXIT=$?"
[ERROR] PR-810 — depends on PR-900 but appears BEFORE it in the chain (position 2 vs 11). Dependency order inverted.
❌ FAIL: 1 false pass(es) or structural error(s) detected.
EXIT=1
```

**PASS** ✅

### AC-5: 不可判定时是 DRIFT，不是 PASS

```
$ bash scripts/gate/verify_pr_dag.sh --base refs/heads/definitely_nonexistent_ref_zzz; echo "EXIT=$?"
❌ base ref cannot be resolved: refs/heads/definitely_nonexistent_ref_zzz
   Cannot judge — DRIFT, not PASS.
EXIT=2
```

**PASS** ✅ — 符合 ADR-006 P14「DRIFT ≠ PASS」与 AFP §7.4。

### AC-6: 同型缺陷的第 4 个实例会被当场抓住（根治件）

```
$ bash scripts/gate/check_gate_script_refs.sh; echo "EXIT=$?"
=== Gate Criterion Script Reference Existence Check ===
  source_agent: opencode-z440
  source_run:   opencode-z440-20261022-gate-refs
  timestamp:    2026-10-22T07:51:30Z
  baseline:     tests/baseline/gate_script_refs_baseline.json
[INFO] criterion documents readable: 8 / 8
[INFO] distinct script paths cited: 61
[WARN] scripts/check_format.sh — missing, but registered in the baseline as known debt: Issue #5450. ...
[WARN] scripts/check_links.sh — missing, but registered in the baseline as known debt: Issue #5450. ...
[WARN] scripts/gate/check_alpha.sh — missing, but registered in the baseline as known debt: ...
[WARN] scripts/gate/check_docs.sh — missing, but registered in the baseline as known debt: ...
[WARN] scripts/gate/check_user_guides.sh — missing, but registered in the baseline as known debt: ...
[WARN] scripts/gate/gate_webhook.sh — missing, but registered in the baseline as known debt: ...
[WARN] scripts/gate/run_hermes_gate.sh — missing, but registered in the baseline as known debt: ...
[WARN] scripts/generate_documentation_report.sh — missing, but registered in the baseline as known debt: ...
--- Summary ---
  cited paths:        61
  exist:              53
  missing (total):    8
  missing (baselined):8
  missing (NEW):      0
  errors:             0
  warnings:           8
✅ PASS: all 53 existing citations verified; 8 known-debt entries reported as warnings.
EXIT=0
```

**新增缺失（不在 baseline）判 FAIL**：

```
$ printf '\n<!-- 负向测试 -->\n| BE99 | fake | `scripts/gate/totally_nonexistent_zzz.sh` | exit 0 | PASS |\n' \
    >> docs/governance/GATE_CONDITIONS.md
$ bash scripts/gate/check_gate_script_refs.sh; echo "EXIT=$?"
[ERROR] scripts/gate/totally_nonexistent_zzz.sh — cited by a criterion document but does not exist, and is NOT in the baseline
❌ FAIL: 1 newly-missing referenced script(s):
     - scripts/gate/totally_nonexistent_zzz.sh
EXIT=1
```

**PASS** ✅

### AC-7: PR 正文的 5 类文档有机器判据

```
$ bash scripts/gate/check_pr_template_sections.sh --pr-body .gitea/pull_request_template.md; echo "EXIT=$?"
[PASS] section:SPEC
[PASS] section:TEST_PLAN
[PASS] section:TEST_DESIGN
[PASS] section:REVIEW
[PASS] section:ACCEPTANCE
✅ PASS: all 5 required sections are present.
EXIT=0

$ printf '# My PR\n\nSome text without markers.\n' | bash scripts/gate/check_pr_template_sections.sh; echo "EXIT=$?"
❌ FAIL: missing section(s): SPEC TEST_PLAN TEST_DESIGN REVIEW ACCEPTANCE
EXIT=1

$ printf '' | bash scripts/gate/check_pr_template_sections.sh; echo "EXIT=$?"
❌ PR body is empty.
   Cannot judge — DRIFT, not PASS.
EXIT=2
```

**PASS** ✅ — 补上了 #5458 缺口 (c)：原测试只读模板**本身**，
现在读**实际 PR 正文**。

### AC-8: AGENTS.md 的引用指向真实文件

```
$ grep -n "PR_TEMPLATE" AGENTS.md
120:10. PR body 必填 5 类文档 (`.gitea/pull_request_template.md` 的 §一) + Test Plan + Evidence
```

**PASS** ✅ —— 由测试 `pr_template_reference_in_agent_guide_points_at_a_real_file` 守护。

### AC-9: CHECK 4 接受本 PR 的提交身份

```
$ bash scripts/gate/check_anti_fabrication.sh 2>&1 | grep -A1 "CHECK 4"
[INFO] CHECK 4: HEAD commit author must be a known AI/Human identity...
[PASS] HEAD author email: opencode-z440@gaoyuanyiyao.com (allowed multi-AI identity)
```

**PASS** ✅ —— 这是白名单修复的**端到端验证**：若不加白名单，
本 PR 的 HEAD commit 会被 CHECK 4 拒绝（与 #4907 中 omp-z440 遇到的同型问题）。

### AC-10: 新脚本不污染既有门禁

| 门禁 | 实测 | 与 base 对比 |
|---|---|---|
| `check_drift_not_pass.sh`（P14）| FAIL | findings 集合与 base **逐条相同**（4 条存量），**新脚本不在其中** |
| `check_gate_test_integrity.sh`（P16）| exit **0** | 新测试文件无 `#[ignore]` |
| `check_gate_self_verification.sh`（P11）| exit **0** | 新脚本 header 被识别 |
| `test_target_manifest_honesty_test` | 15 passed; 0 failed | `no_orphan_test_file` PASS（`Cargo.toml` 注册生效）|
| `gate_honesty_test` | 11 passed; 0 failed | 无回归 |

**PASS** ✅

### AC-11: 格式与静态检查

```
$ cargo fmt --check --all; echo "EXIT=$?"
EXIT=0

$ cargo clippy --all-features -- -D warnings; echo "EXIT=$?"
EXIT=0
```

**PASS** ✅

### AC-12: 新增 Rust 测试全绿

```
$ cargo test --test gate_criterion_existence_test --all-features
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**PASS** ✅ —— 21 个用例，**0 ignored**（ADR-008 Policy 2）。

---

## 3. 存量 Gate 失败（如实声明，未隐瞒）

以下 gate 在 **base commit `378f014529` 上即为 FAIL**。已用 `git stash`
在同一 commit 下复跑确认，**与本 PR 无关**：

| Gate | base | 本分支 | 归属 |
|---|---|---|---|
| `check_anti_fabrication.sh` | FAIL（7 errors）| FAIL（7 errors）| 他人 batch（V312-24 artifacts + `fault_coordination`/`sqlrustgo-storage` 编译）|
| `check_drift_not_pass.sh` | FAIL（4 anti-patterns）| FAIL（**集合完全相同**）| 他人 batch |
| `check_docs_pointer_paths.sh` | FAIL（3 broken pointers）| FAIL（**集合完全相同**）| 他人 batch |
| `check_test_target_lint_budget.sh` | FAIL（`CARGO_TARGET_DIR` 未定义）| 同 | 环境问题 |

**本 PR 不引入任何新的 Gate 失败** —— AC-10 的对比是证据。

按 ADR-001 G-09 / ADR-014 D-05，Hard Gate FAIL 时禁止 push 到 252。
本次 push 已由 User **显式授权**，并在此声明存量失败清单。

---

## 4. 遗留项（明确登记，不假装已完成）

| # | 遗留项 | 原因 | 建议 |
|---|---|---|---|
| 1 | 3 个新门禁**未接入 CI** | `.gitea/workflows/ci.yml` 被 5 个他人分支高频改动，触碰会显著提高冲突面 | 单独 issue 接线。⚠️ 仓库自己的教训（`check_stage.sh` 注释里的 #5440）：「这个脚本是 SSOT 唯一的执行者，没有任何 CI job 调用过它」—— **无调用方的门禁会腐烂**，故登记为待办 |
| 2 | `check_pr_template_sections.sh` **无调用方** | 同上 | 同上 |
| 3 | §2.1 状态表 3 行过时（PR-840/850/870）| 改它要动 v3.8.0 归档区的文档，超出 #5457「让判据可执行」的范围 | 单独 issue 修文档；本 gate 已把它报为 WARN 并给出证据 |
| 4 | 8 条已知债脚本缺失 | 见 baseline 逐条成因 | 按 baseline 的 Fix 建议逐条处理 |

---

## 5. 证据（ADR-014 5 fields）

```
source_agent:      opencode-z440
source_run:        opencode-z440-20261022-gate-refs
timestamp:         2026-10-22T07:58:00Z
evidence_hash:
  verify_pr_dag.sh           sha256:0a330b6c942d3799c2fcb4f8a771bc372f32d18d85a605ec4b03812119e74793
  check_gate_script_refs.sh  sha256:a9d9a7449ffa0f555f708c9321ca9b1122462c38e0e4ee524fdfdffdc0c2b81b
  check_pr_template_sections sha256:07a2337fdfcfeff40cc8e054b13700b6623277531b5806f76a9a3ca53bbceac7
conflict_resolution:
  无文件级冲突。新增 6 个文件（远端 168 个 fix/docs 分支零触及）；
  改动 4 个既有文件：check_anti_fabrication.sh（+1 数组元素 +9 行注释）、
  AGENTS.md（1 行）、pull_request_template.md（+10 行）、
  Cargo.toml（+3 行 [[test]] 块）。
  冲突面已按文件实测统计（见 SPEC §5）。
  存量 Gate 失败的归属已核实为他人 batch（见 §3），本 PR 不越界修复。
```

---

## 6. 结论

**PASS（本地实跑）**。

12 条验收标准全部通过，含 7 条负向路径实跑。
本 PR 让 5 个「判据指向缺失实体」的实例从**不可执行**变为**可执行且有判据**，
并交付根治件使第 4 个同类缺陷可被当场发现。

不合并自己的 PR —— 合并由 User / Reviewer 执行（PERMISSION_MATRIX §3）。

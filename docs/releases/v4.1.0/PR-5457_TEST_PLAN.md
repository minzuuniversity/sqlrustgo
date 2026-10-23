# PR-5457 TEST_PLAN — 门禁判据可执行性修复

> Issue: #5457 / #5458 / #5450 · 分支: `opencode/fix-5457-5458-gate-refs`
> 基线: `origin/develop/v4.1.0` @ `378f014529`
> 配套: `PR-5457_SPEC.md` / `PR-5457_TEST_DESIGN.md` / `PR-5457_TEST_REVIEW.md` / `PR-5457_ACCEPTANCE.md`

---

## 1. 覆盖目标

本 PR 的测试目标不是「覆盖了多少行」，而是**「每个新门禁在它最可能骗人的地方
会不会骗人」**。因此每条用例都对应一个具体的**失效模式**，而不是一个函数。

| 失效模式（为什么需要这条用例） | 对应用例 |
|---|---|
| 门禁在文档为空/不可解析时报 PASS | `verify_pr_dag_rejects_a_plan_without_a_pr_dag_section` |
| 门禁在 base 不可解析时把 DRIFT 当 PASS | `verify_pr_dag_reports_drift_for_unresolvable_base` |
| 门禁认证了从未落地的 PR（虚假通过）| `verify_pr_dag_detects_a_false_pass` |
| 门禁不校验依赖顺序，链写反了也 PASS | `verify_pr_dag_detects_an_inverted_dependency` |
| alias 与实现行为不一致 | `check_pr_dag_alias_exists_and_delegates` |
| 存在性检查对新增缺陷不敏感 | `gate_script_refs_check_detects_a_missing_reference` |
| 存在性检查把已知债当新增缺陷（永红）| `gate_script_refs_check_passes_on_real_repository` |
| baseline 变成藏问题的地方 | `gate_script_refs_baseline_exists_and_every_entry_has_a_reason` |
| PR 正文检查其实只查了模板 | `pr_template_sections_check_accepts_the_template` + `..._rejects_a_body_without_markers` |
| 输出缺 ADR-014 证据字段 | 3 个 `*_emits_evidence_fields` |

---

## 2. 测试层次

### 2.1 层 1：Rust 集成测试（`tests/integration/gate/gate_criterion_existence_test.rs`）

**21 个测试，0 ignored。**

- **shell 脚本的存在性/可执行性** —— 纯文件系统断言，快
- **shell 脚本的 CLI 行为** —— 用真实临时 fixture 文件驱动脚本，
  断言 **exit code**（不是断言 stdout 里出现了某个词）
- **判据文档自身的不变量** —— 从 Rust 侧独立复现存在性检查，
  这样即使 shell gate 还没接进 CI，`cargo test` 也能抓到回归

> **测试纪律**（ADR-008 Policy 2 / P16）：本文件**不含** `#[ignore]`。
> 一旦被 gate 脚本按 `cargo test --test <name>` 引用，它就是 **gate test**，
> 带 `#[ignore]` 即 FAIL。

### 2.2 层 2：shell 门禁脚本自测（负向路径）

构造 fixture 实跑，确认脚本在**该红的时候红**：

| fixture | 期望 |
|---|---|
| 空 plan（无 PR DAG 段）| exit 2（DRIFT）|
| 坏 base ref | exit 2 |
| 声明 `✅ 已合并` 但无 merge commit | exit 1 |
| 依赖倒置 | exit 1 |
| 含虚构缺失脚本引用的判据文档 | exit 1 且点名该路径 |
| 缺 section 标记的 PR 正文 | exit 1 |
| 空 PR 正文 | exit 2 |

### 2.3 层 3：既有仓库门禁（回归）

新脚本不能污染既有门禁的判定：

| 门禁 | 验证点 |
|---|---|
| `check_drift_not_pass.sh`（P14）| 新脚本**不在** findings 里 |
| `check_gate_test_integrity.sh`（P16）| 新测试文件无 `#[ignore]` |
| `check_gate_self_verification.sh`（P11）| 新脚本 header 被识别 |
| `test_target_manifest_honesty_test` | `no_orphan_test_file` PASS（`Cargo.toml` 注册生效）|
| `gate_honesty_test` | 既有 gate 诚实性套件无回归 |

---

## 3. 测试命令与实跑结果

全部命令均在 worktree `.worktrees/opencode-z440-20261022`、
base `378f014529` 上实跑。

| # | 命令 | 期望 | 实测 |
|---|---|---|---|
| T1 | `cargo test --test gate_criterion_existence_test --all-features` | 21 passed, 0 failed, 0 ignored | **21 passed; 0 failed; 0 ignored** |
| T2 | `bash scripts/gate/verify_pr_dag.sh` | exit 0 | **exit 0** |
| T3 | `bash scripts/gate/check_pr_dag.sh` | 与 T2 同 exit code | **exit 0** |
| T4 | `bash scripts/gate/check_gate_script_refs.sh` | exit 0 | **exit 0** |
| T5 | `bash scripts/gate/check_pr_template_sections.sh --pr-body .gitea/pull_request_template.md` | exit 0 | **exit 0** |
| T6 | `cargo fmt --check --all` | exit 0 | **exit 0** |
| T7 | `cargo clippy --all-features -- -D warnings` | 0 warnings | **exit 0, 0 warnings** |
| T8 | `bash scripts/gate/check_drift_not_pass.sh` | 新脚本不在 findings | **新脚本不在 findings**（findings 与 base 完全一致的 4 条存量）|
| T9 | `bash scripts/gate/check_gate_test_integrity.sh` | exit 0 | **exit 0** |
| T10 | `bash scripts/gate/check_gate_self_verification.sh` | exit 0 | **exit 0** |
| T11 | `cargo test --test test_target_manifest_honesty_test --all-features` | 全部 PASS | **15 passed; 0 failed** |
| T12 | `cargo test --test gate_honesty_test --all-features` | 全部 PASS | **11 passed; 0 failed** |

### 3.1 负向路径实跑

| fixture | 期望 exit | 实测 exit |
|---|---|---|
| plan 无 PR DAG 段 | 2 | **2** |
| `--base refs/heads/definitely_nonexistent_ref_zzz` | 2 | **2** |
| 虚假通过（PR-800 改成 `✅ 已合并`）| 1 | **1** |
| 依赖倒置（PR-810 依赖 900）| 1 | **1** |
| 虚构缺失脚本引用 | 1 | **1** |
| PR 正文缺标记 | 1 | **1** |
| PR 正文为空 | 2 | **2** |

### 3.2 存量失败（如实记录，不算本 PR 引入）

在 base `378f014529` 上用 `git stash`（在同一 commit 下）复跑确认：

| Gate | base | 本分支 | 归属 |
|---|---|---|---|
| `check_anti_fabrication.sh` | FAIL (7 errors) | FAIL (7 errors) | 他人 batch |
| `check_drift_not_pass.sh` | FAIL (4 anti-patterns) | FAIL (4 anti-patterns，**集合完全相同**）| 他人 batch |
| `check_docs_pointer_paths.sh` | FAIL (3 broken pointers) | FAIL (3 broken pointers，**集合完全相同**）| 他人 batch |
| `check_test_target_lint_budget.sh` | FAIL (`CARGO_TARGET_DIR` 未定义) | 同 | 环境问题 |

**关键证据**：P14 与 docs-pointer 的 findings 集合在 base 与本分支上**逐条相同**
⇒ 本 PR 未引入任何新的门禁失败。

---

## 4. 未覆盖范围（明确声明）

| 未覆盖 | 原因 |
|---|---|
| 新脚本的 **CI 接线** | `.gitea/workflows/ci.yml` 被 5 个他人分支高频改动，触碰会显著提高冲突面。脚本落地并可本地实跑，CI 接线留作单独 issue（见 SPEC §2.2）|
| §2.1 状态表 3 行过时的**修复** | 文档改动，超出 #5457「让判据可执行」的范围。gate 报 WARN 并列出证据 |
| 历史门禁报告里既有 `✅ PASS` 的**改写** | 那是历史事实；改写超出本 issue 范围 |
| `check_pr_template_sections.sh` 的**调用方** | 同 CI 接线，无调用方的门禁会腐烂 —— 已在 issue 正文登记待办 |

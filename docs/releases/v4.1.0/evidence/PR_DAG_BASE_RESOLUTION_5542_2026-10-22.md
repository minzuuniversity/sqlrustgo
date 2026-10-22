# 迭代 5 · 批次 24b —— PR-DAG 门禁的 base ref 恒不可解析（#5542）

- **分支**：`fix/ci-truth-batch24`（批次 24 追加一个提交）
- **基线**：`develop/v4.1.0` @ `6f26128890eb58d8cf1cab353f9aea3efcc948dc`
- **被验证的树**：`cd4a02c6b50b227886313fbd7371357120c64979`（批次 24 头，本提交之前）
- **新登记 Issue**：#5542
- **改动文件**：3 个 —— `scripts/gate/verify_pr_dag.sh`、`.gitea/workflows/ci.yml`、本证据文档
- **不改动**：`tests/integration/gate/gate_criterion_existence_test.rs`（`opencode-z440` 的文件，见 §6）

---

## 0. 一句话

**`scripts/gate/verify_pr_dag.sh` 的默认 base ref 在任何环境都解析不到** ⇒ 门禁恒判 DRIFT(exit 2)
⇒ `tests/integration/gate/gate_criterion_existence_test.rs` 里 **5 个用例在每个 CI run 上都红**
⇒ **#5535 / #5539 / #5540 三个 PR 的 `test` job 必红**，GA 路径被硬阻塞。

根因不是判据写错了，而是**把「去哪里取历史」写死成了一个只在本仓库某一种 clone 形态下存在的引用**。
规则（要判什么）没有变，变的是环境 —— 于是规则被环境细节击穿。

---

## 1. 缺陷

### 1.1 默认 base

```bash
# scripts/gate/verify_pr_dag.sh（修复前）
BASE_REF="origin/develop/v4.1.0"
...
if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
    echo "❌ base ref cannot be resolved: $BASE_REF"
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi
```

### 1.2 两种环境，两种「没有 `origin/develop/v4.1.0`」

| 环境 | 引用实况 | `git rev-parse --verify origin/develop/v4.1.0` |
|---|---|---|
| 本地 worktree | 远端叫 `gitea252` / `gitee` / `github` / `gitcode` / `gitea250`，**根本没有 `origin`** | `fatal: Needed a single revision` |
| CI runner | `git init` + `git remote add origin` + `git fetch origin +<SHA>`（**只取单个 SHA**）⇒ **不生成任何远程跟踪分支** | `fatal: Needed a single revision` |

### 1.3 CI 形态的**实地复现**（不是推断）

按 `.gitea/workflows/ci.yml` 的 checkout 步骤原样重建一个 checkout：

```bash
git init -q -b main .
git remote add origin "http://192.168.0.252:3000/openclaw/sqlrustgo.git"
git fetch --no-tags --prune --no-recurse-submodules --depth=1 origin "+$SHA"
git checkout -q FETCH_HEAD
```

实测（`SHA=cd4a02c6b50b227886313fbd7371357120c64979`）：

```
HEAD = cd4a02c6b50b227886313fbd7371357120c64979
shallow file present: YES
-- refs/remotes/* --
(空 —— 没有任何远程跟踪分支)
-- rev-parse origin/develop/v4.1.0 --
fatal: Needed a single revision
-- for-each-ref refs/remotes/*/develop/v4.1.0 --
(空)
-- git rev-list --count HEAD --
1
```

⇒ **CI 上也不存在 `origin/develop/v4.1.0`**。

### 1.4 后果：门禁恒为 DRIFT

在**同一个** CI 形态 checkout 里跑修复前的脚本：

```
EXIT=2
[INFO] plan document: docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md
❌ base ref cannot be resolved: origin/develop/v4.1.0
   Cannot judge — DRIFT, not PASS.
```

⇒ 而 `tests/integration/gate/gate_criterion_existence_test.rs` 的 5 个用例期望的是：

| 用例 | 期望 | 实际 | 结论 |
|---|---|---|---|
| `verify_pr_dag_detects_a_false_pass` | exit **1**（FAIL） | exit 2（DRIFT） | 红 |
| `verify_pr_dag_detects_an_inverted_dependency` | exit **1** | exit 2 | 红 |
| `verify_pr_dag_detects_duplicate_status_rows` | exit **1** | exit 2 | 红 |
| `verify_pr_dag_detects_a_status_row_outside_the_chain` | exit **1** | exit 2 | 红 |
| `verify_pr_dag_passes_on_the_real_repository` | exit **0**（PASS） | exit 2 | 红 |

**这 5 个用例在 develop 上就已经红** —— 它们是 `opencode-z440` 的 #5457/#5458 用例，
它们测的正是「门禁能不能真的判」；门禁本身却答不了。

### 1.5 附带缺陷（同一文件）

`scripts/gate/verify_pr_dag.sh:409`：

```bash
n_commits=$(count_merge_commits_with_children "$pr")   # 该 PR 无计数条目时输出**空串**
if [ "$n_commits" -gt 0 ]; then                        # => [: : integer expression expected
```

`count_merge_commits_with_children` 在没有该 PR 的计数条目时输出**空串**，
`[ "" -gt 0 ]` 是未定义比较，bash 每次都往 stderr 打
`scripts/gate/verify_pr_dag.sh: line 409: [: : integer expression expected`。
行为侥幸正确（空串被当成假），但 stderr 噪声会淹没门禁的真实诊断。

实测（修复前，本地完整 checkout）：

```
scripts/gate/verify_pr_dag.sh: line 409: [: : integer expression expected
scripts/gate/verify_pr_dag.sh: line 409: [: : integer expression expected
```

（每个没有合并记录的 PR 各打一次。）

---

## 2. 修复

### 2.1 规则只有一份实现：base = 「发布线」+ 一条确定的解析链

**显式 `--base` 逐字使用，永不回退。** 解析不到 ⇒ 仍 DRIFT(exit 2)，
因为「不能判定 ⇒ DRIFT」是 ADR-006 P14 的要求，而**静默换一个 ref** 会让操作者
以为门禁判的是他指定的范围。

**默认 base 是「发布线规格」`develop/v4.1.0`**，按固定顺序解析：

| 顺序 | 候选 | 说明 |
|---|---|---|
| 1 | `refs/remotes/<remote>/develop/v4.1.0` | remote 固定优先级 `origin → gitea252 → gitea250 → github → gitcode → gitee` |
| 2 | 其它远程的同名线 | `git for-each-ref` 排序后取第一个 ⇒ 确定 |
| 3 | `refs/heads/develop/v4.1.0` | 本地分支 |
| 4 | `HEAD` | CI 的 SHA-only checkout 唯一有的引用 |

全不中 ⇒ 仍然 DRIFT(exit 2)。
**门禁必须打印它实际用的 ref**（`base ref resolved: …`），回退不许静默。

### 2.2 整数比较

`n_commits` 为空或非数字 ⇒ 归一为 `0`（`case "$n_commits" in ''|*[!0-9]*) n_commits=0 ;; esac`）。

### 2.3 门禁自带 `--selftest`

规则要能单独被测 ⇒ 门禁脚本自建三种形态的临时 git 仓库自证：

1. 带远程跟踪分支的 checkout ⇒ 解析到该远程分支；
2. SHA-only checkout ⇒ 解析到 `HEAD`；
3. 显式坏 `--base` ⇒ 仍 DRIFT(2)，**不回退**；
4. 门禁运行**不在 stderr 吐**整数比较噪声；
5. 门禁输出里**看得见**它实际用的 base ref。

### 2.4 CI 接线（AC7）

`.gitea/workflows/ci.yml` 的 `lint-build` job 增加一步：

```yaml
- name: PR-DAG gate base-ref resolution (#5542)
  run: |
    set -euo pipefail
    bash scripts/gate/verify_pr_dag.sh --selftest
```

**门禁本体不在 CI 里跑**：runner 的 checkout 是浅克隆，门禁会正确地拒绝判定（DRIFT），
而 BE5 / B-F5 是**发布期**判据 —— 与同一 job 里 `check_stage.sh --dry-run` 的安排一致。
**不新增门禁数**：挂靠既有 `lint-build` job。

---

## 3. 判据对照（AC1 – AC7）

| # | 判据 | 实测 | 结论 |
|---|---|---|---|
| AC1 | `cargo test --test gate_criterion_existence_test` 5 个原失败用例全绿（且不是靠删用例换来的） | `test result: ok. 23 passed; 0 failed; 0 ignored` | ✅ |
| AC2 | 负对照：显式 `--base refs/heads/nonexistent_zzz` 仍 DRIFT(2)，**不回退** | `EXIT=2` + `❌ base ref cannot be resolved: refs/heads/definitely_nonexistent_ref_zzz (explicitly requested — no fallback is attempted)`；alias `check_pr_dag.sh` 同为 `EXIT=2` | ✅ |
| AC3 | 负对照：解析链退回「只认 `origin/develop/v4.1.0`」⇒ AC1 重新变红 | `test result: FAILED. 18 passed; 5 failed`（恰好同 5 个） | ✅ |
| AC4 | 输出里能看见实际使用的 base ref | `[INFO] base ref resolved: refs/remotes/gitea252/develop/v4.1.0 (6f26128890)`；CI 形态下 `base ref resolved: HEAD (cd4a02c)` | ✅ |
| AC5 | stderr 不再出现 `[: : integer expression expected` | 默认运行 stderr **0 字节**；CI 形态 checkout 内 0 字节 | ✅ |
| AC6 | 门禁带 `--selftest`，覆盖三种形态 | `passed: 5 / failed: 0`（本地与 **CI 形态 checkout 内**均通过） | ✅ |
| AC7 | CI 的 lint-build job 跑一次 `--selftest` | `.gitea/workflows/ci.yml` 已加步；`check_workflow_expressions.sh` EXIT=0 | ✅ |

### 3.1 AC1 的完整输出（修复后）

```
running 23 tests
test check_pr_dag_alias_exists_and_delegates ... ok
test verify_pr_dag_rejects_a_plan_without_a_pr_dag_section ... ok
test verify_pr_dag_reports_drift_for_unresolvable_base ... ok
test verify_pr_dag_detects_a_false_pass ... ok
test verify_pr_dag_detects_an_inverted_dependency ... ok
test verify_pr_dag_detects_duplicate_status_rows ... ok
test verify_pr_dag_detects_a_status_row_outside_the_chain ... ok
test verify_pr_dag_passes_on_the_real_repository ... ok
test verify_pr_dag_emits_evidence_fields ... ok
...
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.92s
```

### 3.2 AC3 的完整输出（负对照：解析链退回单一 ref）

```
  base_ref:     develop/v4.1.0 (release line — resolved below)

[INFO] plan document: docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md
❌ no ref could be resolved for the release line: develop/v4.1.0
   Cannot judge — DRIFT, not PASS.

  left: 2
 right: 0

failures:
    verify_pr_dag_detects_a_false_pass
    verify_pr_dag_detects_a_status_row_outside_the_chain
    verify_pr_dag_detects_an_inverted_dependency
    verify_pr_dag_detects_duplicate_status_rows
    verify_pr_dag_passes_on_the_real_repository

test result: FAILED. 18 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
```

⇒ 变红的**恰好是** AC1 转绿的那 5 个 ⇒ 转绿是这份修复带来的，不是别的什么。

### 3.3 CI 形态 checkout 内的前后对照

| | 脚本 | EXIT | 关键输出 |
|---|---|---|---|
| 修复前 | `cd4a02c6b5` 的 `verify_pr_dag.sh` | 2 | `❌ base ref cannot be resolved: origin/develop/v4.1.0`（**错误的原因**） |
| 修复后 | 本提交的 `verify_pr_dag.sh` | 2 | `[INFO] base ref resolved: HEAD (cd4a02c)` → `⚠️ DRIFT: this is a SHALLOW checkout`（**正确的原因**） |

修复后 `verify_pr_dag_passes_on_the_real_repository` 在浅克隆分支上拿到的正是
它期望的「exit 2 + 文本含 `SHALLOW`」⇒ 浅克隆下门禁**因为正确的原因**拒绝判定，
而不是因为**找不到 ref**。

---

## 4. 修复后门禁的其它行为（未改动，但已复跑确认）

| 行为 | 结果 |
|---|---|
| 默认运行（本地完整 checkout） | `EXIT=0`，`§2 chain nodes: 11` / `§2.1 status rows: 11` / `false passes: 0` / `errors: 0` / `warnings: 5`（5 条是文档陈旧警告，与本缺陷无关） |
| `--base refs/heads/definitely_nonexistent_ref_zzz` | `EXIT=2`（DRIFT） |
| `check_pr_dag.sh`（alias） | 与实现同 exit code |

---

## 5. 未达成 / 未验证（如实记录）

1. **本修复未经 CI 验证**。它进了 `fix/ci-truth-batch24`（PR #5535），
   但 #5535 的 `test` job 仍要等 runner 空出槽位（约 2 小时/周期）。
   在 CI 真正绿之前，本条只算**本地已验证**。
2. **BE5 / B-F5 判据在 CI 上仍然判不了**（浅克隆 ⇒ DRIFT）。这是**设计如此**：
   门禁在截断历史上无法判定“PR-X 有没有 merge commit”，DRIFT 是正确答案。
   要真正执行 BE5/B-F5，需要在**完整历史**的 checkout 上跑门禁 —— 本批**没有**做这件事，
   记为独立欠账。
3. **`resolve_base_ref` 的候选顺序是本批定的**，不是仓库里已有的约定。
   远程优先级里 `gitea252` 排在 `origin` 之后，是为了让「CI 形态（只有 origin）」与
   「本地形态（只有 gitea252）」都命中**同一套代码路径**；若仓库将来统一远端命名，
   这张表应当随约定一起改，而不是散落在脚本里。
4. **5 个旧用例仍然是 `opencode-z440` 的文件**，本批一行未改。
   如果对方认为“测试文件必须与实现同 commit 修改”，需要按协议 §5 交接后再动。

---

## 6. 身份与协作（多 Agent）

- **身份**：`openclaw-mcode`（Mac Mini / Minimax-Code，Gitea 账号 `openclaw`）
- **台账**：#5531 评论 `194312`（CLAIM + 文件归属声明）；Issue #5542 评论 `194318`
- **通知对方**：PR #5536 评论 `194325`（`<!-- coordination: openclaw-mcode -->`）

### 6.1 文件归属矩阵（按**路径**切分，不是按 Issue 号）

| 路径 | 原归属 | 本批 | 规则 |
|---|---|---|---|
| `scripts/gate/verify_pr_dag.sh` | `opencode-z440`（#5536 创建） | **`openclaw-mcode` 接手** | 已声明；若对方同时在改，走 §5 交接，不静默覆盖 |
| `tests/integration/gate/gate_criterion_existence_test.rs` | `opencode-z440` | **不动** | 修复必须让它**原样**变绿 |
| `.gitea/workflows/ci.yml` | `openclaw-mcode`（批次 24） | `openclaw-mcode` | 追加一步，不新增门禁数 |

### 6.2 ADR-014 证据字段

门禁输出的 `source_agent` / `source_run` / `timestamp` 三个字段保留，
且 `source_run` 现在**绑定被判的 commit**（`openclaw-mcode-pr-dag-<短 SHA>`），
默认 `source_agent` 改为本批维护者。历史证据文档里记录的旧值保持原样 —— 那是历史。

### 6.3 合并后的复跑义务

本批进的是 PR #5535。**合并 ≠ 完成**：合并后必须在 `develop/v4.1.0` 的目标 sha 上
复跑 `cargo test --test gate_criterion_existence_test` 与
`bash scripts/gate/verify_pr_dag.sh --selftest`，把结果回填本节，然后才关 #5542。

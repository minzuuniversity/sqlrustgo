# PR-5457 TEST_DESIGN — 门禁判据可执行性修复

> Issue: #5457 / #5458 / #5450 · 分支: `opencode/fix-5457-5458-gate-refs`
> 基线: `origin/develop/v4.1.0` @ `378f014529`
> 配套: `PR-5457_SPEC.md` / `PR-5457_TEST_PLAN.md` / `PR-5457_TEST_REVIEW.md` / `PR-5457_ACCEPTANCE.md`

---

## 1. 测试设计原则

### 1.1 断言 exit code，不断言 stdout 关键词

门禁脚本的**契约是退出码**（`GATE_CONDITIONS.md:258`：
「脚本必须有退出码：0 = PASS, 1 = FAIL」）。断言 stdout 里出现某个词，
测的是**实现细节**而不是**契约** —— 改个措辞就假失败，而退出码错了反而测不出来。

所以本 PR 的测试全部断言 `exit code`，stdout 只在**失败时**作为诊断信息附上。

### 1.2 用真实 fixture，不 mock

三个门禁都是 shell 脚本。用 mock 意味着测的是 mock 的行为。
因此：

- 构造**真实的临时文件**（空 plan、虚假通过的 plan、倒置依赖的 plan、
  含假路径的判据文档、缺标记的 PR 正文）
- 用**真实脚本**跑
- 断言**真实退出码**

代价是每个 CLI 用例要跑一次 git log（~2-3s），换来的是「测的就是会跑的东西」。

### 1.3 跨测试互斥：文件锁

`gate_script_refs_check_detects_a_missing_reference` 会**临时改写**
`docs/governance/GATE_CONDITIONS.md`（追加一个虚构引用，测完立即还原）。
而 `gate_script_refs_check_passes_on_real_repository` 与
`every_gate_script_cited_by_criterion_documents_exists` 会**读**同一批文档。

Rust 在同一二进制内**并行跑测试**，所以读写之间存在真实竞态 ——
初版就撞上了（14 个测试里 2 个随机失败）。

解法是 `O_EXCL` 原子创建 lock 文件，无需引入依赖，跨进程也有效：

```rust
struct FileLock { path: PathBuf }
impl FileLock {
    fn acquire(name: &str) -> FileLock { /* 有界自旋 + 打破陈旧锁 */ }
}
impl Drop for FileLock { fn drop(&mut self) { let _ = fs::remove_file(&self.path); } }
```

自旋有界（600 × 50ms = 30s）：被 kill 的运行会留下陈旧锁，
无界自旋会把测试套件**挂死**。超时后主动打破并接管 ——
被保护的改动是短生命周期的。

### 1.4 测试不依赖网络

`verify_pr_dag.sh` 的规划编号 → Gitea 号映射来自 **git merge commit 标题**，
不是 Gitea API。所以测试不需要网络，也不需要 token。

> 这本身是设计选择：门禁脚本必须在 CI runner 上能跑，那里访问 Gitea API
> 不一定可靠。merge commit 标题是**仓库自带**的权威来源。

---

## 2. 用例设计

### 2.1 `verify_pr_dag.sh`

| 用例 | fixture | 断言 | 为什么这样测 |
|---|---|---|---|
| `verify_pr_dag_script_exists` | — | 文件存在 | 判据引用的脚本必须存在 |
| `verify_pr_dag_script_is_executable` | — | mode & 0o111 | 不可执行的脚本跑不了 |
| `check_pr_dag_alias_exists_and_delegates` | — | alias 存在 + 与实现**同 exit code** | 两个引用名必须等价；用「坏 base」做探针（确定性、便宜）|
| `verify_pr_dag_rejects_a_plan_without_a_pr_dag_section` | 无 PR DAG 段的 plan | exit **2** | 不可判定 ⇒ DRIFT，**不是 PASS**（ADR-006 P14）|
| `verify_pr_dag_reports_drift_for_unresolvable_base` | `--base refs/heads/..._zzz` | exit **2** | 同上 |
| `verify_pr_dag_detects_a_false_pass` | PR-800 声明 `✅ 已合并` | exit **1** + 输出含 "false pass" | **危险方向**：认证从未落地的工作 |
| `verify_pr_dag_detects_an_inverted_dependency` | PR-810 依赖 PR-900 但排在其前 | exit **1** + 输出含 "inverted" | 依赖序倒置 |
| `verify_pr_dag_passes_on_the_real_repository` | 真实 plan | exit **0** | 真实仓库必须满足门禁；不满足要么 plan 真有问题，要么门禁写错了 —— 两种都得暴露 |
| `verify_pr_dag_emits_evidence_fields` | — | stdout 含 3 个字段 | ADR-014 5 evidence fields |

**为什么虚假通过 fixture 用 PR-800**：PR-800 在 §2.1 里是 `⚠️ 部分落地`，
把它改成 `✅ 已合并` 就构造出「声明合并但 git 里没有 merge commit」，
正是要测的危险方向。选一个**确实没有** merge commit 的 PR 才有效 ——
用 PR-830 会误判（它有 72 个子编号 merge commit）。

### 2.2 `check_gate_script_refs.sh`

| 用例 | fixture | 断言 |
|---|---|---|
| `gate_script_refs_check_exists` | — | 文件存在 |
| `gate_script_refs_check_is_executable` | — | mode & 0o111 |
| `gate_script_refs_baseline_exists_and_every_entry_has_a_reason` | — | baseline 是合法 JSON；`known_missing` 非空；**每条都有 reason** |
| `gate_script_refs_check_passes_on_real_repository` | 真实仓库 | exit **0**（8 条已知债不阻塞）|
| `gate_script_refs_check_detects_a_missing_reference` | 临时改写判据文档 | exit **1** + 输出点名该路径；文档已还原 |
| `gate_script_refs_check_emits_evidence_fields` | — | stdout 含 3 个字段 |

**baseline reason 的断言**：一个没有原因的 baseline 条目是**藏问题**，
不是**追踪债**。这个断言直接编码了那条原则。

### 2.3 `check_pr_template_sections.sh`

| 用例 | fixture | 断言 |
|---|---|---|
| `pr_template_sections_check_exists_and_is_executable` | — | 文件存在 + 可执行 |
| `pr_template_sections_check_accepts_the_template` | 真实模板 | exit **0**（模板自身必须满足它定义的规则）|
| `pr_template_sections_check_rejects_a_body_without_markers` | 无标记正文 | exit **1** |
| `pr_template_sections_check_rejects_an_empty_body` | 空正文 | exit **2**（空正文不可判定，不是 PASS）|

### 2.4 判据文档的不变量

| 用例 | 断言 |
|---|---|
| `every_gate_script_cited_by_criterion_documents_exists` | 8 份 SSOT 文档引用的脚本：存在，或在 baseline 中登记 |
| `pr_template_reference_in_agent_guide_points_at_a_real_file` | `AGENTS.md` 含 `PR_TEMPLATE` 的那行里，每个 `.md` 路径都真实存在 |

---

## 3. 判别力自检

一个测试套件最容易的失败模式是「测了但没测到」。本套件的自检：

| 检查 | 结果 |
|---|---|
| 移除 `verify_pr_dag.sh` 后测试会失败吗？| 会 —— `verify_pr_dag_script_exists` |
| 把 `verify_pr_dag.sh` 的 exit 2 改成 exit 0，测试会失败吗？| 会 —— 两个 DRIFT 用例 |
| 删掉虚假通过检测逻辑，测试会失败吗？| 会 —— `verify_pr_dag_detects_a_false_pass` |
| 删掉依赖边检测，测试会失败吗？| 会 —— `verify_pr_dag_detects_an_inverted_dependency` |
| 删掉 baseline 判定（缺失即 FAIL），测试会失败吗？| 会 —— `..._passes_on_real_repository`（8 条已知债会误报）|
| 删掉 baseline reason 断言，测试会失败吗？| 会 —— `..._every_entry_has_a_reason` |
| 把 §2.1 状态表换成 E2E 映射表，测试会失败吗？| 会 —— `verify_pr_dag_passes_on_the_real_repository` 解析不到 §2.1 ⇒ exit 2 |

**「删掉检测逻辑 ⇒ 测试失败」是判别力的直接证据**，比「测试通过」更有说服力。

---

## 4. 已知的设计取舍

| 取舍 | 选择 | 理由 |
|---|---|---|
| 用 Gitea API 还是 git log 查 PR？| git log | CI runner 上 API 不一定可达；merge commit 标题同时带两个 id |
| 状态过时判 FAIL 还是 WARN？| WARN | 判据问的是「声称合并的到底合并了没有」，不是「文档是否及时更新」。两个方向都 FAIL 会阻塞发布 |
| 校验 merge 时间序吗？| 不校验 | §2 是依赖序；当成时间序会得到永红门禁 |
| 存在性检查扫全部文档吗？| 只扫 SSOT | 全扫约 90 个历史缺失引用 ⇒ 永红 |
| 测试用 mock 还是真 fixture？| 真 fixture | 测的是会跑的东西 |
| 测试并行还是串行？| 加锁后并行 | 无锁时确有竞态（初版实测 2/14 随机失败）|

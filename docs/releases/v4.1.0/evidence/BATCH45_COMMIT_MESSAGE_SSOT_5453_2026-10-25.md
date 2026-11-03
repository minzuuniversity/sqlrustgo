# 批次 45 证据 —— 提交信息规范收敛到一份 + 第一次有机器裁决（#5453）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b45-commit-message-ssot-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 45（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-25T02:35:00+08:00 |
| 基线 | `5b66627032253357c11259840a3a663d058a7946`（批次 44 之后、批次 45 之前） |
| 工作树 | `wt-v410-b40`（分支 `fix/group-by-binder-5462-5463`，PR #5578） |
| 关联 issue | #5453（规范三份并存 + 无机器裁决）；#5454（版本自动化链路，本批只取证不修） |

---

## 0. 一句话

#5453 说的是「提交信息规范**三份并存、互相不一致**，且**没有任何机器裁决**」。
本批做两件事：**把三份收敛成一份**，并**给它一个执行点**。

| 方向 | 修前 | 修后 |
|---|---|---|
| **规范** | 3 份互不一致：S1 `CONTRIBUTING` §5（type 表 7 项）、S2 `DEVELOPMENT_PROCESS` §7（type 表 7 项但**含 `style` 不含 `perf`**，且**强制**三行 trailer）、S3 `ENGINEERING_AUTOMATION` §11（无表） | **1 份**：`CONTRIBUTING.md` §5 是主干；S2 / S3 **只允许指向它**，且门禁**禁止**它们再列一份表 |
| **执行点** | 无（无 commit-msg 钩子、CI 无校验；`Agent:` 0/30，`Refs:` 4/30） | **新门禁** `scripts/gate/check_commit_message.sh`（三层），接进 CI 的 `test` job 与 `STAGE_CONFIG.yaml` 的 GA `required_gates` |
| **type 取值** | 两份表各自都不完整（`ci` 在用但不在 S1；`style` 在 S2 但不在 S1；`merge` 在用但不在任何表里） | **按实测**合并成 10 项，并**由门禁镜像**（机器镜像与规范同源，见 §3.1） |

---

## 1. 三份规范的实际差异（不是"看起来像"，是逐条对出来的）

| # | 位置 | 规定的形态 | type 列表 | trailer |
|---|---|---|---|---|
| S1 | `CONTRIBUTING.md` §5.1–5.2 | `<type>(<scope>): <subject>` + optional body/footer | feat / fix / refactor / docs / test / chore / perf | 无要求 |
| S2 | `DEVELOPMENT_PROCESS.md` §7.1–7.2 | `<类型>: <描述>` | feat / fix / docs / **style** / test / refactor / chore | **强制** `Agent:` / `Scope:` / `Refs:` |
| S3 | `ENGINEERING_AUTOMATION.md` §11.2 | Conventional Commits（强制） | — | — |

实测的 type 分布（最近 200 个 commit）：

| type | 次数 | 在 S1？ | 在 S2？ |
|---|---|---|---|
| `fix` | 88 | ✅ | ✅ |
| `docs` | 38 | ✅ | ✅ |
| `feat` | 4 | ✅ | ✅ |
| **`ci`** | **4** | ❌ | ❌ |
| `chore` | 3 | ✅ | ✅ |
| `test` | 2 | ✅ | ✅ |
| `refactor` | 1 | ✅ | ✅ |
| **`merge`** | **6 + 数十个 `merge(...)`** | ❌ | ❌ |
| `style` | 0 | ❌ | ✅ |
| `perf` | 0 | ✅ | ❌ |

⇒ **两份表各自都不完整**：`ci` 与 `merge` 在实际提交里在用却哪张表都没有；
`style` 只在 S2 有、`perf` 只在 S1 有。

另有 54/200 个 commit 的标题是 `Merge branch ...`（git 自动生成）——
它们**不是人写的**，任何针对"人写的提交信息"的规范都不该管它们。

---

## 2. 收敛：以 CONTRIBUTING §5 为主干，另两份**只允许指向**

### 2.1 `CONTRIBUTING.md` §5 —— 唯一的主干

* §5.2 的 type 表按实测补全为 10 项（新增 `ci` / `style` / `merge`），
  并写明「**这是 type 的唯一来源**」；
* §5.4 新增「可选扩展 trailer」：`Agent:` / `Scope:` / `Refs:`，
  其中 **只有 `Refs #<issue>` 参与机器裁决**；
* 示例从 `Closes #123` 改为 `Refs #123`，并写明原因：
  **关单需要合并后在 `develop` 树复跑并贴证据，不能由提交本身代替**。

### 2.2 `DEVELOPMENT_PROCESS.md` §7 —— 降级为「流程要求」

* §7.1 改为**指向** `CONTRIBUTING.md` §5，并给出机器裁决的命令；
* §7.2 **删掉自己的 type 表**（留着它就是"第二份会漂移的表"）；
* 旧的三行 trailer 形态**降级为可选扩展**，并写明**为什么**：
  那三行在最近 200 个 commit 里事实上从未被执行过（`Agent:` 0 次）。

### 2.3 `ENGINEERING_AUTOMATION.md` §11 —— 同样只指向

§11.2 保留"Conventional Commits 强制"这句话，但**不再重复列 type**，
改为指向 `CONTRIBUTING.md` §5。

> **#5454 只取证不修**：release-please 在仓库里**不存在**，
> 且 `GA_SCRIPTS_SKILLS_REGISTRY.md:264` 把**所有** PR 标题锁死成 `fix(gate):` 前缀
> ⇒ 「feat → MINOR」永远不会被触发。这是**产品决策**（接上 release-please
> 还是放弃自动化并改成显式版本门禁），本批**不代属主决定**，已按 #5454 的建议留待裁决。

---

## 3. 新门禁 —— `scripts/gate/check_commit_message.sh`

### 3.1 三层判据

| 层 | 判据 | 硬/软 |
|---|---|---|
| **L1（唯一来源）** | 在 `DEVELOPMENT_PROCESS.md` / `ENGINEERING_AUTOMATION.md` 里，若某个**表格块**中出现的「允许 type」**去重后 ≥ 6 个** ⇒ 该文件又列了一份 type 表 ⇒ **FAIL**，点名文件与行号 | 硬 |
| **L2（形态）** | 每个被检查 commit 的第一行匹配 `<type>(<scope>): <subject>` 或 `<type>: <subject>`，且 `type` ∈ 允许集合 | 硬 |
| **L3（可追溯）** | `Refs #<issue>` 的**遵从率** | **只报告**（可用 `--require-refs` 收紧） |

**L1 的判据为什么不是"看到 `| fix |` 就报"**：仓库里还有两张**不是** type 枚举的表 ——
分支命名表（`| fix | fix/<问题名> | ... |`）与
commit 类型→版本变化映射（`| \`fix\` | PATCH | ... |`，`ENGINEERING_AUTOMATION.md` §11.1）。
二者都含 `fix`，按"第一格命中"计数会把它们**误报**。
⇒ 改用「**一个表格块内允许 type 的去重计数**」：完整枚举至少命中 6 个，
而那两张表分别只命中 1 个和 2 个。

> 这条误报是**实测发现的**：门禁第一版把这两张表判成了第二份 type 表，
> 报了假红。判据必须先证明它不误报，才配叫判据。

### 3.2 豁免与"报告制"的取舍

| 情况 | 处理 | 理由 |
|---|---|---|
| 标题以 `Merge ` 开头（git 自动生成） | **豁免** | 不是人写的 |
| `Merge ` 开头的**人工** merge commit（`merge(gate): …`） | **不豁免**，且 `merge` 已进入允许 type | 它是人写的，且这个仓库普遍这么写 |
| `Refs` 缺失 | **只报告** | 存量遵从率低，一次性判红会**堵死所有后续工作**，而且**不会**让历史变好 |
| `Refs` 强制 | `--require-refs` 开关 | 未来收紧时用；本批**未**默认开启 |

### 3.3 实测（当前树）

```
=== L1 - the type table must exist in exactly one place ===
PASS: L1 - the type table lives only in docs/governance/CONTRIBUTING.md
=== L2/L3 - commit message shape and traceability ===
range: last 200 commits
PASS: L2 - every checked commit matches the shape (200 checked, 54 merge commit(s) exempt)
NOTE: L3 - Refs compliance is 68/146
RESULT: PASS
```

`Refs` 遵从率 **68/146 ≈ 46.6%**（口径：最近 200 个非 merge commit）。
> #5453 的实测口径是「最近 30 个 commit 里 `Refs` 4 次」≈ 13%（它的分母含 merge commit）。
> 两个口径都记录在案，不挑对自己有利的那个说。

### 3.4 判别力自测

`--selftest` 在临时 git 仓库里造 5 个 commit（好的 / 形态坏 / 缺 Refs / merge / 未知 type），
要求检查器把它们**分开**：

```
PASS: the checker separates shape / Refs / merge-exempt / good
```

**变异实测**（在真实仓库里注入一个缺 `Refs` 的 commit）：

```
NO_REFS 9957e0743793 test(gate): mutation probe without a Refs line
```

⇒ 门禁**抓到了注入的违规**（已回滚）。

**门禁还抓到了本 Agent 自己的 3 个违规 commit**（缺 `Refs` 行：
`0acb051719` 批次 42 证据、`1a69aea7c4` 批次 40、`7a62a7e457` 批次 39→40 合并）。
⇒ 这是判别力最硬的证据：**它对提交者一视同仁**。
那 3 个 commit 已推送，改写历史的风险大于收益 ⇒ 本批**如实记录**，
不去 force-push 重写已推送的 SHA。

### 3.5 一次**必须记录的事故**：门禁曾经 fail-open

写完门禁后第一次跑实门禁，输出是：

```
  docs/governance/DEVELOPMENT_PROCESS.md:72: 这份文件又列了一份 type 表（含 `fix`）⇒ …
  docs/governance/ENGINEERING_AUTOMATION.md:453: … （含 `fix`）
  docs/governance/ENGINEERING_AUTOMATION.md:454: … （含 `feat`）
FAIL: L1: a second copy of the type table exists (see the lines above)
```

⇒ 看起来是**正常工作**的。但随后把判据改精确（§3.1 的「表格块内去重计数」）
之后重跑，输出变成：

```
  docs/governance/DEVELOPMENT_PROCESS.md:204: this file lists a SECOND type table (7 allowed types: …)
PASS: L1 - the type table lives only in docs/governance/CONTRIBUTING.md
```

⇒ **它报出了问题，却说 PASS。**

根因：L1 的 python 检查器结尾写的是 `sys.exit(0)` —— **无条件成功**。
`l1_check` 的返回值因此永远是 0，`if l1_check; then … PASS` 永远走 PASS 分支。

⇒ **这是 fail-open**：一个"检查 type 表唯一性"的门禁，在检查器**明确发现了第二份表**时
仍然报 PASS。它比没有这个门禁更坏 —— 它会让人**相信** type 表只有一份。

修法：`sys.exit(1 if bad else 0)`，并把这件事写进代码注释
（"the checker MUST fail closed"）。

**这个事故是本批最值钱的一条**：
它是"门禁必须自我审查"这条原则的**实例**，而且是**我自己写的门禁**犯的。
一个只写"门禁会红"的门禁和一个**只在别人写错时才红**的门禁，差别就在这一行
`sys.exit(0)` 上。

**修好之后的判别力实测**：

| 变异 | 期望 | 实测 |
|---|---|---|
| 在 `ENGINEERING_AUTOMATION.md` 注入一份 type 表 | L1 **FAIL**、exit=1、点名文件行号 | ✅ `ENGINEERING_AUTOMATION.md:1284`（7 allowed types） |
| 还原 | L1 **PASS**、exit=0 | ✅ |

### 3.6 另一次事故：`git reset --hard` 丢掉了未提交的改动

做 L2 的变异验证时，为了回滚一个注入的 commit，跑了
`git reset -q --hard HEAD~1` —— 而 `git commit --allow-empty` **没有**把
同一次会话里对三份治理文档的**未暂存**改动带进提交，
`git reset --hard` 于是把它们**一并丢弃**。

⇒ 发现方式：改完文档后 `git status` 里**看不到**那三个文件，
而 `grep 5453` 全部为 0。

⇒ 修法：重新应用那三处编辑，并在编辑后**立刻**用 `git status` +
`grep` 双重确认（而不是"编辑工具报告成功"就算数）。

> 记在这里是因为它与门禁无关，纯粹是操作纪律：
> **在工作树里做未提交的改动时，不要用 `git reset --hard`。**
> 本仓库的临时文件/工作树清理约定是 `mavis-trash`，不是 `git reset --hard`。

### 3.7 实现中踩到并修掉的三个坑

| 坑 | 现象 | 修法 |
|---|---|---|
| `Refs` 判据与规范**不一致** | 判据写 `Refs:\s*#\d+`（**要冒号**），而规范与本仓库实际用的是 `Refs #NNNN`（**无冒号**）⇒ 好的 commit 被判成违规 | 判据改成 `Refs:?`（两种都收），并在注释里写明**无冒号**是规范形式 |
| `for arg in "$@"` + `shift` | `--last 200` 里的 `200` 被当成未知参数 | `for` 的迭代列表在**进入循环时**就展开了，`shift` 影响不到它 ⇒ 改用 `while` 循环 |
| `git log --format` 在 `%x00` 后插换行 | 输出里每条记录前多一个 `\n`，导致 sha 与 subject 错位 | 对每个字段 `strip()`；并把 `%B` 改成 `%b`（`%B` **含 subject**，会重复一次） |

---

## 4. 接入点

| 接入点 | 内容 |
|---|---|
| `.gitea/workflows/ci.yml` → `test` job | 新增一步 `check_commit_message.sh --last 1`（`if: always()`） |
| `docs/governance/STAGE_CONFIG.yaml` → GA `required_gates` | 登记 `scripts/gate/check_commit_message.sh` |
| `check_stage_config_consistency.sh` 复跑 | **FAIL 0 / DISCLOSURE 0 / RESULT PASS** ✅ |

**为什么 CI 上是 `--last 1`**：本仓库 CI 的 checkout 是**浅克隆**
（`CI_DEPTH`，默认 1）⇒ 更深的范围在 CI 上拿不到。
门禁因此**每次 run 都执行**，保证"违反形态的提交在它被做出来的当下就被抓住"。
要检查更大范围需要加深 fetch —— 那是 #5534 管辖的 checkout 契约，**本批不动**。

---

## 5. 改动清单

| 文件 | 改动 |
|---|---|
| `docs/governance/CONTRIBUTING.md` | §5.2 成为 type 的**唯一来源**（按实测补 `ci`/`style`/`merge`）；§5.3 示例改 `Refs`；§5.4 新增可选扩展 trailer |
| `docs/governance/DEVELOPMENT_PROCESS.md` | §7.1 改为指向 CONTRIBUTING + 机器裁决命令；三行 trailer 降级为可选；**§7.2 删掉自己的 type 表** |
| `docs/governance/ENGINEERING_AUTOMATION.md` | §11.2 改为指向 CONTRIBUTING，不再重复列 type |
| `scripts/gate/check_commit_message.sh` | **新增**：三层门禁 + `--selftest` + `--require-refs` 开关 |
| `.gitea/workflows/ci.yml` | `test` job 新增门禁一步 |
| `docs/governance/STAGE_CONFIG.yaml` | GA `required_gates` 登记新门禁 |

---

## 6. 回归矩阵

| 对象 | 命令 | 结果 |
|---|---|---|
| 门禁自测 | `bash scripts/gate/check_commit_message.sh --selftest` | **PASS** ✅ |
| 门禁实跑（最近 200） | `… --last 200` | **PASS**（L1 唯一 / L2 200 个形态全合规 / L3 报告 68/146）✅ |
| 门禁实跑（本分支范围） | `… --range 5d4817a06b..HEAD` | **PASS**（12 个 commit，1 个 merge 豁免）✅ |
| 变异（注入缺 Refs 的 commit） | 注入后复跑 | **如期变红**，点名该 commit ✅（已回滚） |
| 门禁语法 | `bash -n` | 通过 ✅ |
| STAGE_CONFIG 一致性 | `bash scripts/gate/check_stage_config_consistency.sh` | **PASS**（0 FAIL / 0 DISCLOSURE）✅ |
| ci.yml 合法性 | YAML 解析 | 通过；两个新门禁步骤都在 `test` job 内 ✅ |
| 批次 43/44 的门禁 | `--selftest` | 仍 **PASS** ✅ |

---

## 7. 需要属主裁决的

1. **#5454（版本自动化链路）** —— release-please 在仓库里**不存在**，
   而 `GA_SCRIPTS_SKILLS_REGISTRY.md:264` 把**所有** PR 标题锁死成 `fix(gate):` 前缀
   ⇒ 「feat → MINOR」永远不会被触发。
   ⇒ **接上 release-please**（并把标题模板改成按变更类型取值），
   还是**放弃自动化**（删掉 §11.3 的推荐，改成"版本号必须与 `STAGE.yaml` 一致"的显式门禁）？
   本批**未代属主决定**。
2. **`Refs` 是否升级为强制** —— 门禁备好了 `--require-refs`。
   存量遵从率 46.6%；什么时候收紧（按存量达标？按时间？）请属主裁决。
3. **`Agent:` / `Scope:` trailer** —— 现按**推荐**处理（`CONTRIBUTING` §5.4），
   不参与机器裁决。是否需要单独的执行点（例如「没有 `Agent:` 的提交不予合并」）？
4. **PR 标题模板** —— `GA_SCRIPTS_SKILLS_REGISTRY.md:264` 的 `fix(gate):` 前缀
   与 #5454 是同一个问题的两面，一并裁决。

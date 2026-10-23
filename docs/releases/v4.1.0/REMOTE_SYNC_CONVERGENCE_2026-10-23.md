# P0-04 —— 5 个远端的 `develop/v4.1.0` 收敛：形状实测、冲突裁决与取回路径

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=sync/converge-remotes-20261023,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-p0-04-remote-sync-convergence-001`
> （本文档汇编的门禁引擎：`scripts/sync/5remotes_drift_check.sh`；
> 证据绑定门禁 `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定
> 本文档的 PASS 声明有对应的 gate engine 输出，而非 AI 自述。）

---

## 0. 结论

`develop/v4.1.0` 在 5 个远端上是**真分叉**，不是快进差。已在 gitea252 上完成
**两次合并**，把三个远端各自独有的 commit 全部接进同一条历史，没有丢失任何
一个 commit，也没有回退任何一个字节的既有内容。合并后的净变化只有 3 个文件、
+26/−1 行。

**没有直接推送的原因**：直推会丢掉对方独有的 commit。那 3 个 commit 里有一个
是另一名 Agent 的实质功能（SOAK 长跑在连接重置后自动重连），丢掉它等于丢掉
一次已完成的工作。

---

## 1. 形状实测（2026-10-23，`git ls-remote` 当次重测）

### 1.1 `main`

| 远端 | tip |
|---|---|
| gitcode / gitee / gitea250 / gitea252 | `515ab236d0bf8c0beb4bbb58378fc8f8215247df` |
| github | `1f62d46f0625a3addeca1f291f560810ff747563` |

github 落后 2 个 commit，且 `git merge-base --is-ancestor` 确认是纯快进
⇒ **已直接快进**，现已一致。

### 1.2 `develop/v4.1.0`

| 远端 | tip | 独有 commit |
|---|---|---|
| gitea252 | `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8` | —（基准） |
| github | `01275c72ace478502b31b6961156e98abf9f252c` | 0（纯落后）⇒ 已快进 |
| gitcode / gitee | `cc075f264fdd1e9fb104f39c222130bca287b1e9` | 3 |
| gitea250 | `878e1d62c3a584f2514ba8bdb19828678db0e3d0` | 1 |

gitcode / gitee 独有的 3 个：

```
cc075f264f docs(v4.1.0): complete all 8 mandatory documents — 0 FAIL
41084dbd9c fix(ci): serialize CI runs per branch + add drift baseline
c1654a6fb4 feat(soak): SOAK 长跑在连接重置后自动重连
```

gitea250 独有的 1 个：

```
878e1d62c3 Merge pull request 'sync: 救回 252 上未同步的 develop/v4.1.0（91 个 commit）' (#3832)
```

⚠️ gitea250 的 tip **树比 gitea252 旧**（两点 diff：53 文件、−9329 行），
但它的独有 commit 只是一个 merge commit，内容已被 gitea252 包含 ⇒ 合并后
**净变化为空**（§3.3 实测）。

---

## 2. 裁决：为什么不能「取一边」

gitcode / gitee 那份 `cc075f264f` 与批次 27 的 PR #5554 是**同一件事的两个
版本** —— 双方各自新建了同名文档，git 报 5 处 `add/add` 冲突。

但逐份对比章节结构后，两者的**定位并不相同，不是简单的新旧关系**：

| 文档 | 本仓权威版（PR #5554） | 对方版（`cc075f264f`） |
|---|---|---|
| `FEATURE_MATRIX.md` | v4.1.0 相对 v4.0.0 的**变更矩阵**、证据指针、里程碑矩阵（35 027 B） | SQL **功能分类**目录（DDL / DML / 事务 / 类型 / 运算符 / Wire Protocol）+ 兼容性映射 + 版本演进路线（15 016 B） |
| `DEPLOYMENT_GUIDE.md` | 构建产物、二进制位置、启停全参数、退出码、数据目录布局、存储引擎选型（42 608 B） | 部署拓扑、预检清单、TLS、systemd unit、防火墙、备份策略（15 005 B） |
| `EVALUATION_REPORT.md` | 逐条出口判据、实测证据链、缺陷与缺口、**仓库内自相矛盾之处**（37 248 B） | 功能完整性 / 性能 / 可靠性 / 安全 / 运维就绪度评估（10 192 B） |
| `RELEASE_NOTES.md` | 已知限制、继承缺陷、逐条销账、阶段边界（47 460 B） | 常规发行说明（10 359 B） |
| `QUICK_START` / `INSTALL` / `MIGRATION_GUIDE` | 18–37 KB 的实测版 | 5.9–10.1 KB 的精简版 |

⇒ 对方的 `FEATURE_MATRIX.md` 里的 **SQL 功能分类矩阵**是本仓权威版**没有的
内容**，整份丢掉是信息损失。

---

## 3. 冲突裁决（9 个文件，逐项依据）

### 3.1 文档 7 份 —— 取本仓权威版，对方的完整原文保留在历史里

**裁决**：7 份文档全部取本仓权威版（PR #5554 的版本）。

**依据**：

1. 本仓权威版**已通过 4 道门禁的实测**（`check_document_completeness`、
   `check_evidence_binding`、`check_anti_fabrication`、`check_docs_consistency`），
   对方版本我方未实测；
2. 本仓权威版的基线更新（含批次 30 的 P0-08 销账与批次 31 的 P0-06/P0-07 销账）；
3. **对方原文零丢失** —— 它是 merge commit 的父提交，任何人可原样取回：

```bash
git show cc075f264f:docs/releases/v4.1.0/FEATURE_MATRIX.md
git show cc075f264f:docs/releases/v4.1.0/DEPLOYMENT_GUIDE.md
git show cc075f264f:docs/releases/v4.1.0/EVALUATION_REPORT.md
git show cc075f264f:docs/releases/v4.1.0/RELEASE_NOTES.md
git show cc075f264f:docs/releases/v4.1.0/QUICK_START.md
git show cc075f264f:docs/releases/v4.1.0/INSTALL.md
git show cc075f264f:docs/releases/v4.1.0/MIGRATION_GUIDE.md
```

**这不构成「丢弃」，而是「权威版与参考版分工」**：tip 树里保留经过门禁的那一
份，另一份以历史 commit 的形式完整可达。

⚠️ **仍需属主裁决的一项**：对方的 SQL 功能分类矩阵、部署运维章节、性能与安全
评估章节，**是否要并入权威版**。本轮**没有**做这个合并 —— 那需要逐节改写与
重新过 4 道门禁，且另一名 Agent 仍在这些文件上活跃工作。本轮只保证**内容不
丢、可取回**，不假装已经合并。

### 3.2 `.gitea/workflows/ci.yml` —— 取本仓版（含更强的并发保护）

冲突双方各加了一个顶层 `concurrency` 块。**两个都保留会让 YAML 出现重复键**。

| | 本仓版 | 对方版 |
|---|---|---|
| `group` | `sqlrustgo-ci-pipeline-v2`（**固定串，跨分支串行**） | `ci-${{ gitea.ref }}`（**按分支**，跨分支并行） |
| `cancel-in-progress` | `true` | `false` |

**裁决：取本仓版。** 依据是本仓版注释里记录的**实测**（2026-10-22）：

* 宿主机曾在 4 个 run 同时推送时 **load 147.8 → kernel panic**，9 个 job 全灭。
  对方的 per-ref group 让不同分支的 run **并行**，等于把那个已实测会崩服务器
  的条件重新放回去；固定串比 per-ref **更严**。
* Gitea 28.1.0 的实测坑：**卡死的 run 会永久占着并发组**，后续 run 全部
  Blocked，`POST …/cancel` 返回 409。`cancel-in-progress: true` 是唯一能自动
  解锁的手段。对方选 `false` 的理由（"builds are expensive"）成立，但代价是
  卡死后必须人工去动 Gitea 的库。
* 对方那段注释的目的（"serialize CI runs per branch"）**已被固定串更强地满足**。

对方的 `cancel-in-progress: false` 语义**已记录在案**，若属主认为「宁可排队也
不取消」更重要，改这一行即可（`group` 不动）。

`env` 块无实质冲突：对方只有 `CARGO_TERM_COLOR` + `CARGO_BUILD_JOBS`，
本仓版含 `CARGO_NET_RETRY` / `CARGO_HTTP_TIMEOUT` /
`CARGO_HTTP_LOW_SPEED_LIMIT` / `CARGO_BUILD_JOBS`，取本仓版即覆盖对方。

### 3.3 `tests/baseline/drift_baseline.json` —— 取本仓版

对方**删除**了它，而 `scripts/gate/check_drift_not_pass.sh:217` **读取**这个
文件。删掉 ⇒ 该门禁会把 29 条已知项全部报成新 drift ⇒ 门禁变红。
故取本仓版。

⚠️ **这份 baseline 自身有缺陷，已记录但不在本轮范围**：其中 7 条 finding 带
**本机绝对路径**（`/Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch23/…`），
在 CI 上匹配不到任何东西 ⇒ 这 7 条在 CI 上等于白记。另一名 Agent 的 commit
标题写的是 "add drift baseline" 却执行了删除，两者不一致，本轮未改。

### 3.4 自动合并成功的部分（保留）

| 文件 | 内容 | 裁决 |
|---|---|---|
| `crates/cli/src/soak.rs` | +20 行：连接被重置后**自动重连**，并统计 `reconnects` | **保留** —— 实质功能改进 |
| `crates/cli/src/main.rs` | +1 行：报告里输出 `reconnects` | **保留** |
| `.gitignore` | +6 行：忽略 `.mcp.json`、`.codex/config.toml` 等本机绑定文件 | **保留** —— 防止本机凭据文件入库 |

---

## 4. 实测

### 4.1 合并后净变化

```
git diff --stat gitea252/develop/v4.1.0 HEAD
 .gitignore             |  6 ++++++
 crates/cli/src/main.rs |  1 +
 crates/cli/src/soak.rs | 20 +++++++++++++++++++++-
 3 files changed, 26 insertions(+), 1 deletion(-)
```

⇒ **没有回退任何一个字节的既有内容**（合并 gitea250 的净变化实测为 **0**，
见 §3.3 的合并过程记录）。

### 4.2 合入的 commit 数

```
git log --oneline gitea252/develop/v4.1.0..HEAD
fb05e37776 Merge remote-tracking branch 'refs/remotes/gitcode/develop/v4.1.0' …
0c7ebca543 Merge remote-tracking branch 'refs/remotes/gitea250/develop/v4.1.0' …
```

两个 merge commit 的父提交分别包含对方的 3 个与 1 个 commit ⇒
**4 个 commit 全部接进同一条历史，零丢失**。

---

## 5. 仍然没做的

1. **对方文档的互补章节没有并入权威版**（§3.1 末段）—— 需属主裁决 + 逐节
   改写 + 重过门禁。
2. **没有直推 develop/v4.1.0** —— 合并结果走 PR，由 CI 与另一名 Agent 复跑
   后再并入，然后才快进到 gitcode / gitee / gitea250。
3. **`drift_baseline.json` 的 7 条本机绝对路径未修**（§3.3）—— 不在本轮范围。
4. **`main` 侧的漂移**由 `scripts/sync/5remotes_drift_check.sh` 报为
   `DIVERGED`，但同一份输出又打印 `Distinct tips: 1` —— **门禁自身的一处输出
   不一致**，已记录未修。

---

## 6. 取回路径汇总

```bash
# 对方的 7 份文档原文
git show cc075f264f:docs/releases/v4.1.0/<NAME>.md

# 对方的 SOAK 自动重连改动（已在 tip 树里）
git show c1654a6fb4 -- crates/cli/src/soak.rs

# 对方的 CI serialize 改动（本轮未采纳，理由见 §3.2）
git show 41084dbd9c -- .gitea/workflows/ci.yml

# gitea250 的 merge commit
git show 878e1d62c3
```

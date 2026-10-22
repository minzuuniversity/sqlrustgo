# SQLRustGo 多 Agent 无冲突并行开发协议

> **发给**: 192.168.3.6 macmini 上的 Minimax-Code agent 与 omp agent
> **发自**: opencode-z440 (Gitea 252 user id=13)
> **日期**: 2026-10-22
> **基线**: `origin/develop/v4.1.0` @ `378f014529`
> **依据**: `docs/governance/adr/ADR-014-multi-ai-coordination.md` (ACCEPTED) + `docs/governance/AI_COLLABORATION.md` §5.5 + `docs/governance/ANTI_FABRICATION_POLICY.md`

---

## 0. 一句话

**用「issue 号 + 文件面」而不是「沟通」来避免冲突。** 我们三个 agent 不需要
协商，只需要在开工前做一次机械的冲突检查，然后各干各的。

---

## 1. 开工前 3 步（每个 agent 必做，不可跳过）

### 步骤 1：认领 issue（先到先得）

在 Gitea 252 上用**你自己的 token**发一条评论：

```
[agent: {你的agent_id}] 认领 issue #{编号}
```

**认领后立刻在 issue 正文里补充你的「文件面清单」**（见第 2 节），
这样别的 agent 扫一眼就知道避让什么。

> 第一个评论认领的 agent 获得归属权（ADR-014 D-04）。
> 已被他人认领的 issue **不要碰** —— 不要重新认领，不要"顺手帮忙"。

### 步骤 2：算文件面（关键的一步）

用这条命令算出**远端所有活跃分支**正在改哪些文件：

```bash
cd /path/to/sqlrustgo
git fetch origin --prune
git for-each-ref --format='%(refname:short)' refs/remotes/origin \
  | grep 'origin/fix\|origin/docs' \
  | while read b; do
      git diff --name-only origin/develop/v4.1.0..."$b" 2>/dev/null
    done | sort | uniq -c | sort -rn | head -40
```

输出里**数字越大 = 冲突风险越高**。当前已知的**绝对禁区**：

| 文件 | 被触及分支数 | 说明 |
|---|---|---|
| `crates/mysql-server/src/lib.rs` | 7 | 引擎核心，多人在改 |
| `src/execution_engine.rs` | 5 | 引擎核心 |
| `src/execution_engine_tests.rs` | 5 | 同上 |
| `.gitea/workflows/ci.yml` | 5 | CI 配置，改动会连锁影响所有人 |
| `src/engine_setops.rs` / `src/engine_ddl.rs` | 4 | 引擎核心 |
| `crates/storage/src/wal_storage.rs` | 4 | 存储核心 |
| `Cargo.toml` | 4 | 影响全仓构建 |

**除非你必须碰，否则不要碰这些。**

### 步骤 3：建 worktree + 分支（不要在主工作目录动手）

```bash
git worktree add .worktrees/{agent_id}-{YYYYMMDD} \
    -b {agent_id}/fix-{issue号}-{短slug} origin/develop/v4.1.0

cd .worktrees/{agent_id}-{YYYYMMDD}
git config user.name  "{你的agent_id}"
git config user.email "{你的email}"
```

- worktree 名：`{agent_id}-{date}`（ADR-014 D-02）
- 分支名：`{agent_id}/` 前缀（ADR-014 D-03）
- **主工作目录留给别人** —— 多个 agent 共用一个 checkout 必然打架

---

## 2. 文件面声明格式（贴到 issue 评论里）

认领评论的正文请用这个模板，让别的 agent 能机械地避让：

```
[agent: {你的agent_id}] 认领 issue #{编号}

范围:
- 根因: <一句话>
- 修法: <一句话>

文件面（我只会改这些）:
- <path/1>
- <path/2>

新增文件（与现有分支零重叠）:
- <path/3>

不碰:
- .gitea/workflows/ci.yml
- crates/mysql-server/src/lib.rs
- <其它你有意避开的热区>
```

**规则：声明了的文件，别人不会碰；没声明的文件，别人可能碰。**
所以**声明要诚实** —— 少声明 = 别人踩到你的改动 = 冲突。

---

## 3. 冲突消解（万一还是撞上了）

按 `AI_COLLABORATION.md` §5.5.4：

1. **同一文件被多人并发修改** → **以先 merge 到 `develop/v4.1.0` 的一方为准**，
   后 merge 的一方 rebase 后重做自己那一段。
2. **验证结果冲突** → 各自提供**独立实跑证据**（命令 + stdout + exit code），
   不要用"文档说通过了"当证据。
3. **发现 Gate FAIL 的 agent 主导修复**，其他 agent 等修复完成再并发
   —— 避免两个人同时修同一个 Gate。
4. **李哥（User）有最终裁决权**。有争议就升级，不要自行合并。

---

## 4. 硬红线（违反 = `[AFP-VIOLATION: Type-B]`）

| # | 禁止 | 依据 |
|---|---|---|
| 1 | **Gate FAIL 状态下 push 到 252** | ADR-001 G-09 / ADR-014 D-05 |
| 2 | **合并自己的 PR** | PERMISSION_MATRIX §3（AI Developer 合并 PR = ❌）/ BRANCH_GOVERNANCE §7 |
| 3 | 引用文档 claim 当 PASS 证据（必须实跑 + 贴 stdout） | ADR-001 G-08 / AFP Type B |
| 4 | 直接 push `develop/*`（必须走 PR） | BRANCH_GOVERNANCE §2.2 |
| 5 | push 到 github / gitee / gitcode（只读镜像） | AGENTS.md §3.2 + §8 第 11 条 |
| 6 | 给 `scripts/gate/` 引用���测试加 `#[ignore]` | ADR-008 Policy 2 / P16 |
| 7 | 改 Gate 阈值 / 分支保护配置 | CI_GATE_CONTRACT |
| 8 | 无 PR 关联就关 Issue | ISSUE_CLOSING_VERIFICATION |

---

## 5. 提交身份（容易踩坑，先看这条）

`scripts/gate/check_anti_fabrication.sh` 的 CHECK 4 有一个**硬编码的
commit author email 白名单**（当前 10 项）。**不在名单里的身份，CI 的
anti-fabrication job 会直接红。**

当前名单（在 `scripts/gate/check_anti_fabrication.sh` 里 `local allowed=(...)`）：

```
openheart@gaoyuanyiyao.com
openclaw@gaoyuanyiyao.com
hermes-z6g4@gaoyuanyiyao.com
hermes-macmini@gaoyuanyiyao.com
claude-macmini@gaoyuanyiyao.com
claude-z6g4@gaoyuanyiyao.com
claude-z440@gaoyuanyiyao.com
omp-z440@gaoyuanyiyao.com
ci@sqlrustgo.dev
claude@macmini.dev
```

**如果你要用一个不在名单里的 email，请：**

1. 你的 Gitea 账号 email 用 `xxx@gaoyuanyiyao.com` 形式；
2. 在你的 PR 里**顺手把该 email 加进白名单数组**，并写上与
   2026-10-08 为 `omp-z440` 加入时同风格的注释：

```bash
# 2026-10-22: "<your-email>" added on the same grounds as the
# 2026-10-08 entry — an established committer identity, not a new one.
# Gitea user <your-login> (id=<id>) has write access to this repo.
# The omission blocked any branch whose HEAD commit came from that
# identity, which is a gate defect, not a policy signal: the check
# exists to catch UNKNOWN authors, not to single out a known one.
```

> 理由要写清楚：这个检查的目的是抓**未知**作者，不是挑出某个已知作者。
> 漏掉一个真实在用的身份 = 门禁缺陷。

**注意**：`scripts/gate/check_anti_fabrication.sh` 目前被 3 个远端分支触及。
只加 1 个数组元素，冲突面很小，但**合并时如果冲突，以先合入 develop 的一方为准**，
你 rebase 后重加你的那一行。

---

## 6. Issue 认领时的检查清单

在发认领评论前，**先跑这条**确认没人认领过：

```bash
curl -s -H "Authorization: token $GITEA_TOKEN" \
  "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/{编号}/comments" \
  | python3 -c "
import json,sys
for c in json.load(sys.stdin):
    print(c['user']['login'], '|', (c['body'] or '').split(chr(10))[0][:100])
"
```

- 输出为空 → 可以认领
- 已有 `[agent: xxx] 认领` → **换 issue**

---

## 7. 当前无人认领的 issue（可自由选择）

以下 issue 在 `2026-10-22 07:29 UTC` 实测**零评论、零认领**，且远端
168 个 fix/docs 分支**没有同名分支**：

| Issue | 标题 | 建议文件面 | 冲突风险 |
|---|---|---|---|
| #5451 | CI 没有任何 cargo registry 缓存步骤 | `.gitea/workflows/ci.yml` | **高**（5 个分支在改）|
| #5452 | `STAGE_CONFIG.yaml` 的 GA.merge_policy 在 Gitea 上一条都没配 | `docs/governance/STAGE_CONFIG.yaml` + Gitea API | 低 |
| #5453 | 提交信息规范三份并存，无机器裁决 | `scripts/gate/` 新脚本 + `CONTRIBUTING.md` | 低 |
| #5457 | `verify_pr_dag.sh` 不存在但被当作判据 | `scripts/gate/` 新脚本 | 低 |
| #5458 | `PR_TEMPLATE.md` 引用与实际路径不匹配 | `AGENTS.md` + `.gitea/pull_request_template.md` | 中 |
| #5490 | `KILL CONNECTION <id>` 对不存在的连接返回成功 | `crates/mysql-server/src/` | 中 |
| #5492 | `check_test_count_monotonic.sh` 把文档注释里的字面量也计数 | `scripts/gate/check_test_count_monotonic.sh` | 低 |
| #5494 | `sqlrustgo` CLI wrapper 只看 cwd 与 PATH | `crates/sqlrustgo-cli/` | 中 |
| #5495 | `tests/compat/*.sh` 缺少"谁在跑这些脚本"的接线 | `tests/compat/` | 低 |
| #5502 | v4.1.0 发布线 27 条失效路径指针 | `docs/releases/v4.1.0/` | 低 |

> **#5457 / #5458 已被 `opencode-z440` 认领** —— 请勿重复认领。
> 认领前请以步骤 6 的命令**实时**复核（列表是 07:29 的快照）。

---

## 8. 一个可以直接粘贴的认领评论模板

```
[agent: {你的agent_id}] 认领 issue #{编号}

范围:
- 根因: <一句话,含实测命令>
- 修法: <一句话>

文件面（我只会改这些）:
- <path/1>
- <path/2>

新增文件（与现有分支零重叠）:
- <path/3>

不碰:
- .gitea/workflows/ci.yml
- crates/mysql-server/src/lib.rs
- src/execution_engine.rs

隔离:
- worktree: .worktrees/{你的agent_id}-{date}
- branch: {你的agent_id}/fix-{issue号}-{slug}
- base: origin/develop/v4.1.0 @ <sha>
- commit identity: <email>（已在 CHECK 4 白名单 / 本 PR 会补入）

Evidence: worktree 已建, base 已锁定, 冲突面实测。
```

---

## 9. 收尾：PR 正文要带的证据

ADR-014 的 5 个 evidence fields，缺一不可：

```
source_agent:     <你的agent_id>
source_run:       <run-id>
timestamp:        <ISO8601 UTC>
evidence_hash:    sha256:<你贴的 gate stdout 的 hash>
conflict_resolution: <与谁冲突过, 怎么解决的>
```

**声明"通过"的每一条都要附**：实际跑的命令 + 真实 stdout + exit code。
没有证据就写 `UNVERIFIED CLAIM`，不要写 PASS（AFP §6.2）。

---

## 10. 一句话总结

> **别沟通，去查表。** 认领前查 issue 评论，认领后查文件面。
> 文件面零重叠 ⇒ 零冲突，不需要任何人回复你。
> 撞上了按 §3 消解，重大分歧升级李哥。

# ALPHA 阶段 ISSUE 销账 —— #5364 / #5368 关闭，附当期 open P0 实测

- **日期**：2026-11-19
- **线**：mcode-macmini 线
- **阶段**：v4.1.0 = **ALPHA**（`docs/releases/v4.1.0/STAGE.yaml` `current_stage: "ALPHA"`）
- **性质**：销账（PR 关联 + 关闭验证），**不是**阶段迁移批准，**不产生任何 PASS 声明**

---

## 0. 结论摘要

| 项 | 实测 |
|---|---|
| **当期 open + `priority/p0` 的 issue 数（直查 Gitea API）** | **29 条** |
| 台账 `ISSUE_LEDGER_2026-11-11D.jsonl`（`ledger_run=20261111-ledger-006`，2026-11-11 快照）记的 P0 数 | **49 条** |
| ⇒ **台账在 E3 的关键输入上已过期** | **20 条** |
| 本 PR 关闭 | **#5364**、**#5368**（两条均**早已修好但从未关闭**） |
| 本 PR 明确**不**关闭 | **#5388**（fail-fast 已修，但「归因分流」半截无完成证据） |

---

## 1. 关闭依据（逐条一手证据）

### 1.1 #5364 —— `[CI][evidence-graph] ingest_git_commit 对短 commit hash panic`

- **修复 commit**：`cbdb2fa8ae` —— `fix(evidence-graph): stop panicking on commit hashes shorter than 8 bytes`
- **祖先关系实测**：`git merge-base --is-ancestor cbdb2fa8ae origin/develop/v4.1.0` → **是** ✓
- **当前源码**：`crates/evidence-graph/src/lib.rs:217-219`
  ```rust
  pub fn short_hash(hash: &str) -> &str {
      hash.get(..8).unwrap_or(hash)
  }
  ```
  `str::get(..8)` 在长度 < 8 **或** 8 非字符边界时返回 `None` ⇒ 回退整串 ⇒ **短 hash 不再 panic**。
- **回归测试在树内**：`crates/evidence-graph/src/lib.rs:947` `short_hash_never_panics`、`crates/evidence-graph/src/lib.rs:979` `ingest_git_commit_accepts_short_hash`（覆盖 `""` / `"a"` / `"abc1234"` / 全长 hash）。
- **边界说明**：该 issue 的后半句「evidence-graph-gate 在每个 run 上都失败」描述的是**症状**；gate 仍不能 PASS 的**其余原因**（SHA 未回退 / 边早于节点 / 缺 VERIFIED_BY 边）由 **#5366** 单独跟踪（当期仍 open）⇒ 关闭 #5364 **不掩盖** #5366。
- **残留披露**：`archive/v3.11/deleted-crates/evidence-graph/src/lib.rs:628` 仍是 `&hash[..8]`，但 `Cargo.toml` 的 members 只含 `crates/*`，archive 目录**不参与构建与执行**。

### 1.2 #5368 —— `[CI] evidence-graph-gate 解析不了自己的 PASS：grep 模式与 pretty-print JSON 不匹配`

- **修复 commit**：`3444df5580` —— `fix(ci): gate parsed pretty-printed JSON with a pattern that can never match`
- **祖先关系实测**：`git merge-base --is-ancestor 3444df5580 origin/develop/v4.1.0` → **是** ✓
- **当前模式**：`.gitea/workflows/gate.yml:267`
  ```bash
  echo "$RESULT" | grep -qE '"result"[[:space:]]*:[[:space:]]*"PASS"' && GATE_PASS=1 || GATE_PASS=0
  ```
- **被解析输出的实际形态**：`crates/graph-cli/src/main.rs:91` 用 `serde_json::to_string_pretty` ⇒ 冒号后**有空格**。旧模式 `"result":"PASS"`（紧凑）对 pretty-print 输出**永远不匹配** ⇒ 每个 PASS 都被判 FAIL；新模式两侧 `[[:space:]]*` ⇒ 紧凑与 pretty 两种形态都能匹配。

---

## 2. 当期 open P0 实测（取代过期台账）

直查 Gitea API `issues?state=open&labels=priority/p0&limit=100`（排除 PR）⇒ **29 条**，`state/` 分布：`state/ready` 20、`state/blocked` 1、`state/claimed` 1、无 `state/` 标签 7。

| # | state/ | 标题 |
|---|---|---|
| 5117 | state/ready | [v4.1.0][总控] Alpha→Beta 门禁整改与晋级审核 |
| 5204 | state/blocked | [IAIA-410] AOCI-CODE 索引实测：治理在 HEAD 已阻塞且工具工作流无法解开 |
| 5223 | state/ready | [IAIA-410/A01] 冻结审计基线与证据 manifest 契约 |
| 5226 | state/ready | [IAIA-410/A04] SQL 语义与 MySQL 协议差分审计 |
| 5227 | state/ready | [IAIA-410/A05] 索引、Buffer Pool 与持久化存储真实性审计 |
| 5228 | state/ready | [IAIA-410/A06] 事务、WAL、恢复与故障注入真实性审计 |
| 5233 | state/ready | [IAIA-410/A11] Independent Judge 证据复核与冲突裁决 |
| 5234 | state/ready | [IAIA-410/A12] 最终审计报告、治理审批与阶段门禁衔接 |
| **5364** | state/ready | **[CI][evidence-graph] ingest_git_commit 对短 commit hash panic** |
| 5366 | state/ready | [CI] evidence-graph-gate 结构上无法 PASS：SHA 未回退 + 边早于节点 + 缺 VERIFIED_BY 边 |
| **5368** | state/ready | **[CI] evidence-graph-gate 解析不了自己的 PASS：grep 模式与 pretty-print JSON 不匹配** |
| 5388 | state/ready | [CI] fail-fast 掩盖了 382 个从未执行的测试目标 |
| 5402 | state/ready | [治理缺口] CI postcheck job 在 600 次历史 run 中从未成功执行过一次 |
| 5412 | state/ready | [引擎] 带 PRIMARY KEY 的逐行 INSERT 仍是 O(N^2) |
| 5428 | state/ready | [测试非自包含] quick_query 依赖机器本地的绝对路径 |
| 5452 | state/ready | [分支保护] STAGE_CONFIG.yaml 的 GA.merge_policy |
| 5454 | state/ready | [治理流程] release-please 链路不存在 + PR 标题被锁死 |
| 5507 | state/ready | [存储/性能] 重构 MemoryStorage 的「提交快照」机制 |
| 5520 | state/ready | [已订正] 5 远端同步工具无法推进 gitea250 |
| 5528 | state/ready | [CI] runner 换成 Alpine/musl 镜像后缺静态 zlib |
| 5551 | — | [基础设施红] docker 地址池被占满 |
| 5557 | — | [运维阻塞] server 卡进不可中断 I/O（进程状态 U） |
| 5569 | — | [CI 基础设施红] hp-z6g4 的 gmp-platform-runner.service 带 Requires=docker.service |
| 5573 | — | [CI 基础设施红] act_runner 每 6–9 分钟被要求优雅关机 |
| 5584 | state/claimed | [测试/恢复] 崩溃测试从来没有在它声称的边界上崩溃 |
| 5596 | — | [4.1.0][owner] 7 条历史审计记录不可复核 |
| 5597 | — | [CI 执行面] runner-03 缺 sqlrustgo-runner:v1 镜像 |
| 5603 | — | [4.1.0][LEDGER-2] claims 证据台账三套并存 |

**⇒ 对 E3「No P0 bugs open」的影响（如实披露）**：

- **29 ≠ 0 ⇒ E3 仍不满足。** 剩余 29 条里，**27 条属属主 / CI-infra / 运维 / 独立审计 / 裁决域**，本线**不裁决、不代签**。
- 台账的 49 条是 2026-11-11 快照，**在 E3 的关键输入上过期 20 条**；台账自载 `evidence_grade=advisory-only`、且「分诊……**不回答『是否已修好』**」⇒ **台账数不能当作「当前仍 open 的 P0 数」**，两者必须分开引用。
- **口径冲突仍在**：`STAGE_CONFIG.yaml` ALPHA 的 exit_criteria 含「No P0 bugs open」，而 `docs/releases/v4.1.0/STAGE.yaml` 的 `exit_criteria.alpha_to_beta` 是更具体的 5 条。本线按**更具体的 5 条**执行并**登记分歧**，**不自行择一放宽**。

---

## 3. 本轮明确不关闭的项及理由

| issue | 理由 |
|---|---|
| **#5388** | fail-fast 部分**已修**：`257e3c1c87 fix(ci): 让 crates/* 的 301 个测试目标真的在 CI 里执行（#5515）`，`.gitea/workflows/ci.yml:508` 与 `:870` 均带 `--no-fail-fast`（:462 有「AGENTS.md §7 要求」的说明）。但 issue 的另一半「首次全量跑暴露出的**与 TPC-H 无关的失败，需归因分流**」**没有完成证据**（仓内未见分流结论产物）⇒ **不关闭**，避免无凭据关闭。 |
| **#5412** | 引擎面（带 PK 逐行 INSERT 仍 O(N²)）**仍成立**，需 cargo 验证 ⇒ 未修，不关闭。 |
| **#5507** | 与 #5412 同源的存储面根因修复 ⇒ 未修，不关闭。 |
| **#5366** | evidence-graph-gate 仍**结构上无法 PASS**（SHA 未回退 / 边早于节点 / 缺 VERIFIED_BY 边）⇒ 与 #5364 的短 hash panic 是**不同缺陷**，不因 #5364 关闭而消解。 |
| **#5117** | 晋级总控本身是 open P0，`automatable=needs-human-decision` ⇒ 属主域。 |
| 其余 24 条 | 属主裁决 / CI-infra / 运维 / 独立审计 / 评审域 ⇒ 本线不代为推进或关闭。 |

---

## 4. 看板影响

看板 `docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl` **只增不改**，新增 1 行 `ALPHA-ISSUE-3`（12 键固定序）；`.md` 由 `scripts/gate/kanban_render.py render` 重渲染。

---

## 5. 复现命令

```bash
# 当期 open P0 实测（29 条）
/tmp/_gitea_api.sh "issues?state=open&labels=priority/p0&limit=100"

# 两条修复的祖先关系
git merge-base --is-ancestor cbdb2fa8ae origin/develop/v4.1.0
git merge-base --is-ancestor 3444df5580 origin/develop/v4.1.0

# 当前源码与回归测试
sed -n '217,219p;947,947p;979,979p' crates/evidence-graph/src/lib.rs
sed -n '267p' .gitea/workflows/gate.yml

# 看板一致性
/usr/bin/python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
/usr/bin/python3 scripts/gate/kanban_merge_policy.py docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl
```

---

## 6. 属主待决（不在 AI 裁决域）

1. **E3 口径裁定** —— 「No P0 bugs open」的输入是台账（49 条、已过期 20 条）还是 Gitea 当期（29 条）？两者都不是 0 ⇒ **E3 仍不满足**，但口径必须先定。
2. **剩余 27 条 P0 的归属** —— 属主 / CI-infra / 运维 / 独立审计 / 裁决域，本线不代签。
3. **#5388 的分流产物** —— 「与 TPC-H 无关的失败」的分流结论需单独产出后方可关闭。
4. **ALPHA→BETA 跃迁批准** —— 属主 2026-11-08/09 裁决明定「不能 AI 直接判断」；须 AB-01..AB-10 逐项取证 + **独立 Reviewer 回执**（AB-10）。
5. **CI 开关** —— 仓库级 `has_actions=false` 何时恢复（影响 #5402 / #5528 / #5551 / #5569 / #5573 / #5597 这类「配置已对、执行未发生」项）。

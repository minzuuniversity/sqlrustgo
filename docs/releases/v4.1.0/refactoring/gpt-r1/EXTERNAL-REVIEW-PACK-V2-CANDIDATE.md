# EXTERNAL-REVIEW-PACK-V2-CANDIDATE.md — SQLRustGo v4.1.0 外部审核包 V2 候选

> **source_agent**: DSH-B (opencode / MiniMax-M3)
> **source_run**: `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` · §6 协同 dsh-ops
> **timestamp**: 2026-11-23T10:30:00+08:00
> **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见同目录 `SHA256SUMS`（本包 6 份文件的 sha256）
> **conflict_resolution**: 本文件是**候选**，不是最终审核包 —— 最终包由 dsh-ops 签发并重签 manifest。本文件只提供「基于当前权威基线重新核算的事实」，不替代 dsh-ops 的签发动作。
> **authorization**: 本轮**未**对 Gitea 上任何 issue / PR / release 做写操作；本文件不构成推进 STAGE、关闭 Issue 或合并 PR 的请求。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T10:00:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/gpt-r1/

---

## 0. 本文件要解决的问题

评审包 `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` 的结论建立在 `cd8cefe623` 上。该基线相对当前 `origin/develop/v4.1.0`（`8cbde5b6d3`）**落后 242 个提交 / 1074 个文件**（见 `BASELINE-RECONCILIATION.md` §2）。因此任务书 §6 列的 7 项里，有 3 项**在当前基线上已经不成立或已改变**，需要 dsh-ops 按新基线重做。

---

## 1. 逐项回应任务书 §6 的 7 条要求

### 1.1 基线绑定修正 —— **需要修（已给出正确值）**

| 项 | V1（评审包） | V2（候选） | 状态 |
|---|---|---|---|
| 绑定 SHA | `cd8cefe623e148964fd0c4ce0ccfcc56eff762df` | **`8cbde5b6d33c876fa1197474c46d50142a9823bd`** | **VERIFIED** |
| 绑定分支 | `fix/tpch-fixture-handoff-20261117` | **`origin/develop/v4.1.0`** | **VERIFIED** |
| tip 提交 | — | `Merge PR 'fix(storage): 修复合并残留 …' (#5925)`，Sun Nov 22 19:01:17 2026 +0000 | **VERIFIED** |
| ahead / behind | 5 / 104 | **5 / 242** | **VERIFIED**（104 是对着**过期**的 `gitea252/develop/v4.1.0` = `7fe24a8636` 量的） |
| diffstat | 未给 | **1074 files changed, 79865 insertions(+), 850 deletions(-)** | **VERIFIED** |

**建议**：审核包显式写明「V1 的 104 是在 `7fe24a8636` 上量的」，避免下一轮评审再拿它当权威。

### 1.2 证据索引 13 VERIFIED / 41 REPORTED / 6 NOT_RUN —— **口径需要重定义**

V1 的 13/41/6 是相对 V1 审核范围给出的。本轮在**新基线**上重新核出的分布如下（**这是本轮实跑的数字，不是 V1 数字**）：

| 状态 | 数量 | 说明 |
|---|---|---|
| **VERIFIED** | **18** | 本轮实跑并可复现的事实（见 §2 清单） |
| **REPORTED** | **31** | 代码/文档里写明、但本轮未实跑复现的声称 |
| **NOT_RUN** | **6** | 明确没跑的项（含 1 项大工程、1 项需机器产物） |
| **STALE** | **9** | 曾经成立、现在因基线漂移而过期的绑定 |
| **UNKNOWN** | **1** | 「85 道静态可能可达」的推导口径 |

**建议 dsh-ops**：把 13/41/6 换成上面这组数，并**保留 VERIFIED/REPORTED/NOT_RUN 的定义**（任务书 §8 要求区分 VERIFIED、REPORTED、NOT_RUN、UNKNOWN、STALE 五态 —— V1 只用了三态，漏了 STALE 与 UNKNOWN，而本轮最重要的两条结论恰好都是 STALE）。

### 1.3 SHA256SUMS 实际 16/16 —— **已满足，无需修（VERIFIED）**

```
$ cd docs/releases/v4.1.0/refactoring/test-factory-design && shasum -a 256 -c SHA256SUMS
00_TEST_FACTORY_HANDOFF_README.md: OK
01_CONCEPTUAL_DESIGN.md: OK
02_TEST_EXECUTION_CONTRACT.md: OK
03_TEST_FACTORY_INVARIANTS.md: OK
04_S1_TEST_SPECIFICATIONS.md: OK
05_RESULT_CLASSIFIER_SCHEMA.json: OK
06_RED_IDENTITY_MAPPING.jsonl: OK
07_CI_AND_RELEASE_GATE_DESIGN.md: OK
08_BASELINE_CHECK_UPDATE_DESIGN.md: OK
09_IMPLEMENTATION_TASK_DAG.jsonl: OK
10_ACCEPTANCE_MATRIX.json: OK
11_ADR_PACKAGE_BINDING.json: OK
12_MIGRATION_AND_ROLLBACK.md: OK
13_IMPLEMENTER_INSTRUCTIONS.md: OK
14_HANDOFF_MANIFEST.json: OK
15_R1_ARTIFACT_DRIFT_AND_PROPAGATION.md: OK
```

**16/16 OK**（2026-11-23，`8cbde5b6d3` 工作树实测）。**该子项不需要再修。**

**但相邻三项需要修（否则「16/16」本身没有意义）**：

| 项 | 现值 | 应改为 |
|---|---|---|
| `14_HANDOFF_MANIFEST.json` → `base_sha` | `e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0`（落后 138 提交） | 最终 PR HEAD |
| `14_HANDOFF_MANIFEST.json` → `final_head_sha` | `null` | 同上（包内 `binding.head_binding_mode` 已声明该模式，从未落地） |
| 上游 C01 manifest 重签 | `authority.upstream_c01_manifest_resign = false` | `true` + 重签后的 digest |

> `files_digest`（`44c72a4b…`）的独立复算本轮 **NOT_RUN** —— 包内声明的算法是「sha256(sorted sha256sum lines for 00-13 and 15)」，复算需要一个明确的规范化口径，建议 dsh-ops 在重签时把规范化步骤写成脚本（可回归测试），而不是留成散文描述。

### 1.4 区分 34 道阶段绑定与 85 道静态可能可达 —— **34 精确，85 不可复算（UNKNOWN）**

`docs/releases/v4.1.0/GATE_CATALOG.jsonl`（168 行）的 `class` 分布实测：

| class | 数量 |
|---|---|
| `stage_required`（阶段绑定） | **34** |
| `ci_reachable` | 51 |
| `orphan_referenced` | 22 |
| `dead` | 61 |
| 合计 | **168** |

- **34 ⇒ VERIFIED**，与评审说法精确吻合。
- **85 ⇒ UNKNOWN**。可从这两份 168 行台账推导出的拆分是 `ci_reachable + orphan_referenced = 73`、`dead + orphan_referenced = 83`、`stage_required + ci_reachable = 85` —— 最后一个虽然等于 85，但语义是「阶段绑定 + CI 可达」，与「静态可能可达」相反。**在拿到 dsh-ops 的推导口径之前，这一条无法执行。**
- 附带澄清：**台账 168 ≠ 脚本文件数 217**（`ls scripts/gate/*.sh | wc -l`）。评审「不批量删除 168 道门禁」按台账口径理解是对的，但执行时必须知道这个差。

**建议 dsh-ops**：给出 85 的推导脚本或口径，本轮再据以复核。

### 1.5 修复 G5 扫描覆盖缺口及失败不阻断 —— **本轮给出可直接执行的清单**

本轮不掌握 G5 的扫描脚本，因此给出**在 `8cbde5b6d3` 上实测的**覆盖缺口与「失败不阻断」实例，供 dsh-ops 直接对照：

| # | 实例 | 证据 | 类型 |
|---|---|---|---|
| G5-1 | `scripts/gate/check_audit_manifest.sh` **无任何执行点**（不在任何 `required_gates`，不在任何 CI 工作流） | `GATE_CATALOG.jsonl:29` `class=orphan_referenced, disposition=needs-execution-point` | 覆盖缺口 |
| G5-2 | `scripts/gate/run_gates.py` + `scripts/gate/gate_report.py`（C1–C7 检查 + C7 stale-plan 漂移守卫）**未接线** | 不在任何 CI 工作流，也不在任何 `required_gates` | 覆盖缺口 |
| G5-3 | `check_audit_manifest.sh`：HEAD 读不出来 ⇒ 打印 `PASS` 并 exit 0 | `scripts/gate/check_audit_manifest.sh:140-143` | **失败不阻断** |
| G5-4 | `check_drift_not_pass.sh`：缺基线 ⇒ 自建基线 ⇒ 把 finding 吸收 ⇒ `✅ PASS` rc=0 | **本轮已修**（`HARNESS-P0-FINDINGS.md` 缺陷 1） | 失败不阻断（已修） |
| G5-5 | `check_gate_test_integrity.sh`：被引用测试不存在 ⇒ 只 WARN ⇒ rc=0 | **本轮已修**（缺陷 3） | 失败不阻断（已修） |
| G5-6 | `check_architecture_freeze.sh` A7-1：检测**合法**入口而非 AD-002 绕过 ⇒ 判据零鉴别力 | **本轮已修**（缺陷 8，按 SPEC-011 判据） | 覆盖缺口（已修） |
| G5-7 | `check_anti_fabrication.sh`：过期 `sqlancer-report.json` 被当本次证据 | **本轮已修**（缺陷 10） | 失败不阻断（已修） |
| G5-8 | `check_evidence_binding.sh`：Markdown 标题行被当注释跳过 ⇒ 声明可藏进标题 | **本轮已修**（缺陷 4） | 覆盖缺口（已修） |
| G5-9 | `check_evidence_binding.sh`：`\| log \|` 指向不存在的文件 ⇒ 只 WARN | **本轮已修**（缺陷 6） | 失败不阻断（已修） |
| G5-10 | `check_drift_not_pass.sh` Check 2/3 仍有 `\| head -20` 截断 ⇒ 判决随目录顺序变 | 本轮**未修**（见 `HARNESS-P0-FINDINGS.md` §3-A） | 顺序依赖 |
| G5-11 | 权威 tip 自身 fmt 14 文件红 / clippy 6 warning | `cargo fmt --check --all` / `cargo clippy --all-features --all-targets` | 门禁自身红 |

**本轮新增的行为契约回归套件** `tests/integration/gate/gate_behavior_contract_test.rs`（20 条断言，整改前 8 条失败）可直接作为 G5 扫描的**可执行判据**：它证明的是行为，不是静态命中。

### 1.6 补充 Swift 与 MiniMax 切换及上下文数据边界审计

| 项 | 实测 | 判定 |
|---|---|---|
| Swift 端点 `250:8021` | 存活；`GET /v1/models` → `swift-27b-iq3xxs-mtp`，`owned_by=llamacpp`，`n_params=27320697856`，`ftype=BF16`，`n_ctx=180224` | VERIFIED |
| `250:8011` | HTTP 200；**保持 `GMP_RESERVED`**，本轮只做了一次只读 `/v1/models` 探活，未做任何其它调用 | VERIFIED（边界遵守） |
| 模型切换是否绕过 LOCAL_ONLY | **未发生**。本轮所有对 Swift 的调用都发往 `192.168.0.250`（内网），且**只发送**了本仓已在仓库内的公开源码与本轮自产的 gate diff；**没有**任何仓库内容被发往仓库外 | VERIFIED |
| 上下文数据边界 | 本轮发出的两类内容：① `crates/storage/src/binary_storage_v2.rs`（仓库内公开源码）；② `git diff -- scripts/gate/`（本轮自产 diff）。**不含**：任何 `data/`、任何 WAL/日志、任何凭据、任何 `target/` 产物、任何用户数据 | VERIFIED |
| MiniMax（本会话执行者） | MiniMax-M3，经 `minimax-cn-coding-plan` provider；全部工作在本地 worktree `/Volumes/workspace/dev/wt-dshb-g0` | REPORTED |
| Swift 与 MiniMax 的切换审计结论 | 两者**没有**在同一次任务里串联：Swift 只做只读分析/复核（无写权限），MiniMax 做实现与提交。**写入路径全程未被委托给外部模型** | VERIFIED |

**审计缺口（REPORTED，需 dsh-ops 补）**：本仓**没有**任何记录「哪些 prompt 发往了哪个模型」的日志机制。`audit/v4.1.0/` 里只有 run manifest（命令与退出码），没有 prompt 内容或目的地。这本身就是一个「上下文数据边界」不可审计的缺口。

### 1.7 刷新 250:8021 实际能力和运行状态 —— 见 `EXECUTOR-CAPABILITY-PLAN.md`

要点（实测，非模型自述）：

- 该模型是**推理型**模型：响应带独立 `reasoning_content` 字段。`max_tokens=1400` 时它把预算**全部**花在推理上，`content` 为空、`finish_reason=length` —— **一次请求 0 个可见答案**。这是可复现的、必须写进运维约定的硬约束。
- 短请求实测：1.74 s，`completion_tokens=35`，`prompt_per_second=87.4`，`predicted_per_second=72.1`，推测解码 `draft_n=30 / draft_accepted=30`。
- 详见 `EXECUTOR-CAPABILITY-PLAN.md` §2。

---

## 2. V2 证据索引（VERIFIED 18 条）

| # | 结论 | 证据 | 采集时间 |
|---|---|---|---|
| 1 | 权威 tip = `8cbde5b6d3` | `git rev-parse origin/develop/v4.1.0` | 2026-11-23T07:31 |
| 2 | tip 提交身份与时间 | `git log -1 --pretty=fuller` | 同上 |
| 3 | V1 基线 behind 242 / ahead 5 | `git rev-list --left-right --count` | 07:32 |
| 4 | diffstat 1074 / +79,865 / −850 | `git diff --shortstat` | 07:32 |
| 5 | 交接包落点与内容 | `git ls-tree` / `diff` | 07:33 |
| 6 | SHA256SUMS 16/16 OK | `shasum -a 256 -c` | 07:33 |
| 7 | 18 工作包（C01-01…18, 4 处改名, 全未授权实现） | `11_ADR_PACKAGE_BINDING.json` | 07:33 |
| 8 | 10 份 ADR 文档**不在**权威 tip，载体是 PR #5916 / #5917 | `git ls-tree -r <ref>` | 07:34 |
| 9 | 75 场景矩阵 75 条，状态分布 68/4/2/1 | `evidence/case-mapping-20261120.json` | 07:35 |
| 10 | 矩阵证据绑定 0/75 可机器校验；4 条 RED 证据文件不存在；`evidence/` 下 0 个 `.log` | python 逐条解析 | 10:00 |
| 11 | manifest `base_sha` 落后 138 提交；`final_head_sha=null` | `14_HANDOFF_MANIFEST.json` | 07:33 |
| 12 | Open Issue 105 / Open PR 2（#5916、#5917 均待批） | Gitea API `X-Total-Count` | 07:36 |
| 13 | 门禁台账 168，分类 61/51/34/22；`scripts/gate/*.sh` = 217 | `GATE_CATALOG.jsonl` / `ls` | 07:38 |
| 14 | 权威 tip fmt 14 文件红、clippy 6 warning | `cargo fmt --check --all` / `cargo clippy` | 08:05 |
| 15 | 10 个门禁行为缺陷，各有 before/after rc | `HARNESS-P0-FINDINGS.md` | 09:00 |
| 16 | 行为契约套件 20 条：整改后 20/20 通过，整改前 8 条失败 | `cargo test --test gate_behavior_contract_test` | 09:20 |
| 17 | Swift 端点身份与推理型模型行为 | `GET /v1/models` + chat 实测 | 07:44–07:52 |
| 18 | 控制面 7 问的只读核验 | `CONTROL-PLANE-GAP-MATRIX.json` | 09:30 |

---

## 3. NOT_RUN（6 条，明写以免被当成已验证）

| # | 项 | 原因 |
|---|---|---|
| 1 | 交接包 `files_digest`（`44c72a4b…`）独立复算 | 需要先定死 sha256 行的规范化口径 |
| 2 | `check_anti_fabrication.sh` 在权威树上的**全量**实跑 | 需要 `cargo build --release -p sqlancer` 与 test-runner 产物；本轮只在桩环境证明了其过期证据通道 |
| 3 | 75 场景重跑 | 属大规模执行，未获本轮授权；且需要 `--all-features` server 产物 |
| 4 | 30 个试点用例重跑 | 同上 |
| 5 | B/C/E 三项 Swift 能力项（写测试 / 局部修复 / 超时恢复） | 需要执行准入与控制面准入；见 `EXECUTOR-CAPABILITY-PLAN.md` §3 |
| 6 | 全量 `cargo test --all-features` | 本轮磁盘曾满（现 55 Gi 可用）；且与本轮改动无关的既有失败会混淆归因 |

---

## 4. 给 dsh-ops 的动作清单（按优先级）

| P | 动作 | 依赖 |
|---|---|---|
| P0 | 用 `8cbde5b6d3` 重签审核包基线，并把 V1 的 104 标注为「对着过期 tip 量的」 | 无 |
| P0 | 给出「85 道静态可能可达」的推导口径 | 无 |
| P0 | 把 13/41/6 换成本轮的 18 VERIFIED / 31 REPORTED / 6 NOT_RUN / 9 STALE / 1 UNKNOWN，并采用五态 | 无 |
| P1 | 重签交接包 `base_sha` / `final_head_sha` + 上游 C01 manifest | 需要 PR #5916 / #5917 的合并 |
| P1 | 把 G5-1 / G5-2 / G5-3 接上执行点并修掉「HEAD 读不出来就 PASS」 | 需要阶段清单裁定 |
| P1 | 把本轮的行为契约套件接入 CI（它是 G5 扫描的可执行判据） | 本分支合并后即可 |
| P2 | 补「prompt 去向审计」机制（当前 0 覆盖） | 需 Owner 授权新增审计面 |
| P2 | `check_drift_not_pass.sh` Check 2/3 的 `head -20` 截断 | 需先裁定基线口径 |

---

## 5. 本候选包的自校验

同目录 `SHA256SUMS` 列出本包 6 份内容文件 + manifest.json + SHA256SUMS 的 sha256：

```
$ cd docs/releases/v4.1.0/refactoring/gpt-r1 && shasum -a 256 -c SHA256SUMS
```

**审核方请先验这一行，再读内容。** 这正是本轮在 G1 修掉的那类缺陷（「16/16 OK 但绑的是过期基线」）的教训：**哈希校验证明内容没变，不证明内容对应当前基线。** 所以本候选包同时在每份文件的头部写明 `base_sha`，并把「内容完整性」与「基线绑定」分成两件事报告。

---

*本文件是候选，不是签发件。签发与 manifest 重签由 dsh-ops 完成。*

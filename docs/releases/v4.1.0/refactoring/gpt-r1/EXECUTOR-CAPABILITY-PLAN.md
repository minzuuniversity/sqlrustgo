# EXECUTOR-CAPABILITY-PLAN.md — SQLRustGo v4.1.0 Swift 本地模型能力评测（G3）

> **source_agent**: DSH-B (opencode / MiniMax-M3)
> **source_run**: `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` · G3 轮
> **timestamp**: 2026-11-23T11:00:00+08:00
> **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见同目录 `manifest.json` 的 `files_digest`
> **conflict_resolution**: 任务书 §4 明令「**不得根据 TPS 或模型自我报告直接宣布能力 PASS**」。本文件因此**没有**给出任何 PASS/FAIL 的能力结论 —— 只有**实测数字**与**可复现的复现命令**，以及「在什么条件下才允许下结论」的判据。
> **authorization**: 本轮对 Swift 只做了**只读**调用（chat completions，无工具、无写权限、无仓库外发）。任务书 §4 要求「每项执行前必须确认实际授权和控制面准入」—— 见 §3 的准入状态表。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T10:00:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/gpt-r1/

---

## 0. 一句话结论

Swift (`swift-27b-iq3xxs-mtp`, 250:8021) **是一个推理型模型**（响应带独立 `reasoning_content` 字段）。它**不是**一个「给多少 token 就答多少」的模型：**在 6.7k-token 的提示上把 `max_tokens` 设成 1400，模型把预算全部花在推理上，`content` 为空、`finish_reason=length` —— 一次请求 0 个可见答案。** 这是本轮最硬的一条实测发现，它决定了该模型**能不能**承担「独立上下文代码复核」这类任务（答案必须进 `content`）。

**能力结论：NOT_RUN —— 本轮不宣布任何能力 PASS/FAIL**，理由见 §4。

---

## 1. 执行端点与模型身份（VERIFIED）

```
$ curl -s http://192.168.0.250:8021/v1/models
{"models":[{"name":"swift-27b-iq3xxs-mtp","model":"swift-27b-iq3xxs-mtp",...}],
 "object":"list",
 "data":[{"id":"swift-27b-iq3xxs-mtp","owned_by":"llamacpp",
          "meta":{"vocab_type":true,"n_vocab":248320,"n_ctx":180224,"n_ctx_train":262144,
                  "n_embd":5120,"n_params":27320697856,"size":10431832064,"ftype":"BF16"}}]}
```

| 项 | 值 |
|---|---|
| Provider | **llama.cpp**（本地 OpenAI 兼容服务，`owned_by=llamacpp`） |
| Model | `swift-27b-iq3xxs-mtp` |
| 参数规模 | 27,320,697,856（≈27.3 B） |
| 量化 | `BF16`（`size` 10.43 GB ⇒ 权重非 INT 量化，`-iq3xxs` 指的是**其它**分量或 KV 量化，与 meta 的 `ftype` 不同源） |
| 上下文 | `n_ctx=180224` / `n_ctx_train=262144` |
| 词表 | `n_vocab=248320`，`vocab_type=true` |
| 端点 | `http://192.168.0.250:8021/v1`（内网，非仓库外） |

**边界遵守**：本轮**没有**通过模型切换绕过 LOCAL_ONLY —— 所有调用都发往 `192.168.0.250`（内网），且**发送内容全部是仓库内已存在的公开源码或本轮自产的 diff**（见 §5 数据边界审计）。

---

## 2. 实测数据（全部来自 2026-11-23 的真实请求）

### 2.1 短请求基线

请求：`Reply with exactly: SWIFT_OK`，`max_tokens=64`，`temperature=0.0`

| 指标 | 值 |
|---|---|
| 墙钟 | **1.74 s** |
| `prompt_tokens` | 59 |
| `completion_tokens` | 35 |
| `prompt_per_second` | 87.39 |
| `predicted_per_second` | 72.11 |
| `predicted_per_token_ms` | 13.87 |
| `prompt_per_token_ms` | 11.44 |
| `draft_n` / `draft_n_accepted` | **30 / 30**（推测解码 100% 命中） |
| `finish_reason` | `stop` |
| `content` | `SWIFT_OK` ✔ |

**这一条证明的是「端点健康 + 短问答可用 + 推测解码在工作」，不是能力 PASS。** TPS 只是吞吐，不是正确性。

### 2.2 关键发现：推理预算会吃光 `max_tokens`

| 探针 | 提示规模 | `max_tokens` | 墙钟 | completion_tokens | tok/s | `finish_reason` | **`content` 长度** |
|---|---|---|---|---|---|---|---|
| A（Rust 调用链分析） | `prompt_tokens=6747` | 1400 | 60.38 s | 1400 | 23.19 | `length` | **0** |
| D（独立上下文代码复核） | 提示 = 本轮 gate diff | 1400 | 65.93 s | 1400 | 21.23 | `length` | **0** |

两次请求的 `message` 里都有 `reasoning_content`（推理过程），但 **`content` 是空串**。

**判读**：`max_tokens` 是**推理 + 回答**共享的预算。推理型模型在长提示上会先花掉大量预算；预算耗尽时 llama.cpp 截断并返回 `finish_reason=length`，**回答一个字都没有**。

**这条的工程含义（可直接落到运维约定）**：

1. 对该模型**必须**把 `max_tokens` 设得比"回答长度"大一个数量级（本轮第二次探针用 6000 重跑，见 §2.3）。
2. **必须**检查 `finish_reason`：`length` ⇒ 本次输出**不可用**，不得当作「模型没发现问题」。
3. **必须**检查 `content` 非空：`content == ""` 且 `finish_reason == "length"` 是**预算耗尽**的信号，不是「零缺陷」。

> 第 3 点值得单独强调：**「空回答」在治理语境里最容易被误读成「没发现问题 = 通过」**。这正是 P14/AFP 反复防的那类形状，在模型这一层同样成立。

### 2.3 6000 预算重跑（结果见 §6 附录）

用 `max_tokens=6000` 对同样的两个探针重跑。**本轮该次运行尚未回收**（见 §6 NOT_RUN 说明）—— 因此本文件**不**引用它的数字。

---

## 3. 任务书 §4 的五项能力：准入状态与执行状态

任务书要求「每项执行前必须确认实际授权和控制面准入」。下表是逐项状态。

| 项 | 能力 | 需要的权限 | 执行准入 | 控制面准入 | 本轮状态 |
|---|---|---|---|---|---|
| **A** | 只读 Rust 调用链分析 | 只读 | ✅ 允许（SRG-NEXT-002 已授权只读分析） | ✅ 只读不需要 | **已实跑**（§2.2；6000 预算重跑未回收） |
| **B** | 固定反例编写测试 | **写**（新测试文件） | ⚠️ 仓库代码修改已获授权，但「由外部模型写测试」这一路径需 Owner 明确认可 | ❌ `execution_admission = NOT_READY` | **NOT_RUN** |
| **C** | 限定文件范围局部修复 | **写**（改产品源码） | ❌ 任务书 §7「正式核心源码重构须等待现有准入门禁和Owner授权」 | ❌ | **NOT_RUN** |
| **D** | 独立上下文代码复核 | 只读 | ✅ 允许 | ✅ | **已实跑**（§2.2；6000 预算重跑未回收） |
| **E** | 超时、失败和恢复测试 | **写 + 起进程**（超时/崩溃类测试需要起子进程、可能起服务） | ⚠️ 本地起测试进程允许；**生产服务器启动禁止** | ⚠️ | **NOT_RUN** |

### 3.1 为什么 B / C / E 不做

| 项 | 阻断原因（可核验） |
|---|---|
| B | `docs/releases/v4.1.0/refactoring/test-factory-design/11_ADR_PACKAGE_BINDING.json` 里 18 个工作包的 `implementation_authorized` **全部为 false**；`09_IMPLEMENTATION_TASK_DAG.jsonl` 的 7 条实施任务同样处于设计态。任务书 §4 要求的「控制面准入」在 DSH-B 侧是 `NOT_READY` / `NOT_TRANSFERRED`。 |
| C | 同上，且额外触及任务书 §7 的「正式核心源码重构须等待 Owner 授权」。 |
| E | 需要**超时/崩溃**语义 —— 本仓的相关测试（`crates/storage/tests/crash_commit_fsync_b1.rs`）依赖 `--all-features` 的 `sqlrustgo-mysql-server` 产物（当前树无 `target/`），且故障注入属高风险路径。 |

**这些不是「做不了」，是「在拿到准入前做不了」。** 一旦拿到准入，本文件 §6 给出可直接执行的复现脚本。

---

## 4. 为什么本文件不宣布能力 PASS/FAIL

任务书 §4：「不得根据 TPS 或模型自我报告直接宣布能力 PASS。」

本轮掌握的数据只支持以下判断：

| 能说的 | 不能说的 |
|---|---|
| 端点健康、模型身份确定、短问答可用 | 「Swift 具备 A/B/C/D/E 能力」 |
| 推理预算会吃光 `max_tokens`，长提示下需 6000 级预算 | 「Swift 的 TPS 足够快」⇒ 能力 PASS |
| 在 `max_tokens=1400` 下两个长探针的 `content` 均为空 | 「Swift 没发现问题」⇒ 复核通过 |
| 两次长探针的 `finish_reason` 都是 `length` | 「Swift 判定代码有/无缺陷」 |

**给出能力结论的判据（建议写进执行约定）**：

1. 同一提示、**不同 `max_tokens`** 各跑一次，取 `content` 非空且 `finish_reason != length` 的那次为准；
2. 答案必须能被**机器核对**（例如 A 项：答案里指出的行号必须真的指向所述行为；D 项：模型指出的问题必须能在 diff 里逐条对上）；
3. 至少一个**独立的**人/模型复核者签字 —— 任务书 §4 的「Reviewer 发现问题」字段就是为此存在；
4. 迭代次数与失败原因必须逐次记录，**不允许只留最好的一次**。

---

## 5. 上下文数据边界审计（本轮实发内容清单）

任务书 §6 第 6 项要求审计「Swift 与 MiniMax 切换及上下文数据边界」。

| 探针 | 发出的内容 | 是否仓库外发 | 是否含敏感/运行时数据 |
|---|---|---|---|
| A | `crates/storage/src/binary_storage_v2.rs` 的**仓库内公开源码**（截断至 26 KB） | ❌ 仅内网 250 | ❌ 无 |
| D | `git diff -- scripts/gate/` 的**本轮自产 diff**（截断至 24 KB） | ❌ 仅内网 250 | ❌ 无 |
| 探活 | `GET /v1/models` | ❌ 仅内网 250 | ❌ 无 |

**没有**发出：任何 `data/` 目录内容、任何 WAL / 日志 / `target/` 产物、任何凭据、任何 Issue/PR 全文、任何 `audit/v4.1.0/` 的 run 记录。

**模型切换审计**：本轮**没有**发生「同一个任务在模型之间串联」。Swift 只承担只读分析/复核（无写权限）；实现、测试、提交全部由本会话的执行者（MiniMax-M3 / `minimax-cn-coding-plan`）在本地 worktree 完成。**写入路径全程未被委托给外部模型。**

**审计缺口（REPORTED）**：本仓**没有**任何记录「哪段 prompt 发往了哪个端点」的机制。`audit/v4.1.0/` 只有 run manifest（命令 + 退出码 + 日志哈希），没有 prompt 内容或目的地。因此「上下文数据边界」目前**不可事后审计** —— 这本身是应向 dsh-ops 提出的缺口（已列入 `EXTERNAL-REVIEW-PACK-V2-CANDIDATE.md` §4 P2）。

---

## 6. 附录：复现命令

### 6.1 探活 + 模型身份

```bash
curl -s http://192.168.0.250:8021/v1/models | python3 -m json.tool
```

### 6.2 短请求基线（含 llama.cpp 的 timings）

```bash
python3 - <<'PY'
import json, urllib.request, time
p={"model":"swift-27b-iq3xxs-mtp",
   "messages":[{"role":"user","content":"Reply with exactly: SWIFT_OK"}],
   "max_tokens":64,"temperature":0.0,"stream":False}
r=urllib.request.Request("http://192.168.0.250:8021/v1/chat/completions",
    data=json.dumps(p).encode(),headers={"Content-Type":"application/json"})
t=time.time(); b=json.loads(urllib.request.urlopen(r,timeout=120).read().decode())
print("wall %.2fs"%(time.time()-t)); print(json.dumps(b,ensure_ascii=False,indent=2))
PY
```

### 6.3 长提示探针（必须放大 `max_tokens`，并检查 `finish_reason` / `content` 非空）

本轮使用的探针脚本在会话临时区（不提交进仓库，因为它是评测工具而非产品代码）。其判定逻辑：

```
if finish_reason == "length" or content == "":
    verdict = "BUDGET_EXHAUSTED — 本次输出不可用，不得读作「没发现问题」"
```

### 6.4 NOT_RUN 说明

`max_tokens=6000` 的重跑在本轮结束时尚未回收结果（本地 27B 模型 + 6.7k-token 提示，单次墙钟约数分钟）。**因此本文件不引用它的任何数字**，也不基于它下任何结论。需要它的读者请按 §6.3 重跑。

---

## 7. 给属主 / dsh-ops 的条目

| # | 条目 | 建议 |
|---|---|---|
| 1 | Swift 的 `max_tokens` 运维约定 | 至少 6000 级；调用方必须检查 `finish_reason` 与 `content` 非空；**空回答不得读作「无问题」** |
| 2 | B / C / E 三项的准入 | 需要 Owner 明确「外部模型写测试 / 改源码 / 起超时进程」是否在 LOCAL_ONLY 边界内；本仓目前没有可核验的准入记录 |
| 3 | prompt 去向审计 | 建议在 run manifest 里加 `model_endpoint` / `prompt_digest` 字段，使「上下文数据边界」可事后审计 |
| 4 | 推理预算的成本 | 一次 6.7k-token 提示的推理约 60 s / 1400 token（≈22 tok/s 有效吞吐）；批量复核前先估算预算 |
| 5 | `250:8011` | 保持 `GMP_RESERVED`。本轮只做过一次只读 `/v1/models` 探活，未做其它调用 |

---

*本文件的每一个数字都来自 2026-11-23 对 `192.168.0.250:8021` 的真实请求。模型自述与 TPS 均未用于任何能力判定。*

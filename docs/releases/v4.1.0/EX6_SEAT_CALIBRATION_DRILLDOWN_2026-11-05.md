# EX-6：flash-next-252 标定从 4/4 掉到 3/4，掉的是哪道题、为什么（2026-11-05）

## 0. 结论先行

| 问 | 答 |
|---|---|
| 掉的是哪道题？ | **`code-truth-len`**（第 4 题） |
| 根因？ | **超时（主要假设，未被直接证实）** —— 该题当时返回的是**空串**；归档记录里**没有**那一次调用是否超时的字段 |
| 今天复现得出来吗？ | **复现不出能力缺陷**：同一席位同一题，现在答 `"5"`，全量标定 **4/4** |
| 那它能不能参与裁定？ | **能，但有一个前提**：超时必须与「答错」**分别记账**（见 §5） |

## 1. 掉的那道题（逐字取自两次归档 run）

两次 run 都是 `flash-next-252` 同一席位、同一 endpoint、同一 model，
四道题完全相同，只有第 4 题的结果不同。

`audit/v4.1.0/runs/20261104-seats-001/logs/001-seats-health.stdout` 第 2 行：

```json
"calibration": {"score": 4, "total": 4, "ratio": 1.0, "items": [
  {"id": "arith-17x23", "ok": true, "answer": "391"},
  {"id": "json-contract", "ok": true, "answer": "{\"verdict\":\"INCOMPLETE\",...}"},
  {"id": "gate-trust-two-state", "ok": true, "answer": "{   \"verdict\": \"NOT_TRUSTWORTHY\" }"},
  {"id": "code-truth-len", "ok": true, "answer": "5"}]}
```

`audit/v4.1.0/runs/20261104-seats-002/logs/001-seats-health.stdout` 第 2 行：

```json
"calibration": {"score": 3, "total": 4, "ratio": 0.75, "items": [
  {"id": "arith-17x23", "ok": true, "answer": "391"},
  {"id": "json-contract", "ok": true, "answer": "{\"verdict\":\"INCOMPLETE\",...}"},
  {"id": "gate-trust-two-state", "ok": true, "answer": "{   \"verdict\": \"NOT_TRUSTWORTHY\" }"},
  {"id": "code-truth-len", "ok": false, "answer": ""}]}
```

前 3 题两次**逐字相同**，只有第 4 题：`ok` 由 true 变 false，答案由 `"5"` 变空串。

该题的定义在 `scripts/gate/expert_ladder.py:276-280`：

```python
{
    "id": "code-truth-len",
    "prompt": "What number does this print? print(len([x for x in range(10) if x%2])) Reply with only the number.",
    "expect": lambda t: "5" in t and not any(x in t for x in ("4", "6", "10")),
},
```

期望答案 `5`（0..9 里的偶数个数）。判据是**子串包含**，不是精确相等 ——
所以只有「答成别的数」或「答成空」才会判红。空串必然判红。

## 1.5 口径更正（本轮由第二血统裁定触发，2026-11-05）

**本文原先把「根因是超时」写成了已证事实。那是过头了。**

第二血统（gpt 家族，`chatgpt-web`）在拿到两份归档 run 的原始记录后裁定：

> "A在80项可比范围内成立；B仅证实空串，**未证实超时**；C缺少完整证据，不能据此推翻#5466。"
> —— verdict `INCOMPLETE`，confidence 0.99

拆分来看：

| 主张 | 第二血统的判断 | 依据 |
|---|---|---|
| A：归档证据没漂移 | **成立** | 80/80 可比文件逐字节一致（争议个案 `agg_group_001` 也逐字节一致） |
| B：根因是超时 | **只证实了「该题返回空串」，未证实「它超时了」** | seats-002 的归档记录**没有**该次调用是否超时的字段 |
| C：#5466 核心论断不成立 | 该次裁定的材料**不足以**支撑 | 给它的材料是摘要（80/80 的结论），不是逐文件对照 |

⇒ 本文的口径改为：

- **直接证实的**：该题在 seats-002 返回**空串**（seats-001 返回 `"5"`）；前 3 题两次逐字相同；
  该席位自身延迟由 15.0 s 涨到 **101.14 s**；同批另一个席位 **120.01 s 超时**。
- **未被直接证实的**：那一题的空串**是不是**超时造成的。归档里没有这一条。
  「当时主机负载高」是**旁证**（延迟暴涨 + 同批另一席位超时），
  但「这题超时」是从旁证**推断**出来的，不是记录里写的。
- **因此**：「根因是超时」是**主要假设**，不是已证事实；确定的部分是
  **「它不是能力问题」**（同一题今天答对、全量标定 4/4）**，
  以及**「当时的账把两种失效混成了一个形状」**（这一点是代码事实，已由
  `expert_ladder.run_calibration` 的改动 + 反向变异固化）。

第二血统同时确认了那个**记账缺陷**本身是实的：超时与答错被压成同一个
`{ok:false, answer:""}`，台账读者分不出「席位不可信」和「席位当时正忙」。
这与「那一题**具体**是不是超时」是两件事 —— 前者是代码事实，后者是历史记录的空白。

§3 的现场复现补上了**今天**的证据（两遍：第一遍前两题卡在 120.01 s / 120.00 s 超时，
第二遍 4/4），它证明的是「这个席位会超时」这件事**真实存在**，
但依然不能倒推 2026-11-04 那一题就是超时。

裁定记录见归档 run `20261105-adjudicate-001` 的 `adjudicate-5466` 命令日志
（`--no-evidence-dir`，stdout 即产物）；问题与机器证据在
`audit/v4.1.0/adjudications/` 下，可复现。

## 2. 根因：超时（主要假设，未直接证实），不是能力缺陷

判据链在 `scripts/gate/expert_ladder.py:288-305`（`run_calibration`）：

```python
rec = ask(seat, item["prompt"], max_tokens=64, **kw)
text = rec.get("content") or ""
try:
    ok = bool(item["expect"](text))
except Exception:
    ok = False
if not rec.get("ok"):
    ok = False
```

⇒ **超时（`rec.ok=False`）与答错（`expect` 不满足）在账上被压成同一个形状**：
`{"id": ..., "ok": false, "answer": ""}`。台账里看不出「答错了」和「没答上来」的区别。

而 `http_chat`（`scripts/gate/expert_ladder.py:213-231`）在超时/空内容时
把 `content` 留空、`ok` 置 false，于是 `answer` 就是空串 —— 与 seats-002 记录的完全一致。

**同一份 seats-002 里就有两条独立佐证指向「当时主机很忙」：**

| 席位 | seats-001（14:49） | seats-002（15:55） |
|---|---|---|
| flash-next-252 延迟 | **15.0 s** | **101.14 s**（6.7 倍） |
| flash-next-252 tps | 1.8 | 0.29 |
| local-qwen35-9b | ok，28.68 s | **TimeoutError: timed out**（120.01 s） |
| gmp35b-250 | finish_reason=length | finish_reason=length（两次都思考预算耗尽） |

seats-002 里 flash 的延迟涨到 101 s，而同一批次另一个本地席位直接 120 s 超时。
在这个负载下，flash 的第 4 题没能及时返回 ⇒ 空串 ⇒ 该题判红 ⇒ 3/4。
⇒ **这是「慢到超时」，不是「答不对」。**

## 3. 现场复现：今天这道题是好的（2026-11-05 实测）

用**仓内自己的代码路径**（`expert_ladder.load_seats()` → `ask()` → `http_chat()` → `run_calibration()`，
与两次归档 run 同一套代码）现场重测该席位：

```
endpoint: http://192.168.0.252:8001/v1/chat/completions
model:    qwen3.8-flash-next-iq3_s
timeout_s: 120   single_session: true
```

**第一遍（逐题追踪，冷启动）**：

| 题 | item_ok | call_ok | answer | 延迟 | error |
|---|---|---|---|---|---|
| arith-17x23 | false | false | `""` | 120.01 s | TimeoutError: timed out |
| json-contract | false | false | `""` | 120.00 s | TimeoutError: timed out |
| gate-trust-two-state | true | true | `{"verdict": "NOT_TRUSTWORTHY"}` | 68.93 s | — |
| **code-truth-len** | **true** | **true** | **`"5"`** | **10.76 s** | — |

**第二遍（`run_calibration`，与归档 run 完全同口径）**：

```json
{"kind": "calibration", "seat": "flash-next-252", "score": 4, "total": 4, "ratio": 1.0,
 "items": [{"id": "arith-17x23", "ok": true, "answer": "391"},
           {"id": "json-contract", "ok": true, "answer": "{\"verdict\":\"INCOMPLETE\",\"confidence\":0,\"reason\":\"probe\"}"},
           {"id": "gate-trust-two-state", "ok": true, "answer": "{   \"verdict\": \"NOT_TRUSTWORTHY\" }"},
           {"id": "code-truth-len", "ok": true, "answer": "5"}]}
```

⇒ **掉的那道题今天答对了，标定回到 4/4**（两遍共 8 次调用，耗时 508 s）。
⇒ 3/4 **不是该席位的能力边界**，是那一次的超时。

第一遍前两题整整卡在 120.01 s / 120.00 s（正好是席位自己的 `timeout_s: 120`），
第二遍同样的题立刻就答对了 —— 失败形态是**超时**，不是推理错误。
第二遍能过也说明：**冷启动/排队**会吃掉这个单会话席位的预算。

## 4. 结论：它能不能参与裁定

**能，但要把前提钉死。**

理由：
1. 四道标定题（含**需要真读代码语义**的 `code-truth-len`）它现在全对，
   2026-11-05 实测两遍 4/4；
2. seats-002 的 3/4 已被定位为超时，不是答错 —— 若把它当成「能力不足」而弃用该席位，
   是因为一次负载尖峰冤枉了一个能答对的席位；
3. 交接提示词 §6 要求「任何双血统复核必须来自两个互不可见的血统，单系估算要标注」。
   丢掉这个席位，双血统就只剩一条血统，IL-2 的 47 条 P0 复核会退化成单系估算。

**前提（必须先做，否则它的失败会被误读成「它答错了」）：**

- 标定与复核记录里，**超时必须与答错分开记账**。
  现在 `run_calibration` 把两者压成同一个 `{"ok": false, "answer": ""}`，
  台账读者无法区分「这席位不可信」和「这席位当时正忙」。
  这正是 §3 里 IL-3 那条 P0 的同一种误判机制（把「环境不具备」当成「能力不足」）。
- 席位的 `timeout_s`（120 s）相对实测延迟（15 s → 101 s）余量太小。
  在负载尖峰下，超时会被记成答错。**要么把超时显式记为第三态（NOT RUN），
  要么把 timeout 提到明显高于 p99 延迟。**

## 5. 建议的两处改动（与 IL-3 / IL-10 同族）

| # | 改动 | 落点 |
|---|---|---|
| 1 | `run_calibration` 的 item 记录增加 `error` / `timeout` 标记，把「超时」与「答错」分列 | `scripts/gate/expert_ladder.py:288-305` |
| 2 | 席位 `timeout_s` 按实测 p99 延迟重定（当前 120 s，实测曾达 101 s） | `docs/governance/EXPERT_SEATS.yaml` |

改动 1 属于**修改既有门禁脚本**（🟡），按交接提示词 §4 须附
「改前漏什么 / 改后多报什么」+ 反向变异证据：
改前漏的是「超时被压成答错」这一区分，改后多报的是**第三态**，历史判据**不放松**。

## 6. 残余风险

- 本轮只重测了 `flash-next-252` 一个席位；`local-qwen35-9b`（本次超时）
  与 `gmp35b-250`（两次都思考预算耗尽）**没有**重测，
  它们各自的边界不在本条范围内。
- 两遍重测共 8 次调用耗时 508 s，平均单次 63.5 s ——
  这个席位**很慢**。在双血统复核里跑两个这种席位，耗时与超时风险都要计入。
- 本条**不构成任何门禁通过声明**：本仓未跑 `ci_gate.sh`。
- 席位能力边界的复核仍受 IL-9（CI 执行面）影响：归档 run 侧的第二血统复核没有入口。

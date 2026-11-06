# IL-2：把 47 条 P0 的双血统复核补上（2026-11-05）

## 0. 结论先行

IL-2 的验收口径是：

> ledger 输出 `graded_crosschecked >= 47` 且这 47 行 `triage_families` 含 >= 2 family；
> `credited_lineages < 2` 时仍一律 `advisory-only`

本轮的做法：**用 gpt 血统把缺的 P0 批量复核一遍**，让每条被两个不同家族分诊过。

| | 接手时 | 现在 |
|---|---|---|
| 台账行数 | 110 | 110 |
| `cross-checked` | **16** | **59** |
| `advisory-only` | 94 | **51** |
| 有 >= 2 个 family 的行 | 16 | **59** |

两条独立血统的来源（同一批 issue、同一份问题、两个不同模型家族）：

| 血统 | 席位 | 家族 | 产物 |
|---|---|---|---|
| qwen | `flash-next-252`（`qwen3.8-flash-next-iq3_s`） | `qwen-strata` | `ISSUE_TRIAGE_2026-11-05.jsonl`（主批）+ `ISSUE_TRIAGE_2026-11-05B.jsonl` |
| gpt | `chatgpt-web`（`web-session-chat`） | `gpt` | `ISSUE_TRIAGE_CROSSCHECK_2026-11-05C.jsonl`（本轮批量）+ `…2026-11-05.jsonl` / `…B.jsonl`（原有零散复核） |

## 1. 为什么「补一批 gpt」是这件事的正解

台账升级规则在 `scripts/gate/issue_ledger.py`：

```python
grade = "cross-checked" if (ladder_ready and len(famset) >= 2) else "advisory-only"
```

- `ladder_ready` = `credited_lineages` 里有 >= 2 个家族 —— 席位**能力面**够不够
- `famset` >= 2 —— 这一条 issue 有没有被**>= 2 个不同家族分别分诊过**

接手时 `ladder_ready` 早就是 `True`（`credited_lineages = [gpt, qwen]`），
但 `famset >= 2` 只有 16 条 ⇒ 卡在第二项：**gpt 血统只零散复核过 12 条**。

所以缺的正是「让 gpt 血统把剩下的 P0 跑一遍」。

### 这一步用的是**已存在的批量通道**，不是新造

`scripts/gate/expert_triage.py` 本来就有 `--seat` / `--only` / `--batch` / `--resume`。
本轮只是用 `--seat chatgpt-web --batch 5` 跑了一批 35 条 P0 ——
**没有新增任何工具**，也没有改分诊逻辑。

## 2. 顺带修掉的一个可复现性缺口

台账要记输入指纹（`input_hashes`），而它的 `--issues` 输入是 Gitea 的
`all.json` —— 一个**临时文件**。于是「重生成后指纹一致」这件事**无法复现**：
旧台账记的 issues 指纹对不上仓内任何产物。

修法（两处）：
1. `scripts/gate/issue_ledger.py` 的 `load_json` 现在**同时接受 JSON 数组与 JSONL**
   （仓内禁 `*.json`，交接提示词 §4；而输入必须在仓内，否则指纹无从复现）。
2. 新提交 `docs/releases/v4.1.0/ISSUE_SNAPSHOT_2026-11-05.jsonl`
   （110 条 issue 的标签/标题/创建时间/评论数快照）。

⇒ 台账重生成现在**完全可从仓内复现**。

顺带记录一次**实测到的漂移**：live 标签下 P0 从 49 涨到 **57**
（LB-1 那轮标签补齐之后）。所以本轮按**live P0** 而不是旧台账的 P0 来选复核对象，
多出 8 条是新变成 P0 的。

## 3. 三条判据的实测

### 3.1 主判据：`graded_crosschecked >= 47` 且这些行 `triage_families >= 2`

见 §0 的表。判定脚本：

```python
cc = [r for r in rows if r["evidence_grade"] == "cross-checked"]
bad = [r["number"] for r in cc if len(r["triage_families"]) < 2]
assert len(cc) >= 47
assert not bad
```

`bad` 为空 ⇒ **每一条**升级为 `cross-checked` 的行都**真的**有两个不同 family。
（`issue_ledger.py` 的 `grade` 判定本身就带 `len(famset) >= 2`，
但台账里同时还有 `triage_families` 字段可供独立核对 —— 两处一致才算数。）

### 3.2 单血统条款：`credited_lineages < 2` ⇒ 一律 `advisory-only`

实测（只给 `--credited-lineages qwen`，但分诊数据里**确实**有 2 个 family 的行）：

```
{"grades": {"advisory-only": 110}, "rows_with_2_families": 10}
```

⇒ 即使行级有 2 个 family，只要**席位能力面**声明的是单血统，
台账就一律 `advisory-only`。这正是「单系估算要标注」（交接提示词 §6）那一条。

### 3.3 校验器：`verify_ledger.py` 按 triage 文件**重算**分级

`audit/v4.1.0/verify_ledger.py` 的 `DEFAULT_TRIAGE` 已加入本轮的新产物
（`ISSUE_TRIAGE_CROSSCHECK_2026-11-05C.jsonl`）。它不信任台账里的分级，
而是**从 triage 文件重算**再比对 —— 台账与夹具/分诊对不上就红。

实测：`verify_ledger[B]: OK`。

## 4. 本轮没做 / 不能做的事

- **没有改任何 issue 的状态、标签或优先级**。台账是**台账**，不是 issue；
  issue 的收口是 Owner 的动作（见 IL-3 报告里的裁定清单）。
- **`triage_families >= 2` 是「两个家族都分诊过」，不是「两个家族都同意」**。
  两个家族可能给出不同判断 —— 那属于**分歧**，需要人裁定，
  不是「交叉验证通过」。本轮只把**证据面**补齐到两血统，**没有**假装分歧已解决。
  原本就有 2 family 的那 16 条里，若两家族结论不一致，本轮同样没动它。
- **第二血统复核的输入仍是「同一份 issue 事实」**。若两家族拿到的是同一份**有偏**的
  事实材料，它们的交叉验证价值有限 —— 这条限制写在整改方案里，不是本轮引入的。
- **本条不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
  归档 run 侧的第二血统复核仍被 IL-9（CI 执行面，runner-03 缺镜像）挡住。

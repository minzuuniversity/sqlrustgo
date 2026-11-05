# P1-4：每道门禁写 `effect`，并核对「声明」与「夹具」一致（2026-11-05）

## 0. 结论先行

之前的状态：168 道门禁**没有一处**声明自己「能抓什么、抓不到什么」。
所以「声称」与「夹具」之间**没有任何可比对的东西** ——
`NEG_FIXTURES_v4.1.0.jsonl` 记了 57 行夹具账，
但没人说「这道门禁抓的是哪几类」，也就没人能发现
「它声称抓 X，夹具其实只碰了 Y」。

本轮：给 168 道门禁**每一道**补上 `effect` 声明，并建了核对器。

| | 值 |
|---|---|
| 有 `effect` 声明的门禁 | **168 / 168** |
| 其中 `catches` 非空（敢声称能抓东西） | **12** |
| `catches` 为空（**诚实地**说「还不能声称」） | **156** |
| `honesty.py check` | **OK**（0 FAIL） |

关键设计：**「我现在还不能声称能抓任何东西」本身也是一种诚实的声明**。
156 道门禁的 `catches` 是空的 —— 因为它们确实没有可证伪夹具
（NEG_FIXTURES 里 44 行 `pending-fixture`、1 行 `unfalsifiable`）。
把它们的 `catches` 写成空的，比编一个像模像样的性质列表**诚实得多**。

## 1. `effect` 的形状

```json
"effect": {
  "catches":     ["registry-drift"],
  "cannot_catch": [],
  "reason":      "由夹具 fx02-anti-ignore-registry-drift 证明（kind=mutation-verified）"
}
```

- `catches` —— 这道门禁**已被证明**能抓的性质（每项必须落在夹具证据上）
- `cannot_catch` —— 明确抓不到的
- `reason` —— 为什么是这个形状（尤其是 `catches` 为空时，为什么还不能声称）

`effect` 是**人工撰写**的声明，登记在 `docs/releases/v4.1.0/GATE_CATALOG.jsonl`。
`scripts/gate/gate_registry.py` 的 `AUTHORED` 已加上 `effect` ——
**重生成登记表时不能把人工声明冲掉**（原来只保留 `disposition` / `note` / `evidence`）。

## 2. 四条判据

`audit/v4.1.0/honesty.py check`（探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动）：

| 判据 | 内容 | 对应验收口径 |
|---|---|---|
| **C1** | 每个门禁都必须有 `effect` 声明 | 缺声明 = 不可核对 = 红 |
| **C2** | `catches` 非空 ⇒ 必须有一行**可证伪**夹具（kind ∈ {mutation-verified, selftest-verified}） | **「声称能抓 X 但没有 X 的夹具 ⇒ 红」** |
| **C3** | `catches` 里每个性质 id 必须能在该夹具的 `fixture_id` / `note` 里**逐字找到**（归一化后子串匹配） | 声称的性质没落在任何夹具证据上 ⇒ 红 |
| **C4** | `kind == unfalsifiable` 的门禁 ⇒ `catches` 必须为空（除非写了 reason） | 不可证伪的门禁不许声称能抓任何东西 |

外加 **C5**（反向）：夹具账里有可证伪夹具、但门禁却没声明 `catches` ⇒ 红
（夹具白做了，判据白弱了）。

归一化：转小写，非字母数字折成 `-` ⇒ 「registry drift」与「registry-drift」是同一个 token。

### 12 道敢声称的门禁

| 门禁 | catches | 夹具 |
|---|---|---|
| `check_anti_ignore_gate.sh` | registry-drift | fx02 |
| `check_cross_version_debt.sh` | doc-missing | fx05 |
| `check_docs_links.sh` | links-broken | fx07 |
| `check_drift_not_pass.sh` | new-antipattern | fx03 |
| `check_gate_executability.sh` | missing-script | fx01 |
| `check_int_debt.sh` | doc-missing | fx06 |
| `check_tests_dont_write_archived_evidence.sh` | write-archived-evidence | fx04 |
| `check_alpha_entry_v4.10.sh` | doc-missing | fx08 |
| `check_stage.sh` | version-uninferable | fx14 |
| `check_commit_message.sh` | 坏提交信息 | 自带 selftest |
| `check_coverage_single_implementation.sh` | scan_hits | 自带 selftest |
| `check_gate_test_integrity.sh` | 掩码检测器 | 自带 selftest |

每个 id 都逐字落在对应夹具的 `fixture_id` 或 `note` 里 —— C3 会核对这一点。

## 3. 反向变异

| 臂 | 做法 | 期望 | 实测 |
|---|---|---|---|
| M1 | 给一个只有 `pending-fixture` 的门禁（`check_arch_sem_debt.sh`）声称能抓 `fake-property` | 判红 | rc=1，`C2 … 夹具 kind=pending-fixture —— 声称能抓 X 却没有 X 的夹具` |
| M2 | 把 `check_docs_links.sh` 的声称改成夹具证据里没有的性质 | 判红 | rc=1，`C3 … 声明没落在证据上` |
| M3 | 删掉一道门禁的 `effect` 声明 | 判红 | rc=1，`C1 … 缺 effect 声明 —— 不可核对` |
| 恢复 | 全部还原 | 判绿 | rc=0，`honesty: OK` |

M1 是验收口径的原话（「声称能抓 X 但没有 X 的夹具 ⇒ 红」），M3 保证「没有声明」不能蒙混过关。

## 4. 顺带发现：本会话我自己写坏的三处字符

全量扫了 317 个仓内文件（`docs/releases/v4.1.0/*.md`、`audit/v4.1.0/*.py`、
`scripts/gate/*.sh`、两个台账），找出 **U+FFFD 替换字符**：

| 文件 | 处数 | 状态 |
|---|---|---|
| `audit/v4.1.0/honesty.py` | 4 | 我本轮写的 ⇒ **已修**（两处中文字被截断） |
| `docs/releases/v4.1.0/EX6_SEAT_CALIBRATION_DRILLDOWN_2026-11-05.md` | 3 | 我写的 ⇒ **已修**（「超时风险」被截断） |
| `scripts/gate/check_audit_manifest.sh` | 3 | 我改的 ⇒ **已修**（注释里「同一个提交」被截断） |
| `scripts/gate/check_evidence_binding.sh` | 11 | **HEAD 上就有**（commit `cb42ee82b9`，不是我）⇒ 未动 |

那 11 处**全在注释里**，不影响该门禁的行为 —— 但它证明「写进仓的文件会被截断」这件事
在本仓是真实发生的。所以本条把它记下来，而不是默默修掉别人的提交。

## 5. 口径变化（如实记录）

| 指标 | 改动前 | 改动后 | 说明 |
|---|---|---|---|
| `check_n` | 168 | 168 | 分母未动（探针不进 STAGE_CONFIG，文件名不叫 `check_*`） |
| `claims_hits` | 1144 | 1144 | 未动 |
| `docs410` | 88 | **89** | 本报告占 1 份 |

## 6. 残余风险与未闭环部分

- **C3 是子串匹配**，不是语义判定。它能抓住「声称的性质根本没落在夹具上」，
  抓不住「落在夹具上但那条夹具其实证明得不够」——
  后者需要人读夹具，那是 P1-3 的 `mutation-verified` 判定要管的事。
- **`catches` 为空不等于门禁没用**：156 道门禁的 `catches` 为空，
  只说明**本轮没有可复现的证据**说它能抓什么，不说明它抓不到什么。
  这两件事在报告口径上必须分开说 —— 所以 `cannot_catch` 也是空的。
  真正的能力清单要等 44 条 `pending-fixture` 补齐。
- **`effect` 是人工声明**：它可以被写错。honesty.py 核对的是
  「声明与夹具账是否自洽」，**不是**「声明是否属实」。
  属实性靠夹具本身的反向变异证据（P1-3 已建）。
- **本条不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
  归档 run 侧的第二血统复核仍被 IL-9（CI 执行面，runner-03 缺镜像）挡住。
- 44 条 `pending-fixture` 补齐后，`catches` 会从 12 涨上去 ——
  那时 IL-11（生产存储门禁扩面）才有可消费的「哪些测试声称验证存储语义」清单。

# LB-1：issue 标签口径补齐，让分层估算对全部 open issue 可核对（2026-11-05）

## 1. 这条为什么要做

分层估算（按 priority × cost × cap 排人日）此前对 111 个 open issue 里的
43 处维度缺口不可核对：14 条无优先级、17 条无成本、12 条无能力域。
不可核对的那部分约 12–15%，正好是最容易漏的 CI 基础设施与门禁结构性缺陷类。

## 2. 判定依据（不是随手打标）

台账 `docs/releases/v4.1.0/ISSUE_LABEL_LEDGER_v4.1.0.jsonl` 是唯一依据，17 行，
每行必须有 `rationale`（为什么这个级别）、`basis`（依据什么判的）、`decided_by`。
核对器对「没有台账理由的缺维」直接判 `UNLEDGERED` 并非零退出——
它不允许在缺标签的当场凭空补一个标签而不留理由。

分级口径（沿用仓内既有标签语义，未新增标签）：

- `priority/p0`：阻断发布判定或制造假结论（假红、假绿、required gate 结构性不可通过、已提交证据被抹）
- `priority/p1`：判定质量或测试有效性受损，但不当场阻断当期判定
- `cost/t0`：改配置 / systemd 单元 / 登记，不需要改代码
- `cost/t1`：单点改动，0.5–2 人日
- `cost/t2`：跨模块或需要新机制，2–5 人日
- `cap/build / cap/test / cap/governance`：运行时与 CI 基础设施 / 测试有效性 / 门禁与发布判定本身

## 3. 门禁口径的复核结果

`python3 scripts/gate/gov_probe.py issues` 的 `issue_summary`（当期 live）：

```
open_total=111  pages=3
priority: {p0:57, p1:52, p2:2}                 合计 111
cost:     {t0:24, t1:64, t2:23}                合计 111
cap:      {analysis:3, architecture:1, benchmark:1, build:44, docs:2,
           governance:24, refactor:2, review:7, test:27}   合计 111
```

三个维度都不再出现 `(none)` 键。整改前/后的现场快照进了仓：
`audit/v4.1.0/neg/snapshots/labelgap-open-before-20261105.jsonl`（43 处缺维）、
`audit/v4.1.0/neg/snapshots/labelgap-open-after-20261105.jsonl`（0 处）。

## 4. 反向变异：证明核对器不是摆设

`python3 audit/v4.1.0/issue_labels_mutations.py` 七臂，实测 `arms=7 discriminating=7 not_discriminating=0 drift=0`：

| 臂 | 施加的变异 | 期望 | 实测 |
|---|---|---|---|
| m1 | `has()` 永远返回 True（缺标签全部豁免） | 整改前夹具必须变绿 | rc=0（未变异同一夹具 rc=1） |
| m5 | 去掉「台账必须有理由」校验 | 空理由台账必须被放过 | rc=0（未变异判 LEDGER-BAD，rc=1） |
| dim-priority | 单缺 priority 的合成夹具：只查 priority / 摘掉 priority | 只查该维必须红；摘掉该维必须绿 | rc=1 / rc=0 |
| dim-cost | 同上，cost 维 | rc=1 / rc=0 | rc=1 / rc=0 |
| dim-cap | 同上，cap 维 | rc=1 / rc=0 | rc=1 / rc=0 |
| m6 | 台账少登记一条（`rows[0]["assign"]` 去掉 `cost/t0`） | `verify` 的 `added_labels` 必须与台账算出的 `expected_ops` 不一致而判红 | rc=1（`added=43 expected=42`）；真台账对照 rc=0 |
| m7 | 去掉 `verify` 的「标签被删即判红」守卫（`if lost: return 1` → `pass`） | 有标签被删时必须判红；摘掉守卫后必须放行 | control rc=1；变异后 rc=0（夹具：after 状态比 before 少一个 `priority/p1`） |

外加两条基线：整改前夹具必须判红（判红条件仍在），整改后夹具必须判绿
（不是永远红的摆设）。两条基线任一失效，跑法直接返回 1。

### 4.1 臂数是数出来的，不是写死的

上一版本文件（commit `3ed6092802`）的实际执行臂是 5 条（m1、m5、dim×3），
但脚本末尾**写死**打印 `arms=6`；再上一版写死 `arms=7` 而只执行 6 条。
报告据此声称「七臂」，表格却只有 6 行 —— 数字是编的，不是测的。

现在臂数由实际执行的臂累加得出（`arms_run`），并加了一条自洽不变式：
`discriminating + not_discriminating + drift` 必须等于 `arms`，否则退出码 1。
反向变异实测（把计数改回写死）：

| 变异 | 打印 | 结论 |
|---|---|---|
| 写死 `arms=7` | `arms=7 discriminating=7 ...` | 与真值巧合相同 |
| 写死 `arms=6` | `arms=6 discriminating=7 ...` | 自相矛盾 |
| 写死 `arms=99` | `arms=99 discriminating=7 ...` | 明显造假 |

写死 `arms=6` 那一行就是判据的破绽：臂数与判别数对不上。
因此不变式放在**外部**核对（归档 run 的 `lb1-mutation-7arms` 命令同时核对
`arms=` 数值与逐臂 verdict 行数），脚本内部的不变式只是第二道防线 ——
同一个程序无法证明自己的打印没被写死。

m7 臂补的是一处真实覆盖缺口：此前 `verify` 的「标签被删即判红」守卫
**没有任何反向变异**，删掉它照样一路绿灯。为此 `verify` 增加了
`--after <state.jsonl>` 参数（默认仍是 `live`），使「标签被删」这条守卫
可以离线夹具化，不必去动线上 issue 的标签。

## 5. 「只加不删」的机器核对

`apply` 只调用 `POST /issues/{n}/labels`（该端点只追加）。逐条 diff 前后标签：
111 个 open issue，`lost_labels=0`，`added_labels=43`，与 `plan` 的 43 条 ops 逐一对应。

口径已经固化成子命令，不再靠人眼比对：

```
python3 audit/v4.1.0/issue_labels.py verify \
    audit/v4.1.0/neg/snapshots/labelgap-open-before-20261105.jsonl
```

它把「整改前快照」与 live 逐条 diff，返回 0 需要同时满足两件事：
没有任何标签被删除（`lost_labels==0`），且新增数正好等于台账对这份快照算出的 ops 数
（`added_labels == expected_ops_from_ledger`）。前者挡「顺手删标签」，后者挡「有写入没落地」
与「有人加了台账外的标签」。实测输出：

```
{"kind": "label_apply_verify", "issues_checked": 111,
 "lost_labels": 0, "added_labels": 43, "expected_ops_from_ledger": 43}
```

## 6. 过程偏差（如实记录）

第一版 `apply` 把成功写成了失败：Gitea 的 `POST /issues/{n}/labels` 直接返回该 issue 的**标签数组**，
我却按 `{"labels": [...]}` 解析，于是 17 条全部误报 `failed`。
写入本身是成功的（live 复查三维 gap 已归 0）。判据已修，并幂等重跑确认 `added: 0 failed: 0`。
这条按 §2.3 的口径记在案：判据错报 ≠ 事实如此，工具报失败时必须回读 live 再定性。

另一处是我自己的核对脚本用错了 API base（把仓库级路径拼到已含仓库路径的 helper 上），
111 条全 404，一度显示成「标签全丢」。改用仓内 `issue_labels.py` 的 `req` 后重测，结论才是
`lost_labels_total=0`。教训：跨工具复用 helper 时，base 语义必须核对，不能假设。

第三条（本次接手时发现）：**臂数是写死的**。`issue_labels_mutations.py` 末尾
`print(f"arms=7 ...")` 里的 7 是字面量，而实际执行的臂只有 6 条；
上一版同样写死 6 而只执行 5 条。报告与看板 evidence 据此声称的臂数因此是编的。
这是本项目 §10 已记过的「生成器硬编码结论」同一类缺陷，这次出现在变异跑法自己身上。
已改为按实际执行累加，并补一条外部核对（归档 run 同时核对 `arms=` 数值与逐臂 verdict 行数）。
详见 §4.1。

## 7. 残余风险

- 分级由 mcode-macmini 单人判定，`basis` 已写明依据（标题口径 + 当期 census 标签 + 归档证据），
  未逐条重写正文。owner 可按台账逐条改标签；工具只加不删，不会掩盖既有事实。
- 台账只覆盖当期 open issue。新开的 issue 若缺维，核对器会在下一次 census 里报 `MISSING`/`UNLEDGERED`，
  但这条核对器目前没有 CI 执行点（被 IL-9 挡），需要 #5597 之后挂进 CI 才算长期闭环。
- 本文件不构成门禁通过声明：本仓未跑 `ci_gate.sh`。

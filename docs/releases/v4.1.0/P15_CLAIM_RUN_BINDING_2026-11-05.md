# P1-5：把文档里的「通过」声明绑回可复现的 run（2026-11-05）

## 1. 这条要解决什么

整改方案 §1.2 / §1.4 与看板 Q3 指向同一个机制：
`docs/releases/v4.1.0/*.md` 里成千上万处「通过」记号，绝大多数**指不回任何已归档 run**。
这就是「报告好看、一用就崩」的直接机制 —— 文档写通过，背后没有可复现的执行记录。

看板 P1-5 的原始证据是 `claims_hits=1123 claims_ref=0`。
`claims_ref=0` 就是问题本身：**一处绑定都没有**。

## 2. 分级口径

`audit/v4.1.0/evidence_inventory.py`（探针/夹具，**不是门禁**，不进 STAGE_CONFIG，
分母 168 不动）按 **markdown 小节**（ATX 标题分块）给每处声明记号分级：

| 级别 | 判定 | 含义 |
|---|---|---|
| bound | 同小节内出现已归档 run id、`audit/v4.1.0/<run 目录>`，或 `docs/releases/v4.1.0/evidence/*.md` | 能追到执行记录 |
| unbound | 同小节内没有上述任何出处 | 「文档声称」 |

两个口径必须与既有工具一致，否则两边的数对不上、两边都不可信：

- 记号口径与 `scripts/gate/gov_probe.py` 的 `claim_re` 逐字一致 ——
  本工具数出的 `claims_hits` 与 `gov_probe` 的 `claims_hits` 实测**都是 1144**。
- run id 命名口径与 `scripts/gate/check_audit_manifest.sh` 的 `RUN_RE` 一致
  （`iaia410-*` 或 `YYYYMMDD-*`）。

两处收紧，都是实测踩出来的：

1. `audit/v4.1.0/` 后若允许任意 token，`audit/v4.1.0/evidence_inventory.py`
   这种**工具路径**会被误当成出处、凭空把声明算成 bound。
   现在要求其后必须是真正的 run 目录名。
2. run id 后缀是 `-<slug>-<NNN>`（**三位**流水号，如 `20261105-lb1-006`）。
   早先按四位写会把这类真 run 引用**漏掉**，bound 被系统性少算 21 处。
   这个 bug 是被反向变异的 M2 臂抓出来的（见 §5），不是靠读代码看出来的。

## 3. 实测结果：绑定率 8.13%

```
$ python3 audit/v4.1.0/evidence_inventory.py report
{"kind": "claims_inventory_report", "files": 59, "claims": 1144, "bound": 93,
 "unbound": 1051, "bound_pct": 8.13}
```

| 度量 | 值 |
|---|---|
| 带声明的文档 | 59 份 |
| 声明记号总数 | **1144** |
| 有 run/evidence 出处（bound） | **93** |
| 无出处（unbound） | **1051** |
| 绑定率 | **8.13 %** |

`claims_ref` 从 **0 变成 93** —— 看板 P1-5 记的那个 0，现在有了具体数字。

未绑定最集中的文档：

| 未绑定 | 总数 | 文档 |
|---|---|---|
| 186 | 205 | FEATURE_CHECKLIST.md |
| 71 | 71 | GA_GATE_REPORT.md |
| 68 | 68 | PR-5457_ACCEPTANCE.md |
| 58 | 59 | PR-5379_ACCEPTANCE.md |
| 55 | 55 | ALIGNMENT_AUDIT_2026-09-30.md |
| 53 | 53 | PR-5379_TEST_REVIEW.md |
| 53 | 53 | RELEASE_GATE_CHECKLIST.md |
| 46 | 46 | PERFORMANCE_TARGETS.md |
| 41 | 41 | EVIDENCE_STATUS.md |
| 41 | 41 | PR-5457_TEST_REVIEW.md |
| 38 | 38 | PR-5537_ACCEPTANCE.md |

`GA_GATE_REPORT.md`（71/71 全未绑定）和 `RELEASE_GATE_CHECKLIST.md`（53/53 全未绑定）
最刺眼：名字里带 gate report、gate checklist 的文档，恰恰一处出处都没绑。

绑定台账落在 `docs/releases/v4.1.0/CLAIMS_BINDINGS.jsonl`
（21 个小节、93 处声明，每条带 `run_refs` / `evidence_refs` / `excerpt`）；
基线落在 `docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl`（59 行，按文件记账）。

两个台账都放 `docs/releases/v4.1.0/` 而不是 `audit/v4.1.0/`：
后者的一级子目录会被 `gov_probe` 计进 `audit_runs` 指标，多一个目录等于污染别人对账的分母；
而 run 目录命名正则会要求 manifest.yaml，放那儿等于自找红。

## 4. 判据：只卡增量，不清算历史

```
$ python3 audit/v4.1.0/evidence_inventory.py check
{"kind": "claims_inventory_check", "live_files": 59, "ledger_files": 59,
 "live_claims": 1144, "live_bound": 93, "live_unbound": 1051,
 "ledger_unbound": 1051, "fails": 0}
claims_inventory: OK
```

三条规则，都能被机器判：

1. 新文档带任何未绑定声明 ⇒ 红（`NEW-FILE … 带 N 处未绑定声明`）
2. 已有文档的未绑定数上升 ⇒ 红（`未绑定声明 A -> B（新增 K 处）`）
3. 未绑定数下降或不变 ⇒ 永远不红

**为什么不追溯那 1051 处历史存量**：它们是既成事实，逐份补出处要重跑当年的执行，
很多已不可复现（代码已改、CI 执行面仍被 IL-9 挡住）。
把历史文档批量改写成「看着有出处」的样子，等于用文档骗文档 —— 那恰恰是本条要防的事。
所以本判据的用途是**止血**：从此刻起，新写的无出处声明进不来。
存量怎么清算，是 Owner 的决定（§7）。

## 5. 反向变异：判据可被证伪

四臂，全部按期望行为（完整输出见归档 run 的 `p15-mutation-*` 日志）：

| 臂 | 做法 | 期望 | 实测 |
|---|---|---|---|
| M1 | 新建文档放 1 处声明记号、同小节无出处 | 红 | rc=1，`NEW-FILE … 带 1 处未绑定声明`，未绑定 1051→1052 |
| M2 | 新建文档放 1 处声明记号、同小节引用 run `20261105-lb1-006` | 绿 | rc=0，`live_bound` 93→**94** |
| M3 | 已有文档（EVIDENCE_STATUS.md）追加 1 处无出处声明 | 红 | rc=1，`未绑定声明 41 -> 42（新增 1 处）` |
| M4 | 撤销 M3，回到基线 | 绿 | rc=0，`fails: 0` |

M1 与 M2 这一对是关键：**同样一处新声明，无出处进不来、有出处放行**。
判据判的是「有没有出处」，不是「有没有写通过」——
它不会把文档里的通过表述一律禁掉，只要求写明是谁跑的。

M2 同时证明 bound 判定**不是恒假**：新加的小节确实被认成 bound
（`live_bound` 从 93 涨到 94）。若 bound 判定恒假，M2 也会红。

**M2 第一次跑是红的**，这才是它最有价值的地方：它把 §2 第 2 条那个
run id 后缀位数的 bug 抓了出来（bound 少算 21 处）。判据不是摆设，是真的在判。

## 6. 口径变化（如实记录）

| 指标 | 改动前 | 改动后 | 说明 |
|---|---|---|---|
| `check_n` | 168 | 168 | 分母未动（硬约束） |
| `claims_hits` | 1144 | 1144 | 未动（口径与 gov_probe 一致） |
| `claims_ref` | 0 | **93** | 本条整改的产出 |
| `docs410` | 79 | **81** | 本条报告 + IL-6 报告各占 1 份；两份自身声明记号均为 0 |
| `audit_runs` | — | — | 台账落 docs/，未新增 `audit/v4.1.0/` 子目录 |

本工具放在 `audit/v4.1.0/` 下、文件名**不叫** `check_*`，因此不进入
`gov_probe.check_scripts()` 的 168 道门禁全集，分母不动。

## 7. 残余风险与需要 Owner 决定的部分

- **1051 处存量未绑定声明没有被消解**，只是被记账并冻结。逐份补 run 需要重跑当年的执行，
  很多已不可复现（对应 CI 执行面仍被 IL-9 挡住）。需 Owner 定：逐份补，
  还是给历史文档统一加「文档声称」标注后从口径里剔除。
- **bound 是「同小节提到 run」的弱绑定**，不是「该 run 真的证明了该结论」的强绑定。
  强绑定需要 run 里的 finding 与文档结论一一对应，本仓目前没有这个粒度的数据。
  本判据能防「完全没出处」，防不住「贴了个不相干的 run 号」。
- 本条**不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
- 归档 run 侧的第二血统复核仍被 IL-9（CI 执行面，runner-03 缺镜像）挡住。

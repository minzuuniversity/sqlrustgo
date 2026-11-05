# IL-3：疑似重复对 #5466 → #5389 的事实认定（2026-11-05）

> 本文件**不裁定**。合并/关闭 issue 属 ISSUE_CLOSING_VERIFICATION + PR 关联，
> 按 AGENTS 口径不在 Agent 自行处置范围。本文件给 Owner 备料。

## 0. 结论先行

**这两条不是重复。** 台账里 `dup_of: 5389` 这个标记是分诊机器（单血统
`flash-next-252`）写的，与两侧 issue 的实际内容不符。

它们是**因果相连但缺陷不同**的两件事：

| | #5389 | #5466 |
|---|---|---|
| 主张 | 跑一次测试会改写仓库里 81 个证据文件 | 已入库证据内部自相矛盾、无法从 clean checkout 复现 |
| 缺陷位置 | 测试的产物落盘位置 | 入库证据的内容本身 |
| 修好对方能修好自己吗 | **不能** | **不能** |
| 当前树内状态 | **已修**（默认落 `target/a04-5226/`） | **主张不成立**（见 §4） |

而且顺着查下去，找到了这两条的**共同源头**：一个至今仍活着的
「引擎不可用被当成可比结果」的缺陷（§5）。它不是重复问题，是**同一个根因的两个投影**。

## 1. 两侧事实（实时取自 Gitea API，2026-11-05）

取数方式：`/tmp/srg-api/gitea.py` 的 `req()`（PAT 只经 `git credential fill`，不进 argv、不落盘）。

### #5389

| 字段 | 值 |
|---|---|
| 标题 | `[Test] a04_mysql_diff_matrix_test 跑一次就改写仓库里 81 个证据文件，污染工作区` |
| 状态 | open；`state/ready` |
| 标签 | `cap/test`、`cost/t2`、`priority/p1`、`state/ready` |
| 创建 / 更新 | 2026-10-19T21:02:15Z / 2026-11-04T21:35:10Z |
| 评论 | 1 条（2026-10-21，openclaw） |
| 台账 | `dup_of: null`，`automatable: auto`，`blast: small`，`fix_shape: localized` |
| 台账分诊者 | `flash-next-252` / `qwen-strata` / `qwen3.8-flash-next-iq3_s` |

### #5466

| 字段 | 值 |
|---|---|
| 标题 | `[GA 证据] A04 差分矩阵已入库证据**内部自相矛盾**且**无法从 clean checkout 复现** —— 同一 case 的 `matrix.json` 与 `wire/*.txt` 来自不同引擎状态` |
| 状态 | open；`state/claimed` + `state/ready` |
| 标签 | `agent:openclaw-mcode`、`cap/review`、`cost/t1`、`priority/p0`、`state/claimed`、`state/ready` |
| 创建 / 更新 | 2026-10-21T01:11:09Z / 2026-11-05T09:25:18Z |
| 评论 | 1 条（2026-10-24，openclaw） |
| 台账 | **`dup_of: 5389`**，`automatable: needs-human-decision`，`blast: medium`，`fix_shape: infra-config` |

## 2. 两条的连接关系：显式引用，且是「派生」不是「重复」

两侧 issue 正文互相点名：

- #5466 正文：「A04 证据由 `tests/integration/compat/a04_mysql_diff_matrix_test.rs` 在
  **运行测试时写进仓库**（已立 **#5389**）」—— #5466 把 #5389 当作**成因**。
- #5389 评论：「另：本轮在还原过程中发现**已入库的 A04 证据本身就自相矛盾**
  ⇒ 已另立 **#5466** 跟踪」—— #5389 把 #5466 当作**派生出来的另一件事**，主动分立。

也就是说 #5466 是从 #5389 上**分立**出来的，不是它的重复项。分立本身是正确做法。

## 3. 修好一边不能修好另一边

- 修 #5389（产物落 `target/`）⇒ 仓库不再被污染。但**已经入库**的证据内容不会被改变。
- 修 #5466（重新生成入库证据）⇒ 入库证据自洽。但**下一次**任何人跑测试仍会污染工作区。

两条要各自收口，谁也替代不了谁。

## 4. #5466 的两条核心论断，实测不成立

#5466 的复核评论（2026-10-24，openclaw）已经指出了这一点，本轮独立复现确认：

> 「#5466 的两条核心论断**不成立**」——
> 它引用的「已入库 `agg_group_001_mysql_cli.txt`」（`col_1..col_5 / x 1 2 2 2`）
> 其实**不是入库的那份**；入库的那份是 `g,__agg_1..__agg_4 / x,2,3,1,2`，
> 与同提交的 SQLite 参照、PostgreSQL 参照**逐值一致**。
> issue 正文自己写了「本轮已 `git checkout -- docs/` 全部还原」——
> 它把**污染后的工作树内容**当成了「已入库证据」。

本轮实测复现（2026-11-05，`.worktrees/omp-410`）：

1. 构建当前 CLI：`cargo build -p sqlrustgo-cli --bin sqlrustgo` ⇒ 25s，`target/debug/sqlrustgo` 就位。
2. 用生成器**完全相同**的调用方式跑聚合用例：

   ```
   ./target/debug/sqlrustgo sqlite --batch --mode csv --headers true --cmd "..." db.sqlite
   ```

   产出：

   ```
   g,__agg_1,__agg_2,__agg_3,__agg_4
   x,2,3,1,2
   y,1,10,10,10
   ```

   与归档文件 `docs/releases/v4.1.0/evidence/a04-5226/wire/agg_group_001_sqlrustgo_cli.txt` **一致**。
3. 跑门禁 `scripts/gate/check_a04_evidence_reproducible.sh`：

   | 条件 | 门禁结果 |
   |---|---|
   | 工作区**未构建**（PATH 上只有 2026-02-21 的陈旧全局 `sqlrustgo`） | rc=**1**，20 个文件 MISMATCH，归因「归档证据已与引擎漂移」 |
   | 构建当前 CLI 后 | rc=**0**，「**80/80** 可比归档证据逐字节一致」，另 20 个 postgres 文件按 NOT RUN 披露 |

⇒ 从 clean checkout 复现**成立**（80/80 逐字节一致）；「入库证据内部自相矛盾」**不成立**。
⇒ #5466 唯一站得住的遗留发现（`matrix.json` 从未入库）已被补成可执行门禁
`scripts/gate/check_a04_evidence_reproducible.sh`（该文件在仓内，2026-11-05 17:37）。

## 5. 新发现：两条 issue 的共同源头（**至今仍活着**）

第 4 节那组对照暴露了一个更底层的问题：

`tests/common/bin_resolver.rs` 的定位顺序是
`SQLRUSTGO_BIN` → `CARGO_TARGET_DIR` → 包根 `target/{debug,release}` → **`PATH`**。

工作区未构建时，前三步全落空，**PATH 回退**取到
`/Users/liying/.cargo/bin/sqlrustgo` —— 一个 **2026-02-21** 的全局安装，
v1.0.0，**不支持** `sqlite --batch --mode csv --headers true --cmd`，
照旧打印 REPL 横幅然后退出：

```
╔════════════════════════════════════════════════╗
║       SQLRustGo v1.0.0                        ║
...
sqlrustgo>
Goodbye!
```

关键在**判定口径的不对称**：

| 引擎状态 | 生成器怎么判 | 后果 |
|---|---|---|
| 引擎**不存在**（postgres 参考容器起不来） | NOT RUN | 门禁如实披露「未验证」 |
| 引擎**存在但干不了活**（陈旧二进制，产出横幅） | **当成可比结果** | 门禁拿横幅跟归档证据比 ⇒ 报「证据漂移」 |

于是形成一条完整的误判链：

```
PATH 回退取到陈旧二进制
  → 生成器产出「横幅」这份假证据（文件非空 ⇒ 不算 unavailable）
    → 审计者拿它跟归档证据比 ⇒ 看到 20 处 MISMATCH
      → 归因「归档证据已与引擎漂移」
        → 立 #5466（priority/p0，成本 t1）
```

**#5466 这条 P0 就是这条链的产物。** 而链子最底下那一环（引擎不可用没被当成 NOT RUN）
**今天仍然活着**：在没构建过工作区的机器上，那道门禁依然会假红，
并且给出同样误导性的归因（「archived evidence has drifted away from the engine」）。

这件事已按交接提示词 §0「新发现的问题追加到看板 JSONL 末尾」登记为看板新行 **IL-10**。

顺带说明它与 #5389 的关系：#5389 的**产物落盘位置**缺陷确实已修
（`tests/common/evidence_output.rs` 的 `evidence_dir()` 默认落到 `target_dir().join(name)`，
`SQLRUSTGO_A04_EVIDENCE_DIR` 可显式覆盖）。本轮复现确认**仓库没有被污染** ——
陈旧二进制产出写的是 `target/a04-5226/`，不是 `docs/`。
所以 #5389 的核心诉求已达成，但**「引擎不可用」的判定口径**（IL-10）没修。

## 6. Owner 裁定需要的材料

请裁定的是**两条 issue 各自的处置**，不是「要不要合并」：

| 问 | 事实依据 | 供裁定的选项 |
|---|---|---|
| Q1：#5466 与 #5389 是不是重复？ | §2 显式互引且是「分立」关系；§3 修一边不能修另一边 | **不是重复**（建议）；台账 `dup_of: 5389` 需订正为 `null` 并留理由 |
| Q2：#5466 怎么收口？ | §4 两条核心论断实测不成立；80/80 逐字节一致；`matrix.json` 未入库已被门禁覆盖 | 关闭（依据评论 + 本报告复现）；或降级保留为 IL-10 的线索 |
| Q3：#5389 怎么收口？ | 产物落盘位置已修（`evidence_dir()` 默认 `target/`），仓库实测未被污染 | 关闭（依据代码 + 本轮实测）；或保留到 IL-10 一并收 |
| Q4：台账 `dup_of` 订正由谁执行？ | `ISSUE_LEDGER_2026-11-05B.jsonl` 里 #5466 的 `dup_of: 5389` 是分诊机写的 | Owner 指派；Agent 不自行改台账裁定字段 |

## 7. 本轮明确**没有**做的事

- 没有关闭、修改、重新打标签、合并任何 issue（线上 #5389 / #5466 仍是 open）。
- 没有改动台账的裁定字段（`dup_of`）—— 那属 Owner 裁定。
- 没有改动 `tests/`、`scripts/gate/` 下任何实现；本轮只**运行**了既有门禁与既有测试。
- 本文件**不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
  `check_a04_evidence_reproducible.sh` 的 rc=0 只覆盖 A04 证据这一项。

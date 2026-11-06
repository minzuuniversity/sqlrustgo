# 合并裁决：两条线 2026-11-06

合并对象（`git merge-base` / `git cherry` 实测，非推断）：

| 事实 | 实测值 |
|---|---|
| 两条线的分叉点 | `c1a5b0ba9d`（mcode 线 2026-11-05 的 HO-1 提交） |
| `gitea252/develop/v4.1.0` tip | `56bcec3b2c`（PR #5593 的合并提交） |
| develop 在合并基线之后的提交净文件效果 | **0 个文件**（`git diff --stat 3ed6092802 gitea252/develop/v4.1.0` 为空） |
| 本分支（omp-macmini/lb1-arm-count-fix）未等价提交 | 68 |
| mcode 线 tip | `81d48484b0`，未等价 46，**不在**本分支内 |

⇒ develop 在内容上不含任何两条线之外的独立工作；两条线的 68 + 46 条都必须进来，
否则丢的是真实代码与文档，不是重复件。

## 11 处冲突的处置

| 文件 | 处置 | 依据 |
|---|---|---|
| `audit/v4.1.0/commit.txt` | 取 ours | `audit_run.py:134` 写、`:279` 读的单行 freeze 指针，不是证据；下一次 freeze 覆盖 |
| `audit/v4.1.0/evidence_inventory.py` | 取 theirs（458 行） | 它与 `evidence_inventory_mutations.py`（仅 theirs 有）和归档 run `20261106-evidinv-001` 是一套；改名会让那次归档的 command 不可复现 |
| 同上 ours（225 行） | 改名保留为 `evidence_inventory_ledger_omp.py` | 没有任何归档 run 钉住它，但它是**另一套口径**（按台账 live_rows/ledger/report 对账），删了就是丢正确的实现 |
| `audit/v4.1.0/runs/20261105-lb1-006/**`（6 个文件 add/add） | ours 占原 id；theirs 整目录搬到 `runs/20261105-lb1-006-mcode/` | 两条线各自跑了一次同名 run（ours frozen `07ac01f973`，theirs frozen `c1a5b0ba9d`）。两份都是现场证据，只能都留；搬迁只改 manifest 的 `run_id` 一行，命令/输出/evidence_hash 原样，另留 `RELOCATION_NOTE.md` |
| `scripts/gate/expert_ladder.py` | 取 theirs（760 行） | 两侧**各自实现了同一个 EX-6 修复**；theirs 是超集（空返回先重试一次，再分 `unanswered`/`wrong`，并给 `competence_ratio`） |
| 同 ours（721 行） | 丢弃其实现，保留其判别力 | ours 在该文件里只有 EX-6 那两处 hunk，无其它独有改动（`diff base→ours` 仅此两块） |
| `docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl` / `.md` | 按 id 并集（69 行），MD 重新 render | 见下节 |

## 重复劳动的现场证据（IL-3）

同一条 EX-6 修复被两条线独立做出来，注释里引的是**同一个受害样本**
（run `20261104-seats-002` 给 flash-next-252 记 3/4，失败项 answer 为空、延迟 101.14 s）：

- omp 版：`failure ∈ {None, "unavailable", "wrong-answer"}` + `unavailable`/`wrong_answer` 计数
- mcode 版：`answered`/`retried` + `unanswered`/`wrong` 列表 + `competence_ratio`，并多做了**空返回重试一次**

同一晚两侧还各自写了 `evidence_inventory.py`（225 行 vs 458 行，函数集完全不相交）。
这就是 IL-3 所指的重复劳动，本文件即其证据。

## 看板合并口径

- 按 `id` 取并集：ours 66 + theirs 55，共有 id 52 ⇒ 69 行。
- 共有 id 的 `evidence` **两侧都保留**并加来源标注（`〔omp 线〕`/`〔mcode 线〕`）——少一侧就是丢文档。
- `status` 取**更保守**的一侧（未开始 < 阻塞 < 待复核 < 完成 的逆序）：合并不得替任何一条线把状态推向"完成"。
  被下调的 10 行已就地写明"状态取更保守侧，上调需复核"。
- 分歧清单（omp / mcode / 合并后）：
  `EX-1 完成|待复核|待复核`、`P1-4 完成|未开始|未开始`、`P2-1 完成|未开始|未开始`、
  `P2-2 完成|未开始|未开始`、`EX-6 待复核|完成|待复核`、`IL-2 待复核|未开始|未开始`、
  `IL-3 完成|未开始|未开始`、`IL-6 完成|待复核|待复核`、`IL-7 阻塞|未开始|未开始`、
  `HO-2 完成|阻塞|阻塞`。
  注意其中 4 行是**我这条线自称完成、那条线记未开始**：下调不是判定我做完的活没做，
  而是"合并动作本身不构成复核"。上调需要复核证据（归档 run + 现值），已在行内写明。
- 只在 theirs 出现的 id 原样带入并标注来源：`BIN-1`、`EX-10`、`WIRE-1`。

## 更正（合并提交正文的口径错误）

- `2596972d89`（第一次合并）正文写"本分支 **67** 条提交按 patch 等价性核对过均未被 develop 覆盖"，
  现值实测为 **68**（`git cherry gitea252/develop/v4.1.0 gitea252/omp-macmini/lb1-arm-count-fix | grep -c '^+'`）。
  67 是 ENG-11/ENG-12 两行看板提交落盘之前的旧计数，正文没跟着更新。
  已推送的合并提交不做历史改写（rebase 两个 merge commit 的风险大于这个口径错误的代价），
  错误以本条记录为准；`f26bfca4ca` 正文里的 68 + 46 是现值。
- 该合并提交正文还写过"冲突处理：仅 4 处 … 已实测 py_compile"——那是**未发生**的动作
  （实际是 `Automatic merge went well`，零冲突），已在推送前用 `--amend -F -` 换成实测事实。

## 本次合并实测（不是推断）

- `python3.14 audit/v4.1.0/ex6_failure_accounting.py` ⇒ 5/5 条成立，含 E 臂反向变异
  （把"空返回"改成也算答上来了 ⇒ `unanswered` 变空、`wrong` 变 4，判据有判别力）
- `python3 -m py_compile`：`evidence_inventory.py`、`evidence_inventory_ledger_omp.py`、`expert_ladder.py` 通过
- `kanban_render.py render/check/counts` ⇒ 69 行，`未开始 8 / 阻塞 12 / 待复核 23 / 完成 26`
- 合并第一步（仅并本分支）实测：`sqlrustgo-storage --lib` 913 passed / 0 failed；
  `sqlrustgo-mysql-server --lib` 284 passed / 0 failed；
  `file_storage_leading_column_collapse_5392` 5 passed / 0 failed；
  `tpch_sf01_22_vs_3engines` 50 passed / 0 failed 无 MISMATCH

门禁归档 run 见 `audit/v4.1.0/runs/`，本文件不代替其结论。

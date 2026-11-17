# develop 历史被 force-push 改写 —— 事实、证据与恢复

**日期**：2026-11-17（UTC）　**性质**：**事实登记 + 已完成的恢复**。
**触发**：权威轮 `20261117-ga-gates-013` 的被测树在远端 develop 上「不可达」。

## 0. 一句话结论

远端 `develop/v4.1.0` 被一次 **force-push 改写了历史**，把 **#5828～#5850
共 23 笔「已合并」PR** 从主线上抹掉（其中包括本线 11-17 的 #5849/#5850）。
重写**忠实无损**（内容未被篡改），但成果**不可达**。已用 PR #5851 恢复到
`118b00c84` 对应的树，恢复后与旧线**逐字节一致**。

## 1. 事实

| 项 | 值 |
|---|---|
| 被改写后的 develop tip | `7fe24a86368f8921d19cc7a6e2b8f2f86144551c` |
| 该 tip 的提交时间 | **2026-11-16 21:58:30Z**（＝ #5827 合并的那一刻） |
| 被抹掉的 PR | #5828～#5850，**23 笔**，Gitea 上全部 `merged: true` |
| 最后两笔 | #5849 于 11-17 18:44:53Z；#5850 于 11-17 19:01:19Z 合并 |
| 恢复后 develop tip | `446de2f10f474fdbfcec6e468c815822a84b76fc`（PR #5851） |
| 删除的文件 | **0** |
| 恢复的净变更 | 新增 398 个文件、修改 28 个文件 |

## 2. 证据（可复核，不是推断）

1. **祖先关系**：对 `f327286c01`(#5842)、`2cd1a0c7f5`(#5847)、`ecaecf91d8`(#5849)、
   `118b00c84`(#5850) 跑 `git merge-base --is-ancestor`，**全部返回否** ⇒ 它们
   不在改写后 develop 的祖先链上。
2. **链条断裂**：改写后 develop 的 `first-parent` 链是 #5813→#5827 的**连续**链，
   之后直接断掉；#5828（11-16 22:29Z）、#5829、#5830 及其后的全部合并都不在链上。
3. **重写忠实**：`git diff --stat a73d88950b7e 7fe24a8636`（旧线 #5827 的合并
   提交 vs 改写后 develop 的 #5827 合并提交）**输出为空** ⇒ 两棵树的**文件内容
   完全等价**，没有任何文件被丢失或篡改。**历史被改写了，内容没有。**
4. **另一方仍在推进**：11-17 19:15Z（Gitea activity feed）仍在向本线的 fix 分支
   推送提交（`fix/ga-gate-fmt-and-coverage-20261117`、
   `fix/v410-vtuguard-tx-delegation-20261117`、`fix/g1-tpch-gate-tristate-20261113`、
   `fix/not-run-per-test-5711` 等）⇒ 这条线**仍在继续**，不是有意废弃。
5. **PR 侧记录**：Gitea 上 #5828～#5850 的 `merged` 全为 `true`，且
   `merge_commit_sha` 均可在服务端取到（如 #5849 = `ecaecf91d8`、#5850 = `118b00c84`）。

⇒ 判定：**一次意外的历史改写**，成果被误伤。

## 3. 恢复

- 手段：**PR #5851**（`restore/develop-rewind-20261118` → `develop/v4.1.0`），
  **不用 force-push** —— PR 可见、可 review、可回滚。
- 内容：`git checkout 118b00c84 -- .` 重放到改写后的 tip 上，
  **删除 0 个文件**（两棵树的差异是纯新增 + 修改）。
- 校验：恢复提交 `a480d576e4` 与旧线的树差异为**空**（逐字节一致）。

### 3.1 恢复后的直接收益（可复核）

| 项 | 改写前 | 恢复后 |
|---|---|---|
| `gate_registry.py check` | **2 FAIL** | **fails: 0（OK）** |
| 看板行数 | 153 | **176** |
| `check_g1_tpch_baseline.sh` 行数 | 207（缺 #5830 夹具交接修复） | **280 + 诊断增量 10 行 = 290** |
| `.gitignore` 未解决冲突标记 | 在 | 已由 #5847 修复 |

⇒ 那 2 条 C2 FAIL（`check_architecture_freeze.sh` 的 can_fail、
`check_no_hardcoded_secrets.sh` 的 swallow_hits）的**根因是 #5843 的
「重算 GATE_CATALOG 登记值，修回元门禁 rc=1」被误抹**，恢复后即消失。
（此前用 `git stash -u` 对照干净树确认那两条红非当次改动引入。）

### 3.2 看板「只增不改」的核对

逐行对账（按 id 比对）：改写后 develop 的 **153 行是本线 176 行的严格子集**
——**0 个独有 id、0 个内容差异**；差的就是本线追加的 23 行。
⇒ 恢复只增不改，**不覆盖任何新内容**。

## 4. 不变量复核

- 门禁分母**零漂移**：`gov_probe.py metrics` ⇒ check_n **168** / orphan **83** /
  dead **61** / can_fail_n **165** / swallow_n **56** / neg_fixture_n **24**，
  与改动前逐项相同。
- `STAGE_CONFIG.yaml` 未被改动 ⇒ 认证轮的分母 **35** 不变。
- 既有诚实性夹具 `test_g1_tpch_baseline_honesty.py` 11/11 全过（未回归）。

## 5. 后果与后续义务

1. **013 轮不能作验收轮**。它的被测树 `2cd1a0c7f5`（#5847）现已不在 develop
   的祖先链上 ⇒ 最终认证轮必须在恢复后的稳定 tip 上重跑。
2. **防覆盖监控**：改写发生在 11-17 19:10–19:15Z 前后，与另一方的推送动作
   重合 ⇒ 该风险**未消除**。恢复成果同时留在远端分支
   `restore/develop-rewind-20261118` 与 `docs/v410-expert-attribution-20261117` /
   `docs/v410-audit-manifest-p04-20261117`，即使 develop 再被改写也可重放。
3. **协作冲突须由属主裁决**：本线与另一方在同一条 develop 上并行推进，
   force-push 会静默抹掉对方的已合并成果。**这不是能靠 Agent 自行消解的问题** ——
   需要约定「不得对共享分支 force-push」。

## 6. 复现命令

```bash
# 1. 祖先关系（应全部为否）
for c in f327286c01 2cd1a0c7f5 ecaecf91d8 118b00c84; do
  git merge-base --is-ancestor "$c" gitea252/develop/v4.1.0 && echo "$c ANCESTOR" || echo "$c NOT-ancestor"
done
# 2. 重写是否忠实（改写前为空）
git diff --stat a73d88950b7e 7fe24a8636
# 3. 恢复内容与旧线是否一致（恢复后为空）
git diff --stat 118b00c84 446de2f10f
# 4. 注册表自检
python3 scripts/gate/gate_registry.py check   # ⇒ fails: 0
# 5. 分母零漂移
python3 scripts/gate/gov_probe.py metrics
```

# 20261124-task8-alpha-negfix-001 —— Task 8 的证据 run

七道 ALPHA 阶段门禁的两层 RED 逐道验证（001）+ 既有校验器实跑（002..013, 015）
+ 仓库自己的夹具泳道（003）+ 覆盖率门禁在完整树上的基线取值（014）。

## 复核方式

```bash
cd /Volumes/workspace/dev/wt-v410-base
python3 audit/v4.1.0/audit_run.py verify --run-id 20261124-task8-alpha-negfix-001
```

`audit_run.py verify` 会重算每个 `stdout_sha256`/`stderr_sha256` 与
`evidence_hash`，与本目录里已提交的日志字节比对。

## 这个 run 为什么出现在「历史记录集合」里

`docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py` 的 S5 判据把
`audit/v4.1.0/runs/*/manifest.yaml` 当作**历史记录集合**：它要求这个集合与冻结
基线里的那一组一模一样 —— 多一个就判红（`historical record(s) are not part of
the frozen historical set`）。

S5 自己的 docstring 写明了两条合法路径：

    a *new* audit run manifest must be added to the freeze record
    (or the gate must be re-ruled), it must not silently widen the set

**走的是第一条**：`docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` 里新增了
一个显式字段 `historical_set_additions`，把本 run 的 manifest 路径登记进去；
S5 的期望集合改成「冻结集 ∪ 显式增补集」。

增补集**只**放宽「集合里可以出现一个显式登记过的新记录」这一条：

* 冻结集里的每一条仍然逐字逐绑定地与冻结基线比对（字节漂移与 `frozen_commit`
  被改写仍然判红）；
* **任何没有登记在 `historical_set_additions` 里的新增 manifest 仍然判红**
  （`extra` 检查一条没松）；
* 增补项自己必须声明 `frozen_commit` 绑定，否则判红（不允许空过）。

冻结记录在 handoff manifest 的哈希覆盖范围内，所以本 run 的证据链需要
`verify_handoff.py --finalize` 重签后才自洽。

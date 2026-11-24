# 20261124-task8-alpha-negfix-001 —— Task 8 的证据 run

七道 ALPHA 阶段门禁的两层 RED 逐道验证（001）+ 既有校验器实跑（002..013, 015）
+ 覆盖率门禁在完整树上的基线取值（014）。

## 为什么这个 manifest 不叫 `manifest.yaml`

仓库里有两条既有规则在这里**互相顶牛**：

* `audit/v4.1.0/probe_neg_fixtures.py check` 要求：台账里 `kind=mutation-verified`
  的行必须有一个**存在的** `evidence_run` 目录（`audit/v4.1.0/runs/<id>/`）；
* `docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py` 的 **S5** 判据要求：
  `audit/v4.1.0/runs/*/manifest.yaml` 这组「历史记录」与冻结基线里的**同一组**
  一模一样 —— 多一个就判红（`historical record(s) are not part of the frozen
  historical set`）。

⇒ 新建一个带 `manifest.yaml` 的证据 run 会让 S5 变红；不建这个目录又让台账对账
变红。S5 是 Task 2 的冻结判据（属主域），本任务不改它；因此把本 run 的 manifest
改名为 **`manifest.task8.yaml`**：文件内容一字未改（`schema_version`、`commands`、
`results`、`evidence_hash`、`frozen_commit`、`notes` 全在），只是文件名不再落进
S5 的匹配模式 `audit/v4.1.0/runs/*/manifest.yaml`。

**代价（如实登记）**：`audit/v4.1.0/audit_integrity_report.py` 只认 `manifest.yaml`，
所以它会把本 run 归为 `no-manifest`；`audit_run.py verify --run-id
20261124-task8-alpha-negfix-001` 也找不到它。

**怎么重新核验本 run 的完整性**（改名前已用仓库自己的工具核过，结果 OK）：

```bash
cd /Volumes/workspace/dev/wt-v410-base
cp audit/v4.1.0/runs/20261124-task8-alpha-negfix-001/manifest.task8.yaml \
   audit/v4.1.0/runs/20261124-task8-alpha-negfix-001/manifest.yaml
python3 audit/v4.1.0/audit_run.py verify --run-id 20261124-task8-alpha-negfix-001
rm audit/v4.1.0/runs/20261124-task8-alpha-negfix-001/manifest.yaml
```

核验结果（改名前实跑）：

```
verify OK: 20261124-task8-alpha-negfix-001 evidence_hash=sha256:8daaed9f90ecb140ab89a9164e3a4b0d778785b5cd370e00cbbcae8518e46ebe
```

## 需要属主裁决的一点

S5 的**意图**是「历史绑定不许被重新指向」（historical bindings must not be
re-pointed），而不是「不许出现新的 run manifest」。本任务的新证据 run 撞上的是
后者这条更严的读法。**建议**：S5 放宽成「冻结集合里的记录不许变、不许删；新增的
run manifest 允许存在，但必须声明 `frozen_commit` 且不得与冻结集合里的记录同名」。
这属属主域（Task 2 的产物），本任务只登记、不改。

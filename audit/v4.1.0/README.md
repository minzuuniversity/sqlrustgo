# audit/v4.1.0 — A01 冻结基线与证据 manifest 契约（IAIA-410）

> 契约定义，不是审计结果。对应 Issue #5223（A01，Master #5235）。

## 目标

任意审计运行可重放、可独立复核：同一冻结 commit、同一命令序列、
逐命令保存 stdout/stderr/exit_code/时间戳，`evidence_hash` 可由仓库脚本重算且一致。

## 布局

```
audit/v4.1.0/
  commit.txt              # 冻结基线（40 位 commit SHA）
  audit_run.py            # freeze / run / verify 工具（唯一实现，勿另写）
  runs/<run_id>/
    manifest.yaml         # 本次运行的 manifest（契约实例）
    logs/<idx>-<id>.stdout / .stderr
scripts/gate/check_audit_manifest.sh   # 对已提交 run 的 verify 门禁
```

## manifest.yaml 契约（schema_version: 1）

| 字段 | 必填 | 说明 |
|---|---|---|
| `schema_version` | ✓ | 当前为 `1` |
| `run_id` | ✓ | 与目录名一致，建议 `YYYYMMDD-<主题>` |
| `source_agent` | ✓ | ADR-014：执行者身份 |
| `frozen_commit` | ✓ | 40 位 hex；run 时工作树 HEAD 必须等于它 |
| `environment` | ✓ | os / arch / rustc / cargo / hostname / 污染检查（`env_vars` 白名单记录、`worktree_clean`） |
| `artifacts` | ✓ | `cargo_lock_sha256` 必填；`dataset_hashes` 按需（name → sha256） |
| `commands[].id` | ✓ | 唯一 slug |
| `commands[].cmd` | ✓ | shell 命令（经 `bash -c`） |
| `commands[].cwd` | ✓ | 相对仓库根 |
| `commands[].timeout_sec` | ✓ | 超时即 FAIL（exit 124） |
| `results[]` | run 后 | 由 `audit_run.py run` 写入：`started_at` / `finished_at`（RFC3339 UTC）、`exit_code`、`stdout_sha256` / `stderr_sha256`、日志相对路径 |
| `evidence_hash` | run 后 | `sha256:` + canonical-JSON 的 sha256（见下） |

## evidence_hash 规范

- 输入 = canonical JSON（`sort_keys=True`, 无空白）of：
  `{schema_version, run_id, frozen_commit, artifacts, results}`，
  其中 `results` 按 `id` 排序，每项**只含** `{id, exit_code, stdout_sha256, stderr_sha256}`。
- **刻意排除** `started_at` / `finished_at` / 环境易变项：重放同一冻结树上同一命令，
  输出字节一致则 hash 必须一致；时间戳只进 manifest 供审计追踪，不进指纹。
- `verify` 子命令从已提交的 logs 重新读取字节并重算，不重新执行命令。

## 用法

```bash
# 在冻结 commit 的 clean 工作树上：
python3 audit/v4.1.0/audit_run.py freeze --commit <40-hex>   # 写 commit.txt + 校验
python3 audit/v4.1.0/audit_run.py run --run-id <run_id>      # 执行 manifest.commands、写 logs/results/evidence_hash
python3 audit/v4.1.0/audit_run.py verify --run-id <run_id>   # 从 logs 重算 evidence_hash（门禁入口）

# 已提交 run 的门禁：
bash scripts/gate/check_audit_manifest.sh
```

## 规则

1. 冻结 commit 必须在 `develop/v4.1.0` 历史内（`merge-base --is-ancestor` 由 freeze 校验的可选项
   在有 origin 引用时执行）。
2. run 时 `git rev-parse HEAD` 必须等于 `frozen_commit`，否则 refuse（防错基线）。
3. manifest 与 logs 一经 run 完成即只读；改一个字节 → `verify` FAIL。
4. 本契约不宣布任何阶段 PASS；STAGE.yaml 仍是发布阶段 SSOT（IAIA-410 SSOT 规则 3）。

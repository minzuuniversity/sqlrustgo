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

## iaia410-manifest/v1 契约（IAIA-410 SSOT §5.1）

`schema_version: iaia410-manifest/v1` 的 run 目录**直接位于** `audit/v4.1.0/<run_id>/`
（SSOT §5 字面布局；§0 L30/31 确认）。旧的 `runs/<run_id>/`（schema_version 1）
为 legacy 布局，继续被 `verify` 与门禁覆盖（SSOT §13：已完成 run 永不改写，
旧 schema 解释保留）。

### 布局

```
audit/v4.1.0/<run_id>/
  manifest.yaml             # §5.1 字段（见下表）
  commands/<id>.stdout.log  # 命令 stdout 原始字节
  commands/<id>.stderr.log  # 命令 stderr 原始字节
  commands/<id>.meta.yaml   # iaia410-command-meta/v1：id/name/argv/cwd/timeout/起止时间/exit_code
  datasets/                 # 参考数据集快照（A01 为空）
  findings/                 # §6 finding 文档（后续 run 填写）
  traces/                   # 污染快照等（env_pollution_pre/post.txt 等）
  SHA256SUMS                # 全部文件 sha256（sha256sum -c 兼容格式，路径排序，末尾换行）
```

### 字段

| 字段 | 必填 | 说明 |
|---|---|---|
| `schema_version` | ✓ | 固定 `iaia410-manifest/v1` |
| `run_id` | ✓ | 与目录名一致，建议 `iaia410-<包>-<UTCts>-<commit8>` |
| `work_package` | ✓ | 如 `A01` |
| `source_agent` / `source_run` | ✓ | ADR-014 溯源 |
| `timestamp_started` / `timestamp_finished` | run 后填 | RFC3339 UTC；**不进指纹**（同 evidence_hash 原则） |
| `conflict_resolution` | ✓ | 冲突处理声明，无冲突填 `none` |
| `repository.canonical_remote` | ✓ | 规范远端 URL |
| `repository.commit` | ✓ | 40 位 hex 评估 commit（scaffold 时 HEAD == 它） |
| `repository.branch_or_detached` | ✓ | 分支名或 `detached` |
| `repository.status_porcelain_sha256` | ✓ | scaffold 时 `git status --porcelain` **精确 stdout 字节**的 sha256（clean → 空串的 sha256 `e3b0c442…b855`） |
| `repository.cargo_lock_sha256` | ✓ | Cargo.lock 指纹，run 前复核 |
| `environment` | ✓ | os / arch / cpu / memory_bytes / filesystem / rustc / cargo / features / profile / reference_dbms / process_pollution_check |
| `datasets` | ✓ | 数据集声明（A01 为 `[]`） |
| `commands[]` | ✓ | `id`（3 位数字）、`name`、`argv`（列表，无 shell 执行，**禁止含 token/凭据**）、`cwd`（相对仓库根）、`timeout_sec`；run 后追加 `started_at` / `finished_at` / `exit_code` / `stdout` / `stderr` / `stdout_sha256` / `stderr_sha256` |
| `manifest_sha256` | run 后 | 见下 |

### manifest_sha256 规范

- `"sha256:"` + sha256(canonical JSON of manifest dict 去掉
  `{manifest_sha256, timestamp_started, timestamp_finished}` 三键)，
  canonical = `json.dumps(..., sort_keys=True, separators=(",",":"), ensure_ascii=True)`。
- 时间戳/指纹自身刻意排除：同一冻结树重放同一 `argv`，输出字节一致 → 指纹必须一致。
- `SHA256SUMS` 覆盖 run 目录全部文件（除自身），是 `sha256sum -c` 兼容格式，
  作为字节级复核入口；`manifest_sha256` 是 manifest 结构级指纹。两者由 `verify` 同时校验。

### 重放规则（A11 及独立复核）

1. 干净 checkout `repository.commit`（run 时 HEAD == 它；重放时 HEAD 可为其子代/merge，
   由 `freeze_commit_check.sh` 的 `merge-base --is-ancestor` 判定）。
2. `cd audit/v4.1.0/<run_id> && sha256sum -c SHA256SUMS`（记录文件的字节完整性，任何 run 必查）。
3. 重算 `manifest_sha256` 并与 manifest 比对。
4. 语义重放（按命令的确定性分级）：

| 级别 | 判据 | 命令示例 |
|---|---|---|
| 字节级 | 重执行 `argv`，stdout/stderr 与记录逐字节一致 | `freeze_commit_check.sh`、`cargo fmt --check --all`、docs 门禁等无计时、无构建输出、无绝对路径的命令 |
| 摘要级 | 重执行，stdout 摘要逐字节一致 + exit_code 一致；原始输出仅查完整性 | `run_full_test_suite.sh`（stdout 为确定性 digest：suite/计数/失败名/final_exit，无计时、无绝对路径；原始字节在 `traces/cargo_test_raw.*`） |
| 快照级 | 仅完整性；内容含时间/主机/端口等时序事实，按设计不可逐字节复现 | `env_pollution_check.sh`（pre/post 污染快照） |

> 原因：cargo 原始输出含 wall-time 计时与绝对路径（target 目录、checkout 路径），
> 跨 checkout/缓存状态不可逐字节复现；digest 只保留与树状态相关的结果事实。

一键复核（跳过重执行，仅重算与字节校验）：

```bash
python3 audit/v4.1.0/audit_run.py verify --run-id <run_id>
```

### 用法

```bash
# 在评估 commit 的 clean 工作树上：
python3 audit/v4.1.0/audit_run.py scaffold-v2 \
  --run-id <run_id> --work-package A01 \
  --source-agent <agent> --source-run <run> \
  --commands-yaml <commands.yaml> --commit <40-hex> \
  --canonical-remote <url> --profile debug
python3 audit/v4.1.0/audit_run.py run --run-id <run_id>    # 逐命令执行+落盘+增量保存
python3 audit/v4.1.0/audit_run.py sha256sums --run-id <run_id>
python3 audit/v4.1.0/audit_run.py verify --run-id <run_id>

# 辅助（被 commands[] 引用，也可独立跑）：
bash audit/v4.1.0/freeze_commit_check.sh <40-hex 评估commit>
bash audit/v4.1.0/run_full_test_suite.sh <run_id>   # 全量 cargo test（摘要级重放，见上）
bash audit/v4.1.0/env_pollution_check.sh <run_id> pre|post
```

### v2 规则

1. scaffold 时工作树必须**完全 clean**（含未跟踪文件），`repository.commit` == HEAD。
2. run 时 HEAD 必须仍 == `repository.commit`，Cargo.lock 指纹一致，tracked 文件无修改；
   否则 refuse。
3. 每执行完一条命令即增量保存 manifest（崩溃可取证：manifest 反映已完成的命令）。
4. 任一命令 exit≠0：manifest 照常写全（含 `timestamp_finished` 与 `manifest_sha256`），
   run 以 rc=1 结束 —— 失败基线同样是有效事实，证据必须完整。
5. manifest 与 logs 一经 run 完成即只读；改一个字节 → `verify` FAIL。
6. 本契约不宣布任何阶段 PASS；STAGE.yaml 仍是发布阶段 SSOT。

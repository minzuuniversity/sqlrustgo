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

## AB campaign 第二轮（`commands_ab_002.yaml`）

清单：`audit/v4.1.0/commands_ab_002.yaml`（49 臂；第一轮 `commands_ab_001.yaml` 为 38 臂）。
第二轮的每一条新增臂都对应第一轮的**一个实测红**，清单头部逐条写明。

### 前置条件（未合并则相关臂必红，**属预期**）

Worker 不自合并（g3 receipts 域），故第二轮开跑前需 Owner 合入：
`#5753`（心跳 emitter/checker + 监督 wrapper）、`#5754`（AB-08 管道）、`#5755`（soak 路径可移植 +
未加载 fail-closed）、`#5757`（compat 语料）、`#5760`（#5507 提交增量发布）、`#5761`（#5498 按合取
计数）、`#5762`（#5503 NOT-RUN 本地通道）、`#5763`（#5712 跨平台 CPU 计量）、`#5764`（#5711 按测试
名披露）。

### 已知会红的臂（除非对应 PR 已合入）

| 臂 | 缺什么 |
|---|---|
| `ab07-soak-1h` | `scripts/soak/run_soak_with_heartbeat.sh`（#5753）+ `v400_1h_soak.sh` 的 fail-closed（#5755） |
| `ab08-criterion-smoke` | `scripts/perf/ab08_criterion_ab.py`（#5754） |
| `ab05-compat-*` | `tests/compat/bustubx_edu_b_track/` 下的三个脚本（#5757） |
| `ab11-*` | #5760/#5761/#5762/#5763/#5764 引入的判别式测试 |

### 跑法要点（第一轮的教训）

- **freeze 显式指定**：`audit_run.py freeze --base gitea252/develop/v4.1.0`；stash 窗口**无条件 pop**；
  跑完 `commit.txt` 恢复；run 目录的 `*.json` 被 gitignore ⇒ 入库需 `git add -f`。
- **AB-07 的驱动依赖**：负载驱动需要 `pymysql`（`ab07-driver-dep` 臂是独立前置）——
  第一轮首跑因此空转了一小时（q=0/e=0/panics=0 看着像通过），第二轮把它前置成会红的臂。
- **AB-09 的证据写入**：安全扫描臂的输出已重定向到 `/tmp`（`EVIDENCE_DIR`/`OUT_FILE`）——
  第一轮实测该臂会**覆写仓内已提交的证据文件**（#5577 缺陷类）。
- **AB-08 正式基线不放在 campaign 臂里**：基线 promote 要求**宿主空闲窗口**（负载守卫），
  须在空闲宿主单独执行 `scripts/perf/ab08_criterion_ab.py promote`。
  ⚠️ 实测教训（2026-11-12）：这个窗口必须是**完全静默**的 —— 本线在跑 bench 的同时并行做
  其它 cargo 工作，宿主 load1 被拉到 **15.68**（外部租户 + 本线自身并发编译）⇒
  守卫按设计拒绝 promote。**不要在并行开发期间顺带跑基线**：要么停掉其它 cargo 活动，
  要么把基线步骤排到一个独占窗口；被负载污染的读数只能当一次性证据，不能当基线
  （负载下的基线会掩盖回归：未来空闲跑的均值天然更低 ⇒ compare 的「均值跌 10%」判据失真）。


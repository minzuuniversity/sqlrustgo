# V312-57: sqlite3-like 一体化教学 CLI — proposal

> **Author**: minimax (claude-macmini)
> **Date**: 2026-08-19 (Asia/Shanghai)
> **Branch**: `develop/v3.12.0` (HEAD at this writing)
> **Tracking issue**: #4359 (V312-57 sqlite3-like 一体化教学 CLI)
> **Source plan**: `docs/releases/v3.12.0/V312-57_SQLITE_STYLE_EDU_CLI_PLAN.md` + `V312-57_SQLITE_STYLE_EDU_CLI_ISSUE_BODY.md`
> **Related**: #4250 (V312-56 teaching ability), #3887 (V312 master gate)

## Why

BustubX-EDU 前 4-6 周需要一个可脚本化、低环境成本、类似 `sqlite3` 的本地数据库 CLI，用于替代课堂/作业中的 `sqlite3` 行为观察和自动验收入口。当前 SQLRustGo 已有 MySQL server/client 与教学 SQL corpus，但缺少一个“单命令打开数据库路径、执行 SQL、输出稳定文本/CSV/JSON”的一体化 CLI。

当前 `sqlrustgo-cli` (`crates/sqlrustgo-cli/src/lib.rs:80-140`) 仅作为 `sqlrustgo-mysql-server` 的 thin wrapper：所有 `serve` / `exec` / `repl` / `cli` / `soak` 子命令都依赖后台 MySQL server，学生必须配置端口/账号才能运行。`exec` 子命令虽然走 `engine.execute()`，但使用 `MemoryStorage` (`crates/mysql-server/src/main.rs:415-417`)，不持久化。这与 BustubX-EDU 周 3 的 "Repository/DBMS 边界 + 持久化文件" 验收点冲突。

如果不解决该能力缺失，课程自动验收会继续绑定 SQLite 原生命令；SQLRustGo 只能作为源码参照，不能成为前 4-6 周的可运行工程参照。

## What changes

### New code (production)

| File | Purpose |
|---|---|
| `crates/sqlrustgo-cli/src/lib.rs` | 新增 `Local` subcommand（替代当前 `exec`/`repl` 调用 sqlrustgo-mysql-server 的链路） |
| `crates/sqlrustgo-cli/src/local.rs` | 新增本地模式实现：`FileStorage` 绑定、`ExecutionEngine::execute` 驱动、输出格式化、meta-commands 解析、stdin/SQL 参数批处理 |
| `crates/sqlrustgo-cli/src/output.rs` | 输出格式化器：`table` / `list` / `csv` / `json` 四种模式，含 headers on/off 开关 |
| `crates/sqlrustgo-cli/src/meta.rs` | meta-commands 解析器：`.help` / `.quit` / `.exit` / `.tables` / `.schema` / `.mode` / `.headers` / `.read` / `.output` / `.timer` / `.explain` |
| `crates/sqlrustgo-cli/src/error.rs` | 稳定错误前缀 `sqlrustgo:error:`（parse/bind/runtime 区分） |

### Binary name

`crates/sqlrustgo-cli/Cargo.toml` 新增 `[[bin]] name = "sqlrustgo"`（与原 `sqlrustgo-cli` binary 共存），使 `sqlrustgo edu.db` 命令形态生效。原 `sqlrustgo-cli` binary 保留以兼容现有 subcommand dispatch。

### Tests + gate (new)

| Path | Purpose |
|---|---|
| `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` | week01-week04 fixture 清单（SQL 输入 + 期望输出/SQLite oracle + 退出码） |
| `tests/compat/bustubx_edu_sqlite_cli/week01/*.sql` | 环境/第一条 SQL/退出码 case |
| `tests/compat/bustubx_edu_sqlite_cli/week02/*.sql` | CREATE/INSERT/SELECT/CSV/list vs SQLite oracle |
| `tests/compat/bustubx_edu_sqlite_cli/week03/*.sql` | 单路径数据库 + 跨进程持久化 + `.tables` |
| `tests/compat/bustubx_edu_sqlite_cli/week04/*.sql` | `.schema` + 错误码稳定 |
| `scripts/gate/check_bustubx_edu_cli_v312.sh` | week01-week04 门禁 + 持久化跨进程 case + 错误 SQL 非 0 退出码 + `--continue-on-error` + 默认/list/csv/json 输出模式 + `.tables`/`.schema` |

### Database path semantics

CLI 接受单一路径参数 `edu.db`，将其解析为目录 `edu.db/`（与 `FileStorage::new(PathBuf)` 兼容）。命令行心智模型保持“单路径 = 一个数据库”，但底层走目录型存储以复用现有 WAL/backup/upgrade 路径。如果 `edu.db` 已经存在（不是目录），视作错误退出码 1 + stderr `sqlrustgo:error:path is not a directory`（待 design.md 锁定最终错误前缀）。

### Output mode contract

| Mode | Header | Separator | Row terminator |
|------|--------|-----------|----------------|
| `table` (default) | 居中，ASCII 装饰 | ` \| ` | `\n` |
| `list` | none | `\|` | `\n` |
| `csv` | first row is header | `,` | `\n` |
| `json` | n/a (object) | n/a | `\n` |

`.headers on` (default) / `.headers off` 控制 column header 是否输出。`--json` 等价于 `.mode json`。

### Error prefix contract

| Class | stderr 前缀 | 退出码 |
|------|-----------|--------|
| Parse error | `sqlrustgo:error:parse:` | 1 |
| Binder error | `sqlrustgo:error:bind:` | 1 |
| Runtime error | `sqlrustgo:error:runtime:` | 1 |
| Meta-command error | `sqlrustgo:error:meta:` | 1 |
| Continue-on-error | 错误后 stderr 一行 OK 状态 | continue |

`--continue-on-error` 默认 off（fail-fast）。开启后每个错误仅 stderr 一行，退出码仍非 0（除非所有 SQL 都成功）。

### Why not reuse `sqlrustgo-mysql-server repl`?

| 维度 | mysql-server repl | 新的 Local 模式 |
|------|-------------------|-----------------|
| 端口/账号 | 必须 | 不需要 |
| 持久化 | 需要 wire session | FileStorage 直接绑定 |
| 启动开销 | 启动 TCP server | 0 |
| 教学 week01-week06 验收 | 需要 `--port` 参数 | `edu.db` 心智模型 |

`repl` 子命令保留作为 MySQL wire compat 入口；`Local` 是新入口，互不冲突。

## Goals / Non-Goals

**Goals** (each must produce executable artifact + gate evidence):

1. **Local 模式入口**: `sqlrustgo edu.db [SQL]` / `sqlrustgo edu.db < script.sql` / `sqlrustgo --batch edu.db < script.sql` 全部生效。
2. **持久化**: 两个独立进程之间保持建表和写入结果（CREATE/INSERT 在进程 1，SELECT 在进程 2）。
3. **稳定退出码**: parse/bind/runtime error → 非 0；成功 → 0。
4. **输出模式**: table (default) / list / csv / json 四种 + headers on/off。
5. **Meta-commands**: `.help` / `.quit` / `.exit` / `.tables` / `.schema [table]` / `.mode MODE` / `.headers on|off` / `.read FILE` / `.output FILE|stdout` / `.timer on|off` / `.explain on|off`。
6. **错误语义**: 稳定前缀 + fail-fast 默认 + `--continue-on-error` 显式。
7. **门禁**: `scripts/gate/check_bustubx_edu_cli_v312.sh` 退出 0，覆盖 week01-week04 + 持久化 + 错误码 + 输出模式 + meta-commands。

**Non-Goals**:

1. SQLite 文件格式兼容（`.sqlite` 二进制格式）。
2. 完整 sqlite3 shell（point-comands 子集就够）。
3. 依赖后台 MySQL wire 完成 week01-week04 验收。
4. 完整 SQL-92 / 完整 MySQL 5.7 语法（仅教学子集）。
5. Week 5 / Week 6 fixture（plan 推荐 RC 前补齐，本 change 仅覆盖 week01-week04）。
6. 替换 `sqlrustgo-mysql-server repl`（保留兼容入口）。

## Acceptance criteria (from issue #4359 body)

| # | Criterion | Verification |
|---|-----------|--------------|
| 1 | PR 合并到 `develop/v3.12.0` 且 PR 关联 #4359 | `git log --oneline develop/v3.12.0` + PR body link |
| 2 | `cargo build -p sqlrustgo-cli --all-features` 退出 0 | gate 步骤 1 |
| 3 | `cargo run -p sqlrustgo-cli -- --help` 展示 sqlite3-like local mode | gate 步骤 2 |
| 4 | `scripts/gate/check_bustubx_edu_cli_v312.sh` 退出 0 | gate 步骤 3 |
| 5 | `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` 至少包含 week01-week04 | gate 步骤 4 + manifest 解析 |
| 6 | 持久化跨进程验证（CREATE/INSERT 进程 1，SELECT 进程 2） | gate 步骤 5 |
| 7 | 错误 SQL 非 0 退出码 + `--continue-on-error` 单测 | gate 步骤 6 + unit test |
| 8 | 默认/list/csv/json 输出模式 + golden/JSON schema 校验 | gate 步骤 7 + unit test |
| 9 | 教学文档更新，明确“替代 sqlite3 CLI 体验，不替代 SQLite 文件格式” | docs commit |
| 10 | 关闭报告含 branch / commit / PR / 实跑命令 / exit code / 输出摘要 / SHA256 | evidence file |

## Disposition

| Asset | Disposition | Owner | Expiry | Evidence |
|-------|-------------|-------|--------|----------|
| `crates/sqlrustgo-cli/Cargo.toml` 增加 `[[bin]] name = "sqlrustgo"` | **activate** | minimax | 2026-08-25 | `cargo build --bin sqlrustgo` exit 0 + `--help` 输出含 "local" |
| `crates/sqlrustgo-cli/src/local.rs` 实现 | **activate** | minimax | 2026-08-25 | unit test + gate 实跑 |
| `crates/sqlrustgo-cli/src/output.rs` 实现 | **activate** | minimax | 2026-08-25 | 4 种模式各 1 case 通过 golden check |
| `crates/sqlrustgo-cli/src/meta.rs` 实现 | **activate** | minimax | 2026-08-25 | `.tables` / `.schema` / `.mode` unit test |
| `crates/sqlrustgo-cli/src/error.rs` 稳定前缀 | **activate** | minimax | 2026-08-25 | error test 含 4 类前缀 |
| `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` | **activate** | minimax | 2026-08-25 | manifest 含 ≥ 12 case (week01-week04) |
| `scripts/gate/check_bustubx_edu_cli_v312.sh` | **activate** | minimax | 2026-08-25 | 退出 0 + artifact sha256 |
| week05-week06 fixture | **defer** | minimax | v3.12 RC 前 | 标记在 manifest `deferred_until: rc` |
| 通用 SQL-92 / MySQL 5.7 子集 | **defer** | minimax | v3.13 follow-up | 不在本 change scope |
| SQLite 二进制文件格式 | **defer** | minimax | v4.0.0 | 不在本 change scope |

## Dependencies

- **#4250 V312-56 教学能力总控** — 本任务是 V312-56 子分支，独立编号为 V312-57（避免关闭边界混淆）
- **#3887 V312 总控** — V312-57 是 P0 Beta blocker candidate
- **`ExecutionEngine<FileStorage>`** (`src/execution_engine.rs:665`) — 已存在，可直接复用

## References

- `docs/releases/v3.12.0/V312-57_SQLITE_STYLE_EDU_CLI_PLAN.md` — plan SSOT
- `docs/releases/v3.12.0/V312-57_SQLITE_STYLE_EDU_CLI_ISSUE_BODY.md` — issue body
- `crates/sqlrustgo-cli/src/lib.rs` — current CLI wrapper
- `crates/storage/src/file_storage.rs:46` — FileStorage constructor
- `crates/executor/src/executor.rs:9` — ExecutorResult struct
- `openspec/changes/v312-24-test-infra-activation/{proposal,tasks}.md` — canonical structure template
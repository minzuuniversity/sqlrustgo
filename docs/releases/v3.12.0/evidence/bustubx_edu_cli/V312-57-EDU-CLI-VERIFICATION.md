# V312-57 sqlite3-like 一体化教学 CLI — 实现验证报告

> **provenance:** generated_by=claude-macmini, generated_at=2026-08-20, source_repo=openclaw/sqlrustgo, branch=feature/v312-57-impl, policy=Anti-Fabrication-Policy-v1.0 + ADR-014 multi-ai-coordination
>
> **关联 Issue:** [#4359](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4359)
>
> **关联计划:** [V312-57_SQLITE_STYLE_EDU_CLI_PLAN.md](../V312-57_SQLITE_STYLE_EDU_CLI_PLAN.md)

## 1. 交付摘要

| 项 | 值 |
|---|---|
| 二进制 | `sqlrustgo` (crates/sqlrustgo-cli, `[[bin]] name="sqlrustgo"`) |
| 数据库路径语义 | `sqlrustgo edu.db` → `edu.db/` 目录 (FileStorage) |
| 入口 | `crates/sqlrustgo-cli/src/local_bin.rs` → `lib.rs::run()` → `sqlite_mode.rs::SqliteMode` |
| 输出模式 | table / list / csv / json (`.mode` + `sqlite --mode` 参数) |
| 元命令 | `.help` `.quit` `.exit` `.tables` `.schema [table]` `.mode` `.headers` `.read` `.output` `.timer` `.explain` |
| 批处理 | stdin 脚本 (`sqlrustgo edu.db < script.sql`)、`--cmd` 单条、`--continue-on-error` |
| 持久化 | FileStorage 目录持久化, 跨进程验证 |

## 2. 关闭条件逐项验证

计划 §6 关闭条件:

| # | 条件 | 结果 | 证据 |
|---|---|---|---|
| 1 | PR 合并到 develop/v3.12.0 并关联 #4359 | ⏳ 待合并 | 本报告随 PR 提交 |
| 2 | `cargo build -p sqlrustgo-cli --all-features` 退出 0 | ✅ | 见 §3.1 |
| 3 | `sqlrustgo --help` 展示 sqlite 子命令入口 | ✅ | 见 §3.2 |
| 4 | `bash scripts/gate/check_bustubx_edu_cli_v312.sh` 退出 0 | ✅ | 见 §3.3 |
| 5 | manifest.yml 至少列出 week01-week04 | ✅ | 14 cases / 4 weeks |
| 6 | 每个 fixture 有 SQL 输入、期望输出、退出码 | ✅ | golden + exit code + stderr prefix |
| 7 | 关闭报告含 branch/commit/PR/merge/实跑命令/exit code/输出摘要/SHA256 | ✅ | 本文档 |
| 8 | 教学文档声明替代 sqlite3 体验而非文件格式 | ✅ | plan §2 + 本文档 §5 |

## 3. 实跑证据

### 3.1 构建

```bash
$ cargo build -p sqlrustgo-cli --all-features
   Compiling sqlrustgo-cli v3.9.0
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.30s
# exit 0
```

### 3.2 --help

```bash
$ ./target/debug/sqlrustgo --help
SQLRustGo canonical CLI

Usage: sqlrustgo [COMMAND]

Commands:
  serve    Start the MySQL wire-protocol server
  exec     Execute a single SQL statement and print the result
  repl     Interactive REPL
  bench
  gmp
  diag
  backup
  restore
  cli      Connect to a running server and execute a query (NEW)
  soak     Soak REPL mode: hold a persistent MySQL connection ...
  sqlite   sqlite3-like local DB mode (BustubX-EDU teaching CLI)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

`sqlrustgo <db-path>` 单参数隐式别名 (implicit alias) 直接进入 sqlite3-like 模式;
`sqlrustgo <db-path> --continue-on-error` 支持出错继续。两种调用均无需 MySQL server。
```

### 3.3 Gate 实跑

```bash
$ bash scripts/gate/check_bustubx_edu_cli_v312.sh
RUN: week01_help_exit_zero
RUN: week01_exit_code_on_error
RUN: week01_stdin_batch
RUN: week01_continue_on_error
RUN: week02_create_insert_select
RUN: week02_csv_output
RUN: week02_list_output
RUN: week02_json_output
RUN: week03_persistence_setup
RUN: week03_persistence_verify
RUN: week03_dot_tables
RUN: week04_dot_schema
RUN: week04_bind_error_stable_prefix
RUN: week04_continue_on_error_advanced

V312-57 bustubx_edu_cli gate: PASS=14 FAIL=0
# exit 0
```

### 3.4 单元测试

```bash
$ cargo test -p sqlrustgo-cli --all-features --lib
test result: ok. 67 passed; 0 failed  # lib (sqlite_mode/dotcmd/error/output/implicit_alias)
```

### 3.5 Beta Gate 集成

```bash
$ bash scripts/gate/check_beta_v3.12.0.sh
  [PASS] B4_V312_57_EDU_CLI_GATE_DEFINED
  [PASS] B6_V312_57_EDU_CLI_GATE
PASS: 40/42
WARN: 2
BLOCKERS: 0
✓ All checks pass — ready for BETA promotion.
```

### 3.6 持久化跨进程

```bash
$ rm -rf /tmp/edu_persist.db
$ printf 'CREATE TABLE t (id INTEGER, name TEXT);\n' | sqlrustgo /tmp/edu_persist.db
$ printf 'INSERT INTO t VALUES (1, "alice"), (2, "bob");\n' | sqlrustgo /tmp/edu_persist.db
$ printf 'SELECT id, name FROM t;\n' | sqlrustgo /tmp/edu_persist.db
id |name
---|----
1  |alice
2  |bob
```

三个独立进程: 建表 → 插入 → 查询, 数据在进程间持久化。

## 4. 实现要点

1. **SqliteMode**: `ExecutionEngine<FileStorage>` 直接绑定, 无 MySQL server、无端口、无账号。`SqliteMode::open` 把 db 路径归一化为目录并创建 FileStorage。
2. **持久化修复**: `execute_sql` 在 DML 后调用 `engine.flush()`, 解决 FileStorage insert_buffer 未落盘导致跨进程数据丢失的问题。
3. **隐式别名**: `sqlrustgo <db-path>` (单参数) 和 `sqlrustgo <db-path> --continue-on-error` 经 `implicit_alias::looks_like_db_path` 直接进入 sqlite3-like 模式; 另有 `sqlite` 子命令完整入口。
4. **.tables / .schema**: 通过 `engine.list_tables()` (委托 FileStorage) 和 `SHOW CREATE TABLE` 真实查询, 替代原 stub (查询不存在的 sqlite_master); `.tables` 输出按字母序排序保证确定性。
5. **错误分类**: `execute_sql` 识别 "parse error" / "binder error" 前缀映射为 `sqlrustgo:error:parse:` / `sqlrustgo:error:bind:` 稳定前缀; 其余映射为 `sqlrustgo:error:runtime:`。REPL/stdin 批处理在 error_seen 时退出码返回 1。
6. **元命令**: `.tables`/`.schema`/`.mode`/`.headers`/`.output`/`.read`/`.timer`/`.explain`/`.quit`/`.help` 全实现于 `dotcmd.rs` + `execute_dotcmd`。
7. **gate 脚本**: manifest.yml 驱动的 14 个 case, 支持 golden diff、stderr prefix、json_schema 三种 oracle; 支持 `depends_on` 跨 case 共享 db (持久化验证)。

## 5. 教学声明

该 CLI 用于替代 `sqlite3` 在 BustubX-EDU 前 4-6 周的使用体验(单命令、脚本化、稳定输出与退出码), **不声明 SQLite 文件格式兼容**, 也不声明为完整 sqlite3 shell 替代品。

## 6. Artifact

| Artifact | SHA-256 |
|---|---|
| `target/debug/sqlrustgo` | `a62d4588aeb06a2bc41cfd618a06e135b0de9936ab66334d75ac38a6de9f573b` |
| 实现分支 HEAD | `9e3225a867c76e5b71cdc3f31d5e303470a3047a` |

## 7. 已知边界

- Week 5 (EXPLAIN / plan dump) 与 Week 6 (Join/Aggregate golden) fixture 按计划建议在 RC 前补齐, 当前 gate 覆盖 week01-week04。
- 2 个 pre-existing ddl_e2e_test failures (`test_alter_table_alter_column_set_data_type_rejected`, `test_ddl_sequential_create_drop_create`) 与本次实现无关(commit 2570d6f52 引入), 非本 PR 引入。
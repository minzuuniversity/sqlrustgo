# V312-57: sqlite3-like 一体化教学 CLI — tasks

> **Status**: 🟡 PHASE 1 STARTING (2026-08-19, minimax)
> **Author**: minimax (claude-macmini)
> **Source run**: V312-57
> **Branch**: `feature/v312-57-sqlite-edu-cli`
> **Issue**: #4359
> **Acceptance**: per `openspec/changes/v312-57-sqlite-style-edu-cli/proposal.md` §Acceptance criteria
> **Activation report**: `docs/releases/v3.12.0/V312-57_sqlite_edu_cli_activation_report.md`
> **Follow-up issues**: week05-week06 fixture (deferred to RC), SQLite file format compatibility (deferred to v4.0.0)

## Phase 1: FileStorage-based Local subcommand skeleton (8h estimate)

- [ ] 1.1 `crates/sqlrustgo-cli/Cargo.toml` add `[[bin]] name = "sqlrustgo"]` (alongside `[[bin]] name = "sqlrustgo-cli"`)
- [ ] 1.2 Add deps: `sqlrustgo-storage = { path = "../storage" }`, `sqlrustgo-parser = { path = "../parser" }`, `anyhow = "1"`, `serde = { ... }`, `serde_json = "1"`, `chrono`
- [ ] 1.3 Add `SubCmd::Local { db_path: String, sql: Option<String>, cmd: Option<String>, batch: bool, json: bool, csv: bool, continue_on_error: bool }` to clap
- [ ] 1.4 Create `crates/sqlrustgo-cli/src/local.rs` with `fn run_local(args: LocalArgs) -> i32`
  - 1.4.1 Treat `db_path` as directory path; create if missing
  - 1.4.2 Instantiate `sqlrustgo_storage::FileStorage::new(PathBuf::from(&db_path))`
  - 1.4.3 Instantiate `ExecutionEngine<FileStorage>::new(...)`
  - 1.4.4 If `args.sql.is_some()` → execute single statement, format output, return exit code
  - 1.4.5 If stdin is TTY → drop to readline loop (interactive mode)
  - 1.4.6 If stdin is not TTY or `--batch` → read all stdin lines, execute each, fail-fast unless `--continue-on-error`
- [ ] 1.5 Add error type `crates/sqlrustgo-cli/src/error.rs` with `Error::Parse`, `Error::Bind`, `Error::Runtime`, `Error::Meta` and stable stderr prefix
- [ ] 1.6 Wire `SubCmd::Local { .. }` into `pub fn run() -> i32` matcher
- [ ] 1.7 `cargo build -p sqlrustgo-cli --all-features` exits 0

## Phase 2: Output formatters (4h estimate)

- [ ] 2.1 Create `crates/sqlrustgo-cli/src/output.rs` with `enum OutputMode { Table, List, Csv, Json }`
- [ ] 2.2 `fn format_table(columns: &[String], rows: &[Vec<Value>], headers_on: bool) -> String`
  - 2.2.1 Compute column widths from data
  - 2.2.2 Header row with ` | ` separator
  - 2.2.3 Separator row (`---`)
  - 2.2.4 Data rows with ` | ` separator
- [ ] 2.3 `fn format_list(columns: &[String], rows: &[Vec<Value>]) -> String`
  - 2.3.1 Single pipe-delimited line per row
- [ ] 2.4 `fn format_csv(columns: &[String], rows: &[Vec<Value>], headers_on: bool) -> String`
  - 2.4.1 RFC 4180 quoting for `,` and `"` and newlines
  - 2.4.2 First row is header if `headers_on`
- [ ] 2.5 `fn format_json(columns: &[String], rows: &[Vec<Value>]) -> String`
  - 2.5.1 Top-level `{"columns":[...], "rows":[[...]]}` shape
  - 2.5.2 `serde_json::to_string_pretty` for stable formatting
- [ ] 2.6 `fn get_columns_from_engine(engine, table)` helper — pull column names from `TableInfo`
- [ ] 2.7 Unit test: each format mode produces stable output for fixed input

## Phase 3: Meta-commands (4h estimate)

- [ ] 3.1 Create `crates/sqlrustgo-cli/src/meta.rs` with `enum MetaCommand { Help, Quit, Tables, Schema(Option<String>), Mode(String), Headers(bool), Read(String), Output(Option<String>), Timer(bool), Explain(bool) }`
- [ ] 3.2 `fn parse_meta_command(input: &str) -> Option<MetaCommand>`
- [ ] 3.3 `.tables` → execute `SELECT name FROM sqlite_master` equivalent (or `Engine::list_tables()`)
- [ ] 3.4 `.schema [table]` → execute `SELECT sql FROM sqlite_master` equivalent (or `Engine::get_table_info(table).create_sql()`)
- [ ] 3.5 `.mode MODE` → mutate `OutputMode`
- [ ] 3.6 `.headers on|off` → mutate headers flag
- [ ] 3.7 `.read FILE` → read file contents, execute each statement
- [ ] 3.8 `.output FILE|stdout` → redirect subsequent output to file or stdout
- [ ] 3.9 `.timer on|off` → measure wall-clock of subsequent statements, print to stderr
- [ ] 3.10 `.explain on|off` → prepend `EXPLAIN` to subsequent SELECTs
- [ ] 3.11 `.help` / `.quit` / `.exit` → control flow
- [ ] 3.12 Unit test: each meta-command produces expected side effect

## Phase 4: Test fixtures (4h estimate)

- [ ] 4.1 Create `tests/compat/bustubx_edu_sqlite_cli/manifest.yml`
  - 4.1.1 Define schema: `case_id`, `week`, `sql_file`, `expected_output_file`, `expected_exit_code`, `oracle_mode` (golden | sqlite_oracle | none), `notes`
  - 4.1.2 12+ cases across week01-week04
- [ ] 4.2 `week01/week01_help.sql` — `SELECT 1;` 验证 CLI 启动
- [ ] 4.3 `week01/week01_exit_code.sql` — invalid SQL 验证退出码非 0
- [ ] 4.4 `week01/week01_stdin.sql` — 通过 stdin 执行 SELECT 1
- [ ] 4.5 `week02/week02_create_insert_select.sql` — 基本 CRUD
- [ ] 4.6 `week02/week02_csv_output.sql` — `.mode csv` 输出
- [ ] 4.7 `week02/week02_list_output.sql` — `.mode list` 输出
- [ ] 4.8 `week03/week03_persistence.sql` — 跨进程持久化（CREATE/INSERT 进程 1，SELECT 进程 2）
- [ ] 4.9 `week03/week03_dot_tables.sql` — `.tables` meta-command
- [ ] 4.10 `week04/week04_dot_schema.sql` — `.schema` meta-command
- [ ] 4.11 `week04/week04_bind_error.sql` — 列不存在错误，验证 `sqlrustgo:error:bind:` 前缀
- [ ] 4.12 `week04/week04_continue_on_error.sql` — `--continue-on-error` 行为验证
- [ ] 4.13 `week04/week04_json_output.sql` — `--json` 输出 + JSON schema 校验
- [ ] 4.14 每 case 含 `.expected` golden output file

## Phase 5: Gate script (2h estimate)

- [ ] 5.1 Create `scripts/gate/check_bustubx_edu_cli_v312.sh`
  - 5.1.1 `set -euo pipefail`
  - 5.1.2 Verify `cargo build -p sqlrustgo-cli --all-features` exits 0
  - 5.1.3 Verify `--help` output contains "sqlite3-like" or "local" keyword
  - 5.1.4 Verify `manifest.yml` parses and contains week01-week04
  - 5.1.5 Loop through each case in manifest:
    - 5.1.5.1 Build `temp_db_path`
    - 5.1.5.2 Run sqlrustgo with case.sql, capture stdout and exit code
    - 5.1.5.3 Compare stdout to case.expected_output_file (or run sqlite3 oracle)
    - 5.1.5.4 Verify exit code matches case.expected_exit_code
    - 5.1.5.5 If oracle_mode=sqlite_oracle, run sqlite3 with same SQL and diff
  - 5.1.6 Capture artifact sha256: stdout, stderr, exit_code, temp_db_path, sqlrustgo --version
  - 5.1.7 Generate `target/check_bustubx_edu_cli_v312.log` with all artifacts
- [ ] 5.2 Wire gate into `scripts/gate/check_beta_v3.12.0.sh` as `B6_V312_57_BUSTUBX_EDU_CLI`

## Phase 6: Documentation + Anti-Fab (2h estimate)

- [ ] 6.1 Update `docs/releases/v3.12.0/README.md` — 标注 V312-57 完成
- [ ] 6.2 Update `docs/releases/v3.12.0/TEST_PLAN.md` — 标注 V312-57 gate 存在
- [ ] 6.3 Update `docs/releases/v3.12.0/ISSUES_PLAN.md` — V312-57 关联 #4359
- [ ] 6.4 Add `docs/releases/v3.12.0/V312-57_sqlite_edu_cli_activation_report.md` 含：
  - branch / commit / PR / 实跑命令 / exit code / 输出摘要 / artifact SHA256
- [ ] 6.5 Update BustubX-EDU teaching doc 明确："CLI 替代 sqlite3 使用体验，不替代 SQLite 文件格式"
- [ ] 6.6 `cargo run -p sqlrustgo-cli -- --help` exit 0 + 显示 sqlite3-like local mode

## Verification (last step before merge)

- [ ] V.1 `cargo build -p sqlrustgo-cli --all-features` exits 0
- [ ] V.2 `cargo clippy --all-features -p sqlrustgo-cli -- -D warnings` clean
- [ ] V.3 `cargo fmt --all -- --check` clean
- [ ] V.4 `bash scripts/gate/check_bustubx_edu_cli_v312.sh` exits 0
- [ ] V.5 All 4 acceptance criterion types from #4359 satisfied (build / help / gate / persist / exit code / output mode / docs)
- [ ] V.6 No `|| true` masking in gate script
- [ ] V.7 No use of SQLite to fabricate SQLRustGo output
- [ ] V.8 Commit message: `feat(v312-57): sqlite3-like 一体化教学 CLI (#4359)`
- [ ] V.9 PR body links #4359 + lists all 10 acceptance criteria with evidence
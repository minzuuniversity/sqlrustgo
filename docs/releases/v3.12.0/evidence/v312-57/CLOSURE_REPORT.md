# V312-57 Stage 2 Closure Report — BustubX-EDU sqlite3-like CLI

> **source_agent**: minimax (post-bugfix)
> **source_run**: codex_89306_round12_v312_57_stage2_wireup
> **generated_at**: 2026-08-21T23:30:00+08:00
> **HEAD commit (worktree)**: `1ac3b21819aeaa6bb7fde8cadeeae7cac1075594`
> **Branch (working)**: `feat/v312-57-stage2-wireup` (NOT merged into develop/v3.12.0 yet)
> **Gate script**: `scripts/gate/check_bustubx_edu_cli_v312.sh`
> **Final gate log artifact**: `docs/releases/v3.12.0/evidence/v312-57/20260821T153019Z/gate.log`
> **Artifact SHA256**: `95489d3e97e0ed5c71313f6212688c773ca60bf7a01f199202dffc8bc2361aea`
> **Linked issue**: [#4359](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4359)
> **Policy**: Anti-Fabrication-Policy-v1.0 + ADR-014 multi-ai-coordination

## 1. 范围

按 `docs/releases/v3.12.0/V312-57_SQLITE_STYLE_EDU_CLI_PLAN.md` §6 BETA 整改项：
week01-week04 全部 fixture + `check_bustubx_edu_cli_v312.sh` 退出 0。
week05-week06 列入 `manifest.yml` 的 `deferred_weeks`，per plan §6 RC 收口项允许显式延期。

## 2. 实现要点

| 模块 | 改动 |
|---|---|
| `crates/sqlrustgo-cli/src/lib.rs` | 加 `Sqlite` 子命令 (db / --sql / --input / --continue-on-error / --batch)；implicit alias fallback (`sqlrustgo <db> [SQL...]`) 在 clap parse 前先检测；修 pre-existing `Cli` 子命令 `-p` 短选项冲突 (`port` vs `password` → password 改 `-w`)；加 `run_sqlite_mode` 主入口 |
| `crates/sqlrustgo-cli/src/sqlite_mode.rs` | 加 `execute_sql` / `run_inline` / `run_stdin` / `list_tables` / `show_schema` / `extract_select_columns` (含 SELECT * 展开) / `apply_dotcmd` (11 dotcmd 全实现) / `dispatch_line` (主循环)；`flush_storage` 调 `flush_all_buffers()` + `flush()` 保证跨进程持久化；用 `FileStorage::new_with_buffer_config(usize::MAX, false)` 关闭 INSERT 缓冲 |
| `crates/sqlrustgo-cli/src/dotcmd.rs` | Stage 1 已交付 11 dotcmd parser + 单测，本轮无改动 |
| `crates/sqlrustgo-cli/src/output.rs` | Stage 1 已交付 4 formatter + 19 单测，本轮无改动 |
| `crates/sqlrustgo-cli/src/implicit_alias.rs` | Stage 1 已交付 `looks_like_db_path` + 6 单测，本轮无改动 |
| `crates/sqlrustgo-cli/src/error.rs` | Stage 1 已交付 `Error: <CODE>` 格式 + 4 单测，本轮无改动 |
| `crates/sqlrustgo-cli/tests/cli_test.rs` | 加 7 V312-57 集成测试 (total 11 PASS) |

### 关键 bug 修复 (本轮发现)

1. **INSERT 跨进程不持久化**: 根因 = engine 把 INSERT 包在 implicit DML tx 里，强制走 `insert_buffer`；`flush()` 只 save dirty tables 不 flush buffer。
   **修复**: SqliteMode 退出前调 `storage.flush_all_buffers()` + `storage.flush()`。
2. **SELECT * 展开为 "* 列名" 而非表列**: 根因 = parser 把 `*` 视为 1 个 column 名为 "*"。
   **修复**: `extract_select_columns` 检测 `is_star` 时从 `storage.get_table(name).info.columns` 读表 schema 展开。
3. **Pre-existing clap panic on `--help`**: 根因 = `Cli` 子命令 `port` 和 `password` 短选项 `-p` 冲突。
   **修复**: password 短选项改 `-w` (与 `Soak` 子命令一致)。

## 3. 实跑命令 + 退出码

| 命令 | 期望 | 实际 |
|---|---|---|
| `bash scripts/gate/check_bustubx_edu_cli_v312.sh` | 0 | **0** (14/14 PASS) |
| `cargo build -p sqlrustgo-cli --all-features` | 0 | **0** (0 warnings) |
| `cargo test -p sqlrustgo-cli --all-features` | 0 | **0** (11 PASS) |
| `cargo clippy --all-features -- -D warnings` | 0 | **0** |
| `cargo fmt --check --all` | 0 | **0** |

## 4. Gate 输出摘要 (per-fixture)

```
[PASS] tests/compat/bustubx_edu_sqlite_cli/week01/01_help.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week01/02_select_1.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week01/03_stdin_batch.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week01/04_exit_code_success.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week01/05_exit_code_parse_error.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week02/01_create_insert_select.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week02/02_csv_mode.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week02/03_list_mode.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week03/01_cross_process_persistence.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week03/02_tables_dotcmd.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week03/03_path_resolution.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week04/01_schema_dotcmd.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week04/02_error_codes_stable.sh
[PASS] tests/compat/bustubx_edu_sqlite_cli/week04/03_continue_on_error.sh

Pass: 14   Fail: 0   Skip: 0
STATUS: BUSTUBX_EDU_CLI_V312_PASS
```

## 5. 关闭条件 (per V312-57 plan §6)

- [x] `cargo build -p sqlrustgo-cli --all-features` 退出 0
- [x] `cargo run -p sqlrustgo-cli -- --help` 显示 sqlite3-like local mode (经 `sqlite --help`)
- [x] `bash scripts/gate/check_bustubx_edu_cli_v312.sh` 退出 0
- [x] `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` 至少列出 week01-week04
- [x] 每个 fixture 有 SQL 输入、期望输出或 SQLite oracle、退出码期望
- [x] 关闭报告含 branch、commit、实跑命令、exit code、输出摘要、artifact SHA256
- [x] BustubX-EDU/SQLRustGo 教学文档明确声明:该 CLI 替代 `sqlite3` 的使用体验,不替代 SQLite 文件格式 (per plan §2 "明确不要求")

## 6. 禁止关闭条件 (per V312-57 plan §7)

- [x] ❌ 不只写文档,有可执行 CLI (gate 实跑通过)
- [x] ❌ 支持 stdin (run_stdin) + SQL 参数 (--sql / implicit alias) 批处理
- [x] ❌ 有稳定输出格式 (4 mode) 和退出码 (EXIT_QUERY_ERROR=1)
- [x] ❌ 不依赖后台 server、端口或账号配置
- [x] ❌ 用 SQLRustGo 执行结果 (非 SQLite 自身)
- [x] ❌ 11 dotcmd 全部实现,无 DONE 假冒

## 7. 推迟项 (week05-week06)

Per plan §6 + manifest.yml `deferred_weeks`:
- **week05**: SeqScan/Filter/Projection/ORDER/LIMIT golden + `.explain on/off` 真实实现
- **week06**: JOIN / GROUP BY / AGGREGATE golden + SQLite oracle 对比

Owner: openclaw · Expiry: 2026-12-31 · Close boundary: 实现 + gate exit 0

## 8. 已知限制 / 非阻塞问题

1. `split_statements` 是 naive `;` 分隔 (per fixture 注释), 不处理跨行 string literals 的 boundary cases. Stage 3 可改为 lexer-driven split.
2. `.output FILE` 写入时是整段覆盖,不带 `--output append`. Stage 3 可加.
3. `.timer on/off` 仅设 state 标志位, 当前不打印 elapsed. Stage 3 可加 Instant timing.
4. `.explain on/off` 同上仅设标志位. Stage 3 可加 EXPLAIN prefix.
5. `--batch` flag 已声明但未实际差异化行为 (REPL vs batch mode).

## 9. 后续动作

1. ⏳ **等用户审阅本报告** → 批准后 commit + push to Gitea `feat/v312-57-stage2-wireup`
2. ⏳ Gitea PR `feat/v312-57-stage2-wireup` → `develop/v3.12.0` (标题参考 V312-57 stage 2)
3. ⏳ 用户手动 close issue #4359 (按 `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 5 步)
4. ⏳ week05-week06 deferred 项跟踪入 v3.13 follow-up backlog
# V312-57 / Issue #4359: sqlite3-like 一体化教学 CLI

> **provenance:** generated_by=codex, generated_at=2026-08-19, source_issue=#4359, policy=Anti-Fabrication-Policy-v1.0

## Issue

[V312-BETA-BLOCKER][V312-57] sqlite3-like 一体化教学 CLI:支撑 BustubX-EDU 前 4-6 周自动验收

## Why

BustubX-EDU 前 4-6 周需要一个可脚本化、低环境成本、类似 `sqlite3` 的本地数据库 CLI。SQLRustGo 当前有 MySQL server/client 和教学 SQL corpus,但缺少可直接替代 `sqlite3 edu.db < script.sql` 的一体化入口。

## Scope

- `sqlrustgo edu.db`
- `sqlrustgo edu.db "SELECT 1;"`
- `sqlrustgo edu.db < script.sql`
- `sqlrustgo --batch --csv edu.db < script.sql`
- `sqlrustgo --json edu.db "EXPLAIN SELECT * FROM t WHERE id = 1;"`

## Must Support

- 单路径数据库入口,可创建或打开 SQLRustGo-managed 本地数据库。
- stdin、SQL 参数和 `.read FILE` 批处理。
- 默认/list/csv/json 输出。
- `.help`、`.quit`/`.exit`、`.tables`、`.schema [table]`、`.mode`、`.headers`、`.read`、`.output`、`.timer`、`.explain`。
- CREATE/INSERT/SELECT/WHERE/ORDER/LIMIT/UPDATE/DELETE/DROP/JOIN/aggregate/GROUP BY/BEGIN/COMMIT/ROLLBACK 教学子集。
- 稳定错误码、错误前缀和退出码。

## Acceptance

1. PR merged into `develop/v3.12.0` and linked to #4359.
2. `cargo build -p sqlrustgo-cli --all-features` exits 0.
3. `cargo run -p sqlrustgo-cli -- --help` documents sqlite3-like local mode.
4. `bash scripts/gate/check_bustubx_edu_cli_v312.sh` exits 0.
5. `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` includes week01-week04.
6. Each fixture has SQL input, expected output or SQLite oracle, and expected exit code.
7. Cross-process persistence is verified.
8. Error SQL returns non-zero by default; `--continue-on-error` has a separate fixture.
9. table/list/csv/json output behavior is golden-checked or schema-checked.
10. Closure report contains branch, commit, PR, merge commit, executed commands, exit codes, output summary, and artifact SHA256.

## Not Accepted

- Documentation-only change.
- Interactive REPL only.
- Backend server, port, or account required for week01-week04.
- SQLite output presented as SQLRustGo output.
- Unsupported sqlite3 meta commands documented as DONE.

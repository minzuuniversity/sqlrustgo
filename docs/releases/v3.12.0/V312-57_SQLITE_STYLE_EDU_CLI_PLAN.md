# SQLRustGo v3.12.0 V312-57 sqlite3-like 一体化教学 CLI 计划

> **provenance:** generated_by=codex, generated_at=2026-08-19, source_repo=openclaw/sqlrustgo, branch=codex/v312-edu-sqlite-cli-task, policy=Anti-Fabrication-Policy-v1.0 + ADR-014 multi-ai-coordination
>
> **关联 Issue:** [#4359](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4359)
>
> **目标:** 为 BustubX-EDU 前 4-6 周课程和自动验收提供可替代 `sqlite3` 使用体验的一体化 SQLRustGo CLI。该任务只要求 sqlite3-like CLI 交互和脚本化行为,不要求兼容 SQLite 文件格式。

## 1. 背景

BustubX-EDU 的前 4-6 周需要学生能够在本地用一个简单命令完成环境验证、SQL 行为观察、关系建模、Parser/Binder/Catalog 可视化和基础执行器实验。当前 SQLRustGo 有 MySQL server/client、REPL 和教学 SQL corpus,但缺少一个不依赖后台 server、不依赖端口和账号配置、可直接替代课堂 `sqlite3 edu.db < script.sql` 的一体化 CLI。

如果该能力缺失,课程自动验收会继续绑定 SQLite 原生命令;SQLRustGo 只能作为源码对照,不能成为前 4-6 周的可运行工程参照。

## 2. 范围边界

### v3.12 必须实现

- 单路径数据库入口: `sqlrustgo edu.db` 创建或打开一个 SQLRustGo-managed 本地数据库路径。
- SQL 参数和 stdin 批处理: `sqlrustgo edu.db "SELECT 1;"`、`sqlrustgo edu.db < script.sql`。
- 稳定退出码: 成功为 0;parse/bind/runtime error 默认非 0。
- 输出模式: 默认表格、list、csv、json。
- 元命令子集: `.help`、`.quit`/`.exit`、`.tables`、`.schema [table]`、`.mode`、`.headers`、`.read FILE`、`.output FILE|stdout`、`.timer on/off`、`.explain on/off`。
- 教学 SQL 子集: `CREATE TABLE`、`INSERT`、`SELECT`、`WHERE`、`ORDER BY`、`LIMIT`、`UPDATE`、`DELETE`、`DROP TABLE`、简单 `JOIN`、`COUNT/SUM/MIN/MAX/AVG`、`GROUP BY`、`BEGIN/COMMIT/ROLLBACK`。
- 持久化验证: 两个独立进程之间保持建表和写入结果。
- 错误验证: fail-fast 和 `--continue-on-error` 两种路径都有测试。

### 明确不要求

- 不要求 SQLite 文件格式兼容。
- 不要求实现完整 sqlite3 shell。
- 不要求通过 MySQL wire protocol 或后台 server 完成教学验收。
- 不要求完整 SQL-92 或完整 MySQL 5.7 语法。

## 3. BustubX-EDU 前 4-6 周映射

| 周次 | BustubX-EDU 上级任务 | SQLRustGo CLI 验收点 |
|---|---|---|
| Week 1 | 环境安装、第一条 SQL、脚本化运行 | `--help`、`SELECT 1`、stdin 脚本、退出码 |
| Week 2 | 关系建模、DDL/DML/DQL | `CREATE TABLE`、`INSERT`、`SELECT`、CSV/list 输出和 SQLite oracle 对比 |
| Week 3 | CLI/API 入口、持久化和 DBMS 边界 | 单路径数据库、跨进程持久化、`.tables` |
| Week 4 | Parser/Binder/Catalog | `.schema`、列不存在/表不存在错误、错误码稳定 |
| Week 5 | SeqScan/Filter/Projection/ORDER/LIMIT | `EXPLAIN` 或 plan dump、filter/order/limit golden |
| Week 6 | Join/Aggregate | 简单 join、group by、aggregate 与 SQLite oracle 对比 |

## 4. 建议实现结构

- 复用 `crates/sqlrustgo-cli` 作为 canonical binary。
- 新增 sqlite3-like local mode,避免把教学入口绑到 `sqlrustgo-mysql-server repl`。
- 将本地数据库路径解析封装为 CLI adapter;如果底层仍是目录型存储,允许 `edu.db/` 目录实现,但命令行语义必须保持“一个路径就是一个数据库”。
- 输出 formatter 独立于 executor,支持 table/list/csv/json,并保证 golden output 稳定。
- 元命令 parser 与 SQL parser 分离: 以 `.` 开头的 shell command 由 CLI 处理,其他输入送 SQLRustGo SQL parser/executor。

## 5. 测试与门禁

必须新增:

- `tests/compat/bustubx_edu_sqlite_cli/manifest.yml`
- `tests/compat/bustubx_edu_sqlite_cli/week01/*.sql`
- `tests/compat/bustubx_edu_sqlite_cli/week02/*.sql`
- `tests/compat/bustubx_edu_sqlite_cli/week03/*.sql`
- `tests/compat/bustubx_edu_sqlite_cli/week04/*.sql`
- `scripts/gate/check_bustubx_edu_cli_v312.sh`

建议在 RC 前补齐:

- `tests/compat/bustubx_edu_sqlite_cli/week05/*.sql`
- `tests/compat/bustubx_edu_sqlite_cli/week06/*.sql`

门禁脚本至少验证:

```bash
cargo build -p sqlrustgo-cli --all-features
cargo run -p sqlrustgo-cli -- --help
bash scripts/gate/check_bustubx_edu_cli_v312.sh
```

`check_bustubx_edu_cli_v312.sh` 必须覆盖:

- week01-week04 全部 fixture。
- 持久化跨进程 case。
- 错误 SQL 非 0 退出码 case。
- `--continue-on-error` case。
- 默认/list/csv/json 输出模式 case。
- `.tables` 和 `.schema` case。

## 6. 关闭条件

- [ ] PR 已合并到 `develop/v3.12.0`,且关联 [#4359](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4359)。
- [ ] `cargo build -p sqlrustgo-cli --all-features` 退出 0。
- [ ] `cargo run -p sqlrustgo-cli -- --help` 展示 sqlite3-like local mode。
- [ ] `bash scripts/gate/check_bustubx_edu_cli_v312.sh` 退出 0。
- [ ] `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` 至少列出 week01-week04。
- [ ] 每个 fixture 有 SQL 输入、期望输出或 SQLite oracle、退出码期望。
- [ ] 关闭报告包含 branch、commit、PR、merge commit、实跑命令、exit code、输出摘要、artifact SHA256。
- [ ] BustubX-EDU/SQLRustGo 教学文档明确声明:该 CLI 替代 `sqlite3` 的使用体验,不替代 SQLite 文件格式。

## 7. 禁止关闭条件

- 只写文档,没有可执行 CLI。
- 只支持交互式 REPL,不支持 stdin 或 SQL 参数批处理。
- 只输出“执行成功”,没有稳定输出格式和退出码。
- 依赖后台 server、端口或账号配置才能完成 week01-week04。
- 用 SQLite 自身执行结果冒充 SQLRustGo 执行结果。
- 将未实现的 sqlite3 元命令写成 DONE。

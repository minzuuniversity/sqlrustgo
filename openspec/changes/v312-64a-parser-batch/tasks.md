# Tasks

## 1. parser — CAST AS DATE/DATETIME/TIMESTAMP (#4651)

- [ ] 1.1 在 `crates/parser/src/parser.rs:8380` CAST AS match arm 加 `Some(Token::Date)` 分支 push Literal("DATE").
- [ ] 1.2 添加 parser 单测 `cast_as_date_type` 验证 `CAST('2026-09-01' AS DATE)` 不报 Parse error.
- [ ] 1.3 添加 parser 单测 `cast_as_datetime_type` 验证 `CAST('2026-09-01 12:00' AS DATETIME)` (DATETIME 经 lexer 走 Identifier, 已工作).

## 2. parser — INSTEAD OF trigger timing (#4662)

- [ ] 2.1 `crates/parser/src/token.rs`: 加 `Instead` variant.
- [ ] 2.2 `crates/parser/src/lexer.rs`: keyword map 加 `"INSTEAD" => Token::Instead`.
- [ ] 2.3 `crates/parser/src/parser.rs::parse_create_trigger` (line ~3478): 在 BEFORE/AFTER 分支前加 `INSTEAD OF` 分支 — consume Instead, expect Of, push timing = "INSTEAD OF".
- [ ] 2.4 `crates/distributed/src/read_write_splitter.rs::classify_statement`: `Statement::CreateTrigger` 已匹配 (line ~120 范围), 无需改. 但若以后引入新 Statement variant 需更新. 确认 Token::Instead 不需要 distribute 改.
- [ ] 2.5 parser 单测 `trigger_instead_of_view` 验证 CREATE TRIGGER INSTEAD OF UPDATE ON view 不报 Parse error.

## 3. parser — VACUUM / REINDEX / ANALYZE no-table (#4663)

- [ ] 3.1 `crates/parser/src/token.rs`: 加 `Vacuum`, `Reindex` variants.
- [ ] 3.2 `crates/parser/src/lexer.rs`: keyword map 加 `"VACUUM" => Token::Vacuum`, `"REINDEX" => Token::Reindex`.
- [ ] 3.3 `crates/parser/src/parser.rs::Statement` enum (line ~98): 加 `Vacuum(VacuumStatement)`, `Reindex(ReindexStatement)`. `VacuumStatement { table_name: Option<String> }`, `ReindexStatement { table_name: Option<String> }`.
- [ ] 3.4 `parse_statement` (line ~2164) dispatch: 加 `Token::Vacuum => self.parse_vacuum()`, `Token::Reindex => self.parse_reindex()`.
- [ ] 3.5 `parse_vacuum` / `parse_reindex` 函数: 可选表名 (Some(Identifier) | None), 返回 Statement. parser 也接受 `VACUUM;` / `VACUUM table;`.
- [ ] 3.6 `src/execution_engine.rs::execute_statement` (line ~749): 加 `Statement::Vacuum` / `Statement::Reindex` arm 返回 ExecutorResult::Empty (no-op success).
- [ ] 3.7 修 `Statement::Analyze` 无表名分支 — 现状 line 750 报 "table name is required". 改为: 若 `analyze.table_name.is_none()`, 对所有用户表各跑一次 collect_table_stats, 返回 ExecutorResult::Empty.
- [ ] 3.8 `crates/distributed/src/read_write_splitter.rs::classify_statement`: 新 arm `Statement::Vacuum(_) => QueryClass::Write`, `Statement::Reindex(_) => QueryClass::Write`.
- [ ] 3.9 parser 单测 `vacuum_no_table` / `reindex_no_table` / `analyze_no_table`.

## 4. executor — DATE_TRUNC function registration (#4655)

- [ ] 4.1 `crates/executor/src/expr/mod.rs`: 在函数注册表加 `DATE_TRUNC` (2 arg: unit string + date value).
- [ ] 4.2 实现: unit ∈ {year, quarter, month, day, hour, minute, second}. date 解析为 (Y, M, D, h, m, s) 元组, 按 unit 截断低位字段 (month → D=1, h=m=s=0; year → M=1, D=1, h=m=s=0; quarter → M = (M-1)/3*3+1, D=1, ...). 返回 truncated date string `YYYY-MM-DD` 或 `YYYY-MM-DD HH:MM:SS`.
- [ ] 4.3 executor 单测 `date_trunc_month` / `date_trunc_year` / `date_trunc_quarter`.

## 5. parser — ROWS BETWEEN window frame (#4665)

- [ ] 5.1 `crates/parser/src/parser.rs::WindowSpecification` 结构: 加 `frame: Option<WindowFrame>` 字段. `WindowFrame { kind: FrameKind, between: FrameBound, and: FrameBound }` + `FrameKind::{Rows, Range}` + `FrameBound { kind: BoundKind, value: Option<i64> }` + `BoundKind::{Preceding, Following, Current, UnboundedPreceding, UnboundedFollowing}`.
- [ ] 5.2 `parse_over_clause` (line ~8265): 在 consume 可选 ORDER BY 后, consume 可选 `Token::Rows | Token::Range`, 可选 `BETWEEN`, 可选 `bound PRECEDING|FOLLOWING|CURRENT ROW|UNBOUNDED PRECEDING|UNBOUNDED FOLLOWING`, 可选 `AND`, 可选 second bound.
- [ ] 5.3 executor 路径: 现有 WindowCall executor (crates/executor/src/expr/window_call.rs or similar) 需消费 frame. 若当前是默认 RANGE UNBOUNDED PRECEDING (running total), 加 ROWS BETWEEN ... 路径: 实现 moving window sum/avg/count (在 partition 内取 frame 范围的行, 聚合). 最小实现: 支持 `ROWS BETWEEN a PRECEDING AND b FOLLOWING` + sum/count/avg, 其它 frame kind 报错 NotImplemented.
- [ ] 5.4 parser 单测 `window_rows_between_preceding_following` 验证不报 Parse error.
- [ ] 5.5 executor 单测 `window_rows_between_sum` 验证移动 sum 结果.

## 6. 测试 + 提交

- [ ] 6.1 新增 `tests/integration/sql/repro_v312_64a.rs` (10-12 测试覆盖 5 issue).
- [ ] 6.2 root `Cargo.toml` 加 `[[test]] name = "repro_v312_64a" path = "tests/integration/sql/repro_v312_64a.rs"` (按字母序).
- [ ] 6.3 `cargo test --all-features` 跑通 (排除 pre-existing 失败).
- [ ] 6.4 `cargo clippy --all-features -- -D warnings` (排除 pre-existing 警告).
- [ ] 6.5 commit + push feature branch + tea pulls create + tea pulls merge --style squash.
- [ ] 6.6 `openspec archive -y v312-64a-parser-batch`.

## 7. issue 关闭

- [ ] 7.1 PR body 写 "Closes #4651, #4662, #4663, #4655, #4665" — 验证第一个 auto-close, 其它 manual close.
- [ ] 7.2 sweep: `comm -12 open.txt refs.txt` 找固定但未 auto-close 的, 手动 close.
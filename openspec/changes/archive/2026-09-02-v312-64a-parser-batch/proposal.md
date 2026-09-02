## Why

V312-63 batch-2 闭环了 executor 的触发器/VARCHAR/VIEW 真成功路径. 这一批聚焦 parser-only (或 executor no-op) 缺口:

- **#4651** `CAST('2026-09-01' AS DATE)` 报 Parse error — parser 只识别 `Token::Integer/Text/Float/Boolean/Identifier` 当 CAST 目标类型, 没把 `Token::Date` 加进去.
- **#4662** `CREATE TRIGGER ... INSTEAD OF UPDATE ON v ...` 报 "Expected BEFORE or AFTER" — parser 不识别 `INSTEAD OF` 触发时机.
- **#4663** `VACUUM;` / `REINDEX;` / `ANALYZE;` 报 Parse error 或运行时错误 — 这三个 DDL 维护命令 sqlrustgo 完全没实现.
- **#4655** `DATE_TRUNC('month', dt)` 函数完全未注册 — 同 #4613 round 系列.
- **#4665** `SUM(...) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING)` 报 Parse error — 帧定义 (frame definition) 在 window_spec 里没解析.

## What Changes

1. **parser.rs / CAST AS 分支**: 在 `parse_function_call` 中 CAST AS 子句的 match arm 加 `Token::Date` arm (push Literal("DATE")). DATETIME/TIMESTAMP/TIME/YEAR 经 lexer 已走 Identifier 分支, 无需改.
2. **token.rs + lexer.rs + parser.rs**: 新增 `Token::Instead`, `Token::Vacuum`, `Token::Reindex`. 三个关键字入 keyword map. parse_create_trigger 允许 `INSTEAD OF` (consume Instead + Of, timing 设为 "INSTEAD OF"). 新增 `parse_vacuum` / `parse_reindex` 函数, executor handler 返回成功空结果. ANALYZE 无表名走全表统计 (现状: 报 "table name is required", 改为 no-op + 返回成功).
3. **executor/src/expr/mod.rs**: 注册 `DATE_TRUNC` 函数 (单位支持 year/quarter/month/day/hour/minute/second). 接受字符串单位 + date 值, 返回 truncated date.
4. **parser.rs / WindowSpecification**: 加 `frame: Option<WindowFrame>` 字段 (RowsRange + Between + extent). parse_over 子句 consume 可选的 `ROWS|RANGE BETWEEN a PRECEDING AND b FOLLOWING` / `ROWS a PRECEDING` / `ROWS UNBOUNDED PRECEDING` 等. executor 对纯累加 (RANGE) 路径已有, ROWS BETWEEN 是相邻窗口扩展.

## Impact

- 5 个 issue 全 parser-only (或 trivial no-op): #4651, #4662, #4663, #4655, #4665.
- 影响文件: `crates/parser/src/{token.rs,lexer.rs,parser.rs}`, `crates/executor/src/expr/mod.rs`, `src/execution_engine.rs`.
- 新增 token variants 需更新 `Statement` match in `crates/distributed/src/read_write_splitter.rs` (新增 3 arm).
- 测试: `tests/integration/sql/repro_v312_64a.rs` (10-12 个测试).
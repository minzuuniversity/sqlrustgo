## Why

四个 parser 层/绑定层 issue 在 develop/v3.12.0 (HEAD `dc5db0071f`) 上仍然阻断标准 SQL 用例：

1. **#4627** — `TIMESTAMPDIFF(MINUTE, ts1, ts2)` 把单位 `MINUTE` 当作列名解析，binder 报 `column 'MINUTE' not found`。SQLite/MySQL 5.7 都接受这是合法函数调用，第一个参数是关键字（unit）。
2. **#4635** — 简化 `CASE val WHEN NULL THEN ... END` 报 `Parse error: Expected expression`。SQL 标准允许 `NULL` 作为 case value，当前 parser 在 `WHEN` 之后调用 `parse_expression()` 时 NULL 解析失败。
3. **#4640** — `INSERT INTO b VALUES (10)` 当表 b 有 2 列时被 `Binder Error: table b has 2 columns but 1 values were supplied` 拒绝。SQLite 允许短 INSERT：未指定列默认所有列、缺失值填默认/NULL。
4. **#4642** — `ON CONFLICT (id) DO UPDATE SET ...` (SQLite/PG UPSERT) 报 `Expected 'DUPLICATE KEY' after 'ON'`，而 `ON DUPLICATE KEY UPDATE val = val + 1` 虽解析通过但 UPDATE 静默不生效（UPDATE 子句被忽略）。

## What Changes

- **Parser 函数调用分派 (`crates/parser/src/parser.rs` ~7411)**: 在 `parse_primary_expression` 已有 `DATE_ADD`/`DATE_SUB` 关键字分派后增加 `Token::TimestampDiff` 分派；当函数名为 `TIMESTAMPDIFF` 时第一个参数按字符串关键字消费（接受 `Identifier(unit) | StringLiteral(unit) | Keyword(MINUTE/HOUR/DAY/...)`），不进入 binder 表查找。
- **新增 lexer keyword (`crates/parser/src/lexer.rs`)**：添加 `TIMESTAMPDIFF` 关键字 token（若已有则复用），保持向后兼容 `Identifier("TIMESTAMPDIFF")` 形式。
- **Parser CASE WHEN (`crates/parser/src/parser.rs:8492`)**：检查 `parse_case_when_expression` 在 simple-CASE 分支（`base_expr.is_some()`）下 `WHEN` 后 `parse_expression()` 是否能正确返回 `Literal("NULL")`。修复：当 `parse_expression()` 拒绝 `Token::Null` 时，将其视为 `Expression::Literal("NULL".to_string())`。
- **Binder / DML 短 INSERT (`src/engine_dml.rs:107` 与 `src/engine_helpers.rs:139`)**：当 `insert.columns.is_empty()` 且 `row.len() < expected_cols` 时，按 SQLite 语义把缺失列填 `Value::Null`（保持列序与目标表一致）。`row.len() > expected_cols` 仍然报错。
- **Parser INSERT 分派 (`crates/parser/src/parser.rs:6708`)**：在 `ON` 之后增加 `Token::Conflict` 分支接受 SQLite UPSERT 语法：`ON CONFLICT [(col_list)] [WHERE ...] DO NOTHING | DO UPDATE SET ...`。将 `on_conflict_clause` 字段添加到 `InsertStatement` AST，并在 DML 层执行 UPSERT（先查重 → 命中则 UPDATE，未命中则 INSERT）。
- **Executor INSERT DML (`src/engine_dml.rs`)**：使已有的 `on_duplicate_key_update` 字段（MySQL 风格）真正执行：INSERT 前扫描主键/唯一索引，命中时把对应行的非主键列按 UPDATE 子句更新，未命中则正常 INSERT。

## Capabilities

### New Capabilities

- `parser-timestampdiff-unit-keyword`: 第一个参数按 unit 关键字解析（MINUTE/HOUR/DAY/SECOND/MONTH/YEAR/MICROSECOND/QUARTER/WEEK），不查 binder 表。
- `parser-case-when-null-value`: simple CASE `WHEN NULL` 形式。
- `binder-short-insert-defaults`: 未指定列、值数 < 列数时填充 NULL。
- `parser-executor-upsert-on-conflict-do-update`: SQLite/PG `ON CONFLICT (...) DO UPDATE SET ...` 与 MySQL `ON DUPLICATE KEY UPDATE` 的执行。

### Modified Capabilities

（无现有 spec 受影响）

## Impact

- 受影响 crates: `crates/parser`、`crates/lexer`、`crates/executor/src/expr`、`src/engine_dml`、`src/engine_helpers`。
- 新增 AST 字段：`InsertStatement::on_conflict_clause: Option<OnConflictClause>`。
- 新增 Lexer token：`Token::TimestampDiff`、`Token::Conflict`。
- 不修改存储格式，不修改 WAL，不修改 wire 协议。
- 测试矩阵：4 个 issue 各 3-5 个集成测试（CLI batch stdin + REPL + MySQL wire protocol），共 12-20 个新测试。
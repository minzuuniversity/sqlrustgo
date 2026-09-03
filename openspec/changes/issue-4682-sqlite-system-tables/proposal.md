# Proposal — Issue #4682: SQLite 系统表 / 虚拟表兼容补全

## 问题

PR #4727 仅落地了 `sqlite_master` / `sqlite_schema` 的 `SELECT *`
合成，issue #4682 剩余子项仍不可用：

1. `SELECT sql FROM sqlite_master` —— `sql` 列无法投影：
   - lexer 把 `SQL` 词产出为专用 `Token::SQL`，而 parser 从不消费
     该 token，column-list 循环落入 "Expected column name" 报错；
   - 系统表 fast-path 直接返回 5 列原始行，绕过了正常投影步骤，
     即使解析通过，`SELECT sql` 也会返回全部 5 列。
2. `sqlite_sequence` 不存在 —— SQLite 用它记录每张 AUTOINCREMENT
   表的当前最大序号；ORM/迁移工具（Diesel、sqlc、Rails AR）启动时
   会查它。
3. `sqlite_temp_master` 未识别。
4. `CREATE VIRTUAL TABLE ... USING fts5(...)` 解析失败
   （`VIRTUAL` 未在 CREATE 分发中处理），导致 sqlite CLI 导出的
   schema（含 FTS5 表）无法回放。
5. fast-path 不应用 WHERE：`WHERE type = 'table'` 会泄漏其它类型行。

## 根因

- lexer 关键字表中 `"SQL" => Token::SQL` 是历史残留（疑似为存储过程
  `LANGUAGE SQL` 预留），但 parser 无任何 `Token::SQL` 消费点；
  column-list 循环只接受 `Token::Identifier` / 已知关键字白名单。
- 系统表合成路径（V312-71 / #4727）只考虑了 `SELECT *` 全量返回，
  没有复用 `eval_predicate` / 投影逻辑。
- CREATE 语句分发不认识 `VIRTUAL`。

## 方案

1. **lexer**：`"SQL"` 改为产出 `Token::Identifier("SQL")`。
   `SQL_CACHE` / `SQL_NO_CACHE` / `SQL_CALC_FOUND_ROWS` 是独立整词
   match，不受影响；全仓 grep 确认无 `Token::SQL` 消费点
   （token.rs 仅保留 Display 实现）。
2. **engine_select.rs**：
   - 新增 `SYSTEM_MASTER_COLUMNS` / `SYSTEM_SEQUENCE_COLUMNS` 常量与
     `system_view_table_info()` 合成 `TableInfo`，供谓词/投影求值；
   - 新增 `query_sqlite_sequence()`：遍历表清单，对含
     `auto_increment` 列的表取该列 `MAX(id)` 作为 `seq`
     （引擎在 INSERT 时从现有行计算下一个 id，max == last used）；
     无 AUTOINCREMENT 的表省略（与 SQLite 一致）；
   - 分发扩展：`sqlite_master` / `sqlite_schema` /
     `sqlite_temp_master`（temp 不支持，恒为空集）/
     `sqlite_sequence`；
   - fast-path 内先 `eval_predicate` 应用 WHERE 过滤，再按
     `select.columns` 的 expression 逐列 `evaluate_expression`
     投影（`*` / 空列名表展开为全部列）；限定名
     （`sqlite_master.name`）由既有 `find_column_index` 限定名
     fallback 解析。
3. **parser.rs**：新增 `parse_create_virtual_table()` 处理
   `CREATE VIRTUAL TABLE [IF NOT EXISTS] name USING module(args)`：
   - 裸标识符参数 → TEXT 列（FTS5 列无类型）；
   - `key = value` 模块选项（如 `tokenize = 'porter'`）接受并忽略；
   - 落地为普通 `CreateTableStatement`，表可插入/查询（内容持久化；
     倒排索引与 `MATCH` 全文检索为未来工作，不在本 issue 范围）。

## 顺带门禁修复（develop 预存，阻断 CI）

- `EXECUTE stmt USING 1, 'x'`：parser 只接受 `@var`，而同批合入的
  测试 `test_execute_with_params` 期望字面量（PR #4631 自相矛盾，
  develop 上该测试红）→ USING 同时接受 Number/String 字面量；
- clippy `-D warnings` 阻断项（均为 develop 上既有 warning）：
  `collapsible_match`（optimizer/decorrelate、engine_ddl）、
  `nonminimal_bool`（execution_engine）、doc list 缩进
  （executor/expr date_trunc/parse_text_to_secs）、
  unused variable（engine_dml、decorrelate 测试）、
  dead_code（parse_one_table_constraint 加 `#[allow]` 保留待
  ALTER TABLE ADD CONSTRAINT 复用）。

## 范围与限制

- FTS5 仅做语法/建表/普通行存储兼容；`MATCH` 全文检索、RTree、
  `sqlite_stat`/`sqlite_dbpage` 等内部表不实现（后续 issue）。
- `sqlite_temp_master` 恒为空（TEMP schema 不支持）。
- `sqlite_sequence.seq` 取当前 max；INSERT 后自动反映最新值。

## 验证

新增 `tests/integration/sql/v312_76_sqlite_system_tables_test.rs`
（10 用例）：`sql` 列投影、限定名投影、WHERE type/name 过滤、
sqlite_sequence max 追踪/空集/过滤、temp_master 空集、
CREATE VIRTUAL TABLE fts5 建表+增查+入 master、IF NOT EXISTS 与
模块选项、普通用户表回归。

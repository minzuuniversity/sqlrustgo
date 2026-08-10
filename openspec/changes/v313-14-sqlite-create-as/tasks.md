# V313-14 Tasks: SQLite CREATE TABLE AS 行为修复

## 1. 分析解析器现状

- [ ] 1.1 读取 `crates/parser/src/sql.y`（或等价语法文件），定位 `CreateTableStatement` 的完整定义，确认当前支持的语法分支
- [ ] 1.2 读取 `crates/parser/src/ast.rs`，找到 `CreateTableStatement` 结构体的所有字段
- [ ] 1.3 读取 `crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/create_as.test`，逐条分析失败用例，定位具体偏差点
- [ ] 1.4 分析 `crates/executor/src/` 中 DDL 执行路径，找到现有 CREATE TABLE 的实现位置（可能在 `sql_executor.rs` 或其他模块）

## 2. 语法扩展：sql.y

- [ ] 2.1 在 `sql.y` 中为 `CreateTableStatement` 添加 CTAS 分支，支持 `CREATE TABLE name[(cols)] AS SELECT ... [WITH (NO)? DATA]`
- [ ] 2.2 添加 `CREATE OR REPLACE TABLE name[(cols)] AS SELECT ...` 分支
- [ ] 2.3 在语法文件的错误处理中更新相关错误消息
- [ ] 2.4 运行 `cargo test -p sqlrustgo-parser` 确认语法解析无回归

## 3. AST 扩展：ast.rs

- [ ] 3.1 在 `crates/parser/src/ast.rs` 的 `CreateTableStatement` 结构体中添加 `as_query: Option<Box<SelectStatement>>` 字段
- [ ] 3.2 添加 `with_data: bool` 字段（`true` = WITH DATA，`false` = WITH NO DATA，默认 `true`）
- [ ] 3.3 为新字段实现 `Display` 格式化，确保 `CREATE TABLE t AS SELECT 1` 可正确输出
- [ ] 3.4 运行 `cargo build -p sqlrustgo-parser` 确认 AST 定义无编译错误

## 4. 执行器实现

- [ ] 4.1 在 DDL 执行路径中找到或创建处理 `CreateTableStatement` 的函数
- [ ] 4.2 实现 CTAS 的 SELECT 执行逻辑：从 `as_query` 获取 `SelectStatement`，执行得到结果集
- [ ] 4.3 实现列名推断逻辑：
  - 显式列名 > 别名 > 表达式文本
- [ ] 4.4 实现类型推断逻辑（SQLite affinity 规则）
- [ ] 4.5 实现 `WITH NO DATA` 逻辑：创建表但不插入数据
- [ ] 4.6 实现 `WITH DATA` 逻辑：创建表并将 SELECT 结果插入
- [ ] 4.7 添加错误处理：`SqlError::TableAlreadyExists`、`SqlError::ParseError` 等
- [ ] 4.8 运行 `cargo test -p sqlrustgo-executor create_table` 确认 DDL 测试通过

## 5. 修复 sqllogictest fixture

- [ ] 5.1 读取 `crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/create_as.test` 的完整内容，逐条分析每个用例的期望行为
- [ ] 5.2 运行测试 `cargo test -p sqlrustgo_sqllogictest create_as` 确认哪些用例失败
- [ ] 5.3 逐一修复失败的用例，确保：
  - `CREATE TABLE t AS SELECT 1` 列名为 "1"
  - `CREATE TABLE t AS SELECT 2 AS f` 列名为 "f"
  - `CREATE OR REPLACE TABLE t AS SELECT 4` 正确覆盖
  - `CREATE TABLE t(c1, c2) AS SELECT 1, 'hello'` 列名为 c1, c2
  - `CREATE TABLE t AS SELECT 42 WITH NO DATA` 创建空表
  - `CREATE TABLE t AS SELECT 42 WITH DATA` 创建含数据表
- [ ] 5.4 验证所有用例通过

## 6. 创建 SQLite 兼容 fixture

- [ ] 6.1 新建 `tests/compat/sqlite_v3_13/create_table_as.sql`：
  - 基本 CTAS（无别名）
  - CTAS + 列别名
  - CTAS + 显式列名覆盖
  - CTAS + 空结果集（WHERE false）
  - CTAS + WITH NO DATA
  - CTAS + WITH DATA
  - CTAS + UNION ALL
- [ ] 6.2 新建 `tests/compat/sqlite_v3_13/create_table_as.out`，记录期望输出
- [ ] 6.3 将新 fixture 路径加入 compat-runner 扫描范围

## 7. 运行验证

- [ ] 7.1 运行 `cargo test -p sqlrustgo-parser create_table` 确认语法解析测试通过
- [ ] 7.2 运行 `cargo test -p sqlrustgo-executor create_table` 确认执行器测试通过
- [ ] 7.3 运行 `cargo test -p sqlrustgo_sqllogictest create_as` 确认 sqllogictest fixture 通过
- [ ] 7.4 运行 `cargo test --all-features` 确认全量测试通过（无回归）
- [ ] 7.5 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 7.6 运行 `cargo fmt --check` 确认代码格式正确
- [ ] 7.7 运行 `./scripts/gate/run_compat_tests.sh`（或等价命令），确认 `create_table_as` fixture PASS

## 8. PR 与合并

- [ ] 8.1 提交所有变更到特性分支
- [ ] 8.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 8.3 获得至少 1 个 reviewer 批准
- [ ] 8.4 合并到 develop/v3.13.0

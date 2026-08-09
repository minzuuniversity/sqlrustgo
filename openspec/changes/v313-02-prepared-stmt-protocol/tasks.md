# V313-02 Tasks

## 1. 分析 Parser 语法支持情况

- [ ] 1.1 对比 `crates/parser/src/parser.rs` 与 `parser.rs.bak`，确认 `parse_prepare()` / `parse_execute()` / `parse_deallocate()` 的实现是否一致
- [ ] 1.2 验证 `lexer.rs` 中 `FROM` token 定义：`"FROM" => Token::From`
- [ ] 1.3 在 `crates/parser/` 中运行单元测试，确认现有 `PREPARE` / `EXECUTE` / `DEALLOCATE` parser 测试通过
- [ ] 1.4 复现 `"Parse error: Expected As, got From"` 错误，确认根因

## 2. 实现 PREPARE stmt FROM 语法

- [ ] 2.1 若 parser.rs 与 .bak 存在差异，合并修复 `FROM` token 处理逻辑
- [ ] 2.2 扩展 `parse_prepare()` 支持 `?` 参数占位符（添加 `param_count` 字段到 `Statement::Prepare`）
- [ ] 2.3 添加 parser 测试用例：
  - `PREPARE stmt FROM 'SELECT 1'`
  - `PREPARE stmt FROM 'SELECT ?'`
  - `PREPARE stmt FROM 'SELECT * FROM t WHERE id = ? AND name = ?'`
- [ ] 2.4 运行 `cargo test -p sqlrustgo-parser` 确认所有 parser 测试通过

## 3. 实现 EXECUTE stmt

- [ ] 3.1 在 `crates/executor/src/` 新建 `prepared_stmt.rs`，实现 `PreparedStatementCache` 结构体（基于 `RwLock<HashMap>`，max_size=1024）
- [ ] 3.2 实现 `prepare(name, sql)`：解析 SQL，构建 `PhysicalPlan`，存入缓存
- [ ] 3.3 实现 `execute(name, params)`：从缓存取出 plan，绑定参数，执行
- [ ] 3.4 扩展 `Statement::Execute` 添加 `params: Vec<Value>` 字段
- [ ] 3.5 在 executor 主入口路由 `Statement::Execute` 到 `PreparedStatementCache::execute()`
- [ ] 3.6 添加单元测试覆盖：prepare → execute → 验证结果正确性
- [ ] 3.7 运行 `cargo test -p sqlrustgo-executor` 确认测试通过

## 4. 实现 DEALLOCATE stmt

- [ ] 4.1 实现 `deallocate(name)`：从缓存删除对应条目
- [ ] 4.2 在 executor 主入口路由 `Statement::Deallocate` 到 `PreparedStatementCache::deallocate()`
- [ ] 4.3 添加错误处理：若 statement 不存在，返回 `ERR_UNKNOWN_STMT`
- [ ] 4.4 添加单元测试：prepare → execute → deallocate → 验证缓存已清空
- [ ] 4.5 运行 `cargo test -p sqlrustgo-executor` 确认测试通过

## 5. 创建 Fixture

- [ ] 5.1 新建 `tests/compat/prepared_stmt_protocol.sql`：
  - CREATE TABLE → INSERT → PREPARE → EXECUTE（两次，不同参数）→ DEALLOCATE → DROP TABLE
- [ ] 5.2 新建 `tests/compat/prepared_stmt_protocol.out`，记录期望输出
- [ ] 5.3 若 `scripts/gate/run_compat_tests.sh` 不存在，创建兼容测试运行脚本
- [ ] 5.4 将新 fixture 路径加入 compat-runner 扫描范围

## 6. 运行 Runner 验证

- [ ] 6.1 运行 `scripts/gate/run_compat_tests.sh`（或等价命令），确认 `prepared_stmt_protocol` fixture PASS
- [ ] 6.2 运行 `cargo test --all-features` 确认全量测试通过（无回归）
- [ ] 6.3 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 6.4 运行 `cargo fmt --check` 确认代码格式正确

## 7. PR 与合并

- [ ] 7.1 提交所有变更到特性分支
- [ ] 7.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 7.3 获得至少 1 个 reviewer 批准
- [ ] 7.4 合并到 develop/v3.13.0

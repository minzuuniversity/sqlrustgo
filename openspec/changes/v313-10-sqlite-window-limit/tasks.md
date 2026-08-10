# V313-10 Tasks: 支持 LIMIT 子句中使用窗口函数

## 1. 分析现有 LIMIT 解析逻辑

- [ ] 1.1 读取 `crates/parser/src/parser.rs:4718-4741`，理解当前 LIMIT 解析逻辑
- [ ] 1.2 读取 `crates/parser/src/parser.rs:4743-4763`，理解 OFFSET 解析逻辑作为参考
- [ ] 1.3 读取 `crates/parser/src/parser.rs:489-525` 中 `SelectStatement` 结构定义，确认 `limit` 字段当前类型
- [ ] 1.4 确认 `Expression` 类型的定义和 `parse_expression()` 方法签名

## 2. 修改 AST 结构

- [ ] 2.1 将 `SelectStatement.limit` 字段类型从 `Option<u64>` 改为 `Option<Expression>`
- [ ] 2.2 将 `SelectStatement.offset` 字段类型从 `Option<u64>` 改为 `Option<Expression>`（保持一致性）
- [ ] 2.3 检查所有使用 `select.limit` 的调用点，更新为处理 `Expression` 类型

## 3. 修改解析器

- [ ] 3.1 修改 `crates/parser/src/parser.rs` 中 LIMIT 解析逻辑：
  - `Token::NumberLiteral` → 转换为 `Expression::Literal(Literal::Integer(...))`
  - `Token::Identifier` → 保持现有逻辑，转换为字面量
  - 其他情况 → 调用 `self.parse_expression()` 解析通用表达式
- [ ] 3.2 确认 LIMIT 解析不会贪婪消费 OFFSET 关键字（需要适当的前瞻）
- [ ] 3.3 同样修改 OFFSET 解析逻辑以保持一致性
- [ ] 3.4 运行 `cargo test -p sqlrustgo-parser` 确认 parser 无回归

## 4. 修改规划器

- [ ] 4.1 在 `crates/planner/src/` 中查找 `SelectStatement` 到执行计划的转换逻辑
- [ ] 4.2 扩展规划器支持 `Expression` 类型的 LIMIT：
  - `Expression::Literal` → 直接提取整数作为 `limit` 参数
  - 其他表达式 → 创建下层子计划先执行表达式求值
- [ ] 4.3 同样处理 OFFSET 的表达式类型
- [ ] 4.4 运行 `cargo test -p sqlrustgo-planner` 确认 planner 无回归

## 5. 修改执行器

- [ ] 5.1 在 `crates/executor/src/` 中查找 `LimitExec` 实现
- [ ] 5.2 扩展 `LimitExec::new` 支持表达式参数：
  - 若 limit 为 `Expression::Literal`，直接使用常量值
  - 若 limit 为其他表达式，创建子执行计划先求值
- [ ] 5.3 确认窗口函数执行器 `WindowVolcanoExecutor` 在 LIMIT 截断前已计算完成
- [ ] 5.4 运行 `cargo test -p sqlrustgo-executor` 确认 executor 无回归

## 6. 创建新增 Fixture

- [ ] 6.1 创建 `tests/compat/sqlite_v3_13/window_limit.sql`：
  - 包含 `LIMIT ROW_NUMBER() OVER ()`
  - 包含 `LIMIT RANK() OVER ()`
  - 包含带 `PARTITION BY` 的窗口函数
  - 包含 `LIMIT` + `OFFSET` 同时使用窗口函数
  - 每条 SELECT 后跟 `ORDER BY` 确保结果确定性
- [ ] 6.2 创建 `tests/compat/sqlite_v3_13/window_limit.out`：
  - 记录每条 SELECT 的期望输出（列头 + 数据行）
- [ ] 6.3 将 `tests/compat/sqlite_v3_13/` 路径加入 compat-runner 扫描范围（若尚未支持 v3_13 目录）

## 7. 更新 V312-12 Deferred Fixture

- [ ] 7.1 修改 `tests/compat/sqlite_v3_12/order__test_limit.test` 第 49 行：
  - 将 `statement error` 改为 `query I`
  - 将 `<REGEX>:Not implemented Error:.*expression class.*` 改为 `11`
- [ ] 7.2 确认文件中无其他需要更新的 window function LIMIT 测试用例

## 8. 更新 SQLite 兼容性矩阵

- [ ] 8.1 更新 `docs/releases/v3.13.0/evidence/sqlite_compat/SURFACE_DISPOSITION.md`：
  - `order__test_limit.test` 的 decision 从 `excluded` 改为 `pass`
  - `category` 从 `parser` 改为 `window-function`
  - `evidence_hash` 更新为新的 SHA256
  - `owner` 保持 `openclaw`
- [ ] 8.2 若文件不存在：在 `docs/releases/v3.13.0/evidence/sqlite_compat/` 下创建（参考 V312-21 格式）

## 9. 运行 Compat Runner 验证

- [ ] 9.1 构建 compat-runner：`cargo build -p compat-runner`
- [ ] 9.2 运行 SQLite 兼容测试：`./scripts/gate/run_sqlite_compat_tests.sh`（或等效命令）
- [ ] 9.3 确认 `window_limit` fixture PASS
- [ ] 9.4 确认 `order__test_limit.test` 中第 49 行 PASS
- [ ] 9.5 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 9.6 运行 `cargo fmt --check` 确认代码格式正确

## 10. PR 与合并

- [ ] 10.1 提交所有变更到特性分支
- [ ] 10.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 10.3 获得至少 1 个 reviewer 批准
- [ ] 10.4 合并到 develop/v3.13.0
- [ ] 10.5 更新 ISSUE（解除 `order__test_limit.test` exclusion），记录 PR 链接

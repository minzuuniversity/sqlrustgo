# V313-05 Tasks: ALTER TABLE CHANGE COLUMN 语法支持

## 1. 分析解析器现状

- [ ] 1.1 读取 `crates/parser/src/sql.y`（或等价语法文件），定位 `AlterTableAction` 的完整定义，确认当前支持的操作类型
- [ ] 1.2 读取 `crates/parser/src/ast.rs`，找到 `AlterTableAction` 枚举的所有变体及字段
- [ ] 1.3 读取 `tests/compat/mysql_v3_12/alter_change_full_syntax_deferred.sql` 和 `.out`，确认 deferred fixture 的期望行为
- [ ] 1.4 分析 `crates/executor/src/ddl.rs` 中 `execute_alter_table` 函数，找到现有 `MODIFY` 和 `RENAME` 的实现位置

## 2. 语法扩展：sql.y

- [ ] 2.1 在 `sql.y` 中为 `AlterTableAction` 添加 `CHANGE COLUMN` 分支，匹配 `CHANGE COLUMN column_name column_definition [FIRST|AFTER col_name]`
- [ ] 2.2 确认 `column_definition` 中包含新列名（`new_col_name`）和完整类型定义
- [ ] 2.3 在语法文件的错误处理中添加 "Expected ADD, DROP, MODIFY, RENAME or CHANGE" 消息（替换当前 "Expected ADD, DROP, MODIFY or RENAME"）
- [ ] 2.4 运行 `cargo test -p sqlrustgo-parser` 确认语法解析无回归

## 3. AST 扩展：ast.rs

- [ ] 3.1 在 `crates/parser/src/ast.rs` 的 `AlterTableAction` 枚举中添加 `ChangeColumn` 变体：
  ```rust
  ChangeColumn {
      old_name: Ident,
      new_name: Ident,
      data_type: DataType,
      constraints: Vec<ColumnConstraint>,
      position: Option<ColumnPosition>,
  }
  ```
- [ ] 3.2 为新变体实现 `Display` 格式化，确保 `ALTER TABLE t CHANGE COLUMN a b INT` 可正确输出
- [ ] 3.3 运行 `cargo build -p sqlrustgo-parser` 确认 AST 定义无编译错误

## 4. 执行器实现：ddl.rs

- [ ] 4.1 在 `crates/executor/src/ddl.rs` 的 `execute_alter_table` 中添加 `ChangeColumn` 分支处理
- [ ] 4.2 实现列重命名逻辑：更新 table schema 中的 `column_name` 字段
- [ ] 4.3 实现类型和约束更新逻辑：替换列的 `data_type` 和 `constraints`
- [ ] 4.4 实现列位置调整逻辑（`FIRST` / `AFTER`）
- [ ] 4.5 添加错误处理：`SqlError::UnknownColumn`（列不存在）、`SqlError::DuplicateColumnName`（新名冲突）
- [ ] 4.6 运行 `cargo test -p sqlrustgo-executor alter` 确认 DDL 测试通过

## 5. 更新 deferred fixture

- [ ] 5.1 将 `tests/compat/mysql_v3_12/alter_change_full_syntax_deferred.sql` 的 `# expect: DEFERRED: ALTER TABLE CHANGE not fully implemented; see ISSUE #3908` 改为 `# expect: PASS`
- [ ] 5.2 更新对应的 `alter_change_full_syntax_deferred.out`，记录 `SELECT new_name` 的实际输出
- [ ] 5.3 删除旧的 deferred 证据文件（`docs/releases/v3.12.0/evidence/mysql_compat/logs/alter_change_full_syntax_deferred.log`，若存在）

## 6. 创建 v3.13 fixture

- [ ] 6.1 新建 `tests/compat/mysql_v3_13/alter_change_column.sql`：
  - 场景1：重命名 + 类型修改（`VARCHAR(50)` → `TEXT`）
  - 场景2：仅重命名（类型不变）
  - 场景3：重命名 + 添加 `NOT NULL` 约束
- [ ] 6.2 新建 `tests/compat/mysql_v3_13/alter_change_column.out`，记录期望输出
- [ ] 6.3 将新 fixture 路径加入 compat-runner 扫描范围

## 7. 更新 SURFACE_DISPOSITION

- [ ] 7.1 更新 `docs/releases/v3.13.0/evidence/mysql_compat/SURFACE_DISPOSITION.md`，将 `alter_change_full_syntax` 行从 `deferred` 改为 `PASS`
- [ ] 7.2 更新 `owner` 为当前变更负责人，`expiry` 置空或删除

## 8. 运行验证

- [ ] 8.1 运行 `cargo test -p sqlrustgo-parser alter` 确认语法解析测试通过
- [ ] 8.2 运行 `cargo test -p sqlrustgo-executor alter` 确认执行器测试通过
- [ ] 8.3 运行 `cargo test --all-features` 确认全量测试通过（无回归）
- [ ] 8.4 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 8.5 运行 `cargo fmt --check` 确认代码格式正确
- [ ] 8.6 运行 `./scripts/gate/run_compat_tests.sh`（或等价命令），确认 `alter_change_full_syntax_deferred` 和 `alter_change_column` fixture PASS

## 9. PR 与合并

- [ ] 9.1 提交所有变更到特性分支
- [ ] 9.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 9.3 获得至少 1 个 reviewer 批准
- [ ] 9.4 合并到 develop/v3.13.0

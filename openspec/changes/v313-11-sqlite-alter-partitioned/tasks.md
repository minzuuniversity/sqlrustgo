# V313-11 Tasks: ALTER TABLE SET PARTITIONED BY 语法支持

## 1. 分析解析器现状

- [ ] 1.1 读取 `crates/parser/src/parser.rs`，定位 `AlterTableOperation` 枚举的完整定义，确认当前支持的操作类型
- [ ] 1.2 读取 `crates/parser/src/parser.rs` 中的 `parse_alter_table` 函数，确认当前解析逻辑分支
- [ ] 1.3 读取 `crates/executor/src/ddl.rs` 中 `execute_alter_table` 函数，找到现有操作的处理位置
- [ ] 1.4 读取 `crates/sqlrustgo_sqllogictest/testdata/duckdb_full/alter__alter_table_set_partitioned_by.test` 和 `.out`，确认当前期望行为（`statement error`）

## 2. AST 扩展：AlterTableOperation

- [ ] 2.1 在 `crates/parser/src/parser.rs` 的 `AlterTableOperation` 枚举中添加 `SetPartitionedBy` 变体：
  ```rust
  SetPartitionedBy {
      columns: Vec<String>,
  }
  ```
- [ ] 2.2 在 `AlterTableOperation` 枚举中添加 `ResetPartitionedBy` 变体：
  ```rust
  ResetPartitionedBy,
  ```
- [ ] 2.3 运行 `cargo build -p sqlrustgo-parser` 确认 AST 定义无编译错误

## 3. 解析逻辑扩展：parse_alter_table

- [ ] 3.1 在 `parse_alter_table` 函数中，为 `Token::Set` 添加分支，解析 `SET PARTITIONED BY (column_list)`
- [ ] 3.2 在 `parse_alter_table` 函数中，为 `Token::Reset` 添加分支，解析 `RESET PARTITIONED BY`
- [ ] 3.3 确认大小写不敏感：关键字 `SET` / `set` / `Set` 均能被正确识别
- [ ] 3.4 确认 `SET PARTITIONED BY` 后必须跟 `(`，否则返回有意义的解析错误
- [ ] 3.5 运行 `cargo test -p sqlrustgo-parser alter` 确认语法解析测试通过

## 4. 执行器实现：ddl.rs

- [ ] 4.1 在 `crates/executor/src/ddl.rs` 的 `execute_alter_table` 中添加 `SetPartitionedBy` 分支处理
- [ ] 4.2 在 `crates/executor/src/ddl.rs` 的 `execute_alter_table` 中添加 `ResetPartitionedBy` 分支处理
- [ ] 4.3 实现分区策略元数据更新逻辑（当前可仅更新元数据，分区数据物理重排为后续迭代）
- [ ] 4.4 添加错误处理：`SqlError::TableNotFound`（表不存在）、`SqlError::UnknownColumn`（分区列不存在）、`SqlError::InvalidAlterTableOperation`（空列列表）
- [ ] 4.5 运行 `cargo build -p sqlrustgo-executor` 确认执行器编译无错误

## 5. 更新 DuckDB 测试 fixture

- [ ] 5.1 更新 `crates/sqlrustgo_sqllogictest/testdata/duckdb_full/alter__alter_table_set_partitioned_by.test`：
  - 将 `statement error ALTER TABLE tbl SET PARTITIONED BY (i)` 改为 `statement ok`
  - 将 `statement error ALTER TABLE tbl RESET PARTITIONED BY` 改为 `statement ok`
- [ ] 5.2 更新 `crates/sqlrustgo_sqllogictest/testdata/duckdb_full/alter__alter_table_set_partitioned_by.out`：
  - 移除错误输出，添加空输出表示成功
- [ ] 5.3 同步更新 `crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/alter_table_set_partitioned_by.test` 和 `.out`

## 6. 验证大小写不敏感场景

- [ ] 6.1 确认 `case_insensitive_alter.test` 中的 `ALTER TABLE MyTable ALTER BIGCOLUMN SET DATA TYPE VARCHAR` 测试仍然通过
- [ ] 6.2 如有需要，添加 `ALTER TABLE t set partitioned by (i)` 和 `ALTER TABLE t reset partitioned by` 的小写测试用例

## 7. 运行验证

- [ ] 7.1 运行 `cargo test -p sqlrustgo-parser alter` 确认语法解析测试通过
- [ ] 7.2 运行 `cargo build -p sqlrustgo-executor` 确认执行器编译通过
- [ ] 7.3 运行 `cargo test --all-features` 确认全量测试通过（无回归）
- [ ] 7.4 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 7.5 运行 `cargo fmt --check` 确认代码格式正确
- [ ] 7.6 手动验证：
  ```bash
  echo "ALTER TABLE t SET PARTITIONED BY (x, y)" | cargo run --bin sqlrustgo -- --dump-ast
  echo "ALTER TABLE t RESET PARTITIONED BY" | cargo run --bin sqlrustgo -- --dump-ast
  ```

## 8. PR 与合并

- [ ] 8.1 提交所有变更到特性分支
- [ ] 8.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 8.3 获得至少 1 个 reviewer 批准
- [ ] 8.4 合并到 develop/v3.13.0

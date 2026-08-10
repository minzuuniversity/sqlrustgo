# Design: ALTER TABLE SET PARTITIONED BY 语法支持

## 1. 语法分析

DuckDB 的 `ALTER TABLE ... SET PARTITIONED BY` 语法如下：

```sql
ALTER TABLE table_name SET PARTITIONED BY (column_name [, ...])
ALTER TABLE table_name RESET PARTITIONED BY
```

其中：
- `SET PARTITIONED BY`：为表设置新的分区键列列表
- `RESET PARTITIONED BY`：清除表的分区策略
- 分区列列表可包含一个或多个列名

## 2. 当前解析器分析

`crates/parser/src/parser.rs` 中 `AlterTableOperation` 当前定义：

```rust
pub enum AlterTableOperation {
    AddColumn { name, data_type, nullable, default_value },
    DropColumn { name },
    ModifyColumn { name, data_type, nullable, char_max_length },
    RenameTo { new_name },
    RenameColumn { name, new_name },
}
```

**缺失部分**：`SetPartitionedBy` 和 `ResetPartitionedBy` 分支。

当前解析逻辑在 `parse_alter_table` 函数中，遇到 `SET` 或 `RESET` 关键字时未匹配任何分支，直接报错 `Expected ADD, DROP, MODIFY or RENAME`。

## 3. 实现方案

### 3.1 AST 扩展

在 `crates/parser/src/parser.rs` 的 `AlterTableOperation` 枚举中增加变体：

```rust
pub enum AlterTableOperation {
    // ... existing variants ...
    SetPartitionedBy {
        columns: Vec<String>,
    },
    ResetPartitionedBy,
}
```

### 3.2 解析逻辑扩展

在 `parse_alter_table` 函数中添加分支：

```rust
Some(Token::Set) => {
    // 解析 SET PARTITIONED BY (column_list)
    expect_keyword("PARTITIONED")?;
    expect_keyword("BY")?;
    expect(Token::LeftParen)?;
    let columns = parse_comma_separated_identifiers()?;
    expect(Token::RightParen)?;
    Ok(Statement::AlterTable(AlterTableStatement {
        table_name,
        operation: AlterTableOperation::SetPartitionedBy { columns },
    }))
}
Some(Token::Reset) => {
    // 解析 RESET PARTITIONED BY
    expect_keyword("PARTITIONED")?;
    expect_keyword("BY")?;
    Ok(Statement::AlterTable(AlterTableStatement {
        table_name,
        operation: AlterTableOperation::ResetPartitionedBy,
    }))
}
```

### 3.3 执行器实现

在 `crates/executor/src/ddl.rs` 的 `execute_alter_table` 中添加分支：

```rust
match operation {
    AlterTableOperation::SetPartitionedBy { columns } => {
        // 1. 验证表存在
        // 2. 验证所有 columns 存在
        // 3. 更新表的分区策略元数据
        // 4. 返回执行结果
    }
    AlterTableOperation::ResetPartitionedBy => {
        // 1. 验证表存在
        // 2. 清除表的分区策略
        // 3. 返回执行结果
    }
    // ... other cases ...
}
```

### 3.4 约束与限制

| 场景 | 状态 |
|------|------|
| SET PARTITIONED BY (单列) | ✅ 实现 |
| SET PARTITIONED BY (多列) | ✅ 实现 |
| RESET PARTITIONED BY | ✅ 实现 |
| 大小写不敏感关键字 | ✅ 实现（SET / set / Set 均支持） |
| 分区数据物理重排 | ❌ 后续迭代 |
| 分区键类型验证 | ❌ 后续迭代 |

## 4. Fixture 设计

### 4.1 更新 duckdb_full fixture

`crates/sqlrustgo_sqllogictest/testdata/duckdb_full/alter__alter_table_set_partitioned_by.test`：

```sql
# name: alter__alter_table_set_partitioned_by
# description: Test ALTER TABLE SET PARTITIONED BY
# group: [alter]

statement ok
CREATE TABLE tbl(i INTEGER);

statement ok
ALTER TABLE tbl SET PARTITIONED BY (i)
----

statement ok
ALTER TABLE tbl RESET PARTITIONED BY
----
```

### 4.2 更新 duckdb_samples fixture

`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/alter_table_set_partitioned_by.test`：

```sql
# name: alter_table_set_partitioned_by
# description: Test ALTER TABLE SET PARTITIONED BY
# group: [alter]

statement ok
CREATE TABLE tbl(i INTEGER);

statement ok
ALTER TABLE tbl SET PARTITIONED BY (i)
----

statement ok
ALTER TABLE tbl RESET PARTITIONED BY
----
```

### 4.3 验证大小写不敏感

`case_insensitive_alter.test` 已包含 `ALTER TABLE MyTable ALTER BIGCOLUMN SET DATA TYPE VARCHAR` 等测试，无需修改。

## 5. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 大小写敏感度 | 关键字大小写敏感 / 不敏感 | 不敏感 | DuckDB 和 SQL 标准均支持大小写不敏感 |
| 空分区列列表 | 允许 / 拒绝 | 拒绝 | `SET PARTITIONED BY ()` 无意义 |
| 执行失败时 | 返回错误 / 静默忽略 | 返回错误 | 保持与其他 ALTER 操作一致 |
| 多列分区 | 支持 / 仅支持单列 | 支持 | DuckDB 本身支持多列分区 |

## 6. 验证方式

```bash
# 运行 parser 单元测试
cargo test -p sqlrustgo-parser alter

# 运行 DDL 执行测试
cargo test -p sqlrustgo-executor alter

# 运行 DuckDB 兼容测试
./scripts/gate/run_compat_tests.sh --group duckdb

# 验证 SET PARTITIONED BY 解析
echo "ALTER TABLE t SET PARTITIONED BY (x, y)" | cargo run --bin sqlrustgo -- --dump-ast

# 验证 RESET PARTITIONED BY 解析
echo "ALTER TABLE t RESET PARTITIONED BY" | cargo run --bin sqlrustgo -- --dump-ast
```

## 7. 失败模式

- 若表不存在：返回 `SqlError::TableNotFound`
- 若分区列不存在：返回 `SqlError::UnknownColumn`
- 若 `SET PARTITIONED BY` 包含空列表：返回 `SqlError::InvalidAlterTableOperation`
- 若执行器尚未实现分区重排：返回 `SqlError::NotSupported("partitioned tables not yet supported")`

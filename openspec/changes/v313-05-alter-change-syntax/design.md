# Design: ALTER TABLE CHANGE COLUMN 语法支持

## 1. 语法分析

MySQL 的 `ALTER TABLE ... CHANGE COLUMN` 语法如下：

```sql
ALTER TABLE t CHANGE COLUMN old_col_name new_col_name column_definition [FIRST|AFTER col_name]
```

其中：
- `old_col_name`：当前列名（必填）
- `new_col_name`：新列名（必填）
- `column_definition`：新列的类型和约束（必填）
- `FIRST/AFTER`：可选的列位置调整

与 `MODIFY COLUMN` 的区别：

| 特性 | `CHANGE COLUMN` | `MODIFY COLUMN` |
|------|----------------|-----------------|
| 重命名列 | ✅ | ❌ |
| 修改类型 | ✅ | ✅ |
| 修改约束 | ✅ | ✅ |
| 修改位置 | ✅ | ✅ |

## 2. 当前解析器分析

`crates/parser/src/sql.y`（或等价语法文件）中的 `AlterTableAction` 当前定义大致为：

```yacc
AlterTableAction:
    ADD COLUMN column_definition
  | ADD COLUMN column_definition FIRST|AFTER
  | DROP COLUMN column_name
  | MODIFY COLUMN column_definition
  | MODIFY COLUMN column_definition FIRST|AFTER
  | RENAME COLUMN old_name new_name
```

**缺失部分**：`CHANGE COLUMN` 分支。

## 3. 实现方案

### 3.1 语法扩展

在 `sql.y` 中添加 `CHANGE COLUMN` 分支：

```yacc
AlterTableAction:
    CHANGE COLUMN column_name column_definition
  | CHANGE COLUMN column_name column_definition FIRST|AFTER
```

其中 `column_definition` 需要包含新列名（这与 MySQL 语法一致）。

### 3.2 AST 扩展

在 `crates/parser/src/ast.rs` 的 `AlterTableAction` 枚举中增加变体：

```rust
pub enum AlterTableAction {
    // ... existing variants ...
    ChangeColumn {
        old_name: Ident,
        new_name: Ident,
        data_type: DataType,
        constraints: Vec<ColumnConstraint>,
        position: Option<ColumnPosition>,
    },
}
```

### 3.3 执行器实现

在 `crates/executor/src/ddl.rs` 的 `execute_alter_table` 中添加分支：

```rust
match action {
    AlterTableAction::ChangeColumn { old_name, new_name, data_type, constraints, position } => {
        // 1. 验证 old_name 列存在
        // 2. 更新列定义：重命名 + 应用新类型 + 新约束
        // 3. 若指定了 position，移动列顺序
        // 4. 返回 ALTER TABLE 执行结果
    }
    // ... other cases ...
}
```

### 3.4 约束与限制

本变更优先覆盖以下场景：

| 场景 | 状态 |
|------|------|
| 列重命名 + 类型修改 | ✅ 实现 |
| 仅重命名（类型不变） | ✅ 实现（传相同类型） |
| 列位置调整（FRIST/AFTER） | ✅ 实现 |
| 外键引用的级联更新 | ❌ 后续迭代 |
| 涉及主键列的重命名 | ❌ 后续迭代 |
| 视图/触发器依赖更新 | ❌ 后续迭代 |

## 4. Fixture 设计

### 4.1 解除 deferred：`tests/compat/mysql_v3_12/alter_change_full_syntax_deferred.sql`

```sql
-- name: alter_change_full_syntax_deferred
-- expect: PASS
CREATE TABLE t (old_name INT NOT NULL);
ALTER TABLE t CHANGE COLUMN old_name new_name INT NOT NULL;
-- 验证列已重命名
INSERT INTO t VALUES (1);
SELECT new_name FROM t;
```

### 4.2 新建 fixture：`tests/compat/mysql_v3_13/alter_change_column.sql`

```sql
-- name: alter_change_column
-- expect: PASS

-- 场景1：重命名 + 类型修改
CREATE TABLE t1 (id INT PRIMARY KEY, col_a VARCHAR(50));
ALTER TABLE t1 CHANGE COLUMN col_a col_b TEXT;
INSERT INTO t1 VALUES (1, 'hello');
SELECT col_b FROM t1 WHERE id = 1;

-- 场景2：仅重命名
CREATE TABLE t2 (id INT PRIMARY KEY, old_col INT);
ALTER TABLE t2 CHANGE COLUMN old_col new_col INT;
INSERT INTO t2 VALUES (1, 42);
SELECT new_col FROM t2 WHERE id = 1;

-- 场景3：重命名 + 添加 NOT NULL 约束
CREATE TABLE t3 (id INT PRIMARY KEY, col_x INT);
ALTER TABLE t3 CHANGE COLUMN col_x col_y INT NOT NULL;
INSERT INTO t3 VALUES (1, 100);
SELECT col_y FROM t3 WHERE id = 1;

DROP TABLE t1;
DROP TABLE t2;
DROP TABLE t3;
```

## 5. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| CHANGE vs MODIFY | 合并为一个变体 / 独立变体 | 独立变体 | CHANGE 需要 old_name，语义不同 |
| 类型修改时 NULL 处理 | 自动转为 NOT NULL / 保持原样 | 保持原样 | 与 MySQL 行为一致 |
| 列位置调整 | 支持 FIRST/AFTER / 仅支持末尾 | 支持 FIRST/AFTER | 用户期望的功能 |
| 外键级联 | 拒绝操作 / 自动级联 | 拒绝操作 | 避免隐式破坏引用完整性 |

## 6. 验证方式

```bash
# 运行 parser 单元测试
cargo test -p sqlrustgo-parser alter

# 运行 DDL 执行测试
cargo test -p sqlrustgo-executor alter

# 运行兼容测试
./scripts/gate/run_compat_tests.sh

# 验证 CHANGE COLUMN 解析
echo "ALTER TABLE t CHANGE COLUMN a b INT NOT NULL" | cargo run --bin sqlrustgo -- --dump-ast
```

## 7. 失败模式

- 若列 `old_name` 不存在：返回 `SqlError::UnknownColumn`（与 MySQL 一致）
- 若 `new_name` 与现有列名冲突：返回 `SqlError::DuplicateColumnName`
- 若 `data_type` 转换不兼容（如 `INT` 转 `TEXT` 但有算术运算依赖）：警告但不阻止（MySQL 亦如此）
- 若表被外键引用且 `CHANGE` 影响主键：返回 `SqlError::InvalidAlterTableOperation`

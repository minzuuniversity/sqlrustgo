# V313-05: ALTER TABLE CHANGE COLUMN 语法支持

## Why

V312-21 的 `alter_change_full_syntax_deferred` fixture 揭示了当前实现的真实状态：`ALTER TABLE t CHANGE COLUMN old_name new_name INT NOT NULL` 解析失败，报错 `Parse error: Expected ADD, DROP, MODIFY or RENAME`。

MySQL 语法中 `CHANGE COLUMN` 是独立于 `MODIFY COLUMN` 的操作——前者可以同时重命名列名并修改类型，后者仅修改类型。解析器当前仅支持 `ADD`、`DROP`、`MODIFY`、`RENAME`，缺少 `CHANGE` 关键字支持。这导致：

1. 包含 `CHANGE COLUMN` 的 DDL 脚本无法在 SQLRustGo 中执行
2. 与 MySQL 的语法兼容性存在缺口
3. 用户无法在 ALTER TABLE 中重命名列（`MODIFY` 不提供重命名能力）

本变更将在解析器和执行层实现完整的 `CHANGE COLUMN` 支持，解除 V312-21 的 deferred 状态。

## What Changes

- **`crates/parser/src/sql.y`** 或等价语法定义文件：添加 `CHANGE` 作为 `AlterTableAction` 的合法关键字
- **`crates/parser/src/ast.rs`**：扩展 `AlterTableAction` 变体，增加 `ChangeColumn { old_name, new_name, new_type, constraints }`
- **`crates/executor/src/ddl.rs`** 或等价执行路径：为 `ChangeColumn` 变体实现列重命名 + 类型修改逻辑
- **`tests/compat/mysql_v3_13/alter_change_column.sql`** + `.out`：解除 deferred 的 fixture，验证完整语义
- **`tests/compat/mysql_v3_12/alter_change_full_syntax_deferred.sql`**：更新 `expect: PASS`，正式解除 deferred 状态
- **`docs/releases/v3.13.0/evidence/mysql_compat/SURFACE_DISPOSITION.md`**：更新 `alter_change_full_syntax` 行从 `deferred` 改为 `PASS`

## Capabilities

### 新增能力

- `alter-table-change-column`：解析并执行 `ALTER TABLE t CHANGE COLUMN old_name new_name TYPE [constraints]`，支持列重命名与类型修改同时完成
- `mysql-compat-alter-change`：与 MySQL 5.7/8.0 行为对齐的 `CHANGE COLUMN` 语法

### 修改能力

- `alter-table-modify-column`：现有 `MODIFY` 行为不变，作为独立功能保留
- `deferred-alter-change-full-syntax`：解除 V312-21 的 deferred 状态，fixture 从 `DEFERRED` 升级为 `PASS`

## Impact

- **修改文件**：`crates/parser/src/`（语法定义 + AST）、`crates/executor/src/`（DDL 执行）
- **新增文件**：`tests/compat/mysql_v3_13/alter_change_column.sql` + `.out`
- **风险**：`CHANGE COLUMN` 涉及列重命名，可能影响依赖列顺序的下游系统（如外键引用、视图、触发器）；本变更优先覆盖核心场景（列重命名 + 类型修改），索引/外键重建为后续迭代预留
- **无新增外部 crate 依赖**

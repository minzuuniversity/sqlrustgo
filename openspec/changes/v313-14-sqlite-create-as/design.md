# Design: SQLite CREATE TABLE AS 行为修复

## 1. SQLite CTAS 语义

SQLite 的 `CREATE TABLE AS SELECT` 行为如下：

```sql
CREATE TABLE t AS SELECT 1;
-- 列名: "1"（原生表达式文本），类型: INTEGER affinity

CREATE TABLE t AS SELECT 1 AS f;
-- 列名: "f"（别名），类型: INTEGER affinity

CREATE TABLE t(col_a, col_b) AS SELECT 1, 'hello';
-- 列名: "col_a", "col_b"（覆盖 SELECT 列名），类型: TEXT affinity

CREATE TABLE t AS SELECT 1 WHERE false;
-- 空表（无数据行）

CREATE TABLE t AS SELECT 1 WITH NO DATA;
-- 表结构存在，无数据

CREATE TABLE t AS SELECT 1 WITH DATA;
-- 表结构和数据均存在
```

关键规则：
- **列名推断**：优先使用别名；无别名则使用表达式文本（如 `SELECT 1` → 列名 `1`）
- **列类型**：使用值的 affinity（INTEGER/TEXT/REAL/BLOB/NULL）
- **显式列名**：`CREATE TABLE t(c1, c2) AS SELECT ...` 覆盖 SELECT 的列名；若 SELECT 列数少于显式列数，填充 NULL

## 2. 当前解析器分析

`crates/parser/src/sql.y`（或等价语法文件）中的 `CreateTableStatement` 当前大致定义为：

```yacc
CreateTableStatement:
    CREATE TABLE Ident '(' ColumnDefs ')'           -- 普通建表
  | CREATE TABLE Ident '(' ColumnDefs ')' Primary   -- 带主键
  -- CTAS 分支缺失
```

**缺失部分**：
- `AS SELECT` 关键字和子句
- `WITH (NO)? DATA` 子句
- CTAS 的列名覆盖语义

## 3. AST 扩展

在 `crates/parser/src/ast.rs` 的 `CreateTableStatement` 结构体中增加字段：

```rust
pub struct CreateTableStatement {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,       // 显式列定义（CTAS 时可为空）
    pub constraints: Vec<TableConstraint>,
    pub as_query: Option<Box<SelectStatement>>,  // 新增：AS SELECT 子句
    pub with_data: bool,                         // 新增：true=WITH DATA, false=WITH NO DATA
    pub if_not_exists: bool,
    pub or_replace: bool,
}
```

**注意**：`as_query` 为 `Some` 时表示 CTAS，此时 `columns` 可能为空（无显式列定义）。

## 4. 实现方案

### 4.1 语法扩展

在 `sql.y` 中添加 CTAS 分支：

```yacc
CreateTableStatement:
    CREATE TABLE Ident OptColumnDefs OptCreateTableOptions AS SelectStmt OptWithData
  | CREATE OR REPLACE TABLE Ident OptColumnDefs OptCreateTableOptions AS SelectStmt OptWithData
  | CREATE TABLE Ident OptColumnDefs OptCreateTableOptions AS SelectStmt OptWithData IF NOT EXISTS

OptWithData:
    /* empty */   { true }   -- 默认 WITH DATA
  | WITH DATA     { true }
  | WITH NO DATA  { false }

OptColumnDefs:
    /* empty */       { vec![] }
  | '(' ColumnDefs ')' { $2 }
```

### 4.2 列名推断逻辑

执行 CTAS 时，按以下顺序推断列名：

```
if 显式列名存在（CREATE TABLE t(c1, c2) AS ...）:
    使用显式列名
else if SELECT 列表项有别名:
    使用别名
else:
    使用表达式文本（如 "1", "'hello'"）
```

### 4.3 执行器实现

在 `crates/executor/src/` 的 DDL 执行路径（可能在 `sql_executor.rs` 或新建 `ctas_executor.rs`）实现：

```rust
async fn execute_create_table_as(
    ctx: &mut ExecutionContext,
    stmt: CreateTableStatement,
) -> Result<(), SqlError> {
    let CreateTableStatement { name, columns, as_query, with_data, .. } = stmt;

    // 1. 执行 SELECT 获取结果集
    let select_ctx = execute_select(ctx, as_query).await?;
    let schema = select_ctx.output_schema();

    // 2. 推断目标表列定义
    let inferred_columns = if columns.is_empty() {
        // 无显式列定义：从 SELECT 结果推断
        infer_columns_from_select(&schema)
    } else {
        // 有显式列定义：使用显式列名，类型从 SELECT 结果推断
        infer_columns_with_override(&schema, &columns)
    };

    // 3. 创建表 schema
    create_table(ctx.catalog(), name, inferred_columns).await?;

    // 4. 若 with_data 为 true，将 SELECT 结果写入新表
    if with_data {
        insert_into_table_from_select(ctx, &name, select_ctx).await?;
    }

    Ok(())
}
```

### 4.4 类型推断（Affinity）

SQLite affinity 推断规则：

| 表达式类型 | Affinity |
|-----------|----------|
| 整数文本（无小数点、无指数） | INTEGER |
| 实数文本（有小数点或指数） | REAL |
| 字符串文本（带单引号） | TEXT |
| 十六进制字面量（`0x...`） | BLOB |
| NULL 字面量 | NULL（无 affinity） |

当前实现可能使用类型推断而非 affinity，这需要在实现时确认。

### 4.5 约束与限制

本变更优先覆盖以下场景：

| 场景 | 状态 |
|------|------|
| 基本 CTAS（无别名） | ✅ 实现 |
| CTAS + 列别名 | ✅ 实现 |
| CTAS + 显式列名覆盖 | ✅ 实现 |
| CTAS + WHERE false（空表） | ✅ 实现 |
| CTAS + WITH NO DATA | ✅ 实现 |
| CTAS + WITH DATA | ✅ 实现 |
| CTAS + UNION ALL | ✅ 实现 |
| CREATE OR REPLACE TABLE ... AS SELECT | ✅ 实现 |
| CREATE TABLE ... IF NOT EXISTS AS SELECT | ❌ 后续迭代 |
| 涉及约束（PRIMARY KEY、CHECK 等） | ❌ 后续迭代 |
| 子查询中的 CTAS | ❌ 后续迭代 |

## 5. Fixture 设计

### 5.1 sqllogictest fixture：`crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/create_as.test`

该 fixture 已存在，需验证以下关键用例通过：

```sql
-- 列名推断：无别名
CREATE TABLE tbl1 AS SELECT 1;
SELECT * FROM tbl1;  -- 期望列名 "1"

-- 列名推断：有别名
CREATE TABLE tbl2 AS SELECT 2 AS f;
SELECT f FROM tbl2;  -- 期望列名 "f"

-- CREATE OR REPLACE
CREATE OR REPLACE TABLE tbl1 AS SELECT 4;
SELECT * FROM tbl1;  -- 期望 4

-- 显式列名覆盖
CREATE TABLE tbl4(col1, col2) AS SELECT 1, 'hello';
SELECT * FROM tbl4;  -- 期望列名 col1, col2

-- WITH NO DATA
CREATE TABLE tbl8 AS SELECT 42 WITH NO DATA;
SELECT COUNT(*) FROM tbl8;  -- 期望 0

-- WITH DATA
CREATE TABLE tbl9 AS SELECT 42 WITH DATA;
SELECT COUNT(*) FROM tbl9;  -- 期望 1
```

### 5.2 SQLite 兼容 fixture：`tests/compat/sqlite_v3_13/create_table_as.sql`

```sql
-- name: create_table_as_basic
-- expect: PASS
CREATE TABLE t1 AS SELECT 1;
SELECT * FROM t1;

-- name: create_table_as_with_alias
-- expect: PASS
CREATE TABLE t2 AS SELECT 1 AS num, 'hello' AS msg;
SELECT num, msg FROM t2;

-- name: create_table_as_explicit_cols
-- expect: PASS
CREATE TABLE t3(col_a, col_b) AS SELECT 1, 'text';
SELECT col_a, col_b FROM t3;

-- name: create_table_as_empty_result
-- expect: PASS
CREATE TABLE t4 AS SELECT 1 WHERE false;
SELECT COUNT(*) FROM t4;

-- name: create_table_as_with_no_data
-- expect: PASS
CREATE TABLE t5 AS SELECT 42 WITH NO DATA;
SELECT COUNT(*) FROM t5;

-- name: create_table_as_union
-- expect: PASS
CREATE TABLE t6 AS SELECT 1 UNION ALL SELECT 2;
SELECT * FROM t6;
```

## 6. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 类型推断方式 | Affinity / 精确类型 | Affinity | 与 SQLite 行为一致 |
| 列名冲突处理 | 覆盖 / 报错 | 覆盖 | SQLite 允许显式列名覆盖 |
| 空结果集 | 创建空表 / 报错 | 创建空表 | `WHERE false` 应创建空表 |
| IF NOT EXISTS + AS | 支持 / 不支持 | 不支持 | 优先级低 |
| OR REPLACE + AS | 支持 / 不支持 | 支持 | fixture 中已验证 |

## 7. 验证方式

```bash
# 运行 sqllogictest CTAS 测试
cargo test -p sqlrustgo_sqllogictest create_as

# 运行解析器单元测试
cargo test -p sqlrustgo-parser create_table

# 运行 DDL 执行测试
cargo test -p sqlrustgo-executor create_table

# 运行 SQLite 兼容测试（若脚本存在）
./scripts/gate/run_compat_tests.sh

# 验证 CTAS 解析
echo "CREATE TABLE t AS SELECT 1" | cargo run --bin sqlrustgo -- --dump-ast
```

## 8. 失败模式

- 若 SELECT 语句本身失败：返回 SELECT 的原始错误（如 `SqlError::InvalidSelect`）
- 若表名已存在且无 `OR REPLACE`/`IF NOT EXISTS`：返回 `SqlError::TableAlreadyExists`
- 若 `CREATE TABLE t(c1) AS SELECT a, b`（列数不匹配）：填充 NULL（SQLite 行为）
- 若 `CREATE TABLE t AS SELECT`（无 SELECT）：返回 `SqlError::ParseError`

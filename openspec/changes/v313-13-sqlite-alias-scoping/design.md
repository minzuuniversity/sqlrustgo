# Design: 修复 SELECT 别名在 WHERE 子句中的错误引用

## 1. 问题分析

### 1.1 错误复现

```sql
with test_data as (
  select 'foo' as a
)
select test_data.foobar as new_column from test_data where new_column is not null;
```

错误信息：`query is expected to fail, but actually succeed`

预期行为：Binder 应返回错误，因为 `new_column` 是 SELECT 列表中的别名，WHERE 子句在逻辑上先于 SELECT 别名定义求值。

### 1.2 SQL-92 别名作用域规则

SQL-92 定义了查询表达式的作用域求值顺序：

1. **FROM** → 首先构建表对象列表
2. **WHERE** → 对 FROM 结果进行过滤，引用列时只能使用基表列名或 FROM 中定义的别名
3. **GROUP BY** → 分组
4. **HAVING** → HAVING 过滤
5. **SELECT** → 最后计算 SELECT 表达式，此时才定义别名
6. **ORDER BY** → 排序，可以引用 SELECT 别名（因为 ORDER BY 在 SELECT 之后求值）

因此：
- `WHERE new_column IS NOT NULL` → 非法（`new_column` 在 WHERE 求值时尚未定义）
- `ORDER BY new_column` → 合法（`new_column` 在 ORDER BY 求值时已定义）

### 1.3 数据流

```
SQL: SELECT col AS alias FROM t WHERE alias > 10
  │
  ▼
Binder::resolve()  ──► 语义分析与作用域检查
  │
  ├── 检查 WHERE 子句是否引用了 SELECT 别名
  │
  ▼
错误：WHERE 引用了 SELECT 别名 'alias'
  │
  ▼
返回错误码 10057（alias in where clause）
```

### 1.4 根因分析

`binder__alias_error_10057.test` 的 `statement error` 预期说明：某些数据库系统（如 DuckDB）正确地在 Binder 层拒绝了此查询。SQLRustGo 当前实现允许了此查询通过，说明 Binder 中对 SELECT 别名在 WHERE 子句中的检查逻辑存在漏洞。

可能的问题：

1. **作用域检查缺失**：WHERE 子句引用列时，未检查该列名是否对应 SELECT 列表中的别名
2. **作用域上下文混淆**：Binder 在处理 WHERE 子句时，可能错误地允许访问 SELECT 别名命名空间
3. **列引用解析错误**：`test_data.foobar` 被解析为有效引用，但 `new_column` 在 WHERE 中被错误地允许

### 1.5 相关代码位置（推断）

根据错误信息推断，问题代码位于：

- `crates/sqlrustgo-binder/src/` 中的表达式解析或作用域管理模块
- 可能涉及 `Resolver`、`Scope`、`ColumnRef` 相关的处理逻辑
- 错误码 10057 暗示 Binder 定义了专门的错误码用于别名作用域错误

## 2. 修复方案

### 2.1 定位问题代码

在 `crates/sqlrustgo-binder/src/` 中查找：

```bash
# 搜索别名解析和作用域相关代码
grep -rn "alias\|scope\|where.*alias\|alias.*where\|10057" crates/sqlrustgo-binder/src/
```

关键路径（推断）：
- `crates/sqlrustgo-binder/src/resolver.rs` 或等价模块
- 包含 `resolve_column()` / `check_scope()` / `resolve_alias()` 方法

### 2.2 作用域判定逻辑

SELECT 别名在 WHERE 中的错误引用，应在 Binder 层通过以下逻辑捕获：

```rust
// 伪代码：WHERE 子句列引用解析
fn resolve_where_column_ref(column_name: &str, select_aliases: &HashSet<&str>) -> Result<ColumnRef, Error> {
    // 步骤 1：检查是否是 SELECT 列表中的别名
    if select_aliases.contains(column_name) {
        // WHERE 子句不允许引用 SELECT 别名（ORDER BY 可以）
        return Err(BinderError::AliasInWhereClause(column_name));
    }

    // 步骤 2：检查是否是基表列名
    // ... 正常列解析逻辑
}
```

### 2.3 修复检查表

| 步骤 | 操作 | 预期 |
|------|------|------|
| 1 | `SELECT 1 AS x WHERE x = 1` | 错误（WHERE 引用别名 `x`） |
| 2 | `SELECT 1 AS x ORDER BY x` | 成功（ORDER BY 可引用别名） |
| 3 | `SELECT a AS b FROM t WHERE b > 10` | 错误（WHERE 引用别名 `b`） |
| 4 | CTE 场景（同 original test） | 错误（WHERE 引用 CTE 别名） |

## 3. Fixture 设计

### 3.1 新增 Fixture

#### `tests/compat/sqlite_v3_13/alias_where_error.sql`

```sql
# name: alias_where_error
# expect: PASS
# NOTE: V313-13 修复；WHERE 子句引用 SELECT 别名应报错

WITH test_data AS (
  SELECT 'foo' AS a
)
SELECT test_data.a AS new_column FROM test_data WHERE new_column IS NOT NULL;

# 基础场景
SELECT 1 AS x WHERE x = 1;

# ORDER BY 引用别名（应成功）
SELECT 1 AS x ORDER BY x;

# 多列别名
SELECT 1 AS a, 2 AS b WHERE a > 0;

# 带有表达式的别名
SELECT 1 + 2 AS sum WHERE sum > 0;
```

#### `tests/compat/sqlite_v3_13/alias_where_error.out`

预期输出：每条查询都应返回错误，错误信息包含别名相关描述。

### 3.2 恢复 Exclusion 的 Fixture

#### `crates/sqlrustgo_sqllogictest/testdata/duckdb_full/binder__alias_error_10057.test`

此文件应保持 `statement error` 预期，SQLite 和 DuckDB 行为一致（都应报错）。

## 4. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 修复层 | Binder / Executor | Binder | 别名作用域检查属于语义分析，在执行前拒绝更高效 |
| 错误码 | 新增专用码 / 复用现有码 | 复用 10057 | 测试文件明确使用 10057，说明已有对应错误码 |
| 测试策略 | 先修复后测试 / 先测试后修复 | 先定位再修复 | 确保根因分析准确后再修改代码 |

## 5. 验证方式

```bash
# 1. 构建
cargo build --all-features

# 2. 运行 sqllogictest 中被 exclusion 的测试
cargo test -p sqlrustgo_sqllogictest binder__alias_error_10057

# 3. 运行新增 fixture
./scripts/gate/check_v313_13_alias_where.sh

# 4. 验证 exclusion 已移除
grep "binder__alias_error_10057" crates/sqlrustgo_sqllogictest/testdata/duckdb_full/exclusions.yml
# 应无输出（exclusion 已移除）

# 5. 通用回归测试
cargo test --all-features
```

## 6. 失败模式

- 若修复后 WHERE 别名引用不再报错：说明作用域检查逻辑仍有漏洞
- 若 `ORDER BY alias` 行为异常：说明修复引入了过度限制，需确保只限制 WHERE 而非 ORDER BY
- 若其他查询（如 `SELECT * WHERE alias = 1`）意外失败：说明修复范围过宽，需更精确地定位问题

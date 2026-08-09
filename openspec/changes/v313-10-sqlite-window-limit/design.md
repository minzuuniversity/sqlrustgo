# Design: 支持 LIMIT 子句中使用窗口函数

## 1. 问题分析

### 1.1 错误复现

```sql
CREATE TABLE test (a INTEGER, b INTEGER);
INSERT INTO test VALUES (11, 22);
SELECT a FROM test LIMIT row_number() OVER ();
```

当前错误：

```
Parse error: Invalid LIMIT: invalid digit found in string
```

### 1.2 当前 LIMIT 解析逻辑

文件：`crates/parser/src/parser.rs:4718-4741`

```rust
// Parse LIMIT clause
let limit = if matches!(self.current(), Some(Token::Limit)) {
    self.next();
    match self.current() {
        Some(Token::NumberLiteral(n)) => {
            let val = n
                .parse::<u64>()
                .map_err(|e| format!("Invalid LIMIT: {}", e))?;
            self.next();
            Some(val)
        }
        Some(Token::Identifier(ref s)) => {
            let val = s
                .parse::<u64>()
                .map_err(|e| format!("Invalid LIMIT: {}", e))?;
            self.next();
            Some(val)
        }
        _ => None,
    }
} else {
    None
};
```

问题：仅接受 `NumberLiteral` 和 `Identifier`（变量），不处理函数调用表达式。

### 1.3 数据流

```
SQL: SELECT a FROM test LIMIT row_number() OVER ()
  │
  ▼
Parser::parse_select()  ──► 遇到 LIMIT token，调用 parse_limit()
  │
  ▼  (当前路径)
parse_limit() 仅接受 NumberLiteral / Identifier
  │
  ▼
row_number() OVER () 作为 Function 表达式未被识别
  │
  ▼
"Invalid LIMIT: invalid digit found in string"
```

### 1.4 根因

`SelectStatement.limit` 字段类型为 `Option<u64>`，限制了 LIMIT 子句只能接受字面量值。

支持窗口函数需要：

1. 将 `limit` 字段类型从 `Option<u64>` 扩展为 `Option<Expression>`
2. 扩展解析器支持 `parse_expression()` 作为 LIMIT 值
3. 扩展执行器支持表达式求值得到 u64

## 2. 修复方案

### 2.1 方案 A：LIMIT 支持通用 Expression（推荐）

**优点**：
- 通用性强，支持 `LIMIT <任意表达式>`
- 与 OFFSET 语义一致（OFFSET 也接受表达式）

**缺点**：
- 需要修改 `SelectStatement` 结构
- 需要在执行器中对 Expression 求值得到 u64

#### 2.1.1 AST 层修改

`crates/parser/src/parser.rs` — `SelectStatement` 结构：

```rust
// Before
pub limit: Option<u64>,
pub offset: Option<u64>,

// After
pub limit: Option<Expression>,
pub offset: Option<Expression>,
```

#### 2.1.2 解析层修改

`crates/parser/src/parser.rs` — `parse_limit()`：

```rust
// Parse LIMIT clause
let limit = if matches!(self.current(), Some(Token::Limit)) {
    self.next();
    match self.current() {
        Some(Token::NumberLiteral(n)) => {
            let val = n
                .parse::<u64>()
                .map_err(|e| format!("Invalid LIMIT: {}", e))?;
            self.next();
            Some(Expression::Literal(Literal::Integer(val as i64)))
        }
        Some(Token::Identifier(ref s)) => {
            // 支持 @变量 形式
            let val = s
                .parse::<u64>()
                .map_err(|e| format!("Invalid LIMIT: {}", e))?;
            self.next();
            Some(Expression::Literal(Literal::Integer(val as i64)))
        }
        _ => {
            // 支持通用表达式（窗口函数、子查询等）
            Some(self.parse_expression()?)
        }
    }
} else {
    None
};
```

### 2.2 执行器层修改

`crates/planner/src/` — 查询规划器：

窗口函数在 ORDER BY 之前计算完成，因此 LIMIT 中的窗口函数表达式可以像子查询一样在下层子计划中先执行。

```rust
// 规划器逻辑
if let Some(expr) = select.limit {
    match expr {
        Expression::Literal(Literal::Integer(n)) => {
            // 直接使用常量值
            limit = Some(n as u64);
        }
        _ => {
            // 表达式类型：为 LIMIT 创建下层子计划，先执行表达式求值
            // 例如 LIMIT ROW_NUMBER() OVER ()：
            //   1. 创建子计划：SELECT ROW_NUMBER() OVER () AS __limit_expr
            //   2. 执行子计划得到单行单列值
            //   3. 使用该值作为 LIMIT
        }
    }
}
```

### 2.3 窗口函数求值时机

窗口函数在 SELECT 列表和 ORDER BY 中使用，但 LIMIT 在 ORDER BY 之后处理。

执行顺序：

```
1. FROM / WHERE  ──► 基础行
2. GROUP BY / HAVING
3. 窗口函数计算（PARTITION BY / ORDER BY 完成后得到完整列值）
4. SELECT 列表投影
5. ORDER BY 排序
6. LIMIT / OFFSET 截断
```

因此 `LIMIT ROW_NUMBER() OVER ()` 中的 `ROW_NUMBER()` 在 LIMIT 截断发生前已经计算完毕，可以正常获取值。

## 3. Fixture 设计

### 3.1 新增 Fixture

#### `tests/compat/sqlite_v3_13/window_limit.sql`

```sql
# name: window_limit
# expect: PASS
# NOTE: V313-10 支持 LIMIT 中使用窗口函数

CREATE TABLE test (a INTEGER, b INTEGER);
INSERT INTO test VALUES (11, 22), (12, 21), (13, 22);

-- LIMIT ROW_NUMBER() OVER () 返回第一行
SELECT a FROM test LIMIT row_number() OVER ();
-- 期望: 11

-- LIMIT RANK() OVER () 返回第一行
SELECT a FROM test LIMIT rank() OVER ();
-- 期望: 11

-- 带 PARTITION BY 的窗口函数
SELECT a, b FROM test LIMIT row_number() OVER (PARTITION BY b ORDER BY a);
-- 期望: 12（b=21 的第一行）

-- LIMIT + OFFSET 使用窗口函数
SELECT a FROM test ORDER BY a LIMIT row_number() OVER () OFFSET 1;
-- 期望: 12

DROP TABLE test;
```

### 3.2 更新现有 Fixture

#### `tests/compat/sqlite_v3_12/order__test_limit.test`

第 49 行：

```sql
# 原有（statement error）：
# window function in limit
statement error
SELECT a FROM test LIMIT row_number() OVER ()
----
<REGEX>:Not implemented Error:.*expression class.*

# V313-10 修复后（query I）：
query I
SELECT a FROM test LIMIT row_number() OVER ()
----
11
```

## 4. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| LIMIT 类型 | `Option<u64>` / `Option<Expression>` | `Option<Expression>` | 通用性强，与 OFFSET 语义一致 |
| 表达式求值 | 内联求值 / 子计划求值 | 子计划求值 | 避免修改执行器核心逻辑 |
| 窗口函数时机 | 执行期动态计算 / 执行前预计算 | 执行前预计算 | 符合 Volcano 模型执行顺序 |

## 5. 验证方式

```bash
# 1. 构建
cargo build --all-features

# 2. 运行 parser 测试
cargo test -p sqlrustgo-parser limit

# 3. 运行 planner 测试
cargo test -p sqlrustgo-planner limit

# 4. 运行 executor 窗口函数测试
cargo test -p sqlrustgo-executor window

# 5. 运行 compat 测试
./scripts/gate/check_v313_10_window_limit.sh

# 6. 验证 order__test_limit.test 不再 excluded
grep "order__test_limit.test" docs/releases/v3.13.0/evidence/sqlite_compat/SURFACE_DISPOSITION.md
```

## 6. 失败模式

- 若 `parse_expression()` 在 LIMIT 上下文中误匹配后续关键字：需要在解析器中添加前瞻，确保 LIMIT 表达式不会贪婪消费 OFFSET 关键字
- 若窗口函数在 LIMIT 中无法访问外层 SELECT 的列：窗口函数在子计划中执行时应使用完整数据集，不依赖外层上下文
- 若执行器中 LimitExec 不支持表达式参数：需要修改 LimitExec::new 签名，接受 `Option<Expr>` 而非 `Option<u64>`

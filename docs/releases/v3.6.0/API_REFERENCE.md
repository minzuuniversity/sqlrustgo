# SQLRustGo v3.6.0 SQL 语法参考

> **版本**: v3.6.0
> **HEAD**: 1b2a3c71
> **日期**: 2026-05-30
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 1. DDL (数据定义语言)

### 1.1 CREATE DATABASE

```sql
CREATE DATABASE [IF NOT EXISTS] database_name;
```

### 1.2 DROP DATABASE

```sql
DROP DATABASE [IF EXISTS] database_name;
```

### 1.3 CREATE TABLE

```sql
CREATE TABLE [IF NOT EXISTS] table_name (
    column_name data_type [column_constraint] [DEFAULT default_value],
    ...
    [table_constraint]
);

-- 列约束
column_constraint:
    PRIMARY KEY
  | NOT NULL
  | UNIQUE
  | CHECK (expr)
  | REFERENCES ref_table (ref_column)

-- 表约束
table_constraint:
    PRIMARY KEY (col1, col2, ...)
  | UNIQUE (col1, col2, ...)
  | FOREIGN KEY (col1, ...) REFERENCES ref_table (ref_col, ...)
  | CHECK (expr)
```

#### 支持的数据类型

| 类型 | 描述 | 示例 |
|------|------|------|
| TINYINT | 8-bit 整数 | `age TINYINT` |
| SMALLINT | 16-bit 整数 | `small_val SMALLINT` |
| INT / INTEGER | 32-bit 整数 | `id INT` |
| BIGINT | 64-bit 整数 | `big_val BIGINT` |
| FLOAT | 32-bit 浮点 | `score FLOAT` |
| DOUBLE | 64-bit 浮点 | `price DOUBLE` |
| DECIMAL(p,s) | 精确小数 | `salary DECIMAL(10,2)` |
| VARCHAR(n) | 变长字符串 | `name VARCHAR(100)` |
| CHAR(n) | 定长字符串 | `code CHAR(10)` |
| TEXT | 长文本 | `description TEXT` |
| DATE | 日期 | `hire_date DATE` |
| DATETIME | 日期时间 | `created_at DATETIME` |
| BOOLEAN | 布尔值 | `active BOOLEAN` |
| BLOB | 二进制 | `data BLOB` |

### 1.4 CREATE TABLE ... PARTITION BY

```sql
-- RANGE 分区
CREATE TABLE sales (
    id INT,
    amount DECIMAL(10,2),
    sale_date DATE
) PARTITION BY RANGE (YEAR(sale_date)) (
    PARTITION p2024 VALUES LESS THAN (2025),
    PARTITION p_future VALUES LESS THAN MAXVALUE
);

-- LIST 分区
CREATE TABLE regions (
    id INT,
    name VARCHAR(50)
) PARTITION BY LIST (id) (
    PARTITION p_north VALUES IN (1, 2, 3),
    PARTITION p_south VALUES IN (4, 5, 6)
);

-- HASH 分区
CREATE TABLE logs (
    id INT,
    message TEXT
) PARTITION BY HASH (id) PARTITIONS 4;
```

### 1.5 DROP TABLE

```sql
DROP TABLE [IF EXISTS] table_name;
```

### 1.6 TRUNCATE TABLE

```sql
TRUNCATE TABLE table_name;
```

### 1.7 ALTER TABLE

```sql
ALTER TABLE table_name ADD [COLUMN] column_name data_type [constraints];
ALTER TABLE table_name DROP [COLUMN] column_name;
ALTER TABLE table_name MODIFY [COLUMN] column_name new_type;
ALTER TABLE table_name RENAME TO new_name;
ALTER TABLE table_name ADD PRIMARY KEY (col1, ...);
ALTER TABLE table_name DROP PRIMARY KEY;
ALTER TABLE table_name ADD FOREIGN KEY (col) REFERENCES ref_table (ref_col);
```

### 1.8 CREATE VIEW / DROP VIEW

```sql
CREATE VIEW view_name AS select_statement;
DROP VIEW [IF EXISTS] view_name;
```

### 1.9 CREATE INDEX / DROP INDEX

```sql
CREATE [UNIQUE] INDEX index_name ON table_name (col1, col2, ...);
DROP INDEX index_name ON table_name;
```

---

## 2. DML (数据操作语言)

### 2.1 INSERT

```sql
INSERT INTO table_name VALUES (val1, val2, ...);
INSERT INTO table_name (col1, col2, ...) VALUES (val1, val2, ...);
INSERT INTO table_name SELECT ... FROM ...;
```

### 2.2 REPLACE

```sql
REPLACE INTO table_name VALUES (val1, val2, ...);
REPLACE INTO table_name (col1, col2, ...) VALUES (val1, val2, ...);
```

### 2.3 SELECT

```sql
SELECT [ALL | DISTINCT]
    select_expr [[AS] alias], ...
  [FROM table_references
    [JOIN table ON join_condition]
    [LEFT | RIGHT | FULL OUTER JOIN table ON join_condition]
  ]
  [WHERE where_condition]
  [GROUP BY expr [, ...]]
  [HAVING having_condition]
  [WINDOW window_name AS (window_spec)]
  [ORDER BY expr [ASC | DESC] [, ...]]
  [LIMIT {count | offset, count}]
  [OFFSET offset]
```

### 2.4 UPDATE

```sql
UPDATE table_name SET col1 = val1, col2 = val2, ... [WHERE condition];
```

### 2.5 DELETE

```sql
DELETE FROM table_name [WHERE condition];
```

---

## 3. 窗口函数

```sql
function_name(arg_expr) OVER (
    [PARTITION BY expr [, ...]]
    [ORDER BY expr [ASC | DESC] [, ...]]
    [frame_clause]
)

-- 支持的函数:
-- ROW_NUMBER(), RANK(), DENSE_RANK()
-- LAG(expr, offset, default), LEAD(expr, offset, default)
-- FIRST_VALUE(expr), LAST_VALUE(expr)
-- NTILE(n), NTH_VALUE(expr, n)
-- PERCENT_RANK(), CUME_DIST()

-- frame_clause:
ROWS BETWEEN frame_start AND frame_end
-- frame_start: UNBOUNDED PRECEDING | n PRECEDING | CURRENT ROW
-- frame_end:   CURRENT ROW | n FOLLOWING | UNBOUNDED FOLLOWING
```

---

## 4. 聚合函数

| 函数 | 描述 |
|------|------|
| COUNT(*) | 行数计数 |
| COUNT(expr) | 非 NULL 值计数 |
| SUM(expr) | 求和 |
| AVG(expr) | 平均值 |
| MIN(expr) | 最小值 |
| MAX(expr) | 最大值 |
| GROUP_CONCAT(expr) | 字符串聚合 |

---

## 5. 表达式和运算符

### 5.1 算术运算符

```
+, -, *, /, %
```

### 5.2 比较运算符

```
=, <>, !=, <, <=, >, >=
IS NULL, IS NOT NULL
BETWEEN ... AND ...
IN (...)
LIKE (%, _ 通配符)
```

### 5.3 逻辑运算符

```
AND, OR, NOT
```

### 5.4 聚合支持

- DISTINCT: `SELECT DISTINCT col FROM table`
- HAVING: `GROUP BY ... HAVING aggregate_condition`

---

## 6. 事务控制

```sql
BEGIN TRANSACTION;
COMMIT;
ROLLBACK;

-- 设置隔离级别
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
```

---

## 7. 实用命令 (REPL)

```sql
.quit          -- 退出 REPL
.help          -- 显示帮助
.tables        -- 列出所有表
.schema table  -- 显示表结构
.status        -- 显示状态
```

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*

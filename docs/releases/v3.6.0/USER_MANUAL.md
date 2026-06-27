# SQLRustGo v3.6.0 用户手册

> **版本**: v3.6.0
> **HEAD**: 1b2a3c71
> **日期**: 2026-05-30
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 1. 环境要求

| 依赖 | 最低版本 | 推荐版本 |
|------|----------|----------|
| Rust | 1.70+ | 1.85+ |
| Cargo | 随 Rust | 1.85+ |
| 内存 | 4GB | 16GB+ (全量测试) |
| 磁盘 | 500MB | 10GB+ (含测试数据) |
| OS | Linux/macOS | Ubuntu 22.04+ / macOS 14+ |

---

## 2. 构建

### 2.1 获取源码

```bash
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v3.6.0
```

### 2.2 开发构建 (快速)

```bash
cargo build
```

### 2.3 发布构建 (优化)

```bash
cargo build --release
```

### 2.4 全特性构建

```bash
cargo build --all-features
```

### 2.5 常用特性标志

```bash
# SIMD 向量化加速
cargo build --features simd

# WAL 验证模块
cargo build --features wal-verification

# Knowledge OS (qmd-bridge) 集成
cargo build --features qmd-bridge

# 全量
cargo build --all-features
```

---

## 3. 运行

### 3.1 REPL 模式 (交互式 SQL)

```bash
cargo run --bin sqlrustgo
```

进入 REPL 后，可以直接输入 SQL 语句：

```sql
sqlrustgo> CREATE TABLE users (id INT PRIMARY KEY, name TEXT, age INT);
sqlrustgo> INSERT INTO users VALUES (1, 'Alice', 30);
sqlrustgo> SELECT * FROM users;
+----+-------+-----+
| id | name  | age |
+----+-------+-----+
| 1  | Alice | 30  |
+----+-------+-----+
sqlrustgo> .quit
```

### 3.2 服务器模式

```bash
# 启动 TCP 服务器 (默认端口 3306)
cargo run --bin sqlrustgo-server

# 指定端口
cargo run --bin sqlrustgo-server -- --port 3307
```

### 3.3 MySQL 客户端连接

```bash
mysql -h 127.0.0.1 -P 3306 -u root
```

---

## 4. 基本 SQL 用法

### 4.1 数据库操作

```sql
-- 创建数据库
CREATE DATABASE mydb;

-- 使用数据库
USE mydb;

-- 删除数据库
DROP DATABASE mydb;
```

### 4.2 表操作

```sql
-- 创建表
CREATE TABLE employees (
    id INT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    department VARCHAR(50),
    salary DECIMAL(10,2) CHECK (salary > 0),
    hire_date DATE
);

-- 查看表结构
DESCRIBE employees;

-- 删除表
DROP TABLE employees;

-- 清空表
TRUNCATE TABLE employees;
```

### 4.3 数据操作

```sql
-- 插入
INSERT INTO employees VALUES (1, 'Alice', 'Engineering', 80000, '2024-01-15');
INSERT INTO employees (id, name, department) VALUES (2, 'Bob', 'Marketing');

-- REPLACE (v3.6.0 支持)
REPLACE INTO employees VALUES (1, 'Alice', 'Engineering', 90000, '2024-01-15');

-- 查询
SELECT * FROM employees WHERE department = 'Engineering';
SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 10;

-- 聚合
SELECT department, AVG(salary) AS avg_salary
FROM employees
GROUP BY department
HAVING avg_salary > 50000;

-- JOIN
SELECT e.name, d.dept_name
FROM employees e
JOIN departments d ON e.department = d.dept_name;

-- 更新
UPDATE employees SET salary = salary * 1.1 WHERE department = 'Engineering';

-- 删除
DELETE FROM employees WHERE id = 2;
```

### 4.4 窗口函数

```sql
SELECT
    name,
    salary,
    ROW_NUMBER() OVER (ORDER BY salary DESC) AS row_num,
    RANK() OVER (ORDER BY salary DESC) AS rank,
    DENSE_RANK() OVER (ORDER BY salary DESC) AS dense_rank,
    LAG(salary, 1) OVER (ORDER BY salary) AS prev_salary,
    LEAD(salary, 1) OVER (ORDER BY salary) AS next_salary
FROM employees;
```

### 4.5 分区表

```sql
CREATE TABLE sales (
    id INT,
    amount DECIMAL(10,2),
    sale_date DATE
)
PARTITION BY RANGE (YEAR(sale_date)) (
    PARTITION p2024 VALUES LESS THAN (2025),
    PARTITION p2025 VALUES LESS THAN (2026),
    PARTITION p_future VALUES LESS THAN MAXVALUE
);
```

---

## 5. 测试

```bash
# 运行所有库测试
cargo test --lib --workspace

# 运行特定 crate 测试
cargo test -p sqlrustgo-parser
cargo test -p sqlrustgo-executor

# 运行 WAL 测试
cargo test -p sqlrustgo-transaction -- wal

# 运行文档测试
cargo test --doc

# Clippy 检查
cargo clippy --all-features -- -D warnings

# 格式化检查
cargo fmt --all -- --check
```

---

## 6. 常见问题

| 问题 | 解决方案 |
|------|----------|
| 构建失败 | 确保 Rust 版本 >= 1.70, `rustup update` |
| 测试超时 | 增加内存至 8GB+, 或单独运行特定测试 |
| SIMD 不支持 | 确保 CPU 支持 AVX2, 或使用 `--features default` 禁用 SIMD |
| WAL 验证失败 | 检查磁盘权限和 WAL 目录路径 |

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*

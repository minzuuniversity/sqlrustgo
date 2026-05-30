# SQLRustGo v3.7.0 API Reference

> **版本**: v3.7.0  
> **日期**: 2026-05-30

---

## 1. 服务器 API

### 1.1 启动服务器

```bash
cargo run -p sqlrustgo --bin sqlrustgo-server -- --port 3306
```

### 1.2 连接参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--port` | 3306 | MySQL 协议端口 |
| `--host` | 0.0.0.0 | 监听地址 |
| `--data-dir` | /tmp/sqlrustgo | 数据存储目录 |

---

## 2. SQL API

### 2.1 Data Definition Language (DDL)

| 语句 | 语法 | 支持 |
|------|------|------|
| CREATE TABLE | `CREATE TABLE t (col type [PRIMARY KEY])` | ✅ |
| DROP TABLE | `DROP TABLE t` | ✅ |
| CREATE DATABASE | `CREATE DATABASE db` | ✅ |
| USE DATABASE | `USE db` | ✅ |

### 2.2 Data Manipulation Language (DML)

| 语句 | 语法 | 支持 |
|------|------|------|
| SELECT | `SELECT cols FROM t WHERE cond` | ✅ |
| INSERT | `INSERT INTO t VALUES (...)` | ✅ |
| UPDATE | `UPDATE t SET col=val WHERE cond` | ✅ |
| DELETE | `DELETE FROM t WHERE cond` | ✅ |

### 2.3 Transaction

| 语句 | 语法 | 支持 |
|------|------|------|
| BEGIN | `BEGIN` | ✅ |
| COMMIT | `COMMIT` | ✅ |
| ROLLBACK | `ROLLBACK` | ❌ (v3.8.0) |

---

## 3. 类型系统

| 类型 | 支持 |
|------|------|
| INT | ✅ |
| BIGINT | ✅ |
| FLOAT | ✅ |
| DOUBLE | ✅ |
| TEXT | ✅ |
| VARCHAR(n) | ✅ |
| BOOLEAN | ✅ |
| DATE | ✅ |
| TIMESTAMP | ✅ |

---

## 4. 表达式

| 表达式 | 支持 |
|--------|------|
| `+ - * /` | ✅ |
| `= <> < > <= >=` | ✅ |
| `AND OR NOT` | ✅ |
| `IS NULL / IS NOT NULL` | ✅ |
| `IN (val, ...)` | ✅ |
| `BETWEEN val AND val` | ✅ |
| `LIKE '%pattern%'` | ✅ |

---

## 5. 聚合函数

| 函数 | 支持 |
|------|------|
| COUNT(*) | ✅ |
| COUNT(col) | ✅ |
| SUM(col) | ✅ |
| AVG(col) | ✅ |
| MIN(col) | ✅ |
| MAX(col) | ✅ |

---

## 6. 窗口函数

| 函数 | 支持 |
|------|------|
| ROW_NUMBER() | ✅ |
| RANK() | ✅ |
| DENSE_RANK() | ✅ |
| FIRST_VALUE(col) | ✅ |
| LAST_VALUE(col) | ✅ |

---

## 7. 系统表

```sql
-- 信息schema (有限支持)
SELECT * FROM information_schema.tables;
SELECT * FROM information_schema.columns;
```

---

## 8. 错误代码

| 代码 | 描述 |
|------|------|
| ER_PARSE_ERROR | SQL 语法错误 |
| ER_NO_SUCH_TABLE | 表不存在 |
| ER_DUP_ENTRY | 唯一键冲突 |
| ER_ACCESS_DENIED_ERROR | 认证失败 |
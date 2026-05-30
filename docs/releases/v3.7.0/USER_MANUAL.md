# SQLRustGo v3.7.0 用户手册

> **版本**: v3.7.0  
> **分支**: `origin/develop/v3.7.0` (commit `66d13cf1`)  
> **日期**: 2026-05-30  
> **状态**: GA (待发布)

---

## 1. 产品概述

**SQLRustGo** 是一个用 Rust 实现的关系型数据库 SQL 执行引擎，支持 MySQL wire protocol。

**v3.7.0 定位**: MySQL-compatible SQL Execution Engine with Session-level Transaction Semantics

### 核心功能

| 功能 | 支持状态 | 说明 |
|------|----------|------|
| SQL Parser | ✅ | SELECT/INSERT/UPDATE/DELETE/CREATE TABLE |
| MySQL Wire Protocol | ✅ | mysql CLI 连接 |
| Session-level Transactions | ✅ | BEGIN/COMMIT (COMMIT 持久化) |
| Storage Engine | ✅ | In-memory + File-based |
| DDL/DML | ✅ | 完整 SQL 支持 |
| Prepared Statements | ✅ | 框架已实现 |

### 不包含功能 (v3.8.0)

- WAL / crash recovery
- Full MVCC / ROLLBACK
- ParallelVolcanoExecutor
- Single DML execution path

---

## 2. 快速开始

### 2.1 构建

```bash
cargo build --release -p sqlrustgo
```

### 2.2 启动服务器

```bash
cargo run -p sqlrustgo --bin sqlrustgo-server
```

### 2.3 连接

```bash
mysql -h 127.0.0.1 -P 3306 -u root -p
```

默认密码为空。

---

## 3. SQL 支持

### 3.1 DDL

```sql
CREATE TABLE t (id INT PRIMARY KEY, name TEXT);
DROP TABLE t;
```

### 3.2 DML

```sql
INSERT INTO t VALUES (1, 'alice');
SELECT * FROM t WHERE id = 1;
UPDATE t SET name = 'bob' WHERE id = 1;
DELETE FROM t WHERE id = 1;
```

### 3.3 Transaction

```sql
BEGIN;
INSERT INTO t VALUES (1, 'alice');
COMMIT;
```

---

## 4. 配置

### 4.1 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| SQLRUSTGO_PORT | 3306 | 服务端口 |
| SQLRUSTGO_DATA_DIR | /tmp/sqlrustgo | 数据目录 |

---

## 5. 限制

| 限制 | 说明 |
|------|------|
| 无 ROLLBACK | v3.7.0 不支持 ROLLBACK |
| 无 MVCC | 纯会话级隔离 |
| 无 WAL | crash recovery 需要 v3.8.0 |
| 无 SHOW TABLES | 信息schema 不完整 |

---

## 6. 故障排除

### 6.1 连接失败

检查服务器是否启动：`ps aux | grep sqlrustgo`

### 6.2 认证失败

确保使用正确用户：`mysql -u root` (无密码)

---

## 7. 联系方式

- Issue: http://192.168.0.252:3000/openclaw/sqlrustgo/issues
- Wiki: http://192.168.0.252:3000/openclaw/sqlrustgo/wiki
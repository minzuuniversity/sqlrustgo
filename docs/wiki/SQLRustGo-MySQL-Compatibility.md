# SQLRustGo v3.1.0 MySQL 兼容性矩阵

> **版本**: v3.1.0 Alpha
> **对比**: MySQL 5.7 / MySQL 8.0
> **状态**: Alpha 阶段
> **更新**: 2026-05-13

---

## 兼容性评分总览

| 维度 | v3.1.0 | MySQL 8.0 | 差距 |
|------|---------|-----------|------|
| SQL 语言覆盖度 | ~78% | 95% | -17% |
| 存储引擎 | ~75% | 95% | -20% |
| 事务 ACID | ~80% | 95% | -15% |
| 安全功能 | ~70% | 90% | -20% |
| 可观测性 | ~60% | 85% | -25% |
| 复制/HA | ~50% | 95% | -45% |

---

## DDL 功能矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| CREATE TABLE | ✅ | ✅ | ✅ | 完成 | |
| ALTER TABLE | ✅ | ✅ | ✅ | 完成 | ADD/DROP/MODIFY COLUMN |
| DROP TABLE | ✅ | ✅ | ✅ | 完成 | CASCADE/RESTRICT |
| CREATE INDEX | ✅ | ✅ | ✅ | 完成 | |
| DROP INDEX | ✅ | ✅ | ✅ | 完成 | |
| CREATE VIEW | ✅ | ✅ | ✅ | 完成 | |
| DROP VIEW | ✅ | ✅ | ✅ | 完成 | |
| CREATE DATABASE | ✅ | ✅ | ✅ | 完成 | |
| DROP DATABASE | ✅ | ✅ | ✅ | 完成 | |
| TRUNCATE TABLE | ✅ | ✅ | ✅ | 完成 | |
| RENAME TABLE | ✅ | ✅ | ✅ | 完成 | |
| CREATE SEQUENCE | ❌ | ✅ | ❌ | 未实现 | v3.2.0 |
| FOREIGN KEY | ✅ | ✅ | ⚠️ | 部分 | 语法解析，约束执行待完善 |

---

## DML 功能矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| SELECT | ✅ | ✅ | ✅ | 完成 | |
| INSERT | ✅ | ✅ | ✅ | 完成 | |
| UPDATE | ✅ | ✅ | ✅ | 完成 | |
| DELETE | ✅ | ✅ | ✅ | 完成 | |
| REPLACE INTO | ✅ | ✅ | ✅ | 完成 | |
| INSERT...SELECT | ✅ | ✅ | ✅ | 完成 | |
| **MERGE INTO** | ❌ | ✅ | ✅ | **新增** | v3.1.0 Alpha |
| LOAD DATA | ✅ | ✅ | ⚠️ | 部分 | 基础功能 |
| BATCH INSERT | ✅ | ✅ | ⚠️ | 部分 | 需优化 |
| Multi-table UPDATE | ✅ | ✅ | ⚠️ | 部分 | v3.1.0 补全 |
| Multi-table DELETE | ✅ | ✅ | ⚠️ | 部分 | v3.1.0 补全 |

---

## 事务功能矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| BEGIN/COMMIT/ROLLBACK | ✅ | ✅ | ✅ | 完成 | |
| SAVEPOINT | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| ROLLBACK TO SAVEPOINT | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| SET TRANSACTION | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| LOCK TABLES | ✅ | ✅ | ⚠️ | 部分 | 表级锁 |
| XA TRANSACTION | ✅ | ✅ | ✅ | 完成 | |

---

## 隔离级别矩阵

| 级别 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| READ UNCOMMITTED | ✅ | ✅ | ⚠️ | 基础 | 需完善 |
| READ COMMITTED | ✅ | ✅ | ✅ | 完成 | |
| REPEATABLE READ | ✅ | ✅ | ✅ | 完成 | |
| SERIALIZABLE | ✅ | ✅ | ✅ | 完成 | v3.1.0 完善 Gap Lock |

---

## 查询功能矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| WHERE | ✅ | ✅ | ✅ | 完成 | |
| JOIN (INNER/LEFT/RIGHT) | ✅ | ✅ | ✅ | 完成 | |
| CROSS JOIN | ✅ | ✅ | ✅ | 完成 | |
| NATURAL JOIN | ✅ | ✅ | ✅ | 完成 | |
| GROUP BY | ✅ | ✅ | ✅ | 完成 | |
| HAVING | ✅ | ✅ | ✅ | 完成 | |
| ORDER BY | ✅ | ✅ | ✅ | 完成 | v3.1.0 优化 |
| LIMIT/OFFSET | ✅ | ✅ | ✅ | 完成 | v3.1.0 优化 |
| DISTINCT | ✅ | ✅ | ✅ | 完成 | |
| UNION/UNION ALL | ✅ | ✅ | ✅ | 完成 | |
| Subquery | ✅ | ✅ | ✅ | 完成 | |
| CTE (WITH) | ✅ | ✅ | ✅ | 完成 | 非递归 |
| **Window Functions** | ✅ | ✅ | ✅ | **完成** | 全部 14 种 |
| CASE WHEN | ✅ | ✅ | ✅ | 完成 | |
| COALESCE | ✅ | ✅ | ✅ | 完成 | |
| CAST | ✅ | ✅ | ✅ | 完成 | |

### 窗口函数详情 (v3.1.0)

| 函数 | 状态 | 说明 |
|------|------|------|
| ROW_NUMBER | ✅ | |
| RANK | ✅ | |
| DENSE_RANK | ✅ | |
| PERCENT_RANK | ✅ | |
| CUME_DIST | ✅ | |
| LEAD | ✅ | v3.1.0 完成 |
| LAG | ✅ | v3.1.0 完成 |
| FIRST_VALUE | ✅ | v3.1.0 完成 |
| LAST_VALUE | ✅ | v3.1.0 完成 |
| NTH_VALUE | ✅ | v3.1.0 完成 |
| NTILE | ✅ | v3.1.0 完成 |

---

## 索引类型矩阵

| 类型 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| B-Tree Index | ✅ | ✅ | ✅ | 完成 | |
| HASH Index (MEMORY) | ✅ | ✅ | ⚠️ | 基础 | |
| FULLTEXT Index | ✅ | ✅ | ⚠️ | 基础 | v3.1.0 基础 |
| **Clustered Index** | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| **Secondary Index** | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| **Gap Locking** | ✅ | ✅ | ✅ | **新增** | v3.1.0 |

---

## 存储引擎矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| WAL + MVCC | ❌ | ❌ | ✅ | 自主 | |
| ACID Transactions | ✅ | ✅ | ✅ | 完成 | |
| Crash Recovery | ✅ | ✅ | ✅ | 完成 | |
| **Gap Locking** | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| **Next-Key Lock** | ✅ | ✅ | ✅ | **新增** | v3.1.0 |
| **Page Encryption** | ✅ | ✅ | ✅ | **新增** | AES-256 |
| Buffer Pool | ✅ | ✅ | ✅ | 完成 | |
| Doublewrite Buffer | ✅ | ✅ | ⚠️ | 无需 | 非 InnoDB |

---

## 安全功能矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| RBAC (GRANT/REVOKE) | ✅ | ✅ | ✅ | 完成 | |
| **Column-level Privileges** | ✅ | ✅ | ✅ | **完成** | v3.1.0 |
| Row-level Security | ❌ | ✅ | ⚠️ | 基础 | v3.1.0 基础 |
| TLS/SSL | ✅ | ✅ | ✅ | 完成 | |
| Password Policy | ✅ | ✅ | ⚠️ | 基础 | |
| **Audit Log (SHA-256)** | ✅ | ✅ | ✅ | **完成** | v3.1.0 审计链 |
| **AES-256 Storage** | ✅ | ✅ | ✅ | **完成** | v3.1.0 |

---

## 可观测性矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| INFORMATION_SCHEMA | ✅ | ✅ | ⚠️ | 80%+ | v3.1.0 扩展 |
| Performance Schema | ✅ | ✅ | ⚠️ | 50% | |
| EXPLAIN | ✅ | ✅ | ✅ | 完成 | |
| EXPLAIN ANALYZE | ❌ | ✅ | ✅ | 完成 | |
| SHOW PROCESSLIST | ✅ | ✅ | ✅ | 完成 | |
| SHOW VARIABLES | ✅ | ✅ | ✅ | 完成 | |
| SHOW EVENTS | ✅ | ✅ | ✅ | v3.1.0 | |
| Slow Query Log | ✅ | ✅ | ✅ | 完成 | |
| Error Log | ✅ | ✅ | ⚠️ | 基础 | |

---

## 复制与高可用矩阵

| 功能 | MySQL 5.7 | MySQL 8.0 | v3.1.0 | 状态 | 说明 |
|------|-----------|-----------|---------|------|------|
| Async Replication | ✅ | ✅ | ⚠️ | 基础 | |
| Semi-sync Replication | ✅ | ✅ | ✅ | 完成 | |
| XA Transactions | ✅ | ✅ | ✅ | 完成 | |
| Group Replication | ❌ | ✅ | ❌ | v3.2.0 | |
| Automatic Failover | ❌ | ✅ | ❌ | v3.2.0 | |

---

## 功能差距汇总

### ✅ 已完成 (vs MySQL 8.0)

- SELECT/INSERT/UPDATE/DELETE 完整
- MERGE INTO (v3.1.0 新增)
- 窗口函数 14 种全部
- SAVEPOINT / ROLLBACK TO SAVEPOINT
- Clustered Index / Secondary Index
- Gap Locking / Next-Key Lock
- AES-256 存储加密
- SHA-256 审计链
- 半同步复制

### ⚠️ 部分完成

| 功能 | 当前状态 | 目标 |
|------|---------|------|
| FULLTEXT Index | 基础 | 完整 |
| Row-level Security | 基础 | 完整 |
| LOAD DATA | 基础 | 完整 |
| Performance Schema | 50% | 80% |
| Async Replication | 基础 | 完整 |

### ❌ 未实现 (v3.2.0+)

| 功能 | 说明 |
|------|------|
| Group Replication | 组复制 |
| Automatic Failover | 自动故障转移 |
| CREATE SEQUENCE | 序列 |
| 存储过程增强 | 完整 |

---

## 相关页面

- [v3.1.0 Overview](../wiki/v3.1.0-Overview) — 版本总览
- [OO Document Status](../wiki/OO-Document-Status) — OO 文档状态
- [SQLRustGo-Overview](../wiki/SQLRustGo-Overview) — 项目总览

---

*最后更新: 2026-05-13*

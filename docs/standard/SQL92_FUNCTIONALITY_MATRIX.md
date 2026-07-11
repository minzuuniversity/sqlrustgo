# SQLRustGo SQL-92 功能集成矩阵

> **版本**: v3.9.0 (当前)
> **更新日期**: 2026-06-27
> **维护人**: yinglichina8848
> **目的**: 系统记录已实现 SQL 功能在各层（Parser → Executor → Storage → Transaction）的完整覆盖度

---

## 一、总览

| 层 | 作用 | 文件 |
|----|------|------|
| **Parser** | SQL 字符串 → AST (Statement) | `crates/parser/src/parser.rs` |
| **Executor** | AST / 物理计划 → 结果集 | `crates/executor/src/local_executor.rs` |
| **Storage** | 数据持久化与检索 | `crates/storage/src/engine.rs` |
| **Transaction** | WAL + MVCC + SSI | `crates/transaction/src/` |

### 命名约定

| 标记 | 含义 |
|------|------|
| ✅ | 完整实现 |
| ⚠️ | 部分实现 / 需验证 |
| ❌ | 仅解析 / 占位符 / 未实现 |
| N/A | 不适用 (该层无需实现) |

---

## 二、DML (Data Manipulation Language)

### 2.1 SELECT

| 子功能 | Parser | Executor | 证据 |
|--------|--------|----------|------|
| SELECT * FROM t | ✅ | ✅ `execute_seq_scan` | `parser.rs:1738` |
| SELECT col1, col2 | ✅ | ✅ `execute_projection` | `parser.rs:1856` |
| WHERE | ✅ | ✅ `execute_filter` | `parser.rs:1856` |
| GROUP BY + 聚合 | ✅ | ✅ `execute_aggregate` | `parser.rs:1856` |
| HAVING | ✅ | ✅ | `parser.rs` HAVING handling |
| ORDER BY | ✅ | ✅ `execute_sort` | `parser.rs`, `local_executor.rs:1317` |
| LIMIT | ✅ | ✅ `execute_limit` | `parser.rs`, `local_executor.rs:1428` |
| DISTINCT | ✅ | ⚠️ 需验证 | `parser.rs` |
| JOIN (INNER) | ✅ | ✅ `execute_hash_join` | `parser.rs`, `local_executor.rs:730` |
| JOIN (LEFT/RIGHT/FULL) | ✅ | ✅ `execute_hash_join` | `parser.rs`, `full_outer_join_test.rs` |
| JOIN (CROSS) | ✅ | ✅ | `parser.rs:多个 JOIN 类型` |
| Subquery (FROM) | ✅ | ✅ | `parser.rs:3826` |
| Subquery (WHERE) | ✅ | ✅ | `parser.rs:5326` |
| Subquery (EXISTS) | ✅ | ✅ | `parser.rs:5412` |
| UNION / UNION ALL | ✅ | ✅ | `parser.rs:1742` |
| WITH CTE (SELECT) | ✅ | ⚠️ | `parser.rs:1834` |
| WITH CTE (DML) | ✅ | ⚠️ | `parser.rs:1834-1853` |

### 2.2 Window Functions

| 函数 | Parser | Executor | 证据 |
|------|--------|----------|------|
| ROW_NUMBER() | ✅ | ✅ | `window_executor.rs:185` |
| RANK() | ✅ | ✅ | `window_executor.rs:186` |
| DENSE_RANK() | ✅ | ✅ | `window_executor.rs:187` |
| LEAD() | ✅ | ✅ | `window_executor.rs:188` |
| LAG() | ✅ | ✅ | `window_executor.rs:200` |
| FIRST_VALUE() | ✅ | ✅ | `window_executor.rs:212` |
| LAST_VALUE() | ✅ | ✅ | `window_executor.rs:222` |
| NTH_VALUE() | ✅ | ✅ | `window_executor.rs:232` |
| SUM() OVER | ✅ | ✅ | `window_executor.rs:252` |
| AVG() OVER | ✅ | ✅ | `window_executor.rs:262` |
| COUNT() OVER | ✅ | ✅ | `window_executor.rs:276` |
| MIN() OVER | ✅ | ✅ | `window_executor.rs:289` |
| MAX() OVER | ✅ | ✅ | `window_executor.rs:290` |
| PERCENT_RANK() | ✅ | ✅ | `window_executor.rs:298` |
| CUME_DIST() | ✅ | ✅ | `window_executor.rs:312` |

### 2.3 INSERT / UPDATE / DELETE / MERGE

| 操作 | Parser | Executor (SQL) | Executor (计划) | Storage | Transaction | 综合 |
|------|--------|----------------|----------------|---------|-------------|------|
| INSERT VALUES | ✅ | ✅ | ✅ | ✅ | ✅ | **✅** |
| INSERT SELECT | ✅ | ⚠️ 需验证 | ✅ | ✅ | ✅ | **⚠️** |
| INSERT ON DUPLICATE KEY UPDATE | ✅ | ❌ | ❌ | ❌ | ❌ | **❌** |
| REPLACE | ✅ | ⚠️ 需验证 | ⚠️ | ✅ | ✅ | **⚠️** |
| UPDATE | ✅ | ✅ | ✅ | ✅ | ✅ | **✅** |
| UPDATE WHERE | ✅ | ✅ | ✅ | ✅ | ✅ | **✅** |
| DELETE | ✅ | ✅ | ✅ | ✅ | ✅ | **✅** |
| DELETE WHERE | ✅ | ✅ | ✅ | ✅ | ✅ | **✅** |
| MERGE | ✅ | ❌ | ⚠️ (No Subquery) | ✅ | ✅ | **⚠️** |

#### Executor 层 DML 实现详情

| 路径 | 方法 | 状态 | 文件:行 |
|------|------|------|---------|
| INSERT 物理计划 | `execute_insert()` | ✅ | `local_executor.rs` |
| INSERT SQL 文本 | `execute_insert_sql()` | ✅ | `local_executor.rs:1625` |
| UPDATE 物理计划 | `execute_update()` | ✅ | `local_executor.rs:1371` |
| UPDATE SQL 文本 | `execute_update_sql()` | ✅ | `local_executor.rs:1677` |
| DELETE 物理计划 | `execute_delete()` | ✅ | `local_executor.rs:1331` |
| DELETE SQL 文本 | `execute_delete_sql()` | ✅ | `local_executor.rs:1586` |
| MERGE 物理计划 | `MergeExecutor::execute_merge()` | ⚠️ (无 Subquery) | `merge.rs` |
| MERGE SQL 文本 | 缺失 | ❌ | `local_executor.rs:1562` |

---

## 三、DDL (Data Definition Language)

### 3.1 CREATE/DROP/ALTER

| 操作 | Parser | Executor | Storage | 综合 |
|------|--------|----------|---------|------|
| CREATE TABLE | ✅ | ⚠️ 间接 | ✅ | **⚠️** |
| DROP TABLE | ✅ | ⚠️ 间接 | ✅ | **⚠️** |
| ALTER TABLE ADD COLUMN | ✅ | ⚠️ 间接 | ✅ | **⚠️** |
| ALTER TABLE DROP COLUMN | ✅ | ⚠️ 间接 | ❌ | **❌** |
| ALTER TABLE RENAME TO | ✅ | ⚠️ 间接 | ✅ | **⚠️** |
| ALTER TABLE MODIFY COLUMN | ✅ | ⚠️ 间接 | ❌ | **❌** |
| TRUNCATE | ✅ | ⚠️ 间接 | ❌ | **❌** |

### 3.2 CREATE TABLE 详细约束

| 约束 | Parser | Storage | 证据 |
|------|--------|---------|------|
| PRIMARY KEY | ✅ | ✅ | `parser.rs:7718` |
| NOT NULL | ✅ | ✅ | `parser.rs:7732` |
| FOREIGN KEY | ✅ (行级+表级) | ✅ | `parser.rs:7646,7691` |
| UNIQUE | ✅ | ✅ | `parser.rs` |
| REFERENCES | ✅ | ✅ | `parser.rs:7646` |
| AUTO_INCREMENT | ✅ | ⚠️ | `parser.rs:7746` |
| DEFAULT | ✅ | ⚠️ | `parser.rs` |

### 3.3 Index / View / Trigger / Procedure

| 操作 | Parser | Executor | Storage | 综合 |
|------|--------|----------|---------|------|
| CREATE INDEX | ✅ | ⚠️ 间接 | ✅ | **⚠️** |
| DROP INDEX | ✅ | ⚠️ 间接 | ✅ | **⚠️** |
| CREATE VIEW | ✅ | ❌ | ❌ | **❌** |
| DROP VIEW | ✅ | ❌ | ❌ | **❌** |
| CREATE TRIGGER | ✅ | ❌ | ⚠️ 存Meta | **❌** |
| CREATE PROCEDURE | ✅ | ❌ | ❌ | **❌** |
| CALL | ✅ | ❌ | ❌ | **❌** |

### 3.4 Database / Role

| 操作 | Parser | Executor | Storage | 综合 |
|------|--------|----------|---------|------|
| CREATE DATABASE | ✅ | ⚠️ | ⚠️ 部分 | **⚠️** |
| DROP DATABASE | ✅ | ⚠️ | ⚠️ 部分 | **⚠️** |
| USE DATABASE | ✅ | ⚠️ | ✅ | **⚠️** |
| CREATE ROLE | ✅ | ⚠️ | ❌ | **❌** |
| GRANT | ✅ | ⚠️ | ❌ | **❌** |

---

## 四、Transaction 控制

### 4.1 SQL 事务语句

| 语句 | Parser | Executor | Transaction | 综合 |
|------|--------|----------|-------------|------|
| BEGIN | ✅ | ✅ | ✅ | **✅** |
| BEGIN WORK | ✅ | ✅ | ✅ | **✅** |
| START TRANSACTION | ✅ | ✅ | ✅ | **✅** |
| COMMIT | ✅ | ✅ | ✅ | **✅** |
| ROLLBACK | ✅ | ✅ | ✅ | **✅** |
| SAVEPOINT | ✅ | ⚠️ | ⚠️ | **⚠️** |
| ROLLBACK TO SAVEPOINT | ✅ | ⚠️ | ⚠️ | **⚠️** |
| RELEASE SAVEPOINT | ✅ | ⚠️ | ✅ | **⚠️** |
| SET TRANSACTION | ✅ | ⚠️ | ✅ | **⚠️** |

### 4.2 隔离级别

| 级别 | Parser | Transaction | 证据 |
|------|--------|-------------|------|
| READ UNCOMMITTED | ✅ | ⚠️ | `transaction_manager.rs` |
| READ COMMITTED | ✅ | ⚠️ | `manager.rs` |
| REPEATABLE READ | ✅ | ❌ | - |
| SERIALIZABLE | ✅ | ✅ (SSI) | `ssi.rs` |
| SNAPSHOT | ✅ | ✅ | `mvcc.rs` |

### 4.3 MVCC 组件

| 组件 | 状态 | 证据 |
|------|------|------|
| VersionChain | ✅ | `version_chain.rs` |
| Snapshot visibility | ✅ | `mvcc.rs:Snapshot::is_visible()` |
| SSI conflict detection | ✅ | `ssi.rs` |
| Deadlock detection | ✅ | `deadlock.rs` |
| Row-level lock | ✅ | `lock.rs` |
| Savepoint undo log | ✅ API | `savepoint.rs` |
| Savepoint 物理回滚 | ⚠️ (deferred) | `savepoint.rs #3172` |

---

## 五、Storage 引擎

### 5.1 存储实现

| 引擎 | 持久化 | 索引 | WAL | 用途 |
|------|--------|------|-----|------|
| MemoryStorage | ❌ | ✅ | ❌ | 测试/缓存 |
| FileStorage | ✅ JSON | ✅ B+Tree | ⚠️ 基础 | 默认持久化 |
| WalStorage<S,T> | ✅ | ✅ | ✅ | 生产(事务) |
| BinaryStorage | ✅ 二进制 | ❌ | ❌ | 备份 |
| MmapVectorStore | ✅ | ✅ Vector | ❌ | 向量搜索 |

### 5.2 索引类型

| 索引 | Parser | Storage | 证据 |
|------|--------|---------|------|
| B+ Tree | ✅ | ✅ | `bplus_tree.rs` |
| Hash Index | ✅ | ✅ | `index_registry.rs` |
| Vector Index (HNSW) | ✅ | ✅ | `vector_storage.rs` |

### 5.3 约束

| 约束 | Storage | 证据 |
|------|---------|------|
| Primary Key | ✅ | `engine.rs ColumnDefinition.primary_key` |
| Foreign Key | ✅ | `engine.rs ForeignKeyConstraint` |
| Unique | ✅ | `engine.rs UniqueConstraint` |
| Check | ✅ | `engine.rs evaluate_check_constraint` |

---

## 六、其他 SQL 语句

| 语句 | Parser | Executor | 综合 |
|------|--------|----------|------|
| SHOW TABLES | ✅ | ✅ | **✅** |
| SHOW DATABASES | ✅ | ✅ | **✅** |
| SHOW COLUMNS | ✅ | ✅ | **✅** |
| SHOW CREATE TABLE | ✅ | ✅ | **✅** |
| SHOW INDEX | ✅ | ✅ | **✅** |
| DESCRIBE / DESC | ✅ | ✅ | **✅** |
| EXPLAIN | ✅ | ✅ | **✅** |
| ANALYZE | ✅ | ⚠️ | **⚠️** |
| PREPARE | ✅ | ⚠️ | **⚠️** |
| EXECUTE | ✅ | ⚠️ | **⚠️** |
| DEALLOCATE | ✅ | ⚠️ | **⚠️** |

---

## 七、Gap 汇总追踪

### 7.1 Parser-Executor Gap (Parser 能解析，但 Executor 不可执行)

| # | 功能 | 影响 | 优先级 | 责任层 |
|---|------|------|--------|--------|
| G1 | CREATE TABLE 执行路径 | DDL 不可执行 | P0 | Executor+Storage |
| G2 | DROP TABLE 执行路径 | DDL 不可执行 | P0 | Executor+Storage |
| G3 | CREATE INDEX 执行路径 | 索引不可创建 | P0 | Executor+Storage |
| G4 | ALTER TABLE 执行路径 | DDL 受限 | P1 | Executor+Storage |
| G5 | CREATE VIEW 执行路径 | View 无法使用 | P2 | All layers |
| G6 | CREATE PROCEDURE / CALL | 过程无法使用 | P3 | All layers |
| G7 | MERGE Subquery source | 功能受限 | P1 | Executor |
| G8 | MERGE SQL 文本路径 | 功能受限 | P1 | Executor |

### 7.2 Storage Gap (Storage 未实现的方法)

| # | 方法 | 影响 | 优先级 |
|---|------|------|--------|
| S1 | `drop_column()` | ALTER TABLE DROP COLUMN 失败 | P1 |
| S2 | `modify_column()` | ALTER TABLE MODIFY 失败 | P1 |

### 7.3 Transaction Gap

| # | 功能 | 影响 | 优先级 |
|---|------|------|--------|
| T1 | Savepoint MVCC 物理回滚 | ROLLBACK TO SAVEPOINT 不还原数据 | P0 |
| T2 | Read Committed 隔离级别 | 仅 Snapshot/Serializable 可用 | P2 |

### 7.4 测试覆盖 Gap

| # | 套件 | 状态 | 建议 |
|---|------|------|------|
| C1 | DDL 端到端测试 | ❌ 缺失 | Phase 1 补全 |
| C2 | MERGE Subquery 测试 | ❌ 缺失 | Phase 2 补全 |
| C3 | CTE 端到端测试 | ⚠️ 部分 | Phase 1 验证 |
| C4 | View 端到端测试 | ❌ 缺失 | Phase 4 补全 |

---

## 八、当前版本资源分配

根据 `V390_DEVELOPMENT_PLAN.md` (第 7 行)：

```
资源分配: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / 新 SQL 0%
```

**v3.9.0 不开发新 SQL 功能**，此文档仅用于记录和追踪。Phase 1-4 实现需在后续版本规划中分配资源。

---

*本文档由 yinglichina8848 维护*

# Tasks — V312-56B: SQL 教学 corpus 与多 oracle 对比

## Phase 1: 现状调研 ✅

- [x] 1.1 现有 compat 目录结构 - `tests/compat/mysql_v3_12/` (20 files)
- [x] 1.2 现有 SQL 文件格式 - 使用 `# name:` 和 `# expect:` 注释
- [x] 1.3 缺失项识别:
  - ❌ 没有 `teaching_sql_v3_12` 目录
  - ❌ 没有 manifest.yml
  - ❌ 没有 check_sqllogictest_v312.sh

## Phase 2: 创建 teaching_sql_v3_12 结构

- [ ] 2.1 创建 `tests/compat/teaching_sql_v3_12/` 目录
- [ ] 2.2 创建 manifest.yml 框架
- [ ] 2.3 创建子目录: select/, join/, group/, null/, order_limit/, subquery/, ddl/, dml/, error/, transaction/

## Phase 3: 核心 SQL 覆盖 (需要创建的文件)

### 3.1 SELECT 基础

- [ ] 3.1.1 SELECT * / column list → select/basic.sql
- [ ] 3.1.2 WHERE 条件 → select/where.sql
- [ ] 3.1.3 DISTINCT → select/distinct.sql
- [ ] 3.1.4 别名 (AS) → select/alias.sql

### 3.2 JOIN

- [ ] 3.2.1 INNER JOIN → join/inner_join.sql
- [ ] 3.2.2 LEFT JOIN → join/left_join.sql
- [ ] 3.2.3 多表 JOIN → join/multi_join.sql

### 3.3 GROUP BY + 聚合

- [ ] 3.3.1 GROUP BY 基础 → group/group_by.sql
- [ ] 3.3.2 HAVING → group/having.sql
- [ ] 3.3.3 聚合函数 → group/aggregate.sql

### 3.4 NULL 语义

- [ ] 3.4.1 IS NULL / IS NOT NULL → null/is_null.sql
- [ ] 3.4.2 COALESCE → null/coalesce.sql

### 3.5 ORDER BY / LIMIT

- [ ] 3.5.1 ORDER BY → order_limit/order_by.sql
- [ ] 3.5.2 LIMIT / OFFSET → order_limit/limit_offset.sql

### 3.6 子查询

- [ ] 3.6.1 标量子查询 → subquery/scalar_subquery.sql
- [ ] 3.6.2 IN / NOT IN → subquery/in_subquery.sql

### 3.7 DDL / DML

- [ ] 3.7.1 CREATE TABLE → ddl/create_table.sql
- [ ] 3.7.2 ALTER TABLE → ddl/alter_table.sql
- [ ] 3.7.3 INSERT → dml/insert.sql
- [ ] 3.7.4 UPDATE → dml/update.sql
- [ ] 3.7.5 DELETE → dml/delete.sql

### 3.8 错误语义

- [ ] 3.8.1 除零错误 → error/division_by_zero.sql
- [ ] 3.8.2 类型不匹配 → error/type_mismatch.sql

### 3.9 事务基础

- [ ] 3.9.1 BEGIN/COMMIT/ROLLBACK → transaction/basic_tx.sql

## Phase 4: manifest.yml 创建

- [ ] 4.1 定义 manifest schema (oracle, expected, owner, stage, issue_link, expiry)
- [ ] 4.2 为每个 SQL 文件注册条目
- [ ] 4.3 FAIL/SKIP 关联 issue/owner/expiry

## Phase 5: Gate 脚本

- [ ] 5.1 创建 `scripts/gate/check_sqllogictest_v312.sh`
- [ ] 5.2 区分 teaching/smoke/official corpus
- [ ] 5.3 确保 #[ignore] 静默通过被检测

## Phase 6: 验证

- [ ] 6.1 `bash scripts/gate/check_sqllogictest_v312.sh` PASS
- [ ] 6.2 `bash scripts/gate/check_gate_test_integrity.sh` PASS

## Acceptance Criteria

- [ ] manifest.yml 列出每个 SQL 文件的 oracle、期望状态、owner、适用阶段
- [ ] 每个文件至少有 SQLite oracle
- [ ] 每个 FAIL/SKIP 都有 issue link、owner、expiry、关闭边界
- [ ] check_sqllogictest_v312.sh 能区分三种状态
- [ ] teaching corpus 不允许 #[ignore] 静默通过

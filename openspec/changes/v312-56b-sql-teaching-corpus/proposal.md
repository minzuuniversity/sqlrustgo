# Proposal — V312-56B: SQL 教学 corpus 与多 oracle 对比

## Why

Issue #4252: 需要建立面向教学的 SQL corpus，不能把 full SQLite official corpus 的大目标和教学最小闭环混在一起。教学 corpus 必须覆盖 SELECT、JOIN、GROUP、NULL、ORDER、LIMIT、DDL、DML、错误语义和事务基础。

## What Changes

### 1. 教学 SQL Corpus 建立

创建 `tests/compat/teaching_sql_v3_12/manifest.yml`:
- 列出每个 SQL 文件的 oracle (SQLite/MySQL/PostgreSQL)
- 期望状态 (PASS/FAIL/SKIP)
- owner
- 适用阶段 (teaching/smoke/official)

### 2. Oracle 对比矩阵

| SQL 功能 | SQLite Oracle | MySQL Oracle | PostgreSQL Oracle |
|---|---|---|---|
| SELECT 基础 | ✓ | ✓ | ✓ |
| JOIN (INNER/LEFT/RIGHT) | ✓ | ✓ | ✓ |
| GROUP BY + 聚合 | ✓ | ✓ | ✓ |
| NULL 语义 | ✓ | ✓ | ✓ |
| ORDER BY | ✓ | ✓ | ✓ |
| LIMIT/OFFSET | ✓ | ✓ | ✓ |
| 子查询 | ✓ | ✓ | ✓ |
| DDL (CREATE/ALTER/DROP) | ✓ | - | - |
| DML (INSERT/UPDATE/DELETE) | ✓ | ✓ | - |
| 事务基础 | ✓ | ✓ | - |

### 3. FAIL/SKIP 策略

每个 FAIL/SKIP 必须包含:
- Issue link
- owner
- expiry
- 关闭边界 (什么条件下可以打开)

### 4. Gate 脚本

创建 `check_sqllogictest_v312.sh` 区分:
- `teaching corpus PASS` - 教学语料库全部 PASS
- `smoke PASS` - smoke 测试 PASS
- `official corpus NOT CLAIMED` - 官方语料库不宣称完成

## Capabilities

### New Capabilities

- **教学 SQL corpus** - 面向教学的最小 SQL 功能覆盖
- **多 oracle 对比** - SQLite/MySQL/PostgreSQL oracle 分级补充
- **manifest 系统** - 每个文件的元数据可追溯

### Modified Capabilities

- 现有 sqllogictest → 添加 teaching 层级

## Non-goals

- 不宣称完整 SQLite official corpus 完成
- teaching corpus 不允许 `#[ignore]` 静默通过
- 不实现 PostgreSQL/MySQL 特有的高级特性

## Acceptance Criteria

- [ ] 新增 `tests/compat/teaching_sql_v3_12/manifest.yml`
- [ ] 每个文件至少有 SQLite oracle
- [ ] 每个 FAIL/SKIP 都有 issue link、owner、expiry、关闭边界
- [ ] `check_sqllogictest_v312.sh` 能区分三种状态
- [ ] teaching corpus 不允许 `#[ignore]` 静默通过
- [ ] 运行 `bash scripts/gate/check_sqllogictest_v312.sh` PASS
- [ ] 运行 `cargo test -p sqlrustgo-sql-corpus --test corpus_test -- --nocapture` PASS

## Issue Reference

Issue #4252 (V312-56B)

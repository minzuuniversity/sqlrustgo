## Why

V312-21 MySQL Compatibility 与 SQL Surface Backlog

v3.7-v3.10 历史文档中记录的 MySQL 兼容和 SQL surface 缺口需要复核并补齐。当前缺少 SHOW TABLES/metadata、empty-password auth edge、prepared statements、ALTER TABLE、TIMESTAMP 等边界验证。

## What Changes

- 新增 `crates/sqlrustgo/tests/mysql_compat_show_tables.rs` — SHOW TABLES/metadata 测试
- 新增 `crates/sqlrustgo/tests/mysql_compat_auth_edge.rs` — empty-password auth edge 测试
- 更新 `crates/mysql-server/tests/` — prepared statements E2E 覆盖
- 更新 `crates/sqlrustgo/tests/mysql_compat_alter_table.rs` — ALTER TABLE RENAME/MODIFY/ADD/DROP
- 新增 `crates/sqlrustgo/tests/mysql_compat_timestamp.rs` — TIMESTAMP 边界测试
- 更新 `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` — explicit unsupported 清单

## Capabilities

### New Capabilities

- `mysql-compat-show-tables`: SHOW TABLES / information_schema metadata
- `mysql-compat-auth-edge`: empty-password authentication edge case
- `mysql-compat-alter-table`: ALTER TABLE RENAME/MODIFY/ADD/DROP
- `mysql-compat-timestamp`: TIMESTAMP 边界行为验证

### Explicitly Unsupported (Documented)

- ROLLUP/CUBE (deferred to future release)
- REPLACE INTO (deferred)
- RANK()/DENSE_RANK()/CUBE advanced aggregates (deferred)
- stored procedure tokens (deferred)
- column-level permissions (deferred)
- connection pool (deferred)

## Issue Reference

- GitHub Issue: #3908
- Milestone: v3.12.0
- Priority: P1

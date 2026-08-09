## Why

V312-13 MySQL Wire + LOAD DATA Hardening

MySQL wire protocol 和 bulk import 是 SQLRustGo 投入生产前的关键风险项。当前代码中 COM_STMT_PREPARE/EXECUTE/CLOSE、error packet、reset connection、TLS/compression 缺乏端到端测试；LOAD DATA INFILE 缺乏 row count + hash + memory cap + duration 证据。

## What Changes

- 新增 `crates/mysql-server/tests/wire_smoke_mysql_cli.rs` — COM_QUERY、COM_STMT_PREPARE/EXECUTE/CLOSE、error packet、reset、E2E 测试
- 新增 `crates/mysql-server/tests/load_data_sf_test.rs` — SF=1/SF=10 LOAD DATA 验证脚本
- 更新 `scripts/gate/check_load_data_infile.sh` 支持 SF=10
- 更新 `crates/mysql-server/src/lib.rs` 中 error packet 和 reset connection 处理

## Capabilities

### New Capabilities

- `mysql-wire-e2e`: MySQL wire protocol E2E 测试覆盖 COM_QUERY/COM_STMT_*/error/reset
- `load-data-sf10`: LOAD DATA SF=10 性能与正确性验证

## Issue Reference

- GitHub Issue: #3900
- Milestone: v3.12.0
- Priority: P0

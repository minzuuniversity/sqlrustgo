## Why

OLTP 混合负载压测验证高并发 INSERT 修复（V311-23）在复杂事务下的真实表现。单元测试无法覆盖真实并发场景。

## What Changes

- 新增 `scripts/sysbench/run_oltp.sh` — Sysbench OLTP 压测运行脚本
- 新增 `scripts/sysbench/oltp_schema.sql` — OLTP 测试 schema

## Capabilities

### New Capabilities

- `sysbench-oltp`: Sysbench OLTP 混合负载压测能力

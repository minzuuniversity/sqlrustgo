## Why

增强 sqlrustgo-admin 运维能力，添加 `status` 和 `flush-logs` 命令。

## What Changes

- 新增 `scripts/admin/admin-status.sh` — 显示 Innodb_rows_read、Threads_connected、Uptime
- 新增 `scripts/admin/admin-flush-logs.sh` — FLUSH LOGS 命令

## Capabilities

### New Capabilities

- `admin-status`: Admin status 命令
- `admin-flush-logs`: Admin flush-logs 命令

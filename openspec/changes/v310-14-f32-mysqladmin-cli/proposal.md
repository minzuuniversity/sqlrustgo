## Why

F-32 (mysqladmin equivalent) was closed in v3.8.0 with 11/11 tests passing, but the implementation is an **in-test mock** (`tests/mysqladmin_test.rs::MysqlAdmin`) — a library struct instantiated directly in unit tests, not a real CLI binary. Issue #3768 (V310-14) requires converting this into a real `sqlrustgo-admin` CLI binary that can be invoked from the shell, completing the original intent of the F-32 feature.

## What Changes

- Add `status`, `reload`, `refresh`, `flush-tables`, `processlist`, `kill` subcommands to the existing `sqlrustgo-admin` binary in `crates/admin/`
- Refactor `tests/mysqladmin_test.rs::MysqlAdmin` into a shared `sqlrustgo_admin` library crate consumed by both the CLI binary and the tests
- Each subcommand connects to a running sqlrustgo server process via admin protocol (TCP) to execute the operation
- CLI flags mirror MySQL mysqladmin conventions where applicable

## Capabilities

### New Capabilities

- `mysqladmin-status`: Query server uptime, thread count, query throughput, slow query count
- `mysqladmin-reload`: Reload grant tables (no-op for sqlrustgo, but returns success)
- `mysqladmin-refresh`: Flush logs and tables (no-op for sqlrustgo, but returns success)
- `mysqladmin-flush-tables`: Close all open tables (no-op for sqlrustgo, but returns success)
- `mysqladmin-processlist`: List active server connections/queries
- `mysqladmin-kill`: Terminate a server connection by ID

## Impact

- **Modified**: `crates/admin/src/main.rs` — add 6 new subcommands
- **New**: `crates/admin/src/mysqladmin.rs` — mysqladmin command implementations
- **Modified**: `crates/admin/src/lib.rs` — export mysqladmin module
- **Modified**: `tests/mysqladmin_test.rs` — update to use the refactored `sqlrustgo_admin` crate library (backward compatible, same test assertions)
- **No new dependencies** — uses existing `sqlrustgo-storage` types for connection info

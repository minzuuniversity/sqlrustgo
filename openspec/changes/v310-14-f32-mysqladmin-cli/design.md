## Context

F-32 (mysqladmin equivalent) was closed in v3.8.0 with 11/11 tests passing, but all tests use `tests/mysqladmin_test.rs::MysqlAdmin` — an **in-process mock** that is instantiated directly in tests with no CLI binary. Issue #3768 (V310-14) requires building a real `sqlrustgo-admin` CLI binary that operators can invoke from a shell.

**Architecture overview**: `crates/mysql-server` is the MySQL wire-protocol server (listens on TCP port 3306). The existing `crates/admin` binary (`sqlrustgo-admin`) has backup/restore/pitr commands that operate on local files. The mysqladmin subcommands should be **client commands** that connect to a running sqlrustgo server via TCP (MySQL protocol), mirroring how real mysqladmin works.

**Constraints**:
- No admin protocol server side exists — use existing MySQL wire protocol
- Commands: `status`, `reload`, `refresh`, `flush-tables`, `processlist`, `kill`
- Default connection: `localhost:3306` as `root` with no password (dev defaults)
- CLI flags: `--host`, `--port`, `--user`, `--password`

## Goals / Non-Goals

**Goals:**
- Build `sqlrustgo-admin <subcommand>` CLI matching mysqladmin interface conventions
- Build `sqlrustgo-admin` CLI binary with real MySQL server connections
- `sqlrustgo-admin status` → connects via MySQL TCP, runs `SHOW STATUS` / `SHOW VARIABLES`
- `sqlrustgo-admin processlist` → runs `SELECT * FROM information_schema.processlist`
- `sqlrustgo-admin kill <id>` → runs `KILL <id>`
- `reload`, `refresh`, `flush-tables` → no-ops returning success (sqlrustgo has no grant tables or log files to reload)
- All 11 existing tests pass after refactoring

**Non-Goals:**
- No new server-side admin protocol — use existing MySQL wire protocol only
- No authentication beyond dev-mode root+empty-password (separate auth issue)
- `sqlrustgo-admin` binary lives in `crates/admin/` (not `crates/tools/`)
- Not required to make `tests/mysqladmin_test.rs` import from `sqlrustgo_admin` crate (tests keep their in-memory mock)

## Decisions

### 1. CLI entry point: extend `crates/admin` existing binary

The `crates/admin` crate already has a `sqlrustgo-admin` binary with clap subcommands. We add 6 new `Commands` variants instead of creating a new crate. This avoids binary proliferation.

### 2. Connection approach: `sqlrustgo-mysql-client` MySQL client library

`crates/mysql-client` already implements a MySQL client that connects via TCP. Reuse it for the mysqladmin subcommands rather than reimplementing the MySQL protocol.

### 3. `information_schema.processlist` for `processlist` command

The MySQL `information_schema.processlist` table contains Id, User, Host, db, Command, Time, State, Info columns — matching the fields in `tests/mysqladmin_test.rs::Connection`. Query it via `SELECT * FROM information_schema.processlist`.

### 4. `KILL <id>` for `kill` command

MySQL protocol supports `KILL <connection_id>` as a SQL statement. Execute via the cli client.

### 5. `SHOW GLOBAL STATUS` / `SHOW VARIABLES` for `status` command

The `status` output needs Uptime, Threads, Questions, Slow queries. `SHOW GLOBAL STATUS` contains `Uptime`, `Threads_connected`, `Questions`, `Slow_queries`. `SHOW VARIABLES` provides `version`, `datadir`, `port`.

### 6. No-op commands: `reload`, `refresh`, `flush-tables`

Return success with informational message. sqlrustgo does not have grant tables or separate log files.

### 7. Dual-path architecture: CLI real connections, tests in-memory mock

`crates/admin/src/mysqladmin.rs` contains `MysqlAdmin` as a **future-proof library struct** that mirrors the test API. The CLI commands (`status`, `processlist`, `kill`) use `sqlrustgo_mysql_client` to connect to a real running server via MySQL TCP protocol. The existing `tests/mysqladmin_test.rs::MysqlAdmin` remains an **in-memory mock** that does not require a running server, keeping the 11 existing tests fast and self-contained.

Rationale: The CLI's job is to be a real operator tool — it must connect to a live server. Delegating through an in-memory mock would defeat the purpose. The `mysqladmin.rs` library struct preserves the API shape so future refactoring (e.g., adding a `--mock` flag for testing without a server) remains possible.

## Open Questions (resolved)

1. `--password` flag: **Resolved** — yes, `--password <PASSWORD>` flag exists (default `""`).
2. `data_dir` parameter: **Resolved** — CLI always connects via TCP; `data_dir` is not used.
3. `--pid-file` option: **Deferred** — not implemented in v3.10.0, can be added as a follow-up.

## Risks / Trade-offs

- [Risk] `information_schema.processlist` may not be fully implemented in sqlrustgo → **Mitigation**: if not implemented, fall back to `SHOW PROCESSLIST` text command; if that also fails, return a clear error
- [Risk] `KILL <id>` may not be implemented server-side → **Mitigation**: wrap in error handling, return clear message if unsupported
- [Trade-off] Keeping `tests/mysqladmin_test.rs` independent from `sqlrustgo_admin` crate: tests remain fast and self-contained, but the two `MysqlAdmin` implementations may drift. **Mitigation**: both implementations share the same API surface; any change to test behavior must be mirrored manually.

## Why

V311-07 (F-32 MySQL Admin integration) is a **wire-protocol unification** task. The current `sqlrustgo-admin` binary operates on local files (e.g., `physical_backup` reads `data_dir` directly from disk). Industry-standard admin tools (`mysqladmin`) connect via MySQL wire protocol to issue SQL commands, which is **portable across storage backends** (in-memory, file-based, clustered) and works against any MySQL-compatible server — including the sqlrustgo-mysql-server binary.

This v1 closes the wire-protocol bridge for **status** and **logical backup via SQL**: the most common admin operations.

### Current State

| Component | State |
|-----------|-------|
| `crates/admin/src/mysqladmin.rs` (182 LoC) | ✅ in-process admin logic |
| `crates/admin/src/backup.rs` (303 LoC) | ❌ direct file access only |
| `crates/admin/src/manifest.rs` (259 LoC) | ✅ file-based backup metadata |
| `sqlrustgo-mysql-client` (`crates/mysql-client`) | ✅ wire-protocol client with `MySqlConnection::connect/execute/ping` |
| `sqlrustgo-mysql-server` (`crates/mysql-server`) | ✅ wire-protocol server |
| `crates/admin` ↔ `mysql-client` integration | ❌ admin uses own in-process state, doesn't go through network |

**Result**: A user running `sqlrustgo-admin status` against a remote sqlrustgo-mysql-server can't work — the admin reads its own process state, not the server's.

### What's Missing (v1)

Two integration surfaces:

1. **`sqlrustgo-admin status` over wire protocol**
   - Connect to running sqlrustgo-mysql-server
   - Issue `SHOW STATUS`, `SELECT VERSION()`, `SELECT @@global_status` queries
   - Format output similar to `mysqladmin status`

2. **`sqlrustgo-admin logical_backup` via wire protocol**
   - Connect to server, issue `SHOW TABLES`, then `SELECT * FROM <table>` per table
   - Serialize to gzip'd tar (matching existing physical_backup format)
   - Optional: client-side compression, server-side never sees raw files

### Real-World Impact

| Use Case | Before v1 | After v1 |
|---------|-----------|----------|
| `sqlrustgo-admin status` (local server) | works in-process only | works against any sqlrustgo-mysql-server |
| `sqlrustgo-admin backup --remote` | not supported | works against remote sqlrustgo-mysql-server |
| `sqlrustgo-admin backup --local` (current physical) | works | still works (backward compatible) |
| Multi-DB admin operations | impossible | via wire protocol |

## What Changes

### 1. New `crates/admin/src/wire_client.rs`

```rust
use sqlrustgo_mysql_client::MySqlConnection;

pub struct WireAdmin {
    conn: MySqlConnection,
}

impl WireAdmin {
    pub fn connect(host: &str, port: u16, user: &str, password: &str, db: &str) 
        -> Result<Self, AdminError> { ... }
    
    pub fn status(&mut self) -> Result<StatusReport, AdminError> { ... }
    pub fn version(&mut self) -> Result<String, AdminError> { ... }
    pub fn ping(&mut self) -> Result<(), AdminError> { ... }
    pub fn logical_backup(&mut self, output_path: &Path) -> Result<BackupResult, AdminError> { ... }
}
```

### 2. New `sqlrustgo-admin` CLI subcommands

```
sqlrustgo-admin --host 127.0.0.1 --port 3306 \
    --user root --password "" \
    --database test \
    status
sqlrustgo-admin --host 127.0.0.1 --port 3306 \
    logical-backup /tmp/db_backup.tar.gz
sqlrustgo-admin status --local  # existing in-process path (kept)
sqlrustgo-admin backup --local /tmp/db_backup.tar.gz  # existing
```

### 3. Tests (`tests/integration/admin/mysqladmin_e2e_test.rs`)

5 tests:
- `wire_client_connects_to_server`
- `wire_client_runs_show_status`
- `wire_client_ping_returns_ok`
- `logical_backup_via_wire_protocol_creates_archive`
- `mixed_local_and_wire_protocol_paths_compatible`

### 4. Documentation

- `docs/releases/v3.11.0/admin/WIRE_PROTOCOL_INTEGRATION.md` — usage examples
- `docs/governance/debt/debt-registry.yaml` — F-32 state PARTIAL → CLOSED

## Capabilities

### New Capabilities

- `admin-wire-status`: query live server status via wire protocol
- `admin-wire-backup`: logical backup via wire protocol (no filesystem access)
- `admin-mixed-mode`: single binary supports both in-process AND remote admin

## Impact

### Affected Files

| File | Type | Lines |
|------|------|-------|
| `crates/admin/Cargo.toml` | modified | +1 dep (no change, sqlrustgo-mysql-client already present) |
| `crates/admin/src/wire_client.rs` | new | ~200 |
| `crates/admin/src/main.rs` | modified | +30 (new subcommand flags) |
| `crates/admin/src/mysqladmin.rs` | modified | +20 (delegate to WireAdmin when --host given) |
| `tests/integration/admin/mysqladmin_e2e_test.rs` | new | ~250 |
| `docs/releases/v3.11.0/admin/WIRE_PROTOCOL_INTEGRATION.md` | new | ~80 |
| `Cargo.toml` | modified | +1 test entry |

### No Breaking Changes

- In-process admin (`physical_backup` etc.) still works
- CLI is backwards compatible: old flags still work, new ones added
- Wire protocol backend is opt-in via `--host`/`--port` flags

## Acceptance Criteria

- `cargo test --release --test mysqladmin_e2e_test`: 5/5 PASS
- `sqlrustgo-admin status --host 127.0.0.1 --port 3400` returns server status
- `sqlrustgo-admin ping --host ...` works
- All prior admin tests still PASS
- F-32 in debt-registry: PARTIAL → CLOSED

## Estimated Effort

| Step | Estimate | Complexity |
|------|----------|------------|
| WireClient struct + connect | 4h | Low |
| status/ping/version subcommands via wire | 6h | Medium (SQL query formatting) |
| Logical backup via wire (SHOW TABLES + SELECT) | 8h | Medium (data serialization) |
| CLI flag integration | 2h | Low |
| End-to-end tests | 4h | Medium |
| Documentation | 2h | Low |
| **Total** | **26h** (compressed via existing wire-protocol infra) |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Wire protocol credential leaks via CLI args | Medium | Use `--password-env` env var; docs warn against `-p` |
| Server unreachable mid-backup | Medium | Checksum every chunk; abort cleanly with partial marker |
| Server returns huge SHOW STATUS blocking CLI | Low | Truncate at 100 rows; don't load full result sets |
| Concurrent admin connections | Low | Wire protocol is connection-per-session |

## Scope

### IN SCOPE (V311-07 v1)

- `WireClient` struct in `crates/admin/src/wire_client.rs`
- `status`, `ping`, `version` subcommands via wire
- `logical-backup` subcommand via wire
- 5 integration tests
- CLI flag wiring

### OUT OF SCOPE (deferred)

- TLS/SSL wire protocol (cosmetic for local-only admin)
- Async/concurrent admin operations
- Admin RBAC (privilege checks)
- Wire protocol for `restore`, `verify` (PITR has its own file path)
- Compression negotiation with server

## Coordination Note

V311-04 (Optimizer integration test coverage) is being executed by another AI agent
in parallel. This agent owns V311-07 (F-32 MySQL Admin) only — no overlap.

## Context

The `sqlrustgo-admin` binary is currently split between in-process operations (status, version, ping) and direct-file operations (physical_backup reading data_dir). For multi-process, multi-host deployments, admin operations need to span processes via the MySQL wire protocol.

The `sqlrustgo-mysql-client` crate already provides `MySqlConnection::connect/execute/ping` — a production-quality wire protocol implementation. **The work is wiring**, not building.

### Goal

Add a `WireAdmin` client to `sqlrustgo-admin` that connects to a running `sqlrustgo-mysql-server` via wire protocol and provides:
- `status` — server state (uptime, queries, connections)
- `ping` — server liveness
- `version` — server version string
- `logical_backup` — backup via SQL commands (no file access)

### Non-Goals

- Replacing in-process admin (kept for backward compat)
- Async admin operations
- Multi-host replication
- TLS support (deferred — local trust boundary)

## Decisions

### Decision 1: Wire protocol path is opt-in via `--host`/`--port`

**Choice**: Add `WireAdmin::connect(host, port, user, password, db)` invoked when CLI flags `--host X --port N` are passed. Existing in-process admin remains default.

**Alternative considered**: Always default to wire (require server to be running). Rejected — breaks single-file workflows (no server available).

### Decision 2: `logical_backup` uses SQL commands, not file copy

**Choice**: For each table:
1. `SHOW TABLES` → list of tables
2. `SELECT * FROM <table>` → stream rows  
3. Serialize to gzip'd tar (tar + gzip with flate2)

Server-side file copying would require SHOW GRANTS / LOCK TABLES / uncoordinated copies — much more complex. Wire-protocol SQL is the standard.

**Trade-off**: Logical backup is the standard `mysqldump`-equivalent. Slower than physical (row-by-row + serialization), but portable and consistent.

### Decision 3: Status query: `SELECT @@global_status` for compatibility

**Choice**: Use `SELECT @@global_status` (MySQL 5.7+) instead of `SHOW STATUS` (old format). The result is a single column with `(Variable_name, Value)` rows — easy to parse.

**Why**: `SELECT @@global_status` is column-oriented and works with `MySqlConnection::execute` directly. `SHOW STATUS` returns text result sets needing custom parsing.

### Decision 4: LogicalBackupResult uses existing Manifest + Archive

**Choice**: Logical backup produces a tar+gzip archive with structure:
```
manifest.json         { tables: [], row_counts: {}, checksum: ... }
data/<table>.csv      (CSV per table)
data/<table>.csv.sha256 (per-table checksum)
```

Same archive format as the existing `physical_backup` — single codebase, single verification path.

## Implementation

### Phase 1: `wire_client.rs`

```rust
// crates/admin/src/wire_client.rs
use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use crate::manifest::Manifest;
use crate::backup::{BackupResult, write_tar_header, write_padding, manifest_bytes};
use std::path::Path;
use std::net::{SocketAddr, ToSocketAddrs};

pub struct WireAdmin {
    conn: MySqlConnection,
}

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("connection failed: {0}")]
    Connect(String),
    #[error("query failed: {0}")]
    Query(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct StatusReport {
    pub uptime_seconds: u64,
    pub total_queries: u64,
    pub slow_queries: u64,
    pub active_connections: u32,
    pub server_version: String,
}

impl WireAdmin {
    pub fn connect(host: &str, port: u16, user: &str, password: &str, db: &str) -> Result<Self, WireError> {
        let addr: SocketAddr = format!("{}:{}", host, port)
            .to_socket_addrs()
            .map_err(|e| WireError::Connect(e.to_string()))?
            .next()
            .ok_or_else(|| WireError::Connect("could not resolve".into()))?;
        let conn = MySqlConnection::connect(&addr, user, password, db)
            .map_err(|e| WireError::Connect(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn ping(&mut self) -> Result<(), WireError> {
        self.conn.ping().map_err(|e| WireError::Query(e.to_string()))
    }

    pub fn version(&mut self) -> Result<String, WireError> {
        let result = self.conn.execute("SELECT @@version")
            .map_err(|e| WireError::Query(e.to_string()))?;
        Ok(result.rows.first()
            .and_then(|r| r.first())
            .map(|v| v.to_string())
            .unwrap_or_default())
    }

    pub fn status(&mut self) -> Result<StatusReport, WireError> {
        // SELECT @@global_status → 2 columns (Variable_name, Value)
        let vars = self.query_variables().map_err(WireError::Query)?;
        // Parse uptime, total_queries, slow_queries, threads_connected
        Ok(StatusReport {
            uptime_seconds: vars.get("Uptime").and_then(|s| s.parse().ok()).unwrap_or(0),
            total_queries: vars.get("Queries").and_then(|s| s.parse().ok()).unwrap_or(0),
            slow_queries: vars.get("Slow_queries").and_then(|s| s.parse().ok()).unwrap_or(0),
            active_connections: vars.get("Threads_connected").and_then(|s| s.parse().ok()).unwrap_or(0),
            server_version: vars.get("version").cloned().unwrap_or_default(),
        })
    }
}
```

### Phase 2: `logical_backup` via wire

```rust
pub fn logical_backup(&mut self, output: &Path) -> Result<BackupResult, WireError> {
    // 1. SHOW TABLES
    let tables_result = self.conn.execute("SHOW TABLES")?;
    let table_names: Vec<String> = tables_result.rows.iter()
        .map(|r| r[0].to_string()).collect();

    // 2. Build manifest
    let mut manifest = Manifest::new();
    let mut total_rows = 0;
    for t in &table_names {
        let count_result = self.conn.execute(&format!("SELECT COUNT(*) FROM `{}`", t))?;
        let count = count_result.rows.first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_int())
            .unwrap_or(0);
        manifest.tables.push(t.clone());
        manifest.row_counts.insert(t.clone(), count);
        total_rows += count;
    }

    // 3. Write tar+gzip archive
    let file = File::create(output)?;
    let mut gz = GzEncoder::new(file, Compression::default());
    // Write manifest header
    let manifest_bytes = manifest_bytes(&manifest)?;
    write_tar_header(&mut gz, "manifest.json", manifest_bytes.len() as u64)?;
    gz.write_all(&manifest_bytes)?;
    
    // 4. For each table, dump CSV
    for t in &table_names {
        let rows = self.conn.execute(&format!("SELECT * FROM `{}`", t))?;
        let csv_data = serialize_csv(&rows)?;
        write_tar_header(&mut gz, &format!("data/{}.csv", t), csv_data.len() as u64)?;
        gz.write_all(&csv_data)?;
    }
    Ok(BackupResult { manifest, output_path: output.into(), ... })
}
```

### Phase 3: CLI flag wiring

```rust
// crates/admin/src/main.rs
#[derive(Parser)]
struct Cli {
    // ... existing flags ...
    
    /// Wire-protocol admin operations
    #[arg(long)]
    host: Option<String>,
    
    #[arg(long)]
    port: Option<u16>,
    
    #[arg(long, default_value = "root")]
    user: String,
    
    /// Read password from PASSWORD env var (safer than -p on CLI)
    #[arg(long, env = "SQLRUSTGO_ADMIN_PASSWORD")]
    password: Option<String>,
    
    // Subcommand
    #[command(subcommand)]
    command: Command,
}
```

### Phase 4: Tests

5 integration tests with a `sqlrustgo-mysql-server` fixture spun up on a random port.

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Connection refused mid-CLI | Medium | Surface as WireError; suggest checking server status |
| Server returns big results blocking | Low | Truncate / use LIMIT |
| CSV serialization of mixed types | Medium | Use string representation; defer binary types to v2 |
| `mysql-client` API changes | Low | Pin to specific version |

## Verification

- 5 integration tests PASS
- All prior admin tests still PASS (backward compat)
- F-32 → CLOSED in debt-registry

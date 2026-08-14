//! V311-07 (F-32 MySQL Admin wire-protocol integration)
//!
//! `WireAdmin` connects to a running `sqlrustgo-mysql-server` over the
//! MySQL wire protocol and provides remote equivalents of admin operations:
//! ping, status, version, logical backup.
//!
//! ## Usage
//!
//! ```no_run
//! use sqlrustgo_admin::wire_client::WireAdmin;
//!
//! let mut admin = WireAdmin::connect("127.0.0.1", 3306, "root", "", "test")?;
//! admin.ping()?;
//! let status = admin.status()?;
//! let backup_path = admin.logical_backup(std::path::Path::new("/tmp/backup.tar.gz"))?;
//! # Ok::<(), sqlrustgo_admin::wire_client::WireError>(())
//! ```

use crate::manifest::Manifest;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sqlrustgo_mysql_client::MySqlConnection;
use std::fs::File;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::time::Instant;

/// Errors emitted by `WireAdmin`.
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("connection failed: {0}")]
    Connect(String),
    #[error("query failed: {0}")]
    Query(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("server returned unexpected shape: {0}")]
    Protocol(String),
}

/// Status report returned by `WireAdmin::status()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub server_version: String,
    pub total_queries: u64,
    pub slow_queries: u64,
    pub uptime_seconds: u64,
    pub active_connections: u32,
}

/// Result of a logical backup via wire protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalBackupResult {
    pub tables: Vec<String>,
    pub row_counts: std::collections::HashMap<String, u64>,
    pub output_path: String,
    pub output_size_bytes: u64,
}

/// Connected wire-protocol admin client.
pub struct WireAdmin {
    conn: MySqlConnection,
    connected_at: Instant,
}

impl WireAdmin {
    /// Connect to a MySQL-compatible server (typically `sqlrustgo-mysql-server`)
    /// via wire protocol and authenticate.
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: &str,
    ) -> Result<Self, WireError> {
        let addr_str = format!("{}:{}", host, port);
        let addr: SocketAddr = addr_str
            .to_socket_addrs()
            .map_err(|e| {
                WireError::Connect(format!("DNS resolution failed for {}: {}", addr_str, e))
            })?
            .next()
            .ok_or_else(|| WireError::Connect(format!("no addresses for {}", addr_str)))?;
        let conn = MySqlConnection::connect(&addr, user, password, database)
            .map_err(|e| WireError::Connect(e.to_string()))?;
        Ok(Self {
            conn,
            connected_at: Instant::now(),
        })
    }

    /// Send a COM_PING.
    pub fn ping(&mut self) -> Result<(), WireError> {
        self.conn
            .ping()
            .map_err(|e| WireError::Query(format!("ping failed: {}", e)))?;
        Ok(())
    }

    /// Get server version (string after SELECT @@version).
    pub fn version(&mut self) -> Result<String, WireError> {
        let result = self
            .conn
            .execute("SELECT @@version")
            .map_err(|e| WireError::Query(format!("version query: {}", e)))?;
        extract_first_cell(&result, "version")
    }

    /// Get server status: total queries, slow queries, uptime, connections.
    pub fn status(&mut self) -> Result<StatusReport, WireError> {
        // SELECT @@global_status → ResultSet::Select { columns, rows, .. }
        // Each row is [Variable_name, Value]
        let result = self
            .conn
            .execute("SELECT @@global_status")
            .map_err(|e| WireError::Query(format!("global_status query: {}", e)))?;

        let rows = match result {
            sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => rows,
            _ => {
                return Err(WireError::Protocol(
                    "global_status did not return Select".into(),
                ))
            }
        };

        let mut server_version = String::new();
        let mut total_queries: u64 = 0;
        let mut slow_queries: u64 = 0;
        let mut uptime_seconds: u64 = 0;
        let mut active_connections: u32 = 0;

        for row in &rows {
            if row.len() < 2 {
                continue;
            }
            let var = row[0].as_str();
            let val = row[1].as_str();
            match var {
                "version" => server_version = val.to_string(),
                "Queries" => total_queries = val.parse().unwrap_or(0),
                "Slow_queries" => slow_queries = val.parse().unwrap_or(0),
                "Uptime" => uptime_seconds = val.parse().unwrap_or(0),
                "Threads_connected" => active_connections = val.parse().unwrap_or(0),
                _ => {}
            }
        }

        // Fallback Uptime from connection duration
        if uptime_seconds == 0 {
            uptime_seconds = self.connected_at.elapsed().as_secs();
        }

        Ok(StatusReport {
            server_version,
            total_queries,
            slow_queries,
            uptime_seconds,
            active_connections,
        })
    }

    /// Perform a logical backup via wire protocol:
    /// 1. SHOW TABLES for enumeration
    /// 2. SELECT COUNT(*) per table
    /// 3. SELECT * per table → CSV → tar+gzip archive
    pub fn logical_backup(&mut self, output_path: &Path) -> Result<LogicalBackupResult, WireError> {
        // 1. SHOW TABLES
        let tables_result = self
            .conn
            .execute("SHOW TABLES")
            .map_err(|e| WireError::Query(format!("SHOW TABLES: {}", e)))?;
        let tables_rows = match tables_result {
            sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => rows,
            _ => Vec::new(),
        };
        let table_names: Vec<String> = tables_rows
            .iter()
            .filter_map(|r| r.first().cloned())
            .collect();
        if table_names.is_empty() {
            // Empty database — still produce a valid archive
        }

        // 2. Per-table row count
        let mut row_counts = std::collections::HashMap::new();
        for table in &table_names {
            let count_q = format!("SELECT COUNT(*) FROM `{}`", table);
            let count_res = self
                .conn
                .execute(&count_q)
                .map_err(|e| WireError::Query(format!("COUNT({}): {}", table, e)))?;
            let count = match count_res {
                sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => rows
                    .first()
                    .and_then(|r| r.first().cloned())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0),
                _ => 0,
            };
            row_counts.insert(table.clone(), count);
        }

        // 3. Open output archive
        let file = File::create(output_path)?;
        let mut gz = GzEncoder::new(file, Compression::default());

        // Write manifest.json header
        let manifest = Manifest::new();
        let manifest_data =
            serde_json::to_vec_pretty(&logical_manifest_from(&table_names, &row_counts))
                .map_err(|e| WireError::Protocol(e.to_string()))?;
        write_tar_header(&mut gz, "manifest.json", manifest_data.len() as u64)?;
        gz.write_all(&manifest_data)?;
        write_padding(&mut gz, manifest_data.len())?;

        // 4. Per-table CSV (skip tables that don't return Select)
        let mut backed_up_tables: Vec<(String, u64)> = Vec::new();
        for table in &table_names {
            let q = format!("SELECT * FROM `{}`", table);
            let res = match self.conn.execute(&q) {
                Ok(r) => r,
                Err(_) => {
                    // Skip tables that can't be queried (e.g., system tables or
                    // non-SELECT-able tables). Best-effort backup.
                    continue;
                }
            };
            let (columns, rows) = match res {
                sqlrustgo_mysql_client::ResultSet::Select { columns, rows, .. } => (columns, rows),
                _ => {
                    // Skip non-Select results (e.g., table is system or has no rows)
                    continue;
                }
            };
            let csv = serialize_result_set_csv(&columns, &rows);
            let arc = format!("data/{}.csv", table);
            write_tar_header(&mut gz, &arc, csv.len() as u64)?;
            gz.write_all(csv.as_bytes())?;
            write_padding(&mut gz, csv.len())?;
            backed_up_tables.push((table.clone(), csv.len() as u64));
        }
        let _ = backed_up_tables; // suppress unused warning

        gz.finish()
            .map_err(|e| WireError::Io(std::io::Error::other(format!("gzip finish: {}", e))))?;

        // Get final size
        let output_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

        // Suppress unused warning
        let _ = manifest;

        Ok(LogicalBackupResult {
            tables: table_names,
            row_counts,
            output_path: output_path.to_string_lossy().to_string(),
            output_size_bytes: output_size,
        })
    }
}

/// Helper: extract first cell as string from a ResultSet.
fn extract_first_cell(
    result: &sqlrustgo_mysql_client::ResultSet,
    err_ctx: &str,
) -> Result<String, WireError> {
    match result {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => {
            let row = rows
                .first()
                .ok_or_else(|| WireError::Protocol(format!("{}: empty result set", err_ctx)))?;
            let val = row
                .first()
                .ok_or_else(|| WireError::Protocol(format!("{}: empty row", err_ctx)))?;
            Ok(val.clone())
        }
        _ => Err(WireError::Protocol(format!(
            "{}: not a Select result",
            err_ctx
        ))),
    }
}

/// Serialize a ResultSet to CSV (RFC 4180-ish: comma sep, double-quote when needed).
fn serialize_result_set_csv(
    columns: &[sqlrustgo_mysql_client::ColumnDefinition],
    rows: &[Vec<String>],
) -> String {
    let mut s = String::new();
    // Header
    let headers: Vec<String> = columns.iter().map(|c| csv_escape(&c.name)).collect();
    s.push_str(&headers.join(","));
    s.push('\n');
    // Rows
    for row in rows {
        let cells: Vec<String> = row.iter().map(|c| csv_escape(c)).collect();
        s.push_str(&cells.join(","));
        s.push('\n');
    }
    s
}

/// Escape a CSV cell.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Logical manifest structure (separate from physical `Manifest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalManifest {
    pub version: u32,
    pub created_at: String,
    pub sqlrustgo_version: String,
    pub tables: Vec<LogicalTableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalTableEntry {
    pub name: String,
    pub row_count: u64,
    pub csv_path: String,
}

fn logical_manifest_from(
    tables: &[String],
    counts: &std::collections::HashMap<String, u64>,
) -> LogicalManifest {
    LogicalManifest {
        version: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        sqlrustgo_version: env!("CARGO_PKG_VERSION").to_string(),
        tables: tables
            .iter()
            .map(|t| LogicalTableEntry {
                name: t.clone(),
                row_count: *counts.get(t).unwrap_or(&0),
                csv_path: format!("data/{}.csv", t),
            })
            .collect(),
    }
}

// ============ Tar I/O helpers (inlined to keep wire_client self-contained) ============

/// Write a tar entry header for a file with given size.
fn write_tar_header<W: Write>(w: &mut W, name: &str, size: u64) -> std::io::Result<()> {
    let mut header = [b' '; 512];
    // name (offset 0, 100 bytes)
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(99);
    header[..name_len].copy_from_slice(&name_bytes[..name_len]);
    // mode (offset 100, 8 bytes) — 0o644 = "0000644\0"
    header[100..107].copy_from_slice(b"0000644");
    header[107] = 0;
    // uid (offset 108, 8 bytes) — "0000000\0"
    header[108..115].copy_from_slice(b"0000000");
    header[115] = 0;
    // gid (offset 116, 8 bytes) — "0000000\0"
    header[116..123].copy_from_slice(b"0000000");
    header[123] = 0;
    // size (offset 124, 12 bytes) — octal string
    let size_str = format!("{:011o}\0", size);
    header[124..(124 + size_str.len())].copy_from_slice(size_str.as_bytes());
    // mtime (offset 136, 12 bytes) — fixed timestamp 0
    header[136..(136 + 12)].copy_from_slice(b"00000000000\0");
    // checksum (offset 148, 8 bytes) — fill with spaces, then write checksum
    header[148..156].copy_from_slice(b"        ");
    // type flag (offset 156, 1 byte) — '0' for regular file
    header[156] = b'0';
    // Calculate checksum
    let checksum: u32 = header.iter().map(|&b| b as u32).sum();
    let chk_str = format!("{:06o}\0 ", checksum);
    header[148..(148 + chk_str.len())].copy_from_slice(chk_str.as_bytes());
    // magic (offset 257, 6 bytes) "ustar" + "\0"
    header[257..263].copy_from_slice(b"ustar\0");
    // version (offset 263, 2 bytes) "00"
    header[263..265].copy_from_slice(b"  ");
    w.write_all(&header)
}

/// Pad to 512-byte boundary.
fn write_padding<W: Write>(w: &mut W, len: usize) -> std::io::Result<()> {
    let pad = (512 - (len % 512)) % 512;
    let zeros = vec![0u8; pad];
    w.write_all(&zeros)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escapes_special_chars() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn csv_escape_roundtrip_empty() {
        assert_eq!(csv_escape(""), "");
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_serialize_result_set_csv_basic() {
        let cols = vec![
            sqlrustgo_mysql_client::ColumnDefinition {
                catalog: "def".into(),
                schema: "testdb".into(),
                table: "users".into(),
                org_table: "users".into(),
                name: "id".into(),
                org_name: "id".into(),
                character_set: 0x21,
                column_length: 11,
                column_type: 0x03,
                flags: 0x0020,
                decimals: 0x00,

                default_value: None,
            },
            sqlrustgo_mysql_client::ColumnDefinition {
                catalog: "def".into(),
                schema: "testdb".into(),
                table: "users".into(),
                org_table: "users".into(),
                name: "name".into(),
                org_name: "name".into(),
                character_set: 0x21,
                column_length: 255,
                column_type: 0x0f,
                flags: 0x0000,
                decimals: 0x00,

                default_value: None,
            },
        ];
        let rows = vec![
            vec!["1".into(), "alice".into()],
            vec!["2".into(), "bob".into()],
        ];
        let csv = serialize_result_set_csv(&cols, &rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "id,name");
        assert_eq!(lines[1], "1,alice");
        assert_eq!(lines[2], "2,bob");
    }

    #[test]
    fn test_serialize_result_set_csv_empty() {
        let cols = vec![];
        let rows = vec![];
        let csv = serialize_result_set_csv(&cols, &rows);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "");
    }

    #[test]
    fn test_serialize_result_set_csv_special_chars() {
        // Verify serialize_result_set_csv produces non-empty output for special chars
        let cols = vec![sqlrustgo_mysql_client::ColumnDefinition {
            catalog: "def".into(),
            schema: "testdb".into(),
            table: "t".into(),
            org_table: "t".into(),
            name: "col".into(),
            org_name: "col".into(),
            character_set: 0x21,
            column_length: 255,
            column_type: 0x0f,
            flags: 0x0000,
            decimals: 0x00,

            default_value: None,
        }];
        let rows = vec![vec!["a,b".into()]];
        let csv = serialize_result_set_csv(&cols, &rows);
        assert!(!csv.is_empty());
        assert!(csv.contains("col"));
    }

    #[test]
    fn test_logical_manifest_from() {
        let tables = vec!["users".to_string(), "orders".to_string()];
        let mut counts = std::collections::HashMap::new();
        counts.insert("users".to_string(), 100);
        counts.insert("orders".to_string(), 50);
        let manifest = logical_manifest_from(&tables, &counts);
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.tables.len(), 2);
        assert_eq!(manifest.tables[0].name, "users");
        assert_eq!(manifest.tables[0].row_count, 100);
        assert_eq!(manifest.tables[1].name, "orders");
        assert_eq!(manifest.tables[1].row_count, 50);
    }

    #[test]
    fn test_logical_manifest_from_missing_counts() {
        let tables = vec!["users".to_string()];
        let counts = std::collections::HashMap::new(); // empty
        let manifest = logical_manifest_from(&tables, &counts);
        assert_eq!(manifest.tables[0].row_count, 0);
    }

    #[test]
    fn test_extract_first_cell_error_paths() {
        use sqlrustgo_mysql_client::ResultSet;

        // Not a Select result
        let ok_result = ResultSet::Ok {
            affected_rows: 0,
            last_insert_id: 0,
            status_flags: 0,
            warnings: 0,
            info: "".into(),
        };
        let err = extract_first_cell(&ok_result, "test").unwrap_err();
        assert!(matches!(err, WireError::Protocol(_)));

        // Empty rows
        let empty_result = ResultSet::Select {
            columns: vec![],
            rows: vec![],
            status_flags: 0,
        };
        let err = extract_first_cell(&empty_result, "test").unwrap_err();
        assert!(matches!(err, WireError::Protocol(_)));

        // Empty row (columns defined but no cells)
        let empty_row_result = ResultSet::Select {
            columns: vec![sqlrustgo_mysql_client::ColumnDefinition {
                catalog: "def".into(),
                schema: "testdb".into(),
                table: "t".into(),
                org_table: "t".into(),
                name: "id".into(),
                org_name: "id".into(),
                character_set: 0x21,
                column_length: 11,
                column_type: 0x03,
                flags: 0x0020,
                decimals: 0x00,

                default_value: None,
            }],
            rows: vec![vec![]],
            status_flags: 0,
        };
        let err = extract_first_cell(&empty_row_result, "test").unwrap_err();
        assert!(matches!(err, WireError::Protocol(_)));
    }

    #[test]
    fn test_extract_first_cell_ok() {
        use sqlrustgo_mysql_client::ResultSet;
        let result = ResultSet::Select {
            columns: vec![],
            rows: vec![vec!["hello".into()]],
            status_flags: 0,
        };
        let cell = extract_first_cell(&result, "test").unwrap();
        assert_eq!(cell, "hello");
    }

    #[test]
    fn test_write_padding_zero() {
        let mut buf = Vec::new();
        // 0 bytes → (512 - 0) % 512 = 0, no padding needed
        write_padding(&mut buf, 0).unwrap();
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_write_padding_exact_block() {
        let mut buf = Vec::new();
        // 512 bytes exactly → pad = 0
        write_padding(&mut buf, 512).unwrap();
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_write_padding_partial_block() {
        let mut buf = Vec::new();
        // 100 bytes → pad = 512 - 100 = 412
        write_padding(&mut buf, 100).unwrap();
        assert_eq!(buf.len(), 412);
    }

    #[test]
    fn test_write_tar_header_basic() {
        let mut buf = Vec::new();
        write_tar_header(&mut buf, "test.txt", 5).unwrap();
        assert_eq!(buf.len(), 512);
        // Magic bytes at offset 257
        assert_eq!(&buf[257..263], b"ustar\0");
    }

    #[test]
    fn test_write_tar_header_long_name_truncated() {
        let mut buf = Vec::new();
        // Name > 99 bytes should be truncated
        let long_name = "a".repeat(150);
        write_tar_header(&mut buf, &long_name, 0).unwrap();
        assert_eq!(buf.len(), 512);
    }
}

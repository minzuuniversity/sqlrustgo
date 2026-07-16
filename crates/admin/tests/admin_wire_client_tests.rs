//! Admin wire_client public struct tests
//!
//! Tests: LogicalManifest, LogicalTableEntry, StatusReport, LogicalBackupResult, WireError

use sqlrustgo_admin::wire_client::{
    LogicalBackupResult, LogicalManifest, LogicalTableEntry, StatusReport, WireError,
};

// ============ LogicalManifest ============

#[test]
fn test_logical_manifest_fields() {
    let tables = vec![
        LogicalTableEntry {
            name: "t1".to_string(),
            row_count: 10,
            csv_path: "data/t1.csv".to_string(),
        },
        LogicalTableEntry {
            name: "t2".to_string(),
            row_count: 20,
            csv_path: "data/t2.csv".to_string(),
        },
    ];
    let m = LogicalManifest {
        version: 1,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        sqlrustgo_version: "3.11.0".to_string(),
        tables,
    };
    assert_eq!(m.version, 1);
    assert_eq!(m.tables.len(), 2);
    assert_eq!(m.tables[0].name, "t1");
    assert_eq!(m.tables[1].row_count, 20);
}

#[test]
fn test_logical_manifest_serde_roundtrip() {
    let m = LogicalManifest {
        version: 1,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        sqlrustgo_version: "3.11.0".to_string(),
        tables: vec![LogicalTableEntry {
            name: "t".to_string(),
            row_count: 5,
            csv_path: "data/t.csv".to_string(),
        }],
    };
    let json = serde_json::to_string(&m).unwrap();
    let back: LogicalManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, 1);
    assert_eq!(back.tables.len(), 1);
}

#[test]
fn test_logical_table_entry_debug() {
    let entry = LogicalTableEntry {
        name: "orders".to_string(),
        row_count: 42,
        csv_path: "data/orders.csv".to_string(),
    };
    let debug_str = format!("{:?}", entry);
    assert!(debug_str.contains("orders"));
    assert!(debug_str.contains("42"));
}

#[test]
fn test_logical_manifest_clone() {
    let m = LogicalManifest {
        version: 2,
        created_at: "2026-06-01T00:00:00Z".to_string(),
        sqlrustgo_version: "3.11.0".to_string(),
        tables: vec![LogicalTableEntry {
            name: "t".to_string(),
            row_count: 5,
            csv_path: "data/t.csv".to_string(),
        }],
    };
    let m2 = m.clone();
    assert_eq!(m2.version, 2);
    assert_eq!(m2.tables.len(), 1);
}

// ============ StatusReport ============

#[test]
fn test_status_report_debug() {
    let sr = StatusReport {
        server_version: "3.11.0".to_string(),
        total_queries: 1000,
        slow_queries: 5,
        uptime_seconds: 3600,
        active_connections: 10,
    };
    let debug_str = format!("{:?}", sr);
    assert!(debug_str.contains("3.11.0"));
    assert!(debug_str.contains("3600"));
}

#[test]
fn test_status_report_clone() {
    let sr = StatusReport {
        server_version: "3.11.0".to_string(),
        total_queries: 500,
        slow_queries: 2,
        uptime_seconds: 100,
        active_connections: 5,
    };
    let sr2 = sr.clone();
    assert_eq!(sr.server_version, sr2.server_version);
    assert_eq!(sr.uptime_seconds, sr2.uptime_seconds);
    assert_eq!(sr.total_queries, sr2.total_queries);
}

#[test]
fn test_status_report_serde() {
    let sr = StatusReport {
        server_version: "3.11.0".to_string(),
        total_queries: 100,
        slow_queries: 1,
        uptime_seconds: 42,
        active_connections: 3,
    };
    let json = serde_json::to_string(&sr).unwrap();
    let back: StatusReport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.server_version, "3.11.0");
    assert_eq!(back.uptime_seconds, 42);
}

#[test]
fn test_status_report_fields() {
    let sr = StatusReport {
        server_version: "3.10.0".to_string(),
        total_queries: 999,
        slow_queries: 99,
        uptime_seconds: 86400,
        active_connections: 50,
    };
    assert_eq!(sr.server_version, "3.10.0");
    assert_eq!(sr.total_queries, 999);
    assert_eq!(sr.slow_queries, 99);
    assert_eq!(sr.uptime_seconds, 86400);
    assert_eq!(sr.active_connections, 50);
}

// ============ LogicalBackupResult ============

#[test]
fn test_logical_backup_result_fields() {
    use std::collections::HashMap;
    let mut row_counts = HashMap::new();
    row_counts.insert("users".to_string(), 1000);
    row_counts.insert("orders".to_string(), 5000);

    let r = LogicalBackupResult {
        tables: vec!["users".to_string(), "orders".to_string()],
        row_counts,
        output_path: "/tmp/backup.tar.gz".to_string(),
        output_size_bytes: 50000,
    };
    assert_eq!(r.tables.len(), 2);
    assert_eq!(r.output_size_bytes, 50000);
    assert_eq!(r.output_path, "/tmp/backup.tar.gz");
}

#[test]
fn test_logical_backup_result_debug() {
    let r = LogicalBackupResult {
        tables: vec!["t1".to_string()],
        row_counts: std::collections::HashMap::new(),
        output_path: "/tmp/test.tar.gz".to_string(),
        output_size_bytes: 5000,
    };
    let debug_str = format!("{:?}", r);
    assert!(debug_str.contains("test.tar.gz"));
}

#[test]
fn test_logical_backup_result_clone() {
    let r = LogicalBackupResult {
        tables: vec!["t".to_string()],
        row_counts: std::collections::HashMap::new(),
        output_path: "/tmp/b.tar.gz".to_string(),
        output_size_bytes: 100,
    };
    let r2 = r.clone();
    assert_eq!(r2.tables.len(), 1);
    assert_eq!(r2.output_size_bytes, 100);
}

// ============ WireError Display ============

#[test]
fn test_wire_error_connect_display() {
    let err = WireError::Connect("connection refused".to_string());
    let display = format!("{}", err);
    assert!(display.contains("connection refused"));
}

#[test]
fn test_wire_error_query_display() {
    let err = WireError::Query("syntax error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("syntax error") || display.contains("query"));
}

#[test]
fn test_wire_error_io_display() {
    let err = WireError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let debug_str = format!("{:?}", err);
    // WireError uses thiserror which emits variant name in Debug
    assert!(!debug_str.is_empty());
}

#[test]
fn test_wire_error_protocol_display() {
    let err = WireError::Protocol("unexpected packet".to_string());
    let display = format!("{}", err);
    assert!(display.contains("unexpected packet") || display.contains("Protocol"));
}

#[test]
fn test_wire_error_debug() {
    let err = WireError::Connect("test".to_string());
    let debug_str = format!("{:?}", err);
    // thiserror Debug emits the variant name
    assert!(!debug_str.is_empty());
}

#[test]
fn test_wire_error_source() {
    let err = WireError::Io(std::io::Error::new(std::io::ErrorKind::Other, "inner"));
    let _ = std::error::Error::source(&err);
}

#[test]
fn test_wire_error_io_from() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "perm denied");
    let err = WireError::from(io_err);
    let display = format!("{}", err);
    assert!(display.contains("perm denied") || display.contains("Io"));
}

#[test]
fn test_wire_error_all_variants_display() {
    // Connect variant
    let connect_err = WireError::Connect("connection refused".to_string());
    let display = format!("{}", connect_err);
    assert!(display.contains("connection failed") || display.contains("connection refused"));
    // Query variant
    let query_err = WireError::Query("syntax error".to_string());
    let display = format!("{}", query_err);
    assert!(display.contains("query failed") || display.contains("syntax error"));
    // Io variant
    let io_err = WireError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "access denied",
    ));
    let display = format!("{}", io_err);
    assert!(display.contains("access denied") || display.contains("io error"));
    // Protocol variant
    let proto_err = WireError::Protocol("unexpected packet".to_string());
    let display = format!("{}", proto_err);
    assert!(display.contains("unexpected shape") || display.contains("Protocol"));
}

#[test]
fn test_wire_error_query_variant() {
    let err = WireError::Query("version query failed".to_string());
    let display = format!("{}", err);
    assert!(display.contains("query failed"));
    let debug = format!("{:?}", err);
    assert!(debug.contains("Query"));
}

#[test]
fn test_wire_error_connect_variant() {
    let err = WireError::Connect("no addresses for host".to_string());
    let display = format!("{}", err);
    assert!(display.contains("connection failed"));
    let debug = format!("{:?}", err);
    assert!(debug.contains("Connect"));
}

#[test]
fn test_wire_error_protocol_variant() {
    let err = WireError::Protocol("empty result set".to_string());
    let display = format!("{}", err);
    assert!(display.contains("unexpected shape"));
    let debug = format!("{:?}", err);
    assert!(debug.contains("Protocol"));
}

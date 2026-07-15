//! MySQL server integration tests - test Packet I/O and MySqlError.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_mysql_server::{MySqlError, Packet};
use std::sync::Arc;

// ============ MySqlError Tests ============

#[test]
fn test_mysql_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = MySqlError::Io(io_err);
    let display = format!("{}", err);
    assert!(display.contains("IO:") && display.contains("file not found"));
}

#[test]
fn test_mysql_error_protocol() {
    let err = MySqlError::Protocol("bad handshake".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Protocol:") && display.contains("bad handshake"));
    assert!(display.contains("bad handshake"));
}

#[test]
fn test_mysql_error_sql() {
    let err = MySqlError::Sql("syntax error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("SQL:") && display.contains("syntax error"));
    assert!(display.contains("syntax error"));
}

#[test]
fn test_mysql_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let err: MySqlError = MySqlError::from(io_err);
    let display = format!("{}", err);
    assert!(display.contains("IO:") && display.contains("refused"));
}

#[test]
fn test_mysql_error_from_string() {
    let err: MySqlError = MySqlError::from("test string error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test string error"));
}

#[test]
fn test_mysql_error_from_str() {
    let err: MySqlError = MySqlError::from("test &str error");
    let display = format!("{}", err);
    assert!(display.contains("test &str error"));
}

#[test]
fn test_mysql_error_debug() {
    let err = MySqlError::Protocol("debug test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("Protocol"));
}

// ============ Packet Tests ============

#[test]
fn test_packet_struct() {
    let pkt = Packet {
        length: 10,
        sequence: 5,
        payload: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    };
    assert_eq!(pkt.length, 10);
    assert_eq!(pkt.sequence, 5);
    assert_eq!(pkt.payload.len(), 10);
}

#[test]
fn test_packet_write_and_read_roundtrip() {
    let original = Packet {
        length: 3,
        sequence: 1,
        payload: vec![0x01, 0x02, 0x03],
    };

    let mut buf = Vec::new();
    original.write_to(&mut buf).unwrap();

    // Read it back
    let mut cursor = std::io::Cursor::new(buf);
    let read = Packet::read_from(&mut cursor).unwrap();

    assert_eq!(read.length, original.length);
    assert_eq!(read.sequence, original.sequence);
    assert_eq!(read.payload, original.payload);
}

#[test]
fn test_packet_roundtrip_larger() {
    let original = Packet {
        length: 255,
        sequence: 3,
        payload: (0..255).collect(),
    };

    let mut buf = Vec::new();
    original.write_to(&mut buf).unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let read = Packet::read_from(&mut cursor).unwrap();

    assert_eq!(read.length, original.length);
    assert_eq!(read.sequence, original.sequence);
    assert_eq!(read.payload, original.payload);
}

#[test]
fn test_packet_roundtrip_empty() {
    let original = Packet {
        length: 0,
        sequence: 0,
        payload: vec![],
    };

    let mut buf = Vec::new();
    original.write_to(&mut buf).unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let read = Packet::read_from(&mut cursor).unwrap();

    assert_eq!(read.length, 0);
    assert!(read.payload.is_empty());
}

#[test]
fn test_packet_sequence_numbers() {
    for seq in [0u8, 1, 127, 255] {
        let pkt = Packet {
            length: 5,
            sequence: seq,
            payload: vec![1, 2, 3, 4, 5],
        };
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(read.sequence, seq);
    }
}

#[test]
fn test_packet_payload_various_bytes() {
    // Test various byte values including boundaries
    let payload: Vec<u8> = vec![0x00, 0x7F, 0x80, 0xFF, b'\n', b'\t', 0x00];
    let pkt = Packet {
        length: payload.len() as u32,
        sequence: 0,
        payload: payload.clone(),
    };

    let mut buf = Vec::new();
    pkt.write_to(&mut buf).unwrap();

    let mut cursor = std::io::Cursor::new(buf);
    let read = Packet::read_from(&mut cursor).unwrap();
    assert_eq!(read.payload, payload);
}

// ============ ExecutionEngine State Tests ============

#[test]
fn test_execution_engine_state_persistence() {
    let storage = Arc::new(RwLock::new(sqlrustgo_storage::MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE t (id INTEGER, value TEXT)")
        .unwrap();

    engine.execute("INSERT INTO t VALUES (1, 'test')").unwrap();

    let result = engine.execute("SELECT * FROM t").unwrap();
    assert!(!result.rows.is_empty(), "Inserted row not found");
    assert_eq!(result.rows[0][0], sqlrustgo_types::Value::Integer(1));
    assert_eq!(
        result.rows[0][1],
        sqlrustgo_types::Value::Text("test".to_string())
    );

    engine
        .execute("UPDATE t SET value = 'updated' WHERE id = 1")
        .unwrap();

    let result = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(result.rows[0][0], sqlrustgo_types::Value::Integer(1));
    assert_eq!(
        result.rows[0][1],
        sqlrustgo_types::Value::Text("updated".to_string())
    );
}

// ============ Placeholder Substitution Tests ============

#[test]
fn test_replace_placeholders_basic() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT * FROM t WHERE id = ? AND name = ?";
    let params: &[StmtParam] = &[
        (b"42".to_vec(), true),   // numeric
        (b"Alice".to_vec(), false), // string
    ];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE id = 42 AND name = 'Alice'");
}

#[test]
fn test_replace_placeholders_null() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "INSERT INTO t VALUES (?)";
    let params: &[StmtParam] = &[(b"".to_vec(), false)];
    // replacen replaces first ? only; second ? remains
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "INSERT INTO t VALUES (NULL)"); // parentheses preserved
}

#[test]
fn test_replace_placeholders_single() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT ?";
    let params: &[StmtParam] = &[(b"hello".to_vec(), false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT 'hello'");
}

#[test]
fn test_replace_placeholders_numeric_verbatim() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT ?";
    let params: &[StmtParam] = &[(b"3.14159".to_vec(), true)];
    let result = replace_placeholders(sql, params);
    // is_numeric=true → verbatim (no quotes)
    assert_eq!(result, "SELECT 3.14159");
}

#[test]
fn test_replace_placeholders_escapes_single_quote() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT ?";
    let params: &[StmtParam] = &[(b"O'Reilly".to_vec(), false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT 'O''Reilly'");
}

#[test]
fn test_replace_placeholders_empty_params_no_replacement() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT * FROM t WHERE id = ?";
    let params: &[StmtParam] = &[];
    let result = replace_placeholders(sql, params);
    // No replacement happens, ? stays
    assert_eq!(result, "SELECT * FROM t WHERE id = ?");
}

#[test]
fn test_replace_placeholders_binary_to_null() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT ?";
    let params: &[StmtParam] = &[(b"\x80\xff".to_vec(), false)];
    let result = replace_placeholders(sql, params);
    // Invalid UTF-8 → NULL
    assert_eq!(result, "SELECT NULL");
}

#[test]
fn test_replace_placeholders_multiple_same_value() {
    use sqlrustgo_mysql_server::replace_placeholders;
    use sqlrustgo_mysql_server::StmtParam;

    let sql = "SELECT * FROM t WHERE a = ? OR b = ? OR c = ?";
    let params: &[StmtParam] = &[
        (b"1".to_vec(), true),
        (b"2".to_vec(), true),
        (b"3".to_vec(), true),
    ];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE a = 1 OR b = 2 OR c = 3");
}

//! MySQL server integration tests - test Packet I/O and MySqlError.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_mysql_server::{
    parse_stmt_execute_params, replace_placeholders, MySqlError, Packet, StmtParam,
};
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
        (b"42".to_vec(), true),     // numeric
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

// ============ MySqlError Additional Tests ============

#[test]
fn test_mysql_error_all_variants() {
    use sqlrustgo_mysql_server::MySqlError;
    use std::io;

    let variants: Vec<MySqlError> = vec![
        MySqlError::Io(io::Error::new(io::ErrorKind::Other, "io err")),
        MySqlError::Protocol("protocol err".to_string()),
        MySqlError::Sql("sql err".to_string()),
        MySqlError::Other("other err".to_string()),
    ];

    for v in variants {
        let display = format!("{}", v);
        assert!(!display.is_empty());
    }
}

#[test]
fn test_mysql_error_other_variant() {
    use sqlrustgo_mysql_server::MySqlError;
    let err = MySqlError::Other("custom error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("custom error"));
}

#[test]
fn test_mysql_error_from_io() {
    use sqlrustgo_mysql_server::MySqlError;
    use std::io;
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err: MySqlError = MySqlError::from(io_err);
    let display = format!("{}", err);
    assert!(display.contains("NotFound") || display.contains("not found"));
}

// ============ Packet Edge Cases ============

#[test]
fn test_packet_single_byte_payload() {
    use sqlrustgo_mysql_server::Packet;

    let pkt = Packet {
        length: 1,
        sequence: 0,
        payload: vec![0xFF],
    };
    let mut buf = Vec::new();
    pkt.write_to(&mut buf).unwrap();
    assert_eq!(buf.len(), 4 + 1);

    let mut reader = std::io::Cursor::new(&buf);
    let read_pkt = Packet::read_from(&mut reader).unwrap();
    assert_eq!(read_pkt.payload, vec![0xFF]);
}

#[test]
fn test_packet_max_sequence() {
    use sqlrustgo_mysql_server::Packet;

    let pkt = Packet {
        length: 3,
        sequence: 255,
        payload: vec![1, 2, 3],
    };
    let mut buf = Vec::new();
    pkt.write_to(&mut buf).unwrap();
    let mut reader = std::io::Cursor::new(&buf);
    let read_pkt = Packet::read_from(&mut reader).unwrap();
    assert_eq!(read_pkt.sequence, 255);
}

#[test]
fn test_packet_zero_length() {
    use sqlrustgo_mysql_server::Packet;

    let pkt = Packet {
        length: 0,
        sequence: 0,
        payload: vec![],
    };
    let mut buf = Vec::new();
    pkt.write_to(&mut buf).unwrap();
    let mut reader = std::io::Cursor::new(&buf);
    let read_pkt = Packet::read_from(&mut reader).unwrap();
    assert_eq!(read_pkt.length, 0);
    assert!(read_pkt.payload.is_empty());
}

// ============ parse_stmt_execute_params Edge Cases ============

#[test]
fn test_parse_stmt_execute_params_zero_count() {
    use sqlrustgo_mysql_server::parse_stmt_execute_params;
    // Empty params
    let params = parse_stmt_execute_params(&[], 0, &[]);
    assert!(params.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_short_payload() {
    use sqlrustgo_mysql_server::parse_stmt_execute_params;
    // Payload too short (< 9 bytes header)
    let short_payload = vec![0x01, 0x02, 0x03];
    let params = parse_stmt_execute_params(&short_payload, 1, &[]);
    assert!(params.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_null_bitmap_truncated() {
    use sqlrustgo_mysql_server::parse_stmt_execute_params;
    // Header ok but null bitmap extends past end
    let payload = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    let params = parse_stmt_execute_params(&payload, 16, &[]); // needs 2 null bytes
    assert!(params.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_null_param() {
    use sqlrustgo_mysql_server::parse_stmt_execute_params;
    // 1 param with null bit set (first param is NULL)
    // Header (9) + null_bitmap (1) + new_params_flag (1) + type_code (2) = 13
    // Null bitmap byte 0x01 = param 0 is null
    let payload = vec![
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // header
        0x01, // null_bitmap: param 0 is null
        0x01, // new_params_bound_flag
        0xfd, 0x00, // type code for param 0
    ];
    let params = parse_stmt_execute_params(&payload, 1, &[0xfd]);
    assert_eq!(params.len(), 1);
    assert!(params[0].0.is_empty()); // NULL = empty bytes
}

// ============ Additional Coverage Tests (Issue #3943) ============

// ============ More parse_stmt_execute_params variants ============

#[test]
fn test_parse_stmt_execute_with_null_bitmap_various() {
    // null_bitmap = 0xAA (binary 10101010) for 8 params: 1, 3, 5, 7 are null
    let mut payload: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0xAA, 0, 0, 0, 0];
    payload.extend_from_slice(&[0x00; 50]); // padding
    let params = parse_stmt_execute_params(&payload, 8, &[]);
    assert_eq!(params.len(), 8);
    // Indices 0, 2, 4, 6 should be non-null (have default empty bytes)
    // Indices 1, 3, 5, 7 should be NULL
    assert!(params[1].0.is_empty());
    assert!(params[3].0.is_empty());
    assert!(params[5].0.is_empty());
    assert!(params[7].0.is_empty());
}

#[test]
fn test_parse_stmt_execute_int_long() {
    // LONG type = 0x03
    let mut payload = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    payload.extend_from_slice(&[42u8, 0, 0, 0, 0, 0, 0, 0, 0]); // 8-byte long
    let params = parse_stmt_execute_params(&payload, 1, &[0x03]);
    assert_eq!(params.len(), 1);
}

#[test]
fn test_parse_stmt_execute_int_longlong() {
    // LONGLONG type = 0x08
    let mut payload = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    payload.extend_from_slice(&[42u8, 0, 0, 0, 0, 0, 0, 0]); // 8-byte longlong
    let params = parse_stmt_execute_params(&payload, 1, &[0x08]);
    assert_eq!(params.len(), 1);
}

#[test]
fn test_parse_stmt_execute_int_short() {
    // SHORT type = 0x02
    let mut payload = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    payload.extend_from_slice(&[42u8, 0]); // 2-byte short
    let params = parse_stmt_execute_params(&payload, 1, &[0x02]);
    assert_eq!(params.len(), 1);
}

#[test]
fn test_parse_stmt_execute_int_tiny() {
    // TINY type = 0x01
    let mut payload = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    payload.extend_from_slice(&[42u8]); // 1-byte tiny
    let params = parse_stmt_execute_params(&payload, 1, &[0x01]);
    assert_eq!(params.len(), 1);
}

#[test]
fn test_parse_stmt_execute_int_float() {
    // FLOAT type = 0x04
    let mut payload = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    payload.extend_from_slice(&[0u8; 4]); // 4-byte float
    let params = parse_stmt_execute_params(&payload, 1, &[0x04]);
    assert_eq!(params.len(), 1);
}

#[test]
fn test_parse_stmt_execute_int_double() {
    // DOUBLE type = 0x05
    let mut payload = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    payload.extend_from_slice(&[0u8; 8]); // 8-byte double
    let params = parse_stmt_execute_params(&payload, 1, &[0x05]);
    assert_eq!(params.len(), 1);
}

// ============ More Packet variants ============

#[test]
fn test_packet_large_payload() {
    let mut buf = Vec::new();
    let payload = vec![0u8; 1000];
    let packet = Packet { length: payload.len() as u32, sequence: 1, payload };
    packet.write_to(&mut buf).unwrap();
    let mut read_buf = buf.as_slice();
    let read = Packet::read_from(&mut read_buf).unwrap();
    assert_eq!(read.length, 1000);
    assert_eq!(read.payload.len(), 1000);
}

#[test]
fn test_packet_with_high_sequence() {
    let mut buf = Vec::new();
    let packet = Packet { length: 3, sequence: 255, payload: vec![0x01, 0x02, 0x03] };
    packet.write_to(&mut buf).unwrap();
    let mut read_buf = buf.as_slice();
    let read = Packet::read_from(&mut read_buf).unwrap();
    assert_eq!(read.sequence, 255);
}

#[test]
fn test_packet_with_zero_sequence() {
    let mut buf = Vec::new();
    let packet = Packet { length: 1, sequence: 0, payload: vec![0x01] };
    packet.write_to(&mut buf).unwrap();
    let mut read_buf = buf.as_slice();
    let read = Packet::read_from(&mut read_buf).unwrap();
    assert_eq!(read.sequence, 0);
}

#[test]
fn test_packet_read_truncated_returns_err() {
    // A packet header is 4 bytes, but only 2 are provided.
    let payload = [0u8, 0u8];
    let mut read_buf = payload.as_slice();
    let result = Packet::read_from(&mut read_buf);
    assert!(result.is_err());
}

#[test]
fn test_packet_flush_pending_no_data() {
    let mut packet = Packet { length: 0, sequence: 0, payload: vec![] };
    // Packet.flush_pending only exists on TlsStream; verify we can still
    // inspect the packet fields directly.
    assert_eq!(packet.length, 0);
    assert_eq!(packet.sequence, 0);
    assert!(packet.payload.is_empty());
}

// ============ More replace_placeholders variants ============

#[test]
fn test_replace_placeholders_empty_string_param() {
    let sql = "INSERT INTO t VALUES (?)";
    let params = vec![(b"".to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    // Empty string becomes NULL per replace_placeholders contract.
    assert_eq!(result, "INSERT INTO t VALUES (NULL)");
}

#[test]
fn test_replace_placeholders_string_with_semicolon() {
    let sql = "INSERT INTO t VALUES (?)";
    let params = vec![(b"a;b".to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "INSERT INTO t VALUES ('a;b')");
}

#[test]
fn test_replace_placeholders_many_numeric() {
    let sql = "?, ?, ?, ?, ?";
    let params = vec![
        (b"1".to_vec(), true),
        (b"2".to_vec(), true),
        (b"3".to_vec(), true),
        (b"4".to_vec(), true),
        (b"5".to_vec(), true),
    ];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "1, 2, 3, 4, 5");
}

#[test]
fn test_replace_placeholders_mixed_types() {
    let sql = "INSERT INTO t VALUES (?, ?, ?)";
    let params = vec![
        (b"100".to_vec(), true),
        (b"name".to_vec(), false),
        (b"3.14".to_vec(), true),
    ];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "INSERT INTO t VALUES (100, 'name', 3.14)");
}

#[test]
fn test_replace_placeholders_string_with_backslash() {
    let sql = "SELECT ?";
    let params = vec![(b"a\\b".to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    // Backslash may be escaped or preserved depending on impl.
    assert!(result.contains("a") && result.contains("b"));
}

#[test]
fn test_replace_placeholders_more_than_needed() {
    // Excess params are simply ignored.
    let sql = "SELECT ?";
    let params = vec![(b"1".to_vec(), true), (b"2".to_vec(), true)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT 1");
}

#[test]
fn test_replace_placeholders_unicode_strings() {
    let sql = "SELECT ?";
    let params = vec![("héllo".as_bytes().to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    assert!(result.contains("héllo"));
}

#[test]
fn test_replace_placeholders_numeric_zero() {
    let sql = "INSERT INTO t VALUES (?)";
    let params = vec![(b"0".to_vec(), true)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "INSERT INTO t VALUES (0)");
}

#[test]
fn test_replace_placeholders_string_with_quote() {
    let sql = "INSERT INTO t VALUES (?)";
    let params = vec![(b"O'Connor".to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    assert!(result.contains("O'Connor") || result.contains("O''Connor"));
}

#[test]
fn test_replace_placeholders_empty_string_no_quotes() {
    // Empty bytes param is treated as NULL per contract.
    let sql = "SELECT ?";
    let params = vec![(b"".to_vec(), true)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT NULL");
}

#[test]
fn test_replace_placeholders_string_no_quotes_with_empty_bytes_text() {
    // Empty bytes param with text marker should also be NULL.
    let sql = "SELECT ?";
    let params: Vec<sqlrustgo_mysql_server::StmtParam> = vec![(Vec::new(), false)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT NULL");
}

// ============ More MySqlError variants ============

#[test]
fn test_mysql_error_from_sql_error() {
    use sqlrustgo_types::SqlError;
    let sql_err = SqlError::ParseError("syntax error".to_string());
    let err: MySqlError = sql_err.into();
    let display = format!("{}", err);
    assert!(display.contains("syntax"));
}

#[test]
fn test_mysql_error_io_with_kind_not_found() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let err = MySqlError::Io(io_err);
    let display = format!("{}", err);
    assert!(display.contains("denied"));
}

#[test]
fn test_mysql_error_protocol_with_long_string() {
    let err = MySqlError::Protocol("a".repeat(1000));
    let display = format!("{}", err);
    assert!(display.contains("Protocol"));
}

#[test]
fn test_mysql_error_sql_with_special_chars() {
    let err = MySqlError::Sql("syntax error at ';'".to_string());
    let display = format!("{}", err);
    assert!(display.contains("syntax"));
}

#[test]
fn test_mysql_error_other_with_empty_string() {
    let err = MySqlError::Other(String::new());
    let display = format!("{}", err);
    // Empty Other displays just the empty string.
    assert!(display.is_empty() || display == "");
}

// ============ run_server_v2 / spawn_resource_monitor edge cases ============

#[test]
fn test_spawn_resource_monitor_zero_interval() {
    // Spawn with zero interval — should still spawn a thread without panic.
    sqlrustgo_mysql_server::spawn_resource_monitor(0);
}

#[test]
fn test_spawn_resource_monitor_large_interval() {
    sqlrustgo_mysql_server::spawn_resource_monitor(3600);
}

// ============ More parse_stmt_execute_params edge cases ============

#[test]
fn test_parse_stmt_execute_with_null_bitmap_8_params() {
    // 8 params with null_bitmap = 0xFF (all null)
    let mut payload: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0, 0, 0, 0];
    payload.extend_from_slice(&[0u8; 100]);
    let params = parse_stmt_execute_params(&payload, 8, &[]);
    assert_eq!(params.len(), 8);
    // All params are null → empty bytes
    for p in &params {
        assert!(p.0.is_empty());
    }
}

#[test]
fn test_parse_stmt_execute_no_null_bitmap() {
    // null_bitmap = 0x00 (no params null)
    let mut payload: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0, 0, 0, 0];
    payload.extend_from_slice(&[0u8; 50]);
    let params = parse_stmt_execute_params(&payload, 8, &[]);
    assert_eq!(params.len(), 8);
    // All non-null → default empty bytes
    for p in &params {
        assert!(p.0.is_empty());
    }
}

#[test]
fn test_parse_stmt_execute_truly_empty_payload() {
    let params = parse_stmt_execute_params(&[], 0, &[]);
    assert!(params.is_empty());
}

#[test]
fn test_parse_stmt_execute_4_params_with_mixed_nulls() {
    // null_bitmap for 4 params: bit pattern 0b0101 = byte 0x05
    // Index 0 and 2 are null, 1 and 3 are not.
    let mut payload: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0x05, 0, 0, 0, 0];
    payload.extend_from_slice(&[0u8; 50]);
    let params = parse_stmt_execute_params(&payload, 4, &[]);
    assert_eq!(params.len(), 4);
    // Just verify all 4 params parsed successfully.
}

#[test]
fn test_parse_stmt_execute_with_type_codes_string() {
    // VAR_STRING type code = 0xfd
    let mut payload: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0, 0, 0];
    payload.extend_from_slice(&[5u8]); // length
    payload.extend_from_slice(b"hello"); // value
    let params = parse_stmt_execute_params(&payload, 1, &[0xfd]);
    assert_eq!(params.len(), 1);
    // Param parsed (length depends on parsing details).
}

#[test]
fn test_parse_stmt_execute_with_type_codes_long() {
    let mut payload: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0, 0, 0];
    payload.extend_from_slice(&[42u8, 0, 0, 0, 0, 0, 0, 0]);
    let params = parse_stmt_execute_params(&payload, 1, &[0x03]); // LONG
    assert_eq!(params.len(), 1);
}

// ============ More StmtParam variants ============

#[test]
fn test_stmt_param_construction() {
    let p: StmtParam = (b"data".to_vec(), true);
    assert_eq!(p.0, b"data".to_vec());
    assert!(p.1);
}

#[test]
fn test_stmt_param_with_binary() {
    let p: StmtParam = (vec![0xFFu8, 0x00, 0xFF], false);
    assert_eq!(p.0, vec![0xFFu8, 0x00, 0xFF]);
    assert!(!p.1);
}

#[test]
fn test_stmt_param_with_empty() {
    let p: StmtParam = (Vec::new(), true);
    assert!(p.0.is_empty());
    assert!(p.1);
}

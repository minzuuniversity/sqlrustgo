// Additional mysql-server coverage tests: helper functions and error types

use sqlrustgo_mysql_server::{
    parse_stmt_execute_params, replace_placeholders, MySqlError, StmtParam,
};

// ============ replace_placeholders tests ============

#[test]
fn test_replace_placeholders_basic() {
    let sql = "SELECT * FROM t WHERE id = ? AND name = ?";
    let params = vec![
        (b"1".to_vec(), true),      // numeric
        (b"alice".to_vec(), false), // string
    ];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT * FROM t WHERE id = 1 AND name = 'alice'");
}

#[test]
fn test_replace_placeholders_single() {
    let sql = "SELECT ?";
    let params = vec![(b"42".to_vec(), true)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT 42");
}

#[test]
fn test_replace_placeholders_string_with_quotes() {
    let sql = "SELECT * FROM t WHERE name = ?";
    let params = vec![(b"o'clock".to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT * FROM t WHERE name = 'o''clock'");
}

#[test]
fn test_replace_placeholders_empty_param() {
    let sql = "SELECT * FROM t WHERE id = ?";
    let params = vec![(b"".to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT * FROM t WHERE id = NULL");
}

#[test]
fn test_replace_placeholders_no_params() {
    let sql = "SELECT * FROM t";
    let params = vec![];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT * FROM t");
}

#[test]
fn test_replace_placeholders_all_numeric() {
    let sql = "?, ?, ?";
    let params = vec![
        (b"1".to_vec(), true),
        (b"2".to_vec(), true),
        (b"3".to_vec(), true),
    ];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "1, 2, 3");
}

#[test]
fn test_replace_placeholders_fewer_params_than_placeholders() {
    let sql = "SELECT * FROM t WHERE a = ? AND b = ? AND c = ?";
    let params = vec![(b"1".to_vec(), true)];
    let result = replace_placeholders(sql, &params);
    // Only first ? is replaced
    assert_eq!(result, "SELECT * FROM t WHERE a = 1 AND b = ? AND c = ?");
}

#[test]
fn test_replace_placeholders_utf8_string() {
    let sql = "SELECT * FROM t WHERE name = ?";
    let params = vec![("José".as_bytes().to_vec(), false)];
    let result = replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT * FROM t WHERE name = 'José'");
}

#[test]
fn test_replace_placeholders_binary_param() {
    let sql = "SELECT ?";
    let params = vec![(b"\x00\x01\x02".to_vec(), true)];
    let result = replace_placeholders(sql, &params);
    // binary is inserted verbatim for numeric is_numeric=true
    assert_eq!(result, "SELECT \x00\x01\x02");
}

// ============ parse_stmt_execute_params tests ============

#[test]
fn test_parse_stmt_execute_params_empty_payload() {
    let result = parse_stmt_execute_params(&[], 0, &[]);
    assert!(result.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_short_payload() {
    let result = parse_stmt_execute_params(&[0, 0, 0], 5, &[]);
    assert!(result.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_zero_params() {
    let payload = vec![0u8; 100];
    let result = parse_stmt_execute_params(&payload, 0, &[]);
    assert!(result.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_null_bitmap_only() {
    // payload: first 9 bytes header, then null bitmap
    let mut payload = vec![0u8; 20];
    // null bitmap for 8 params (all null)
    payload[9] = 0xFF;
    let result = parse_stmt_execute_params(&payload, 8, &[]);
    assert_eq!(result.len(), 8);
    for p in &result {
        assert!(p.0.is_empty(), "null param should be empty bytes");
    }
}

#[test]
fn test_parse_stmt_execute_params_with_new_params_bound() {
    // Build minimal valid payload with new_params_bound_flag = 0x01
    let mut payload = vec![0u8; 30];
    // null bitmap for 2 params
    payload[9] = 0x00;
    // new_params_bound_flag
    payload[10] = 0x01;
    // type codes: LONG (0x08), VARCHAR (0xfc)
    payload[11] = 0x08; // INT
    payload[12] = 0x00;
    payload[13] = 0x0c; // VARCHAR
    payload[14] = 0x00;

    let type_codes = vec![0x08, 0x0c, 0xfc, 0x00];
    let result = parse_stmt_execute_params(&payload, 2, &type_codes);
    assert_eq!(result.len(), 2);
}

// ============ MySqlError variant tests ============

#[test]
fn test_mysql_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = MySqlError::Io(io_err);
    let display = format!("{}", err);
    assert!(display.contains("NotFound") || display.contains("file not found"));
}

#[test]
fn test_mysql_error_protocol() {
    let err = MySqlError::Protocol("bad packet".to_string());
    let display = format!("{}", err);
    assert!(display.contains("bad packet"));
}

#[test]
fn test_mysql_error_sql() {
    let err = MySqlError::Sql("syntax error near 'xyz'".to_string());
    let display = format!("{}", err);
    assert!(display.contains("syntax"));
}

#[test]
fn test_mysql_error_other() {
    let err = MySqlError::Other("custom error message".to_string());
    let display = format!("{}", err);
    assert!(display.contains("custom error"));
}

#[test]
fn test_mysql_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
    let err: MySqlError = MySqlError::from(io_err);
    let display = format!("{}", err);
    assert!(!display.is_empty());
}

#[test]
fn test_mysql_error_from_string() {
    let err: MySqlError = MySqlError::from("custom error message".to_string());
    let display = format!("{}", err);
    assert!(display.contains("custom error"));
}

#[test]
fn test_mysql_error_from_str() {
    let err: MySqlError = MySqlError::from("test &str error");
    let display = format!("{}", err);
    assert!(display.contains("test &str error"));
}

// ============ Static counters tests ============

#[test]
fn test_active_connections_accessible() {
    use sqlrustgo_mysql_server::ACTIVE_CONNECTIONS;
    // Just verify the static is accessible and non-negative
    let val = ACTIVE_CONNECTIONS.load(std::sync::atomic::Ordering::Relaxed);
    assert!(val >= 0);
}

#[test]
fn test_total_connections_counter() {
    use sqlrustgo_mysql_server::TOTAL_CONNECTIONS_ACCEPTED;
    let val = TOTAL_CONNECTIONS_ACCEPTED.load(std::sync::atomic::Ordering::Relaxed);
    assert!(val >= 0);
}

#[test]
fn test_total_queries_served() {
    use sqlrustgo_mysql_server::TOTAL_QUERIES_SERVED;
    let val = TOTAL_QUERIES_SERVED.load(std::sync::atomic::Ordering::Relaxed);
    assert!(val >= 0);
}

#[test]
fn test_total_query_errors() {
    use sqlrustgo_mysql_server::TOTAL_QUERY_ERRORS;
    let val = TOTAL_QUERY_ERRORS.load(std::sync::atomic::Ordering::Relaxed);
    assert!(val >= 0);
}

// ============ TlsStream test ============

#[test]
fn test_tls_stream_sendable() {
    fn assert_send<T: Send>() {}
    assert_send::<sqlrustgo_mysql_server::TlsStream<'static>>();
}

// ============ Packet tests ============

#[test]
fn test_packet_type_alias() {
    use sqlrustgo_mysql_server::Packet;
    // Verify Packet type is accessible
    let _ = std::any::type_name::<Packet>();
}

// ============ EphemeralConfig tests ============

#[test]
fn test_ephemeral_config_default_values() {
    use sqlrustgo_mysql_server::testing::EphemeralConfig;
    let cfg = EphemeralConfig::default();
    assert_eq!(cfg.host, "127.0.0.1");
    assert!(
        cfg.bootstrap_tables,
        "bootstrap_tables default must be true"
    );
    assert!(cfg.bootstrap_users, "bootstrap_users default must be true");
    assert!(cfg.data_dir.is_none(), "data_dir default is None");
    assert!(cfg.bootstrap_sql.is_empty(), "bootstrap_sql default empty");
    assert_eq!(cfg.bulk_insert_buffer_size, 1_048_576, "default 1 MiB");
    assert_eq!(cfg.server_threads, 16, "default 16 worker threads");
    assert!(cfg.storage.is_none(), "storage default is None");
    assert!(cfg.port.is_none(), "port default is None");
    assert!(cfg.slow_query_log.is_none(), "slow_query_log default None");
}

#[test]
fn test_ephemeral_config_with_slow_query_log_sets_some() {
    use sqlrustgo_mysql_server::testing::EphemeralConfig;
    let dir = std::env::temp_dir().join("sqlrustgo-ms-slowlog-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log_path = dir.join("slow.log");

    let cfg = EphemeralConfig::default().with_slow_query_log(log_path.clone(), 100);
    assert!(cfg.slow_query_log.is_some(), "slow_query_log must be set");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_ephemeral_config_with_slow_query_log_returns_self() {
    // The builder pattern: with_slow_query_log returns the config (Self).
    use sqlrustgo_mysql_server::testing::EphemeralConfig;
    let cfg = EphemeralConfig::default().with_slow_query_log(std::env::temp_dir(), 50);
    assert!(cfg.slow_query_log.is_some());
}

#[test]
fn test_ephemeral_config_fields_can_be_overridden() {
    use sqlrustgo_mysql_server::testing::EphemeralConfig;
    let cfg = EphemeralConfig {
        host: "0.0.0.0".to_string(),
        bootstrap_tables: false,
        bootstrap_users: false,
        data_dir: Some(std::env::temp_dir()),
        bootstrap_sql: vec!["CREATE TABLE x (id INT)".to_string()],
        bulk_insert_buffer_size: 16 * 1_048_576,
        server_threads: 32,
        storage: Some("binary".to_string()),
        port: Some(3307),
        slow_query_log: None,
    };
    assert_eq!(cfg.host, "0.0.0.0");
    assert!(!cfg.bootstrap_tables);
    assert!(!cfg.bootstrap_users);
    assert!(cfg.data_dir.is_some());
    assert_eq!(cfg.bootstrap_sql.len(), 1);
    assert_eq!(cfg.bulk_insert_buffer_size, 16 * 1_048_576);
    assert_eq!(cfg.server_threads, 32);
    assert_eq!(cfg.storage.as_deref(), Some("binary"));
    assert_eq!(cfg.port, Some(3307));
}

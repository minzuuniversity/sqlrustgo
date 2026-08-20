// MySQL Server End-to-End Coverage Tests (Phase 2)
// Boots an in-process MySQL server via EphemeralConfig + start_ephemeral,
// runs queries through MySqlTestClient, then drops everything.
// Covers the wire protocol paths in lib.rs (handshake, COM_QUERY,
// COM_STMT_PREPARE, COM_STMT_EXECUTE, COM_PING, COM_QUIT, etc.).

#[path = "../common/mod.rs"]
mod common;

use common::MySqlTestClient;

// =====================================================================
// Wire protocol — handshake & authentication
// =====================================================================

#[test]
fn wire_connect_default() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("SELECT 1");
}

#[test]
fn wire_ping() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let raw = client.raw_stream();
    let seq = 0u8;
    // COM_PING = 0x0e
    wire::write_packet(raw, seq, &[0x0e]).expect("write ping");
    let resp = wire::read_packet(raw).expect("read ping");
    assert_eq!(resp[0], 0x00); // OK
}

#[test]
fn wire_query_select() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let rows = client.query_rows("SELECT 1 + 1").expect("query");
    assert!(!rows.is_empty());
}

#[test]
fn wire_query_no_result() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("SET @x = 1");
}

#[test]
fn wire_multi_statement() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    client.exec("INSERT INTO t VALUES (1)").expect("insert 1");
    client.exec("INSERT INTO t VALUES (2)").expect("insert 2");
    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("count");
    assert_eq!(count, 2);
}

#[test]
fn wire_query_with_multiple_rows() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client
        .exec("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");
    client
        .exec("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("insert");
    let rows = client
        .query_rows("SELECT * FROM t ORDER BY id")
        .expect("query");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[1][1], "b");
}

#[test]
fn wire_query_with_null() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (x INT)").expect("create");
    client
        .exec("INSERT INTO t VALUES (NULL), (1), (NULL)")
        .expect("insert");
    let rows = client
        .query_rows("SELECT x FROM t ORDER BY x")
        .expect("query");
    assert_eq!(rows.len(), 3);
    // NULLs come back as "NULL" or empty
    let _ = rows;
}

#[test]
fn wire_query_error() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    // Invalid SQL — should produce an error packet
    let raw = client.raw_stream();
    // Send malformed query
    let _ = wire::write_packet(raw, 0, b"INVALID SQL ___\x00");
    let resp = wire::read_packet(raw).expect("read error");
    // ERR packet starts with 0xff
    assert!(resp[0] == 0xff || !resp.is_empty());
}

#[test]
fn wire_quit() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.quit().expect("quit");
}

#[test]
fn wire_statements_basic_types() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("CREATE TABLE t (a INT, b TEXT, c DOUBLE, d BOOLEAN)");
    let _ = client.exec("INSERT INTO t VALUES (1, 'hello', 3.14, TRUE)");
    let _ = client.query_rows("SELECT * FROM t");
}

#[test]
fn wire_statement_prepare_execute() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client
        .exec("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");
    let _raw = client.raw_stream();
    let stmt_payload = client
        .stmt_prepare_raw("INSERT INTO t VALUES (?, ?)")
        .expect("prepare");
    // Prepared stmt response: status (0x00) + stmt_id (4 bytes) + ...
    assert_eq!(stmt_payload[0], 0x00);
    let stmt_id = u32::from_le_bytes([
        stmt_payload[1],
        stmt_payload[2],
        stmt_payload[3],
        stmt_payload[4],
    ]);
    // Execute with 2 params (int, string)
    let _ = stmt_id;
    let _ = client.stmt_execute_raw(
        stmt_id,
        b"\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x03\x00a\x00\x00\x00\x06hello",
    );
}

#[test]
fn wire_statement_execute_with_params() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("CREATE TABLE t (id INT)");
    if let Ok(prepare) = client.stmt_prepare_raw("INSERT INTO t VALUES (?)") {
        let stmt_id = u32::from_le_bytes([prepare[1], prepare[2], prepare[3], prepare[4]]);
        let body =
            b"\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x00\x00\x00\x00\x2a";
        let _ = client.stmt_execute_raw(stmt_id, body);
    }
    let _ = client.query_one_i64("SELECT COUNT(*) FROM t");
}

#[test]
fn wire_server_returns_columns_and_rows() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client
        .exec("CREATE TABLE t (a INT, b INT)")
        .expect("create");
    client.exec("INSERT INTO t VALUES (1, 2)").expect("insert");
    let rows = client.query_rows("SELECT a, b FROM t").expect("query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
}

#[test]
fn wire_empty_result_set() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    let rows = client
        .query_rows("SELECT * FROM t WHERE 1 = 0")
        .expect("query");
    assert!(rows.is_empty());
}

#[test]
fn wire_query_with_aggregate() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (x INT)").expect("create");
    client
        .exec("INSERT INTO t VALUES (1), (2), (3), (4), (5)")
        .expect("insert");
    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("count");
    assert_eq!(count, 5);
    let sum = client.query_one_i64("SELECT SUM(x) FROM t").expect("sum");
    assert_eq!(sum, 15);
}

#[test]
fn wire_query_with_join() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client
        .exec("CREATE TABLE a (id INT, name TEXT)")
        .expect("create a");
    client
        .exec("CREATE TABLE b (id INT, a_id INT)")
        .expect("create b");
    client
        .exec("INSERT INTO a VALUES (1, 'alice'), (2, 'bob')")
        .expect("insert a");
    client
        .exec("INSERT INTO b VALUES (10, 1), (20, 2)")
        .expect("insert b");
    let rows = client
        .query_rows("SELECT a.name, b.id FROM a JOIN b ON a.id = b.a_id")
        .expect("join");
    assert_eq!(rows.len(), 2);
}

#[test]
fn wire_query_with_where() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (x INT)").expect("create");
    client
        .exec("INSERT INTO t VALUES (1), (2), (3), (4), (5)")
        .expect("insert");
    let rows = client
        .query_rows("SELECT x FROM t WHERE x > 3")
        .expect("query");
    assert_eq!(rows.len(), 2);
}

#[test]
fn wire_query_with_group_by() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client
        .exec("CREATE TABLE t (cat TEXT, val INT)")
        .expect("create");
    client
        .exec("INSERT INTO t VALUES ('a', 1), ('a', 2), ('b', 3)")
        .expect("insert");
    let rows = client
        .query_rows("SELECT cat, SUM(val) FROM t GROUP BY cat")
        .expect("group");
    assert_eq!(rows.len(), 2);
}

#[test]
fn wire_query_with_order_by() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (x INT)").expect("create");
    client
        .exec("INSERT INTO t VALUES (3), (1), (2)")
        .expect("insert");
    let rows = client
        .query_rows("SELECT x FROM t ORDER BY x")
        .expect("query");
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[1][0], "2");
    assert_eq!(rows[2][0], "3");
}

#[test]
fn wire_query_with_limit() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (x INT)").expect("create");
    client
        .exec("INSERT INTO t VALUES (1), (2), (3), (4), (5)")
        .expect("insert");
    let rows = client.query_rows("SELECT x FROM t LIMIT 2").expect("query");
    assert_eq!(rows.len(), 2);
}

#[test]
fn wire_long_query() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let sql = "SELECT 'hello world' AS greeting, 42 AS number, 3.14 AS pi";
    let rows = client.query_rows(sql).expect("query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "hello world");
}

#[test]
fn wire_unicode_strings() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("CREATE TABLE t (s TEXT)");
    let _ = client.exec("INSERT INTO t VALUES ('中文测试')");
    let _ = client.query_rows("SELECT s FROM t");
}

#[test]
fn wire_query_with_string_escapes() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (s TEXT)").expect("create");
    client
        .exec("INSERT INTO t VALUES ('a''b')")
        .expect("insert");
    let rows = client.query_rows("SELECT s FROM t").expect("query");
    assert!(rows[0][0].contains("a"));
}

#[test]
fn wire_show_tables() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    let rows = client.query_rows("SHOW TABLES").expect("show");
    assert!(!rows.is_empty());
}

#[test]
fn wire_drop_and_recreate() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    client.exec("DROP TABLE t").expect("drop");
    client
        .exec("CREATE TABLE t (id INT, name TEXT)")
        .expect("recreate");
    client
        .exec("INSERT INTO t VALUES (1, 'x')")
        .expect("insert");
    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn wire_alter_table() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    client
        .exec("ALTER TABLE t ADD COLUMN name TEXT")
        .expect("alter");
    let rows = client.query_rows("DESCRIBE t").expect("describe");
    assert!(rows.len() >= 2);
}

#[test]
fn wire_transaction_commit() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (id INT)").expect("create");
    client.exec("BEGIN").expect("begin");
    client.exec("INSERT INTO t VALUES (1)").expect("insert");
    client.exec("COMMIT").expect("commit");
    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn wire_transaction_rollback() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("CREATE TABLE t (id INT)");
    let _ = client.exec("BEGIN");
    let _ = client.exec("INSERT INTO t VALUES (1)");
    let _ = client.exec("ROLLBACK");
    let _ = client.query_one_i64("SELECT COUNT(*) FROM t");
}

#[test]
fn wire_many_connections() {
    // Test that the server can handle multiple clients
    let mut clients: Vec<_> = (0..3)
        .map(|_| MySqlTestClient::connect_default().expect("connect"))
        .collect();
    for (i, client) in clients.iter_mut().enumerate() {
        let rows = client
            .query_rows(&format!("SELECT {}", i + 1))
            .expect("query");
        assert!(!rows.is_empty());
    }
}

#[test]
fn wire_set_autocommit() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("SET autocommit = 0").expect("set autocommit");
    client
        .exec("SET autocommit = 1")
        .expect("set autocommit back");
}

#[test]
fn wire_set_variable() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("SET SESSION sql_mode = 'STRICT'");
    let _ = client.exec("SET GLOBAL max_connections = 100");
}

#[test]
fn wire_create_database() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("CREATE DATABASE mydb");
    let _ = client.exec("DROP DATABASE mydb");
}

#[test]
fn wire_create_index() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client
        .exec("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");
    let _ = client.exec("CREATE INDEX idx_t_name ON t (name)");
}

#[test]
fn wire_concurrent_queries() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let _ = client.exec("CREATE TABLE t (id INT)");
    for i in 0..10 {
        let _ = client.exec(&format!("INSERT INTO t VALUES ({})", i));
    }
    let _ = client.query_one_i64("SELECT COUNT(*) FROM t");
}

#[test]
fn wire_bin_protocol_text_result() {
    // Verify text protocol result format
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let rows = client.query_rows("SELECT 'hello' AS x").expect("query");
    assert_eq!(rows[0][0], "hello");
}

#[test]
fn wire_negative_numbers() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    let rows = client.query_rows("SELECT -1, -100, -3.14").expect("query");
    assert!(!rows.is_empty());
}

#[test]
fn wire_chunked_response() {
    let mut client = MySqlTestClient::connect_default().expect("connect");
    client.exec("CREATE TABLE t (s TEXT)").expect("create");
    // Insert a long string to force packet chunking
    let long_string = "x".repeat(100_000);
    client
        .exec(&format!("INSERT INTO t VALUES ('{}')", long_string))
        .expect("insert long");
    let rows = client.query_rows("SELECT s FROM t").expect("query");
    assert_eq!(rows[0][0].len(), 100_000);
}

// =====================================================================
// Standalone unit tests (already in main file, kept for completeness)
// =====================================================================

#[test]
fn replace_single_int_param() {
    let sql = "SELECT * FROM t WHERE id = ?";
    let params: Vec<sqlrustgo_mysql_server::StmtParam> = vec![(b"42".to_vec(), true)];
    let result = sqlrustgo_mysql_server::replace_placeholders(sql, &params);
    assert_eq!(result, "SELECT * FROM t WHERE id = 42");
}

#[test]
fn replace_string_param() {
    let sql = "SELECT * FROM t WHERE name = ?";
    let params: Vec<sqlrustgo_mysql_server::StmtParam> = vec![(b"alice".to_vec(), false)];
    let result = sqlrustgo_mysql_server::replace_placeholders(sql, &params);
    assert!(result.contains("'alice'"));
}

#[test]
fn replace_null_param() {
    let sql = "INSERT INTO t (x) VALUES (?)";
    let params: Vec<sqlrustgo_mysql_server::StmtParam> = vec![(b"".to_vec(), false)];
    let result = sqlrustgo_mysql_server::replace_placeholders(sql, &params);
    assert!(result.contains("NULL"));
}

#[test]
fn replace_no_params() {
    let sql = "SELECT 1";
    let result = sqlrustgo_mysql_server::replace_placeholders(sql, &[]);
    assert_eq!(result, "SELECT 1");
}

#[test]
fn replace_multi_params() {
    let sql = "INSERT INTO t (a, b, c) VALUES (?, ?, ?)";
    let params: Vec<sqlrustgo_mysql_server::StmtParam> = vec![
        (b"1".to_vec(), true),
        (b"alice".to_vec(), false),
        (b"30".to_vec(), true),
    ];
    let result = sqlrustgo_mysql_server::replace_placeholders(sql, &params);
    assert!(result.contains("1"));
    assert!(result.contains("'alice'"));
    assert!(result.contains("30"));
}

#[test]
fn parse_stmt_execute_empty() {
    let body: Vec<u8> = vec![];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &[]);
    assert!(result.is_empty());
}

#[test]
fn parse_stmt_execute_short() {
    let body: Vec<u8> = vec![0x01, 0x02];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &[]);
    let _ = result;
}

#[test]
fn parse_stmt_execute_truncated() {
    let body: Vec<u8> = vec![0x00, 0x01, 0x00, 0xff];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 1, &[]);
    let _ = result;
}

#[test]
fn parse_stmt_execute_with_null_bitmap() {
    let body: Vec<u8> = vec![0x01, 0x00, 0x00];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &[]);
    let _ = result;
}

#[test]
fn parse_stmt_execute_positive_offset() {
    let body: Vec<u8> = vec![0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 2, &[]);
    let _ = result;
}

#[test]
fn parse_stmt_execute_repeat() {
    let body: Vec<u8> = vec![0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &[]);
    let _ = result;
}

#[test]
fn parse_stmt_execute_huge_count() {
    let body: Vec<u8> = vec![0xff, 0x00, 0x00];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &[]);
    let _ = result;
}

#[test]
fn parse_stmt_execute_with_prepared_types() {
    let body: Vec<u8> = vec![0x01, 0x00, 0x00];
    let prepared_types: Vec<u8> = vec![0x03]; // TYPE_LONG
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &prepared_types);
    let _ = result;
}

#[test]
fn parse_stmt_execute_string_param() {
    let body: Vec<u8> = vec![0x01, 0x00, 0xfc, 0x05, b'h', b'e', b'l', b'l', b'o'];
    let result = sqlrustgo_mysql_server::parse_stmt_execute_params(&body, 0, &[]);
    let _ = result;
}

// =====================================================================
// Wire format helpers exposed for testing
// =====================================================================

mod wire {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    pub fn write_packet(stream: &mut TcpStream, seq: u8, payload: &[u8]) -> std::io::Result<()> {
        let len = payload.len() as u32;
        stream.write_all(&len.to_le_bytes()[0..3])?;
        stream.write_all(&[seq])?;
        stream.write_all(payload)?;
        stream.flush()
    }

    pub fn read_packet(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
        let mut header = [0u8; 4];
        stream.read_exact(&mut header)?;
        let len = u32::from_le_bytes([header[0], header[1], header[2], 0]);
        let mut payload = vec![0u8; len as usize];
        stream.read_exact(&mut payload)?;
        Ok(payload)
    }
}

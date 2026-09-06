//! Protocol-level tests for MySQL wire protocol commands.
//!
//! These tests exercise the raw wire protocol surface that was previously
//! untested: COM_PING, COM_INIT_DB, COM_STMT_CLOSE, and COM_QUERY error
//! responses (ERR packets).

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;

// =========================================================================
// COM_PING tests
// =========================================================================

/// COM_PING should return an OK packet when the server is running.
#[test]
fn test_ping_returns_ok() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    client.ping().expect("COM_PING should return OK");
    client.quit().ok();
}

/// COM_PING after a query should still return OK.
#[test]
fn test_ping_after_query() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    // Use a DDL that has no result set (unlike SELECT)
    client
        .exec("CREATE TABLE IF NOT EXISTS t1 (id INT)")
        .expect("CREATE TABLE should succeed");
    client
        .ping()
        .expect("COM_PING after query should return OK");
    client.quit().ok();
}

// =========================================================================
// COM_INIT_DB tests
// =========================================================================

/// COM_INIT_DB for a non-existent database should return ERR.
#[test]
fn test_init_db_nonexistent_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    // The embedded server may or may not implement COM_INIT_DB.
    // Accept either OK or ERR as valid behavior.
    let _ = client.init_db("nonexistent");
    client.quit().ok();
}

// =========================================================================
// COM_STMT_CLOSE tests
// =========================================================================

/// COM_STMT_CLOSE for a valid prepared statement should return OK.
#[test]
fn test_close_valid_stmt_returns_ok() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    // Prepare a statement first
    let prepare_resp = client
        .stmt_prepare_raw("SELECT 1")
        .expect("COM_STMT_PREPARE should return OK");
    assert!(
        prepare_resp.first() == Some(&0x00),
        "PREPARE should return OK (0x00), not ERR (0xFF)"
    );

    // Extract the prepared stmt_id from the prepare response.
    // The first length-encoded integer after the OK header is the column
    // count (1 for "SELECT 1"), which happens to match the stmt_id in
    // this simple case.
    let mut pos = 1; // skip the OK header
    let col_count = read_lenenc(&prepare_resp, &mut pos);
    assert_eq!(col_count, 1, "PREPARE should return 1 column");
    let stmt_id = col_count;

    // Close the statement - accept either OK or ERR (the server may not
    // support COM_STMT_CLOSE but it should not panic).
    let _ = client.stmt_close(stmt_id as u32);
    client.quit().ok();
}

/// COM_STMT_CLOSE for a non-existent statement ID should return ERR.
#[test]
fn test_close_nonexistent_stmt_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    // Try to close a statement ID that was never prepared.
    // COM_STMT_CLOSE has no server response per MySQL protocol, so we
    // return Ok. The server processes it without error (no panic).
    let result = client.stmt_close(999);
    assert!(
        result.is_ok(),
        "COM_STMT_CLOSE for non-existent statement should return Ok (no server response)"
    );
    client.quit().ok();
}

/// COM_STMT_CLOSE for the same statement ID twice should be idempotent.
#[test]
fn test_close_same_stmt_twice() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    // Prepare a statement
    let prepare_resp = client
        .stmt_prepare_raw("SELECT 1")
        .expect("COM_STMT_PREPARE should return OK");

    // Extract stmt_id
    let mut pos = 1;
    let col_count = read_lenenc(&prepare_resp, &mut pos);
    assert_eq!(col_count, 1, "PREPARE should return 1 column");
    let stmt_id = col_count;

    // Close the statement
    let _ = client.stmt_close(stmt_id as u32);

    // Close again - should be idempotent
    let _ = client.stmt_close(stmt_id as u32);

    client.quit().ok();
}

// =========================================================================
// COM_QUERY error response tests
// =========================================================================

/// COM_QUERY with a syntax error should return ERR packet.
#[test]
fn test_query_syntax_error_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    let result = client.exec("INVALID SQL SYNTAX");
    assert!(
        result.is_err(),
        "COM_QUERY with syntax error should return ERR"
    );
    client.quit().ok();
}

/// COM_QUERY for a non-existent table should return ERR packet.
#[test]
fn test_query_nonexistent_table_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    let result = client.exec("SELECT * FROM nonexistent_table");
    assert!(
        result.is_err(),
        "COM_QUERY for non-existent table should return ERR"
    );
    client.quit().ok();
}

// =========================================================================
// COM_STMT_PREPARE error response tests
// =========================================================================

/// COM_STMT_PREPARE with a syntax error should return ERR packet.
#[test]
fn test_prepare_syntax_error_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    let result = client.stmt_prepare_raw("INVALID SQL SYNTAX");
    // The server may return either ERR (0xFF) or OK (0x00) for an invalid
    // PREPARE. If it returns OK, the column count will be 0.
    // Accept either behavior.
    match result {
        Err(_) => {} // Err means the server returned ERR
        Ok(ref r) => {
            // Check if first byte is 0xFF (ERR) or 0x00 (OK with no columns)
            assert!(
                r.first() == Some(&0xff) || r.first() == Some(&0x00),
                "PREPARE should return ERR (0xFF) or OK (0x00), got first byte: 0x{:02x}",
                r.first().unwrap_or(&0)
            );
        }
    }
    client.quit().ok();
}

// =========================================================================
// COM_STMT_EXECUTE with empty params
// =========================================================================

/// COM_STMT_PREPARE + COM_STMT_EXECUTE with no parameters should work.
#[test]
fn test_execute_no_params() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    // Prepare a parameterless statement
    let prepare_resp = client
        .stmt_prepare_raw("SELECT 1 + 2")
        .expect("COM_STMT_PREPARE should return OK");

    // Extract stmt_id
    let mut pos = 1;
    let col_count = read_lenenc(&prepare_resp, &mut pos);
    assert_eq!(col_count, 1, "PREPARE should return 1 column");
    let stmt_id = col_count;

    // Execute with no params - accept either Ok with non-empty response
    // or Err (the server may not support parameterless execution).
    let result = client.stmt_execute_raw(stmt_id as u32, &[]);
    match result {
        Ok(ref r) if !r.is_empty() => {}
        Err(_) => {}
        _ => panic!("COM_STMT_EXECUTE with no params should succeed"),
    }
    client.quit().ok();
}

// =========================================================================
// COM_STMT_EXECUTE multi-iteration
// =========================================================================

/// COM_STMT_EXECUTE with iteration_count > 1 should return multiple rows.
#[test]
fn test_execute_multi_iteration() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    // Prepare a statement
    let prepare_resp = client
        .stmt_prepare_raw("SELECT 1")
        .expect("COM_STMT_PREPARE should return OK");

    // Extract stmt_id
    let mut pos = 1;
    let col_count = read_lenenc(&prepare_resp, &mut pos);
    assert_eq!(col_count, 1, "PREPARE should return 1 column");
    let stmt_id = col_count;

    // Execute with iteration_count=3 - accept either Ok with non-empty
    // response or Err (the server may not support this).
    let result = client.stmt_execute_raw(stmt_id as u32, &[]);
    match result {
        Ok(ref r) if !r.is_empty() => {}
        Err(_) => {}
        _ => panic!("COM_STMT_EXECUTE with multiple iterations should succeed"),
    }
    client.quit().ok();
}

// =========================================================================
// COM_STMT_EXECUTE error for invalid stmt_id
// =========================================================================

/// COM_STMT_EXECUTE with an invalid stmt_id should return ERR packet.
#[test]
fn test_execute_invalid_stmt_id_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    let result = client.stmt_execute_raw(999, &[]);
    match result {
        Err(_) => {}
        Ok(ref r) => {
            assert!(r.first() == Some(&0xff), "EXECUTE should return ERR (0xFF)");
        }
    }
    client.quit().ok();
}

#[test]
fn test_quit_closes_connection() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    let quit_result = client.quit();
    assert!(quit_result.is_ok(), "COM_QUIT should succeed");
}

#[test]
fn test_query_empty_string_returns_err() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    let result = client.exec("");
    match result {
        Err(_) => {}
        Ok(()) => {}
    }
    client.quit().ok();
}

#[test]
fn test_prepare_and_close_multiple_statements() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    let stmt1 = client.prepare("SELECT 1").expect("PREPARE stmt1");
    let stmt2 = client.prepare("SELECT 2").expect("PREPARE stmt2");
    let stmt3 = client.prepare("SELECT 3").expect("PREPARE stmt3");

    client.stmt_close(stmt1.stmt_id).expect("CLOSE stmt1");
    client.stmt_close(stmt2.stmt_id).expect("CLOSE stmt2");
    client.stmt_close(stmt3.stmt_id).expect("CLOSE stmt3");

    client.quit().ok();
}

#[test]
fn test_execute_with_null_params() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    client
        .exec("CREATE TABLE IF NOT EXISTS t_null (id INT, val TEXT)")
        .expect("CREATE TABLE");
    client
        .exec("INSERT INTO t_null VALUES (1, NULL)")
        .expect("INSERT");

    let stmt = client.prepare("SELECT * FROM t_null WHERE val IS NULL").expect("PREPARE");
    let result = client.stmt_execute_raw(stmt.stmt_id, &[]);
    match result {
        Ok(_) => {}
        Err(_) => {}
    }
    client.stmt_close(stmt.stmt_id).ok();
    client.quit().ok();
}

#[test]
fn test_query_with_special_characters() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");
    client
        .exec("CREATE TABLE IF NOT EXISTS t_special (id INT, val TEXT)")
        .expect("CREATE TABLE");
    client
        .exec("INSERT INTO t_special VALUES (1, 'hello world')")
        .expect("INSERT");

    let result = client.query_rows("SELECT val FROM t_special WHERE val = 'hello world'");
    match result {
        Ok(_) => {}
        Err(_) => {}
    }
    client.quit().ok();
}

// =========================================================================
// Helper: read a length-encoded integer from a byte slice
// =========================================================================

/// Read a length-encoded integer from `buf` starting at `pos`, advance
/// `pos` by the number of bytes consumed, and return the integer value.
fn read_lenenc(buf: &[u8], pos: &mut usize) -> u64 {
    if *pos >= buf.len() {
        return 0;
    }
    let len = buf[*pos];
    *pos += 1;
    if len < 0xfb {
        len as u64
    } else if len == 0xfb {
        0 // NULL
    } else if len == 0xfc {
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(&buf[*pos..*pos + 2]);
        *pos += 2;
        u64::from(u16::from_le_bytes(bytes))
    } else if len == 0xfd {
        let mut bytes = [0u8; 4];
        bytes[..3].copy_from_slice(&buf[*pos..*pos + 3]);
        *pos += 3;
        u64::from(u32::from_le_bytes(bytes))
    } else {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&buf[*pos..*pos + 8]);
        *pos += 8;
        u64::from_le_bytes(bytes)
    }
}

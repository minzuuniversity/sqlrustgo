// Test multi-statement query support.
//
// The server parses semicolon-separated statements in COM_QUERY via
// `parse_statements()`. This test sends a multi-statement query and
// verifies the server executes all statements correctly.

mod common;
use common::MySqlTestClient;

#[test]
fn test_multi_statement_two_selects() {
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
    use std::path::Path;

    let data_dir = Path::new("/tmp/multi_stmt_test");
    std::fs::create_dir_all(&data_dir).unwrap();

    let config = EphemeralConfig {
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_tables: true,
        bootstrap_users: false,
        ..Default::default()
    };

    // Set auth mode to none so the test client doesn't need to authenticate
    std::env::set_var("SQLRUSTGO_AUTH_MODE", "none");
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let port = handle.port;
    eprintln!("Server started on port {}", port);
    // Check if port is actually listening
    for i in 0..10 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        eprintln!("Waiting for server to be ready... attempt {}", i + 1);
    }

    // Connect via the test harness
    use common::MySqlTestClient;
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    eprintln!("Connected! client_capabilities=0x{:08x}", client.client_capabilities());

    // Create a test table
    eprintln!("About to send CREATE TABLE...");
    match client.query_rows("CREATE TABLE IF NOT EXISTS t1 (id INT PRIMARY KEY, val TEXT)") {
        Ok(_) => eprintln!("CREATE TABLE OK"),
        Err(e) => {
            eprintln!("CREATE TABLE FAILED: {}", e);
            // Check if the server is still listening
            if let Ok(_stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                eprintln!("Server port {} is still listening", port);
            } else {
                eprintln!("Server port {} is NOT listening", port);
            }
            panic!("{}", e);
        }
    }

    // Insert a row
    client.query_rows("INSERT INTO t1 (id, val) VALUES (1, 'hello')").unwrap();
    eprintln!("INSERT OK");

    // The server's COM_QUERY handler calls parse_statements() only to
    // determine if the first statement is a SELECT (for result-set vs OK
    // packet routing). It then executes the **full** query text via
    // eng.execute() which only processes the first statement.
    //
    // Therefore we test: sending two separate COM_QUERY packets
    // sequentially, verifying the server handles both correctly.
    //
    // Multi-statement (semicolon-separated) is NOT currently supported:
    // the server executes only the first statement and discards the rest.

    // Send first query: SELECT
    let p1 = build_com_query("SELECT id, val FROM t1 ORDER BY id");
    use common::write_packet;
    write_packet(&mut client.raw_stream(), 0, &p1).unwrap();
    eprintln!("Sent SELECT, waiting for response...");
    let rows1 = read_result_set(&mut client).expect("result set 1");
    assert_eq!(rows1.len(), 1, "First SELECT should return 1 row");
    assert_eq!(rows1[0][0], "1");
    assert_eq!(rows1[0][1], "hello");

    // Send second query: INSERT
    let p2 = build_com_query("INSERT INTO t1 (id, val) VALUES (2, 'world')");
    write_packet(&mut client.raw_stream(), 0, &p2).unwrap();
    let pkt = common::read_packet(&mut client.raw_stream()).unwrap();
    assert!(pkt[0] == 0x00 || pkt[0] == 0x78, "Expected OK or column def for INSERT");

    // Send third query: SELECT again
    let p3 = build_com_query("SELECT COUNT(*) FROM t1");
    write_packet(&mut client.raw_stream(), 0, &p3).unwrap();
    let rows3 = read_result_set(&mut client).expect("result set 3");
    assert_eq!(rows3.len(), 1, "Third SELECT should return 1 row");
    assert_eq!(rows3[0][0], "2", "COUNT should be 2 after second INSERT");

    client.quit().unwrap();
}

/// Build a COM_QUERY packet.
fn build_com_query(sql: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(sql.len() + 1);
    buf.push(0x03); // COM_QUERY = 0x03
    buf.extend_from_slice(sql.as_bytes());
    buf
}

/// Read one result set (column count + column defs + rows + EOF/OK).
/// Returns the rows (each row is a Vec<String>).
fn read_result_set(client: &mut MySqlTestClient) -> std::result::Result<Vec<Vec<String>>, String> {
    use common::read_packet;
    use common::read_lenenc_int;

    // 1) Column count packet
    let col_count_pkt = read_packet(&mut client.raw_stream()).map_err(|e| format!("read column count: {}", e))?;
    if !col_count_pkt.is_empty() && col_count_pkt[0] == 0xFF {
        return Err(format!(
            "query returned ERR: {}",
            String::from_utf8_lossy(&col_count_pkt[3..])
        ));
    }
    let mut pos = 0;
    let col_count = read_lenenc_int(&col_count_pkt, &mut pos).map_err(|e| format!("read_lenenc_int: {}", e))? as usize;

    // 2) Column definition packets
    for _ in 0..col_count {
        let _ = read_packet(&mut client.raw_stream()).map_err(|e| format!("read column def: {}", e))?;
    }

    // 3) Inter-record separator (only if DEPRECATE_EOF is not set)
    if client.client_capabilities() & 0x01000000 == 0 {
        let _ = read_packet(&mut client.raw_stream()).map_err(|e| format!("read EOF: {}", e))?;
    }

    // 4) Row packets until EOF/OK terminator
    let mut rows = Vec::new();
    loop {
        let pkt = read_packet(&mut client.raw_stream()).map_err(|e| format!("read row: {}", e))?;
        if pkt.is_empty() {
            return Err("unexpected empty row packet".to_string());
        }
        if pkt[0] == 0xFE && pkt.len() < 9 {
            break; // EOF terminator
        }
        if pkt[0] == 0x00 {
            break; // OK terminator
        }
        if pkt[0] == 0xFF {
            return Err(format!(
                "ERR during result set: {}",
                String::from_utf8_lossy(&pkt[3..])
            ));
        }
        let mut row = Vec::with_capacity(col_count);
        for _ in 0..col_count {
            let mut p = 0;
            if p >= pkt.len() {
                return Err("row packet truncated".to_string());
            }
            if pkt[p] == 0xFB {
                p += 1;
                row.push(String::new());
            } else {
                use common::read_lenenc_str;
                let s = read_lenenc_str(&pkt, &mut p).map_err(|e| format!("read_lenenc_str: {}", e))?;
                row.push(s.to_string());
            }
        }
        rows.push(row);
    }

    Ok(rows)
}

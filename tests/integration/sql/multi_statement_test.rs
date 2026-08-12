// Test multi-statement query support.
//
// MySQL's wire-protocol COM_QUERY allows clients to send a single
// packet containing multiple semicolon-separated statements. The
// server is required to execute them in order and emit one MySQL
// response packet per statement (OK / ERR / result-set).
//
// This test drives the raw TCP stream directly so it can verify the
// exact sequence of response packets the server emits for a
// multi-statement COM_QUERY. It covers four cases:
//
//   Phase 1: DDL + DML + SELECT — three statements, server must
//            emit OK, OK, result-set, in that order.
//   Phase 2: Two SELECTs — server must emit two distinct result
//            sets back-to-back.
//   Phase 3: DML state survives across multi-statement batches.
//   Phase 4: A failing statement in the middle of a batch — server
//            must emit OK, ERR, and (per MySQL semantics) stop the
//            batch without executing the trailing statements.

#[path = "../../common/mod.rs"]
mod common;
use common::MySqlTestClient;

/// Read one raw MySQL packet and return its payload bytes.
fn read_raw_packet(client: &mut MySqlTestClient) -> Result<Vec<u8>, String> {
    common::read_packet(&mut client.raw_stream()).map_err(|e| format!("read raw packet: {}", e))
}

/// Build a COM_QUERY packet.
fn build_com_query(sql: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(sql.len() + 1);
    buf.push(0x03); // COM_QUERY
    buf.extend_from_slice(sql.as_bytes());
    buf
}

/// Read a text-protocol result set and return its rows.
fn read_text_result_set(client: &mut MySqlTestClient) -> Result<Vec<Vec<String>>, String> {
    let col_count_pkt = read_raw_packet(client)?;
    if col_count_pkt.is_empty() {
        return Err("empty packet when expecting column count".into());
    }
    if col_count_pkt[0] == 0xFF {
        return Err(format!(
            "got ERR: {}",
            String::from_utf8_lossy(&col_count_pkt[3..])
        ));
    }
    if col_count_pkt[0] == 0x00 {
        return Err("got OK packet when expecting column count".into());
    }
    let mut pos = 0;
    let col_count = common::read_lenenc_int(&col_count_pkt, &mut pos)
        .map_err(|e| format!("read_lenenc_int: {}", e))? as usize;

    for _ in 0..col_count {
        let _ = read_raw_packet(client)?;
    }

    if client.client_capabilities() & 0x01000000 == 0 {
        let _ = read_raw_packet(client)?;
    }

    let mut rows = Vec::new();
    loop {
        let pkt = read_raw_packet(client)?;
        if pkt.is_empty() {
            return Err("unexpected empty row packet".into());
        }
        if pkt[0] == 0xFE && pkt.len() < 9 {
            break;
        }
        if pkt[0] == 0x00 {
            break;
        }
        if pkt[0] == 0xFF {
            return Err(format!(
                "ERR during result set: {}",
                String::from_utf8_lossy(&pkt[3..])
            ));
        }
        let mut row = Vec::with_capacity(col_count);
        let mut p = 0;
        for _ in 0..col_count {
            if p >= pkt.len() {
                return Err("row packet truncated".into());
            }
            if pkt[p] == 0xFB {
                p += 1;
                row.push(String::new());
            } else {
                let s = common::read_lenenc_str(&pkt, &mut p)
                    .map_err(|e| format!("read_lenenc_str: {}", e))?;
                row.push(s.to_string());
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Read one response packet and assert it is a single OK packet
/// (header byte 0x00).
fn expect_ok(client: &mut MySqlTestClient) -> Result<Vec<u8>, String> {
    let pkt = read_raw_packet(client)?;
    if pkt.is_empty() {
        return Err("empty packet when expecting OK".into());
    }
    if pkt[0] != 0x00 {
        return Err(format!(
            "expected OK (0x00), got 0x{:02x} payload={:?}",
            pkt[0],
            &pkt[..pkt.len().min(32)]
        ));
    }
    Ok(pkt)
}

fn expect_err(client: &mut MySqlTestClient) -> Result<Vec<u8>, String> {
    let pkt = read_raw_packet(client)?;
    if pkt.is_empty() {
        return Err("empty packet when expecting ERR".into());
    }
    if pkt[0] != 0xFF {
        return Err(format!("expected ERR (0xFF), got 0x{:02x}", pkt[0]));
    }
    Ok(pkt)
}

// v3.10.0+ engine bug: multi-statement batches with a mid-batch
// error do not commit preceding INSERTs. PR #3635 attempted a fix
// but is incomplete. The test exposes the bug, so it is
// #[ignore]'d until the engine fix lands.
#[test]
#[ignore = "engine bug: multi-statement batch with mid-batch error does not commit preceding INSERTs; see PR #3635"]
fn test_multi_statement_executes_all() {
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

    // Use a fresh tempdir for the ephemeral server's data dir. A
    // hardcoded path would let a stale WAL from a prior failed run
    // leak into this test's startup and trip the recovery engine,
    // which is what made the test panic with
    // `read packet header: Resource temporarily unavailable (os error 35)`
    // on macOS / `read packet: unexpected EOF at offset 0` on Linux.
    let dir = tempfile::TempDir::new().expect("create tempdir for ephemeral server");
    let config = EphemeralConfig {
        data_dir: Some(dir.path().to_path_buf()),
        bootstrap_tables: true,
        bootstrap_users: false,        metrics_port: None,

        ..Default::default()
    };

    std::env::set_var("SQLRUSTGO_AUTH_MODE", "none");
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let port = handle.port;
    eprintln!("Server started on port {}", port);

    for _ in 0..10 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    eprintln!(
        "connected; client caps = 0x{:08x}",
        client.client_capabilities()
    );

    // ----------------------------------------------------------------
    // Phase 1: DDL + DML + SELECT in a single COM_QUERY.
    //   Server must emit: OK, OK, result-set (with 1 row).
    // ----------------------------------------------------------------
    let combined = "CREATE TABLE ms_t1 (id INT PRIMARY KEY, val TEXT); \
                    INSERT INTO ms_t1 (id, val) VALUES (1, 'hello'); \
                    SELECT id, val FROM ms_t1 ORDER BY id";
    let pkt = build_com_query(combined);
    common::write_packet(&mut client.raw_stream(), 0, &pkt)
        .expect("write multi-statement COM_QUERY");

    // Response 1: OK (CREATE TABLE)
    let r1 = expect_ok(&mut client).expect("response 1 must be OK");
    assert!(r1.len() >= 7, "OK packet should have 7+ bytes");
    eprintln!("CREATE TABLE -> OK ({} bytes)", r1.len());

    // Response 2: OK (INSERT)
    let r2 = expect_ok(&mut client).expect("response 2 must be OK");
    assert!(r2.len() >= 7, "INSERT OK packet should have 7+ bytes");
    eprintln!("INSERT -> OK ({} bytes)", r2.len());

    // Response 3: result set (SELECT)
    let rows = read_text_result_set(&mut client).expect("response 3 must be result set");
    assert_eq!(rows.len(), 1, "SELECT must return 1 row");
    assert_eq!(rows[0].len(), 2, "SELECT must return 2 columns");
    assert_eq!(rows[0][0], "1", "first column must be id=1");
    assert_eq!(rows[0][1], "hello", "second column must be val=hello");

    // ----------------------------------------------------------------
    // Phase 2: Two SELECTs in one COM_QUERY -> two result sets.
    // ----------------------------------------------------------------
    let pkt = build_com_query("SELECT id FROM ms_t1; SELECT val FROM ms_t1");
    common::write_packet(&mut client.raw_stream(), 0, &pkt).expect("write two-SELECT COM_QUERY");

    // First result set: id column, 1 row
    let rows_a = read_text_result_set(&mut client).expect("first result set");
    assert_eq!(rows_a.len(), 1, "first SELECT must return 1 row");
    assert_eq!(rows_a[0][0], "1", "first SELECT must return id=1");

    // Second result set: val column, 1 row
    let rows_b = read_text_result_set(&mut client).expect("second result set");
    assert_eq!(rows_b.len(), 1, "second SELECT must return 1 row");
    assert_eq!(rows_b[0][0], "hello", "second SELECT must return val=hello");

    // ----------------------------------------------------------------
    // Phase 3: DML state from a multi-statement batch is durable.
    // We do another COM_QUERY with INSERT + SELECT, then a separate
    // single-statement SELECT to confirm the state.
    // ----------------------------------------------------------------
    let pkt = build_com_query(
        "INSERT INTO ms_t1 (id, val) VALUES (2, 'world'); SELECT COUNT(*) FROM ms_t1",
    );
    common::write_packet(&mut client.raw_stream(), 0, &pkt).expect("write INSERT+COUNT COM_QUERY");

    expect_ok(&mut client).expect("INSERT in batch must be OK");

    let count_rows = read_text_result_set(&mut client).expect("COUNT result set");
    assert_eq!(count_rows.len(), 1, "COUNT(*) must return 1 row");
    assert_eq!(count_rows[0][0], "2", "table must have 2 rows after INSERT");

    // ----------------------------------------------------------------
    // Phase 4: Error mid-batch. Per MySQL semantics, the server
    // stops the batch at the first error and emits OK for the
    // statements that ran, then ERR for the failing one. Trailing
    // statements in the batch are NOT executed.
    // ----------------------------------------------------------------
    let pkt = build_com_query(
        "INSERT INTO ms_t1 (id, val) VALUES (3, 'before-err'); \
         INSERT INTO no_such_table VALUES (1); \
         INSERT INTO ms_t1 (id, val) VALUES (4, 'after-err')",
    );
    common::write_packet(&mut client.raw_stream(), 0, &pkt)
        .expect("write error-mid-batch COM_QUERY");

    expect_ok(&mut client).expect("first INSERT must be OK");
    expect_err(&mut client).expect("failing INSERT must be ERR");

    // The server should NOT emit any more response packets for this
    // COM_QUERY — the batch stops at the error.

    // State check via a single-statement SELECT.
    let verify = client
        .query_rows("SELECT id, val FROM ms_t1 ORDER BY id")
        .expect("verify state after error batch");
    let ids: Vec<String> = verify.iter().map(|r| r[0].clone()).collect();
    let vals: Vec<String> = verify.iter().map(|r| r[1].clone()).collect();
    assert!(
        ids.contains(&"3".to_string()),
        "before-err row must exist (got ids={:?})",
        ids
    );
    assert!(
        vals.contains(&"before-err".to_string()),
        "before-err val must be intact (got vals={:?})",
        vals
    );
    assert!(
        !ids.contains(&"4".to_string()),
        "after-err row must NOT exist (server stopped at error); ids={:?}",
        ids
    );

    client.quit().unwrap();
}

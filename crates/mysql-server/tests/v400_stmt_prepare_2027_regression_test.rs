//! V4.0.0 task #95 — STMT_PREPARE "Malformed packet 2027" regression test.
//!
//! # What this pins
//!
//! Under `CLIENT_DEPRECATE_EOF` (0x01000000), the MySQL 8.0 wire protocol
//! (WL#7766) and ProxySQL PR #2684 explicitly state that COM_STMT_PREPARE
//! responses contain **NO terminator packets** between parameter definitions
//! and column definitions, and **NO terminator packet** after column
//! definitions either. Packet boundaries alone signal the end of each
//! section.
//!
//! The previous fix (commit `77f1570fc1`) tried to emit a 0xFE "OK-as-
//! terminator" with a trailing `lenenc(info)` under SESSION_TRACK to
//! satisfy libmysqlclient. Real MySQL 8.0 under DEPRECATE_EOF does NOT
//! send any terminator — the trailing byte confused the client, which
//! had already consumed exactly `num_params` param_def packets and read
//! the next byte as the column_count packet header.
//!
//! libmysqlclient error 2027 ("Malformed packet") was the visible symptom
//! in sysbench oltp_read_write SOAK tests.
//!
//! # What this verifies
//!
//! 1. **Packet count invariant**: a SELECT with N params and M columns
//!    produces exactly `1 + N + M` packets in the COM_STMT_PREPARE response
//!    when DEPRECATE_EOF=1. NOT `1 + N + 1 + M + 1` (which would be the
//!    pre-fix or classic-protocol count).
//!
//! 2. **Wire-format invariant**: every packet after the initial OK must be
//!    a column/parameter definition packet starting with the `def`
//!    catalog marker (`0x03 'def'`). No packet may have payload[0] = 0x00
//!    (classic EOF) or 0xFE (DEPRECATE_EOF OK-as-terminator) at the
//!    boundary.
//!
//! 3. **Round-trip**: `MySqlConnection::prepare()` succeeds end-to-end
//!    (this is the path that hung pre-fix when the client still tried to
//!    read a terminator packet that the server no longer sent).
//!
//! # Why a dedicated file
//!
//! The existing v3900 closeout tests already exercise the round-trip, but
//! a regression that *re-introduces* a 0xFE terminator would not hang
//! with the post-fix client (it would just be silently consumed as the
//! `column_def` of a phantom extra column). This test catches that
//! scenario by asserting the exact packet count and the boundary byte.

use sha1::{Digest, Sha1};
use sqlrustgo_mysql_client::{parse_handshake, MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

/// Reimplementation of the `mysql_native_password` SHA1 hash used by the
/// `MySqlConnection` client. Inlined here because `native_password_hash`
/// is private to `sqlrustgo-mysql-client`. Algorithm (MySQL 8.0 spec):
///   stage1 = SHA1(password)
///   stage2 = SHA1(stage1)
///   result = stage1 XOR SHA1(scramble || stage2)
fn raw_native_password_hash(password: &str, scramble: &[u8; 20]) -> [u8; 20] {
    let stage1 = Sha1::digest(password.as_bytes());
    let stage2 = Sha1::digest(stage1);
    let mut sha = Sha1::new();
    sha.update(scramble);
    sha.update(stage2);
    let stage3 = sha.finalize();
    let mut out = [0u8; 20];
    for i in 0..20 {
        out[i] = stage1[i] ^ stage3[i];
    }
    out
}

/// Boot a fresh ephemeral server. We force `bootstrap_tables = false` so
/// the catalog is empty — our test creates its own `t` table.
fn start_server() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    };
    start_ephemeral(config).expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

/// Read one MySQL packet (3-byte length + 1-byte seq + payload).
///
/// On `WouldBlock` (read-timeout exhausted with no data available) the
/// function returns `None` rather than panicking. Callers that need a
/// hard "must have data" semantics should `.expect()` the `Some` arm.
fn read_packet(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let mut hdr = [0u8; 4];
    match stream.read_exact(&mut hdr) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return None,
        Err(e) => panic!("packet header: {}", e),
    }
    let len = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], 0]);
    let seq = hdr[3];
    let mut payload = vec![0u8; len as usize];
    match stream.read_exact(&mut payload) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return None,
        Err(e) => panic!("packet payload: {}", e),
    }
    Some((seq, payload))
}

/// Write one MySQL packet (3-byte length + 1-byte seq + payload).
fn write_packet(stream: &mut TcpStream, seq: u8, payload: &[u8]) {
    let len = payload.len() as u32;
    stream
        .write_all(&len.to_le_bytes()[..3])
        .expect("write len");
    stream.write_all(&[seq]).expect("write seq");
    stream.write_all(payload).expect("write payload");
}

/// Minimal raw handshake: read handshake v10, send handshake response
/// (we don't replicate the full CLIENT_LONG_PASSWORD / DEPRECATE_EOF
/// machinery — we just want a working session for the raw prepare).
///
/// Returns the full 32-bit negotiated capability flag set. We use this
/// to assert that the server advertised CLIENT_DEPRECATE_EOF (bit 24)
/// so the test actually exercises the DEPRECATE_EOF wire-format path.
///
/// Authentication uses `mysql_native_password` (the plugin the ephemeral
/// server selects by default) with the well-known `tester` / `tester`
/// credentials the harness creates.
fn raw_handshake(stream: &mut TcpStream) -> u32 {
    // Read handshake v10 (plaintext) and parse it via the client's parser
    // so we get the scramble + server capability flags without redoing
    // the layout by hand.
    let (_, hs_payload) = read_packet(stream).expect("handshake packet");
    let handshake = parse_handshake(&hs_payload).expect("parse_handshake");
    let server_caps = handshake.capability_flags;

    // Compose handshake response body. We advertise the union of the
    // server's flags and DEPRECATE_EOF=1 so we exercise the post-fix
    // wire format. Capabilities that affect handshake response shape
    // (CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA, CLIENT_PROTOCOL_41,
    // CLIENT_SECURE_CONNECTION, CLIENT_PLUGIN_AUTH) are added so the
    // server accepts the response layout.
    let client_caps = server_caps | DEPRECATE_EOF_FLAG | CAP_PLUGIN_AUTH | CAP_PLUGIN_AUTH_LENENC;
    let auth_response = raw_native_password_hash("tester", &handshake.auth_plugin_data);
    let auth_response_len = auth_response.len() as u8;

    let mut body = Vec::new();
    body.extend_from_slice(&client_caps.to_le_bytes()); // capability flags
    body.extend_from_slice(&0x00ff_ffffu32.to_le_bytes()); // max packet
    body.push(0x21); // charset utf8
    body.extend_from_slice(&[0u8; 23]); // filler
    body.extend_from_slice(b"tester\x00"); // username (null-terminated)
    body.push(auth_response_len); // auth-response length (1-byte form)
    body.extend_from_slice(&auth_response);
    body.extend_from_slice(b"mysql_native_password\x00"); // auth plugin
    write_packet(stream, 1, &body);

    // Read auth OK.
    let (_, auth_pkt) = read_packet(stream).expect("auth result packet");
    assert_ne!(auth_pkt[0], 0xff, "auth must not be ERR: {:?}", auth_pkt);
    server_caps
}

/// Wire-format invariant: under DEPRECATE_EOF, the response to
/// `SELECT name FROM t WHERE id = ?` (1 param, 1 col) must be EXACTLY
/// 3 packets: initial OK + 1 param_def + 1 column_def.
///
/// Pre-fix, this returned 5 packets (with a 0xFE terminator between
/// param_def and column_def, and another after column_def), causing
/// libmysqlclient to surface error 2027 "Malformed packet" when sysbench
/// tried to drive oltp_read_write.
#[test]
fn test_v400_stmt_prepare_no_terminator_under_deprecate_eof() {
    let handle = start_server();
    let port = handle.port;

    // Set up a tiny schema so the parser is happy.
    let mut conn = connect(port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, name VARCHAR(50))")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 'Alice')")
        .expect("insert");

    // Open a fresh raw TCP socket — we need full control over packet reads
    // so we can count them and inspect their first bytes.
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    let mut stream = TcpStream::connect(addr).expect("raw connect");
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();

    let server_caps = raw_handshake(&mut stream);
    // Sanity: the server must have advertised DEPRECATE_EOF. If this
    // fails the test isn't actually exercising the wire-format path the
    // regression covers — fail loudly rather than silently passing.
    assert_ne!(
        server_caps & DEPRECATE_EOF_FLAG,
        0,
        "server did not advertise DEPRECATE_EOF (cap_lower={:x} cap_upper={:x})",
        server_caps & 0xFFFF,
        (server_caps >> 16) & 0xFFFF
    );

    // Send COM_STMT_PREPARE for the failing sysbench query shape.
    let sql = b"SELECT name FROM t WHERE id = ?";
    let mut payload = vec![0x16]; // COM_STMT_PREPARE
    payload.extend_from_slice(sql);
    write_packet(&mut stream, 0, &payload);

    // Read the response. With DEPRECATE_EOF, expect exactly 3 packets:
    //   1. initial OK (status=0x00, stmt_id, col_count=1, param_count=1, ...)
    //   2. param_def (1 packet)
    //   3. column_def (1 packet)
    //
    // Pre-fix shape: 5 packets with 0xFE terminators at positions 2 and 4.
    let mut packets = Vec::new();
    loop {
        match read_packet(&mut stream) {
            Some((_seq, p)) => packets.push(p),
            None => panic!(
                "server closed/timeout before completing STMT_PREPARE response: got {} packet(s)",
                packets.len()
            ),
        }
        if packets.len() >= 3 {
            // Probe: switch to a short read timeout and try once more.
            // A correct DEPRECATE_EOF server sends nothing more here, so
            // the probe must time out (returning None from read_packet).
            stream
                .set_read_timeout(Some(std::time::Duration::from_millis(200)))
                .unwrap();
            let probe = read_packet(&mut stream);
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            assert!(
                probe.is_none(),
                "DEPRECATE_EOF should NOT send a 4th packet — server misbehaving: {:?}",
                probe.map(|(_, p)| p)
            );
            break;
        }
    }

    // INVARIANT 1: total packet count must be exactly 3 (1 OK + 1 param +
    // 1 col). Pre-fix this was 5.
    assert_eq!(
        packets.len(),
        3,
        "expected exactly 3 STMT_PREPARE response packets, got {}",
        packets.len()
    );

    // INVARIANT 2: packet 0 is the initial OK.
    assert_eq!(packets[0][0], 0x00, "first byte of OK must be 0x00");
    assert!(
        packets[0].len() >= 12,
        "initial OK must be at least 12 bytes (got {})",
        packets[0].len()
    );

    // INVARIANT 3: packet 1 is the param_def (starts with lenenc 'def').
    assert!(packets[1].len() > 4, "param_def too short");
    assert_eq!(&packets[1][1..4], b"def", "param_def catalog must be 'def'");

    // INVARIANT 4: packet 2 is the column_def.
    assert!(packets[2].len() > 4, "column_def too short");
    assert_eq!(
        &packets[2][1..4],
        b"def",
        "column_def catalog must be 'def'"
    );

    // INVARIANT 5: no packet (after the initial OK) may have a 0x00
    // classic-EOF marker or 0xFE DEPRECATE_EOF terminator as its first
    // byte. Pre-fix, both terminators slipped through here.
    for (i, p) in packets.iter().enumerate().skip(1).take(2) {
        let fb = p[0];
        assert_ne!(fb, 0x00, "packet {} starts with classic EOF 0x00", i);
        assert_ne!(
            fb, 0xfe,
            "packet {} starts with DEPRECATE_EOF terminator 0xFE — protocol invariant broken",
            i
        );
    }
}

/// Round-trip regression: with the post-fix client and server, a SELECT
/// with both params and columns must `prepare` + `execute_prepared`
/// successfully. Pre-fix this hung because the client read a terminator
/// packet that the server no longer sent.
#[test]
fn test_v400_stmt_prepare_round_trip_with_param_and_column() {
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, name VARCHAR(50))")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 'Alice')")
        .expect("insert");
    conn.execute("INSERT INTO t VALUES (2, 'Bob')")
        .expect("insert");

    let stmt = conn
        .prepare("SELECT name FROM t WHERE id = ?")
        .expect("prepare must succeed");
    assert_eq!(stmt.param_count, 1, "1 ? placeholder");
    assert_eq!(stmt.column_count, 1, "1 selected column");

    let rows = match conn.execute_prepared(stmt.id, &["2"]).expect("exec") {
        ResultSet::Select { rows, .. } => rows,
        other => panic!("expected Select, got {:?}", other),
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "Bob");

    conn.close_statement(stmt.id).expect("close");
}

/// Edge case: SELECT with NO parameter but WITH column — used to
/// confuse earlier "always drain separator" code. Under DEPRECATE_EOF
/// the server must send exactly 2 packets: initial OK + column_def.
#[test]
fn test_v400_stmt_prepare_no_param_with_column() {
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port);
    conn.execute("CREATE TABLE t (x INT)").expect("create");
    conn.execute("INSERT INTO t VALUES (42)").expect("insert");

    let stmt = conn.prepare("SELECT x FROM t").expect("prepare");
    assert_eq!(stmt.param_count, 0);
    assert_eq!(stmt.column_count, 1);

    let rows = match conn.execute_prepared(stmt.id, &[]).expect("exec") {
        ResultSet::Select { rows, .. } => rows,
        other => panic!("expected Select, got {:?}", other),
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "42");
}

/// Edge case: UPDATE with parameter but NO column — symmetric to the
/// SELECT-no-param test. Under DEPRECATE_EOF the server must send
/// exactly 2 packets: initial OK + param_def.
#[test]
fn test_v400_stmt_prepare_param_no_column() {
    let handle = start_server();
    let port = handle.port;
    let mut conn = connect(port);
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, x INT)")
        .expect("create");
    conn.execute("INSERT INTO t VALUES (1, 100)")
        .expect("insert");

    let stmt = conn
        .prepare("UPDATE t SET x = x + 1 WHERE id = ?")
        .expect("prepare");
    assert_eq!(stmt.param_count, 1);
    assert_eq!(stmt.column_count, 0);

    let res = conn.execute_prepared(stmt.id, &["1"]).expect("exec");
    match res {
        ResultSet::Ok { .. } => {}
        other => panic!("expected Ok, got {:?}", other),
    }

    // Verify the update actually happened.
    let rows = match conn
        .execute("SELECT x FROM t WHERE id = 1")
        .expect("verify")
    {
        ResultSet::Select { rows, .. } => rows,
        other => panic!("expected Select, got {:?}", other),
    };
    assert_eq!(rows[0][0], "101");
}

/// DEPRECATE_EOF capability flag bit. Lives in the upper 16 bits of the
/// 32-bit capability flag set (per MySQL 8.0 protocol).
const DEPRECATE_EOF_FLAG: u32 = 0x0100_0000;

/// CLIENT_PLUGIN_AUTH (0x00080000) — server expects the trailing
/// `mysql_native_password\x00` plugin-name string in the response.
const CAP_PLUGIN_AUTH: u32 = 0x0008_0000;

/// CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA (0x00200000) — server reads
/// auth-response as a 1-byte length prefix + bytes (the simple form
/// matches what we send).
const CAP_PLUGIN_AUTH_LENENC: u32 = 0x0020_0000;

//! Handshake version parsing tests (Issue #4176 / V312-38)
//!
//! Verifies that the CLI client correctly identifies non-sqlrustgo MySQL
//! servers by parsing the server version string from the handshake,
//! enabling better diagnostics when REPL connects to system MySQL
//! on 127.0.0.1:3306 instead of sqlrustgo-mysql-server.

use sqlrustgo_soak::client::{parse_handshake_version, ServerKind};

/// HandshakeV10 layout (MySQL protocol):
///   byte 0:           protocol version (0x0a = HandshakeV10)
///   bytes 1..N:       server_version, NUL-terminated
///   bytes N+1..N+5:   connection_id (u32 LE)
///   bytes N+5..N+13:  auth-plugin-data-part-1 (8 bytes)
///   byte N+13:        filler (0x00)
///   ... (capabilities, charset, status, capabilities-upper, auth-plugin-len, reserved, scramble-2)

fn build_handshake(version: &[u8]) -> Vec<u8> {
    let mut p = Vec::new();
    p.push(0x0a); // protocol
    p.extend_from_slice(version);
    p.push(0x00); // NUL terminator
    p.extend_from_slice(&1u32.to_le_bytes()); // connection_id
    p.extend_from_slice(&[0u8; 8]); // auth-plugin-data-part-1
    p.push(0x00); // filler
    p.extend_from_slice(&[0u8; 2]); // cap lower
    p.push(33); // charset (utf8)
    p.extend_from_slice(&[0u8; 2]); // status
    p.extend_from_slice(&[0u8; 2]); // cap upper
    p.push(0u8); // auth-plugin-data-len
    p.extend_from_slice(&[0u8; 10]); // reserved
    p.extend_from_slice(&[0u8; 12]); // auth-plugin-data-part-2 (min 12)
    p
}

#[test]
fn test_parse_handshake_sqlrustgo_version() {
    let h = build_handshake(b"8.0.33-sqlrustgo");
    let kind = parse_handshake_version(&h).expect("parse succeeds");
    match kind {
        ServerKind::SqlRustGo(v) => assert_eq!(v, "8.0.33-sqlrustgo"),
        other => panic!("expected SqlRustGo, got {:?}", other),
    }
}

#[test]
fn test_parse_handshake_system_mysql_version() {
    let h = build_handshake(b"8.0.34");
    let kind = parse_handshake_version(&h).expect("parse succeeds");
    match kind {
        ServerKind::Other(v) => assert_eq!(v, "8.0.34"),
        other => panic!("expected Other, got {:?}", other),
    }
}

#[test]
fn test_parse_handshake_mariadb_version() {
    let h = build_handshake(b"10.11.6-MariaDB");
    let kind = parse_handshake_version(&h).expect("parse succeeds");
    match kind {
        ServerKind::Other(v) => assert_eq!(v, "10.11.6-MariaDB"),
        other => panic!("expected Other, got {:?}", other),
    }
}

#[test]
fn test_parse_handshake_too_short() {
    let h = vec![0x0a, 0x00]; // protocol + NUL only
    assert!(parse_handshake_version(&h).is_err());
}

#[test]
fn test_parse_handshake_not_handshake_v10() {
    let mut h = build_handshake(b"8.0.33");
    h[0] = 0x09; // wrong protocol version
    assert!(parse_handshake_version(&h).is_err());
}

#[test]
fn test_parse_handshake_unterminated_version() {
    // No NUL terminator for version
    let h = vec![0x0a, b'8', b'.', b'0', b'.', b'3', b'3'];
    assert!(parse_handshake_version(&h).is_err());
}

#[test]
fn test_server_kind_is_sqlrustgo_predicate() {
    assert!(ServerKind::SqlRustGo("x".into()).is_sqlrustgo());
    assert!(!ServerKind::Other("8.0.34".into()).is_sqlrustgo());
    assert!(!ServerKind::Unknown.is_sqlrustgo());
}

#[test]
fn test_server_kind_diagnostic_message_for_system_mysql() {
    let kind = ServerKind::Other("8.0.34".into());
    let diag = kind.diagnostic_for_port(3306);
    assert!(diag.contains("non-sqlrustgo"));
    assert!(diag.contains("3306"));
    assert!(diag.contains("8.0.34"));
    // The diagnostic should mention how to fix it
    assert!(diag.contains("sqlrustgo-mysql-server") || diag.contains("--port"));
}

#[test]
fn test_server_kind_diagnostic_empty_for_sqlrustgo() {
    // sqlrustgo server on the expected port should not warn.
    let kind = ServerKind::SqlRustGo("8.0.33-sqlrustgo".into());
    let diag = kind.diagnostic_for_port(3306);
    assert!(diag.is_empty());
}

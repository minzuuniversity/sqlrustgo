//! Embedded server test harness — smoke test
//!
//! Verifies the canonical entry-point contract from
//! `openspec/changes/mysql-server-canonical-entry/specs/server-embedded-test-harness/spec.md`.

use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Read a MySQL packet (4-byte little-endian length+seq header + payload).
/// The length is stored in the lower 3 bytes; the high byte is the
/// sequence number.
fn read_packet(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).expect("read header");
    let len = (u32::from_le_bytes(header) & 0x00FFFFFF) as usize;
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).expect("read payload");
    payload
}

#[test]
fn test_ephemeral_handshake_responds_with_server_version() {
    let handle = start_ephemeral(EphemeralConfig::default())
        .expect("start_ephemeral must succeed within timeout");

    let mut stream = TcpStream::connect(("127.0.0.1", handle.port)).expect("connect");
    stream
        .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
        .expect("set_read_timeout");
    stream
        .set_write_timeout(Some(HANDSHAKE_TIMEOUT))
        .expect("set_write_timeout");

    // Server sends a HandshakeV10 packet first (length is in the first 3 bytes
    // of the 4-byte little-endian length header).
    let handshake = read_packet(&mut stream);
    assert!(!handshake.is_empty(), "server must send a handshake packet");
    // HandshakeV10 protocol is 0x0a.
    assert_eq!(
        handshake[0], 0x0a,
        "first byte of handshake payload must be 0x0a (HandshakeV10)"
    );
    // Server version is null-terminated, starts at offset 1.
    let version_end = handshake[1..]
        .iter()
        .position(|&b| b == 0)
        .expect("server version must be null-terminated");
    let version = std::str::from_utf8(&handshake[1..=version_end])
        .expect("server version must be valid utf-8");
    assert!(
        version.contains("SQLRustGo"),
        "server version must contain 'SQLRustGo', got: {version}"
    );
}

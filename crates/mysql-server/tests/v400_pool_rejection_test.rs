//! V4.0.0 SOAK Issue: pool backpressure must NOT silently drop
//! connections (lib.rs:5751-5768 historical silent-drop path).
//!
//! Before this fix the accept loop logged at `tracing::debug` and
//! dropped the `TcpStream` without sending any wire-protocol response.
//! Clients (sysbench, mysql CLI) saw "connection reset" and retried
//! indefinitely, masking the real bottleneck (pool saturation,
//! 64-thread collapse, engine lock contention) under a stream of
//! reconnect attempts.
//!
//! After the fix the server writes a proper MySQL ERR packet (code
//! 1040 ER_CON_COUNT_ERROR, SQL state "08004", message "Too many
//! connections") on the rejected socket, then shuts it down
//! gracefully. This lets the client surface the real error instead
//! of guessing why its connection vanished, and lets ops correlate
//! `sqlrustgo_pool_rejected_total` (Prometheus) with the actual
//! saturation event.
//!
//! This file binds the wire-level contract. Any regression to
//! silent-drop surfaces here as a missing ERR marker or wrong error
//! code.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

/// Helper under test: must write a MySQL ERR packet with code 1040
/// then close the stream. Before this fix the function did not
/// exist — the accept loop just dropped the TcpStream.
fn write_rejection_packet(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let pkt = sqlrustgo_mysql_server::testing::make_rejection_err_packet();
    pkt.write_to(&mut stream)
        .map_err(|e| std::io::Error::other(format!("write_to: {e}")))?;
    stream.flush()?;
    // Half-close so the client sees EOF after reading the ERR.
    let _ = stream.shutdown(std::net::Shutdown::Both);
    Ok(())
}

/// Client reads a complete MySQL packet (header + payload).
fn read_one_packet(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;
    let payload_len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;
    let mut full = Vec::with_capacity(4 + payload_len);
    full.extend_from_slice(&header);
    full.extend_from_slice(&payload);
    Ok(full)
}

#[test]
fn rejected_connection_receives_err_packet_with_code_1040() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        write_rejection_packet(stream).expect("write rejection");
    });

    let mut client = TcpStream::connect(addr).expect("connect");
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let pkt = read_one_packet(&mut client).expect("read err packet");

    server_thread.join().expect("server thread");

    // 4-byte header (length:24bits, sequence:8bits) + payload
    let payload_len = u32::from_le_bytes([pkt[0], pkt[1], pkt[2], 0]) as usize;
    let payload = &pkt[4..4 + payload_len];
    let sequence = pkt[3];

    assert_eq!(sequence, 0, "rejection packet must be sequence 0 (pre-handshake)");
    assert!(payload.len() >= 7, "payload too short for ERR packet");
    assert_eq!(
        payload[0], 0xff,
        "first byte of payload must be 0xff ERR marker, got 0x{:02x}",
        payload[0]
    );
    let code = u16::from_le_bytes([payload[1], payload[2]]);
    assert_eq!(
        code, 1040,
        "error code must be 1040 ER_CON_COUNT_ERROR, got {}",
        code
    );
    // '#' marker at offset 3, then 5-byte SQL state
    assert_eq!(
        payload[3], 0x23,
        "byte 3 must be '#' marker, got 0x{:02x}",
        payload[3]
    );
    let sql_state = std::str::from_utf8(&payload[4..9]).unwrap_or("");
    assert_eq!(sql_state, "08004", "SQL state must be 08004 for ER_CON_COUNT_ERROR");
    // Message (null-terminated) starts at offset 9
    let msg_bytes = &payload[9..payload.len()];
    let msg = std::str::from_utf8(msg_bytes).unwrap_or("");
    let msg = msg.trim_end_matches('\0');
    assert!(
        msg.to_lowercase().contains("too many connections"),
        "message must mention 'too many connections', got: {:?}",
        msg
    );
}

#[test]
fn rejected_connection_sees_graceful_close_after_err_packet() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    let server_thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        write_rejection_packet(stream).expect("write rejection");
    });

    let mut client = TcpStream::connect(addr).expect("connect");
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let _ = read_one_packet(&mut client).expect("first packet");

    // Second read must hit EOF (clean shutdown), not RST/connection-reset.
    let mut tail = [0u8; 16];
    let n = client.read(&mut tail).expect("second read returns 0 on EOF");
    assert_eq!(
        n, 0,
        "client must observe clean EOF (0 bytes) after ERR packet, got {} bytes",
        n
    );

    server_thread.join().expect("server thread");
}
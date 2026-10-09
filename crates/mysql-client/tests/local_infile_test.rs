//! LOCAL INFILE (0xFB) round-trip tests.
//!
//! Before the fix, a 0xFB first byte fell through to
//! `parse_length_encoded_int`, which maps 0xFB to the NULL escape
//! (`u64::MAX`). The next statement was `Vec::with_capacity(column_count)`,
//! so any `LOAD DATA LOCAL INFILE` aborted the client with
//! "capacity overflow" instead of answering the request.
//!
//! These tests drive the parser over a scripted duplex stream: reads come
//! from a fixed script, writes are captured so the test can assert on the
//! exact packets the client framed back to the server.

use sqlrustgo_mysql_client::{
    parse_result_set, parse_result_set_with_infile, MySqlClientError, Packet, ResultSet,
};
use std::io::{self, Cursor, Read, Write};
use std::sync::{Arc, Mutex};

/// Read from a fixed script, capture everything written back.
struct DuplexStream {
    input: Cursor<Vec<u8>>,
    written: Arc<Mutex<Vec<u8>>>,
}

impl DuplexStream {
    fn new(script: Vec<u8>) -> Self {
        Self {
            input: Cursor::new(script),
            written: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn written(&self) -> Vec<u8> {
        self.written.lock().expect("written lock").clone()
    }
}

impl Read for DuplexStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.input.read(buf)
    }
}

impl Write for DuplexStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written
            .lock()
            .expect("written lock")
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Frame `payload` as one wire packet (3-byte length + 1-byte sequence).
fn frame(seq: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut out = Vec::with_capacity(4 + len);
    out.push((len & 0xff) as u8);
    out.push(((len >> 8) & 0xff) as u8);
    out.push(((len >> 16) & 0xff) as u8);
    out.push(seq);
    out.extend_from_slice(payload);
    out
}

/// Frame the server's 0xFB request.
fn infile_request(seq: u8, path: &str) -> Vec<u8> {
    let mut payload = vec![0xFB];
    payload.extend_from_slice(path.as_bytes());
    frame(seq, &payload)
}

/// Frame an OK packet (DEPRECATE_EOF header 0x00).
fn ok_packet(seq: u8, affected_rows: u64, warnings: u16, info: &str) -> Vec<u8> {
    let mut payload = vec![0x00, affected_rows as u8, 0x00];
    payload.extend_from_slice(&0u16.to_le_bytes()); // status_flags
    payload.extend_from_slice(&warnings.to_le_bytes());
    payload.extend_from_slice(info.as_bytes());
    frame(seq, &payload)
}

/// Frame an ERR packet (protocol 4.1 with '#' marker).
fn err_packet(seq: u8, code: u16, sql_state: &str, message: &str) -> Vec<u8> {
    let mut payload = vec![0xFF];
    payload.extend_from_slice(&code.to_le_bytes());
    payload.push(0x23);
    payload.extend_from_slice(sql_state.as_bytes());
    payload.extend_from_slice(message.as_bytes());
    frame(seq, &payload)
}

/// Frame a minimal but well-formed column definition packet.
fn column_def_packet(seq: u8) -> Vec<u8> {
    let mut p = Vec::new();
    for s in ["def", "", "t", "t", "c", "c"] {
        p.push(s.len() as u8);
        p.extend_from_slice(s.as_bytes());
    }
    p.push(0x0c); // length of the fixed-length block that follows
    p.extend_from_slice(&63u16.to_le_bytes()); // character set
    p.extend_from_slice(&255u32.to_le_bytes()); // column length
    p.push(0xfd); // MYSQL_TYPE_VAR_STRING
    p.extend_from_slice(&0u16.to_le_bytes()); // flags
    p.push(0); // decimals
    p.extend_from_slice(&[0, 0]); // filler
    frame(seq, &p)
}

/// Frame the DEPRECATE_EOF result-set terminator (0xFE header, <= 8 bytes).
fn eof_terminator(seq: u8, status_flags: u16) -> Vec<u8> {
    let mut p = vec![0xFE, 0x00, 0x00];
    p.extend_from_slice(&status_flags.to_le_bytes());
    p.extend_from_slice(&0u16.to_le_bytes()); // warnings
    frame(seq, &p)
}

/// Split captured bytes back into `(sequence, payload)` pairs.
fn unframe_all(wire: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 4 <= wire.len() {
        let len =
            wire[pos] as usize | ((wire[pos + 1] as usize) << 8) | ((wire[pos + 2] as usize) << 16);
        let seq = wire[pos + 3];
        pos += 4;
        assert!(pos + len <= wire.len(), "truncated captured packet");
        out.push((seq, wire[pos..pos + len].to_vec()));
        pos += len;
    }
    assert_eq!(pos, wire.len(), "trailing bytes in captured wire");
    out
}

const PATH: &str = "/private/tmp/sqlrustgo_local_infile_test/orders.tbl";
const BODY: &[u8] = b"1|alpha\n2|beta\n3|gamma\n";

/// Happy path: 0xFB request → content packet → empty terminator → OK.
#[test]
fn local_infile_round_trip_returns_server_ok() {
    let mut script = infile_request(1, PATH);
    script.extend_from_slice(&ok_packet(4, 3, 0, "Records: 3  Deleted: 0"));
    let mut stream = DuplexStream::new(script);

    let mut handler = |p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> {
        assert_eq!(p, PATH, "handler must receive the server's exact path");
        Ok(BODY.to_vec())
    };
    let rs = parse_result_set_with_infile(&mut stream, true, &mut handler, None, false)
        .expect("round trip must succeed");

    match rs {
        ResultSet::Ok {
            affected_rows,
            warnings,
            info,
            ..
        } => {
            assert_eq!(affected_rows, 3, "server-reported loaded rows");
            assert_eq!(warnings, 0);
            assert_eq!(info, "Records: 3  Deleted: 0");
        }
        other => panic!("expected Ok, got {:?}", other),
    }

    // The client must frame the file as content packets terminated by an
    // empty packet, with sequence numbers continuing from the request.
    let sent = unframe_all(&stream.written());
    assert_eq!(
        sent,
        vec![(2, BODY.to_vec()), (3, Vec::new())],
        "expected one content packet at seq 2 and an empty terminator at seq 3"
    );
}

/// The exact original defect: 0xFB through the read-only entry point used
/// to die with "capacity overflow". It must now be a clean protocol error.
#[test]
fn read_only_parser_reports_protocol_error_not_capacity_overflow() {
    let mut stream = DuplexStream::new(infile_request(1, PATH));

    let err = parse_result_set(&mut stream, true).expect_err("must not succeed");
    let msg = err.to_string();
    assert!(
        msg.contains("LOCAL INFILE"),
        "error must name the missing LOCAL INFILE handling, got: {}",
        msg
    );
    assert!(
        !msg.contains("capacity overflow"),
        "must not regress to the capacity-overflow panic: {}",
        msg
    );
    assert!(matches!(err, MySqlClientError::Protocol(_)));
}

/// A handler that cannot produce data aborts the round trip, and the
/// connection must be left with nothing half-written.
#[test]
fn handler_error_aborts_without_writing_anything() {
    let mut stream = DuplexStream::new(infile_request(1, PATH));

    let mut handler = |_p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> {
        Err(MySqlClientError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "no such file",
        )))
    };
    let err = parse_result_set_with_infile(&mut stream, true, &mut handler, None, false)
        .expect_err("handler failure must propagate");
    assert!(matches!(err, MySqlClientError::Io(_)));
    assert!(
        stream.written().is_empty(),
        "nothing may be written when the handler fails"
    );
}

/// Empty content is still an answer: exactly one empty packet, no data
/// packets. That is the protocol's "I have no data" encoding.
#[test]
fn empty_contents_send_only_the_terminator() {
    let mut script = infile_request(1, PATH);
    script.extend_from_slice(&ok_packet(3, 0, 0, "Records: 0"));
    let mut stream = DuplexStream::new(script);

    let mut handler = |_p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> { Ok(Vec::new()) };
    let rs =
        parse_result_set_with_infile(&mut stream, true, &mut handler, None, false).expect("ok");

    assert!(matches!(
        rs,
        ResultSet::Ok {
            affected_rows: 0,
            ..
        }
    ));
    assert_eq!(
        unframe_all(&stream.written()),
        vec![(2, Vec::new())],
        "an empty upload is one empty packet, not zero packets"
    );
}

/// Payloads larger than one packet must be split at MAX_PACKET_SIZE.
#[test]
fn oversized_contents_are_split_into_multiple_packets() {
    const MAX: usize = 0x00ff_ffff;
    // Two full packets plus a remainder, so the last chunk is partial.
    let big = vec![b'x'; MAX * 2 + 5];
    assert!(
        big.len() < 64 * 1024 * 1024,
        "keep the test's memory footprint bounded"
    );

    let mut script = infile_request(1, PATH);
    // seq 2 = first chunk, 3 = second, 4 = remainder, 5 = terminator,
    // server answers at 6.
    script.extend_from_slice(&ok_packet(6, 1, 0, "Records: 1"));
    let mut stream = DuplexStream::new(script);

    let payload = big.clone();
    let mut handler =
        move |_p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> { Ok(payload.clone()) };
    parse_result_set_with_infile(&mut stream, true, &mut handler, None, false).expect("ok");

    let sent = unframe_all(&stream.written());
    assert_eq!(sent.len(), 4, "2 full + 1 remainder + 1 terminator");
    assert_eq!(sent[0].0, 2);
    assert_eq!(sent[0].1.len(), MAX);
    assert_eq!(sent[1].0, 3);
    assert_eq!(sent[1].1.len(), MAX);
    assert_eq!(sent[2].0, 4);
    assert_eq!(sent[2].1.len(), 5, "remainder must be the last 5 bytes");
    assert_eq!(sent[3], (5, Vec::new()), "terminator must be empty");
    let mut rejoined = Vec::new();
    for (_, p) in &sent[..3] {
        rejoined.extend_from_slice(p);
    }
    assert_eq!(rejoined, big, "reassembled payload must equal the input");
}

/// A server-side rejection (e.g. the file is outside `load_infile_dir`)
/// must surface as `ResultSet::Error`, not a silent Ok.
#[test]
fn server_err_after_upload_is_surfaced_as_error() {
    let mut script = infile_request(1, "/etc/passwd");
    script.extend_from_slice(&err_packet(
        4,
        1146,
        "42S02",
        "file not in allowed data_dir",
    ));
    let mut stream = DuplexStream::new(script);

    let mut handler =
        |_p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> { Ok(b"root:x:0:0".to_vec()) };
    let rs =
        parse_result_set_with_infile(&mut stream, true, &mut handler, None, false).expect("parsed");

    match rs {
        ResultSet::Error {
            error_code,
            sql_state,
            error_message,
        } => {
            assert_eq!(error_code, 1146);
            assert_eq!(sql_state, "42S02");
            assert!(
                error_message.contains("not in allowed data_dir"),
                "{}",
                error_message
            );
        }
        other => panic!("expected Error, got {:?}", other),
    }
    // The client still uploaded what it was asked to; the server is the
    // one that refuses it. That is the intended trust boundary.
    assert_eq!(unframe_all(&stream.written()).len(), 2);
}

/// The path may arrive NUL-terminated (spec) or bare (real clients).
#[test]
fn nul_terminated_path_is_trimmed() {
    let mut nul_path = vec![0xFB];
    nul_path.extend_from_slice(PATH.as_bytes());
    nul_path.push(0);
    let mut script = frame(1, &nul_path);
    script.extend_from_slice(&ok_packet(4, 1, 0, ""));
    let mut stream = DuplexStream::new(script);

    let mut seen = String::new();
    let mut handler = |p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> {
        seen = p.to_string();
        Ok(BODY.to_vec())
    };
    parse_result_set_with_infile(&mut stream, true, &mut handler, None, false).expect("ok");
    assert_eq!(seen, PATH, "trailing NUL must not reach the handler");
}

/// `next_seq` receives the sequence the next client packet must carry, so
/// the connection stays in step after the extra round trip.
#[test]
fn next_seq_is_reported_after_the_upload() {
    let mut script = infile_request(7, PATH);
    script.extend_from_slice(&ok_packet(10, 5, 0, ""));
    let mut stream = DuplexStream::new(script);

    let mut handler =
        |_p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> { Ok(BODY.to_vec()) };
    let mut next_seq = 0u8;
    parse_result_set_with_infile(&mut stream, true, &mut handler, Some(&mut next_seq), false)
        .expect("ok");

    assert_eq!(
        next_seq, 11,
        "next packet follows the server's OK at seq 10"
    );
    // Request at 7 → content at 8 → terminator at 9 → server OK at 10.
    let sent = unframe_all(&stream.written());
    assert_eq!(sent[0].0, 8);
    assert_eq!(sent[1].0, 9);
}

/// `next_seq` must not be touched for ordinary (non-INFILE) responses, so
/// wiring the out-param in cannot perturb existing query sequencing.
#[test]
fn next_seq_untouched_for_ordinary_result_set() {
    // 1 column, 1 row, DEPRECATE_EOF terminator.
    let mut script = frame(1, &[0x01]); // column_count = 1
    script.extend_from_slice(&column_def_packet(2));
    script.extend_from_slice(&frame(3, b"\x011\x016")); // text row: lenenc "1", lenenc "6"
    script.extend_from_slice(&eof_terminator(4, 0));
    let mut stream = DuplexStream::new(script);

    let mut handler =
        |_p: &str| -> sqlrustgo_mysql_client::MySqlResult<Vec<u8>> { panic!("must not fire") };
    let mut next_seq = 0xAB;
    let rs =
        parse_result_set_with_infile(&mut stream, true, &mut handler, Some(&mut next_seq), false)
            .expect("parse select");

    match rs {
        ResultSet::Select { rows, columns, .. } => {
            assert_eq!(columns.len(), 1);
            // One declared column, so only the first value is consumed.
            assert_eq!(rows, vec![vec!["1".to_string()]]);
        }
        other => panic!("expected Select, got {:?}", other),
    }
    assert_eq!(
        next_seq, 0xAB,
        "out-param must only move on the INFILE path"
    );
    assert!(stream.written().is_empty());
}

/// Regression guard on the framing helper itself: a 0xFB payload is a
/// 1-byte marker plus a path, and must never be mistaken for a lenenc int.
#[test]
fn packet_round_trips_through_library_framing() {
    let wire = frame(9, &[0xFB, b'a', b'b']);
    let pkt = Packet::read_from(&mut Cursor::new(wire)).expect("read");
    assert_eq!(pkt.sequence, 9);
    assert_eq!(pkt.payload[0], 0xFB);
}

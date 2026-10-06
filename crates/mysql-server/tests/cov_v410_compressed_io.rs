use std::io::{Cursor, Read, Write};

use sqlrustgo_mysql_server::{
    read_compressed_packet, write_compressed_packet, CompressedReader, CompressedWriter,
};

fn zlib_compress(payload: &[u8]) -> Vec<u8> {
    let mut c = flate2::Compress::new(flate2::Compression::default(), true);
    let mut out = Vec::new();
    c.compress_vec(payload, &mut out, flate2::FlushCompress::Finish)
        .expect("compress");
    out
}

fn craft_frame(uncompressed_len: u32, seq: u8, body: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&uncompressed_len.to_le_bytes()[..3]);
    f.push(seq);
    f.extend_from_slice(&(body.len() as u32).to_le_bytes()[..3]);
    f.extend_from_slice(body);
    f
}

#[test]
fn write_then_read_compressed_packet_roundtrip() {
    let mut wire = Vec::new();
    let payload = vec![b'x'; 4096];
    write_compressed_packet(&mut wire, 3, &payload).expect("write");
    let mut cur = Cursor::new(wire);
    let (seq, got) = read_compressed_packet(&mut cur).expect("read");
    assert_eq!(seq, 3);
    assert_eq!(got, payload);
}

#[test]
fn read_compressed_packet_handles_uncompressed_shortcut_frame() {
    // uncompressed_len = 0 → frame is stored as-is (no zlib step)
    let frame = craft_frame(0, 7, b"raw-payload");
    let mut cur = Cursor::new(frame);
    let (seq, got) = read_compressed_packet(&mut cur).expect("read");
    assert_eq!(seq, 7);
    assert_eq!(got, b"raw-payload".to_vec());
}

#[test]
fn read_compressed_packet_rejects_decompressed_length_mismatch() {
    let body = zlib_compress(b"hello");
    let frame = craft_frame(6, 0, &body); // valid zlib, but claims 6 bytes (actual 5)
    let mut cur = Cursor::new(frame);
    assert!(read_compressed_packet(&mut cur).is_err());
}

#[test]
fn read_compressed_packet_reports_io_error_on_truncated_stream() {
    let mut empty = Cursor::new(Vec::<u8>::new());
    assert!(read_compressed_packet(&mut empty).is_err());
    let only_header = craft_frame(4, 0, &[]); // header promises a body that is absent
    let mut truncated = Cursor::new(&only_header[..6]);
    assert!(read_compressed_packet(&mut truncated).is_err());
}

#[test]
fn compressed_writer_reader_roundtrip_with_leftover_drain() {
    let mut wire = Vec::new();
    {
        let mut w = CompressedWriter::new(&mut wire, true);
        w.write_packet(0, b"first-payload").expect("packet 0");
        w.write_packet(1, b"second-payload").expect("packet 1");
    }
    let mut cur = Cursor::new(wire);
    let mut r = CompressedReader::new(&mut cur, true);
    let (s0, p0) = r.read_packet().expect("read 0");
    assert_eq!(s0, 0);
    assert_eq!(p0, b"first-payload".to_vec());
    // observed contract: the buffered frame is served again from the
    // leftover branch before the next wire frame is consumed; that branch
    // derives `seq` from the payload's first byte, not the frame header
    let (s0b, p0b) = r.read_packet().expect("read leftover");
    assert_eq!(p0b, b"first-payload".to_vec());
    assert_eq!(s0b, b'f' as u8, "leftover branch reports payload[0] as seq");
    let (s1, p1) = r.read_packet().expect("read 1");
    assert_eq!(s1, 1);
    assert_eq!(p1, b"second-payload".to_vec());
}

#[test]
fn plain_mode_writer_reader_roundtrip() {
    let mut wire = Vec::new();
    {
        let mut w = CompressedWriter::new(&mut wire, false);
        w.write_packet(5, b"plain-payload").expect("plain packet");
    }
    let mut cur = Cursor::new(wire);
    let mut r = CompressedReader::new(&mut cur, false);
    let (seq, payload) = r.read_packet().expect("plain read");
    assert_eq!(seq, 5);
    assert_eq!(payload, b"plain-payload".to_vec());
}

#[test]
fn write_impl_delegates_to_inner_and_flushes() {
    let mut sink = Vec::new();
    {
        let mut w = CompressedWriter::new(&mut sink, true);
        w.write_all(b"direct-bytes").expect("Write::write");
        w.flush().expect("flush");
    }
    assert_eq!(sink, b"direct-bytes".to_vec());
}

#[test]
fn read_impl_maps_empty_buf_io_and_protocol_errors() {
    let mut wire = Vec::new();
    write_compressed_packet(&mut wire, 0, b"payload").expect("write");

    let mut cur1 = Cursor::new(&wire);
    let mut r1 = CompressedReader::new(&mut cur1, true);
    let mut tiny = [];
    assert_eq!(r1.read(&mut tiny).expect("empty read"), 0);

    let mut cur2 = Cursor::new(&wire);
    let mut r2 = CompressedReader::new(&mut cur2, true);
    let mut buf = [0u8; 4];
    let n = r2.read(&mut buf).expect("read");
    assert!(n > 0);

    // truncated wire → Io error surfaces as io::Error
    let mut cur3 = Cursor::new(Vec::<u8>::new());
    let mut r3 = CompressedReader::new(&mut cur3, true);
    let mut buf = [0u8; 4];
    assert!(r3.read(&mut buf).is_err());

    // zlib failure is a Protocol error → mapped to io::Error::other
    let mut bad = Vec::new();
    bad.extend_from_slice(&10u32.to_le_bytes()[..3]);
    bad.push(0);
    bad.extend_from_slice(&5u32.to_le_bytes()[..3]);
    bad.extend_from_slice(&[1, 2, 3, 4, 5]);
    let mut cur4 = Cursor::new(bad);
    let mut r4 = CompressedReader::new(&mut cur4, true);
    let mut buf = [0u8; 4];
    let err = r4.read(&mut buf).expect_err("zlib garbage must fail");
    assert!(
        !matches!(err.raw_os_error(), Some(_)),
        "must be an io-mapped error"
    );
}

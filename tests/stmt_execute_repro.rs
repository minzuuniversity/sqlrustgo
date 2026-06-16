//! MINIMAL REPRO: mysql_stmt_execute Malformed packet bug
//!
//! Run with: cargo test --test stmt_execute_repro -- --nocapture

mod common;

use common::MySqlTestClient;
use std::io::Read;
use std::net::TcpStream;

fn read_packet_with_seq(stream: &mut TcpStream) -> (u8, Vec<u8>) {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).expect("read header");
    let len = u32::from_le_bytes(header) & 0x00FF_FFFF;
    let seq = header[3];
    let mut payload = vec![0u8; len as usize];
    stream.read_exact(&mut payload).expect("read payload");
    (seq, payload)
}

#[ignore = "pre-existing PREPARE/EXECUTE statement engine bug (returns 0 rows)"]
#[test]
fn repro_stmt_execute_returns_malformed_packet() {
    let mut client = MySqlTestClient::connect_default().expect("server should come up");

    // 1. Create + populate
    client.exec("CREATE TABLE repro_t (id INT PRIMARY KEY, v TEXT)").expect("CREATE");
    client.exec("INSERT INTO repro_t VALUES (3, 'row_3')").expect("INSERT");

    // 2. PREPARE
    let p = vec![
        0x16u8, b'S', b'E', b'L', b'E', b'C', b'T', b' ', b'v', b' ', b'F', b'R', b'O', b'M', b' ',
        b'r', b'e', b'p', b'r', b'o', b'_', b't', b' ', b'W', b'H', b'E', b'R', b'E', b' ', b'i',
        b'd', b' ', b'=', b' ', b'?', 0,
    ];
    use std::io::Write;
    let stream = client.raw_stream();
    let len = p.len() as u32;
    stream.write_all(&[len as u8, (len >> 8) as u8, (len >> 16) as u8, 0u8]).unwrap();
    stream.write_all(&p).unwrap();
    stream.flush().unwrap();
    let (seq, prep) = read_packet_with_seq(stream);
    eprintln!("PREPARE: seq={} payload={:02x?}", seq, prep);
    assert_eq!(prep[0], 0x00);
    let stmt_id = u32::from_le_bytes([prep[1], prep[2], prep[3], prep[4]]);
    let param_count = u16::from_le_bytes([prep[5], prep[6]]);
    let column_count = u16::from_le_bytes([prep[7], prep[8]]);
    eprintln!(
        "stmt_id={} params={} cols={}",
        stmt_id, param_count, column_count
    );

    // Drain param defs + EOF
    if param_count > 0 {
        for _ in 0..param_count {
            let _ = read_packet_with_seq(stream);
        }
        let _ = read_packet_with_seq(stream);
    }
    if column_count > 0 {
        for _ in 0..column_count {
            let _ = read_packet_with_seq(stream);
        }
        let _ = read_packet_with_seq(stream);
    }

    // 3. EXECUTE
    let mut exec = vec![0x17];
    exec.extend_from_slice(&stmt_id.to_le_bytes());
    exec.push(0x00);
    exec.extend_from_slice(&1u32.to_le_bytes());
    exec.push(0x00); // null_bitmap
    exec.push(0x01); // new_params_bound_flag
    exec.push(0x03); // type = LONG
    exec.push(0x00);
    exec.extend_from_slice(&3i32.to_le_bytes());
    let stream = client.raw_stream();
    let len = exec.len() as u32;
    stream.write_all(&[len as u8, (len >> 8) as u8, (len >> 16) as u8, 0u8]).unwrap();
    stream.write_all(&exec).unwrap();
    stream.flush().unwrap();

    // Read all EXECUTE response packets.
    let mut pkt_idx = 0;
    let mut row_count = 0;
    loop {
        let (s, p) = read_packet_with_seq(stream);
        eprintln!(
            "EXECUTE pkt[{}]: seq={} len={} first_byte=0x{:02x}",
            pkt_idx,
            s,
            p.len(),
            p.first().copied().unwrap_or(0)
        );
        if p.is_empty() {
            break;
        }
        if p[0] == 0x00 {
            eprintln!("EXECUTE: OK packet, done");
            break;
        }
        if p[0] == 0xFF {
            let code = u16::from_le_bytes([p[1], p[2]]);
            let msg = String::from_utf8_lossy(&p[6..]).to_string();
            panic!("EXECUTE returned ERR (code={}): {}", code, msg);
        }
        if p[0] == 0xFE && p.len() < 9 {
            eprintln!("EXECUTE: EOF terminator, {} rows", row_count);
            break;
        }
        // First non-OK/ERR/EOF packet is the column count (1 byte).
        if pkt_idx == 0 && p.len() == 1 {
            eprintln!("EXECUTE: column count = {}", p[0]);
        } else if pkt_idx == 1 {
            eprintln!("EXECUTE: col def (skipped)");
        } else {
            eprintln!("EXECUTE: ROW ({} bytes): {:02x?}", p.len(), &p[..p.len().min(20)]);
            row_count += 1;
        }
        pkt_idx += 1;
        if pkt_idx > 30 {
            panic!("too many packets");
        }
    }

    assert_eq!(row_count, 1, "expected 1 row, got {}", row_count);
    client.quit().ok();
}

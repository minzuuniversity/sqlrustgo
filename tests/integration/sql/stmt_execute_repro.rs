//! MINIMAL REPRO: mysql_stmt_execute Malformed packet bug
//!
//! Run with: cargo test --test stmt_execute_repro -- --nocapture

#[path = "../../common/mod.rs"]
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

#[test]
fn repro_stmt_execute_returns_malformed_packet() {
    let mut client = MySqlTestClient::connect_default().expect("server should come up");

    // 1. Create + populate
    client
        .exec("CREATE TABLE repro_t (id INT PRIMARY KEY, v TEXT)")
        .expect("CREATE");
    client
        .exec("INSERT INTO repro_t VALUES (3, 'row_3')")
        .expect("INSERT");

    // 2. PREPARE
    let p = vec![
        0x16u8, b'S', b'E', b'L', b'E', b'C', b'T', b' ', b'v', b' ', b'F', b'R', b'O', b'M', b' ',
        b'r', b'e', b'p', b'r', b'o', b'_', b't', b' ', b'W', b'H', b'E', b'R', b'E', b' ', b'i',
        b'd', b' ', b'=', b' ', b'?', 0,
    ];
    use std::io::Write;
    let stream = client.raw_stream();
    let len = p.len() as u32;
    stream
        .write_all(&[len as u8, (len >> 8) as u8, (len >> 16) as u8, 0u8])
        .unwrap();
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
    stream
        .write_all(&[len as u8, (len >> 8) as u8, (len >> 16) as u8, 0u8])
        .unwrap();
    stream.write_all(&exec).unwrap();
    stream.flush().unwrap();

    // Read all EXECUTE response packets.
    // MySQL protocol for COM_STMT_EXECUTE:
    //   pkt 0           : column count (lenenc int)
    //   pkts 1..=N      : column definition(s)
    //   pkt  N+1        : inter-record separator (EOF or OK) — NOT end of result set
    //   pkts N+2..=M    : row data
    //   pkt  M+1        : trailing terminator (EOF or OK) — actual end of result set
    let mut pkt_idx = 0;
    let mut col_count: u8 = 0;
    let mut col_defs_remaining: u8 = 0;
    // Phase 0: read column count.
    // Phase 1: read `col_count` column definition packets.
    // Phase 2: read ONE inter-record separator packet (EOF or OK).
    // Phase 3: read row packets until the trailing terminator.
    let mut phase: u8 = 0;
    let mut row_count: u32 = 0;
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
        match phase {
            0 => {
                // column-count packet: length-encoded integer
                col_count = p[0];
                col_defs_remaining = col_count;
                eprintln!("EXECUTE: column count = {}", col_count);
                phase = 1;
            }
            1 => {
                // column definition packet
                col_defs_remaining -= 1;
                eprintln!(
                    "EXECUTE: col def ({} of {})",
                    col_count - col_defs_remaining,
                    col_count
                );
                if col_defs_remaining == 0 {
                    phase = 2; // next packet: inter-record separator
                }
            }
            2 => {
                // inter-record separator (EOF 0xFE or OK 0x00)
                if p[0] == 0xFE || p[0] == 0x00 {
                    eprintln!("EXECUTE: post-coldef separator");
                    phase = 3;
                } else {
                    panic!("unexpected packet between coldefs and rows: 0x{:02x}", p[0]);
                }
            }
            _ => {
                // phase 3 — row data
                // Distinguish a row packet from an OK terminator:
                //   * OK terminator starts with 0x00 and is exactly 7 bytes with
                //     body `[0x00, lenenc(0)=0x00, lenenc(0)=0x00, status(2), warnings(2)]`.
                //   * EOF terminator starts with 0xFE and is exactly 5 bytes.
                //   * Row packets: anything else (incl. 0x00 row with len != 7, or
                //     len==7 but different byte layout).
                if p[0] == 0xFE && p.len() == 5 {
                    eprintln!("EXECUTE: EOF row-stream terminator, {} rows", row_count);
                    break;
                }
                if p[0] == 0x00
                    && p.len() == 7
                    && p[1] == 0x00
                    && p[2] == 0x00
                    && p[5] == 0x00
                    && p[6] == 0x00
                {
                    eprintln!("EXECUTE: OK row-stream terminator, {} rows", row_count);
                    break;
                }
                eprintln!(
                    "EXECUTE: ROW ({} bytes): {:02x?}",
                    p.len(),
                    &p[..p.len().min(20)]
                );
                row_count += 1;
            }
        }
        pkt_idx += 1;
        if pkt_idx > 30 {
            panic!("too many packets");
        }
    }

    assert_eq!(row_count, 1, "expected 1 row, got {}", row_count);
    client.quit().ok();
}

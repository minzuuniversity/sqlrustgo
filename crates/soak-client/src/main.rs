//! Minimal MySQL soak client — N threads, persistent connections, high QPS.
use std::time::Instant;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::thread;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("Usage: soak_client <host> <port> <threads> <duration_seconds>");
        std::process::exit(1);
    }
    let host = &args[1];
    let port: u16 = args[2].parse().expect("port");
    let threads: usize = args[3].parse().expect("threads");
    let duration_secs: u64 = args[4].parse().expect("duration");

    println!("Soak client: {} threads, {}s duration", threads, duration_secs);

    let total_q = Arc::new(AtomicUsize::new(0));
    let total_err = Arc::new(AtomicUsize::new(0));

    // Pre-spawn connections
    let mut handles = vec![];
    for tid in 0..threads {
        let host = host.to_string();
        let tq = total_q.clone();
        let te = total_err.clone();
        handles.push(thread::spawn(move || {
            // Import the MySqlTestClient from tests/common
            // We need to use a TCP connection directly
            use std::net::TcpStream;
            use std::io::{Read, Write};

            // Simple MySQL wire protocol client (inline, minimal)
            struct Client {
                stream: TcpStream,
                seq: u8,
            }
            impl Client {
                fn connect(host: &str, port: u16) -> std::io::Result<Self> {
                    let addr = format!("{}:{}", host, port);
                    let mut stream = TcpStream::connect(&addr)?;
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(15))).ok();
                    stream.set_write_timeout(Some(std::time::Duration::from_secs(15))).ok();
                    // Drain server handshake
                    let mut hdr = [0u8; 4];
                    stream.read_exact(&mut hdr).ok();
                    let len = u32::from_le_bytes(hdr) & 0x00FF_FFFF;
                    let mut drain = vec![0u8; len as usize];
                    stream.read_exact(&mut drain).ok();
                    // Send HandshakeResponse41 (empty password)
                    let caps: u32 = 0xBFA285;
                    let user = b"root\x00";
                    let auth_resp = b"\x14\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
                    let mut payload = vec![];
                    payload.extend_from_slice(&caps.to_le_bytes());
                    payload.extend_from_slice(&16777216u32.to_le_bytes());
                    payload.push(45); // collation
                    payload.extend_from_slice(&[0u8; 23]); // filler
                    payload.extend_from_slice(user);
                    payload.push(0);
                    payload.extend_from_slice(auth_resp);

                    let mut seq = 1u8;
                    let n = payload.len() as u32;
                    let hdr = [
                        n as u8,
                        (n >> 8) as u8,
                        (n >> 16) as u8,
                        seq,
                    ];
                    stream.write_all(&hdr).ok();
                    stream.write_all(&payload).ok();
                    stream.flush().ok();

                    // Read OK
                    let mut ok_hdr = [0u8; 4];
                    stream.read_exact(&mut ok_hdr).ok();

                    Self { stream, seq }
                }

                fn query(&mut self, sql: &str) -> bool {
                    let payload = [0x03u8];
                    let sql_bytes = sql.as_bytes();
                    let mut full = Vec::with_capacity(1 + sql_bytes.len());
                    full.extend_from_slice(&payload);
                    full.extend_from_slice(sql_bytes);

                    self.seq = self.seq.wrapping_add(1);
                    let n = full.len() as u32;
                    let hdr = [
                        n as u8,
                        (n >> 8) as u8,
                        (n >> 16) as u8,
                        self.seq,
                    ];
                    if self.stream.write_all(&hdr).is_err() { return false; }
                    if self.stream.write_all(&full).is_err() { return false; }
                    if self.stream.flush().is_err() { return false; }

                    // Read response
                    let mut rhdr = [0u8; 4];
                    if self.stream.read_exact(&mut rhdr).is_err() { return false; }
                    self.seq = rhdr[3];
                    let len = rhdr[0] as u32 | ((rhdr[1] as u32) << 8) | ((rhdr[2] as u32) << 16);
                    // Drain
                    let mut drain = vec![0u8; len as usize];
                    if self.stream.read_exact(&mut drain).is_err() { return false; }
                    // Check if OK (0x00) or ERR (0xFF)
                    drain.get(0).copied() != Some(0xFF)
                }
            }

            let mut client = match Client::connect(&host, port) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Thread {}: connect failed: {}", tid, e);
                    te.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            // Warm up
            if !client.query("SELECT 1") {
                eprintln!("Thread {}: warmup failed", tid);
                te.fetch_add(1, Ordering::Relaxed);
                return;
            }

            let mut cnt = tid as i64;
            let deadline = Instant::now() + std::time::Duration::from_secs(duration_secs);
            let mut local_q = 0usize;
            let mut local_err = 0usize;

            while Instant::now() < deadline {
                if client.query(&format!("SELECT {}", cnt)) {
                    local_q += 1;
                } else {
                    local_err += 1;
                }
                cnt += threads as i64;
            }

            tq.fetch_add(local_q, Ordering::Relaxed);
            te.fetch_add(local_err, Ordering::Relaxed);
        }));
    }

    for h in handles { h.join().ok(); }

    let q = total_q.load(Ordering::Relaxed);
    let e = total_err.load(Ordering::Relaxed);
    let actual = std::time::Duration::from_secs(duration_secs);
    let qps = q as f64 / duration_secs as f64;
    println!("RESULT: q={} err={} qps={:.0f}", q, e, qps);
}

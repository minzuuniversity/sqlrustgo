//! Minimal raw-protocol MySQL client for the V312-21 compat runner.
//!
//! This is a stripped-down fork of `tests/common/mod.rs::MySqlTestClient`
//! containing only the methods needed to drive a SQL fixture through an
//! ephemeral server. Kept self-contained so the compat-runner binary has
//! no dependency on the workspace test tree (which is not a published
//! crate).
//!
//! See: openspec/changes/v312-21-mysql-compat-sql-surface-backlog/

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const CAP_LONG_PASSWORD: u32 = 0x00000001;
const CAP_PROTOCOL_41: u32 = 0x00000200;
const CAP_SECURE_CONNECTION: u32 = 0x00008000;
const CLIENT_CAPABILITIES: u32 = CAP_LONG_PASSWORD | CAP_PROTOCOL_41 | CAP_SECURE_CONNECTION;
const MAX_PACKET_SIZE: u32 = 16 * 1024 * 1024;
const CHARSET_UTF8: u8 = 33;
const SCRAMBLE_LEN: usize = 20;

#[derive(Debug)]
pub enum CompatError {
    Io(std::io::Error),
    Protocol(String),
    Eof,
}

impl std::fmt::Display for CompatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatError::Io(e) => write!(f, "IO: {}", e),
            CompatError::Protocol(s) => write!(f, "Protocol: {}", s),
            CompatError::Eof => write!(f, "EOF"),
        }
    }
}

impl std::error::Error for CompatError {}

impl From<std::io::Error> for CompatError {
    fn from(e: std::io::Error) -> Self {
        CompatError::Io(e)
    }
}

/// Default `E = CompatError` keeps common call sites short while
/// preserving the ability to override at the call site (e.g.
/// `Result<T, std::io::Error>`).
pub type Result<T, E = CompatError> = std::result::Result<T, E>;

pub struct CompatClient {
    stream: TcpStream,
    user: String,
    password: String,
    database: String,
}

impl CompatClient {
    pub fn connect_default() -> Result<Self> {
        Self::connect("tester", "tester", "")
    }

    pub fn connect(user: &str, password: &str, database: &str) -> Result<Self> {
        // Use the canonical pool port range; start_ephemeral picks a
        // free port from this range when called via connect_default.
        // For the compat runner we just connect to a known port
        // provided via the `COMPAT_PORT` env var. The bash gate sets
        // this up via start_ephemeral.
        let port: u16 = std::env::var("COMPAT_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        if port == 0 {
            return Err(CompatError::Protocol(
                "COMPAT_PORT env var not set; the runner must be invoked after start_ephemeral establishes a port".into(),
            ));
        }
        let addr = format!("127.0.0.1:{}", port);
        let mut stream = TcpStream::connect(&addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

        let mut client = CompatClient {
            stream,
            user: user.into(),
            password: password.into(),
            database: database.into(),
        };
        client.handshake_and_auth()?;
        Ok(client)
    }

    fn read_packet(&mut self) -> Result<Vec<u8>> {
        let mut header = [0u8; 4];
        self.stream.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header) & 0x00FF_FFFF;
        let mut payload = vec![0u8; len as usize];
        self.stream.read_exact(&mut payload)?;
        Ok(payload)
    }

    fn write_packet(&mut self, seq: u8, payload: &[u8]) -> Result<()> {
        let len = payload.len() as u32;
        let header = [len as u8, (len >> 8) as u8, (len >> 16) as u8, seq];
        self.stream.write_all(&header)?;
        self.stream.write_all(payload)?;
        self.stream.flush()?;
        Ok(())
    }

    fn parse_handshake(&self, hs: &[u8]) -> Result<[u8; SCRAMBLE_LEN]> {
        if hs.is_empty() || hs[0] != 0x0a {
            return Err(CompatError::Protocol(format!(
                "not HandshakeV10: first byte 0x{:02x}",
                hs.first().copied().unwrap_or(0)
            )));
        }
        let v_end = hs[1..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| CompatError::Protocol("server version not NUL-terminated".into()))?;
        let conn_id_off = 1 + v_end + 1;
        let scramble1_off = conn_id_off + 4;
        let scramble2_off = scramble1_off + 8 + 1 + 2 + 1 + 2 + 2 + 1 + 10;
        if hs.len() < scramble2_off + SCRAMBLE_LEN {
            return Err(CompatError::Protocol(format!(
                "handshake too short: {} bytes",
                hs.len()
            )));
        }
        let mut scramble = [0u8; SCRAMBLE_LEN];
        scramble[..8].copy_from_slice(&hs[scramble1_off..scramble1_off + 8]);
        scramble[8..].copy_from_slice(&hs[scramble2_off..scramble2_off + 12]);
        Ok(scramble)
    }

    fn native_password_auth(password: &[u8], scramble: &[u8; SCRAMBLE_LEN]) -> [u8; SCRAMBLE_LEN] {
        use sha1::{Digest, Sha1};
        let mut h1 = Sha1::new();
        h1.update(password);
        let sha1_pw = h1.finalize();
        let mut h2 = Sha1::new();
        h2.update(sha1_pw);
        let sha1_sha1_pw = h2.finalize();
        let mut h3 = Sha1::new();
        h3.update(scramble);
        h3.update(sha1_sha1_pw);
        let scramble_sha1_sha1_pw = h3.finalize();
        let mut out = [0u8; SCRAMBLE_LEN];
        for i in 0..SCRAMBLE_LEN {
            out[i] = sha1_pw[i] ^ scramble_sha1_sha1_pw[i];
        }
        out
    }

    fn build_handshake_response41(&self, auth_response: &[u8]) -> Vec<u8> {
        let mut p = Vec::with_capacity(64 + self.user.len() + auth_response.len());
        p.extend_from_slice(&CLIENT_CAPABILITIES.to_le_bytes());
        p.extend_from_slice(&MAX_PACKET_SIZE.to_le_bytes());
        p.push(CHARSET_UTF8);
        p.extend_from_slice(&[0u8; 23]);
        p.extend_from_slice(self.user.as_bytes());
        p.push(0x00);
        if auth_response.len() > u8::MAX as usize {
            panic!("auth response too long for SECURE_CONNECTION");
        }
        p.push(auth_response.len() as u8);
        p.extend_from_slice(auth_response);
        p
    }

    fn handshake_and_auth(&mut self) -> Result<()> {
        // 1. Receive HandshakeV10
        let hs = self.read_packet()?;
        let scramble = self.parse_handshake(&hs)?;

        // 2. Compute native_password response
        let auth = Self::native_password_auth(self.password.as_bytes(), &scramble);

        // 3. Send HandshakeResponse41
        let resp = self.build_handshake_response41(&auth);
        self.write_packet(1, &resp)?;

        // 4. Read OK (0x00) or ERR (0xFF)
        let reply = self.read_packet()?;
        if reply.first().copied() == Some(0xFF) {
            let msg = if reply.len() > 9 {
                String::from_utf8_lossy(&reply[9..]).into_owned()
            } else {
                String::from_utf8_lossy(&reply).into_owned()
            };
            return Err(CompatError::Protocol(format!("auth failed: {msg}")));
        }
        if reply.first().copied() != Some(0x00) {
            return Err(CompatError::Protocol(format!(
                "unexpected auth reply: first byte 0x{:02x}",
                reply.first().copied().unwrap_or(0)
            )));
        }
        Ok(())
    }

    pub fn exec(&mut self, sql: &str) -> Result<()> {
        let mut p = vec![0x03];
        p.extend_from_slice(sql.as_bytes());
        self.write_packet(0, &p)?;
        let resp = self.read_packet()?;
        match resp.first().copied() {
            Some(0x00) => Ok(()),
            Some(0xFF) => Err(CompatError::Protocol(format!(
                "ERR: {}",
                String::from_utf8_lossy(&resp.get(9..).unwrap_or(&[]))
            ))),
            _ => Ok(()), // result-set header — caller will use query_rows
        }
    }

    pub fn query_rows(&mut self, sql: &str) -> Result<Vec<Vec<String>>> {
        let mut p = vec![0x03];
        p.extend_from_slice(sql.as_bytes());
        self.write_packet(0, &p)?;
        let resp = self.read_packet()?;
        if resp.first().copied() == Some(0xFF) {
            return Err(CompatError::Protocol(format!(
                "ERR: {}",
                String::from_utf8_lossy(&resp.get(9..).unwrap_or(&[]))
            )));
        }
        if resp.first().copied() == Some(0x00) {
            return Ok(Vec::new());
        }
        // Parse column count (lenenc int)
        let mut pos = 0;
        let col_count = read_lenenc_int(&resp, &mut pos)?;
        // Drain column defs + terminator
        for _ in 0..col_count {
            self.read_packet()?;
        }
        self.read_packet()?;
        // Read rows until terminator. COM_QUERY text-protocol row:
        // no null bitmap. Each cell is a lenenc-string where 0xFB
        // means NULL. (The null bitmap format is only used in
        // COM_STMT_EXECUTE binary protocol, not COM_QUERY.)
        let mut rows = Vec::new();
        loop {
            let pkt = self.read_packet()?;
            if pkt.is_empty() {
                return Err(CompatError::Protocol("unexpected empty row packet".into()));
            }
            if pkt[0] == 0xFE || pkt[0] == 0x00 {
                // EOF / OK terminator
                break;
            }
            if pkt[0] == 0xFF {
                return Err(CompatError::Protocol(format!(
                    "ERR during result set: {}",
                    String::from_utf8_lossy(&pkt.get(3..).unwrap_or(&[]))
                )));
            }
            let mut rpos = 0;
            let mut row = Vec::with_capacity(col_count as usize);
            for _ in 0..col_count {
                if rpos >= pkt.len() {
                    return Err(CompatError::Protocol(format!(
                        "row packet truncated: rpos={} pkt_len={}",
                        rpos,
                        pkt.len()
                    )));
                }
                if pkt[rpos] == 0xFB {
                    rpos += 1;
                    row.push("NULL".to_string());
                } else {
                    let len = read_lenenc_int(&pkt, &mut rpos)?;
                    let end = rpos + len as usize;
                    if end > pkt.len() {
                        return Err(CompatError::Protocol(format!(
                            "row cell out of bounds: rpos={} len={} pkt_len={}",
                            rpos,
                            len,
                            pkt.len()
                        )));
                    }
                    row.push(String::from_utf8_lossy(&pkt[rpos..end]).into_owned());
                    rpos = end;
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }
}

/// Helper: read a length-encoded integer (1/3/4-byte prefix).
fn read_lenenc_int(payload: &[u8], pos: &mut usize) -> Result<u64> {
    if *pos >= payload.len() {
        return Err(CompatError::Protocol("lenenc int: no bytes".into()));
    }
    let first = payload[*pos];
    *pos += 1;
    match first {
        b if b < 0xFB => Ok(b as u64),
        0xFC => {
            if *pos + 2 > payload.len() {
                return Err(CompatError::Protocol("lenenc int: truncated".into()));
            }
            let v = u16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
            *pos += 2;
            Ok(v as u64)
        }
        0xFD => {
            if *pos + 3 > payload.len() {
                return Err(CompatError::Protocol("lenenc int: truncated".into()));
            }
            let v = u32::from_le_bytes([0, payload[*pos], payload[*pos + 1], payload[*pos + 2]]);
            *pos += 3;
            Ok(v as u64)
        }
        0xFE => {
            if *pos + 8 > payload.len() {
                return Err(CompatError::Protocol("lenenc int: truncated".into()));
            }
            let v = u64::from_le_bytes([
                payload[*pos],
                payload[*pos + 1],
                payload[*pos + 2],
                payload[*pos + 3],
                payload[*pos + 4],
                payload[*pos + 5],
                payload[*pos + 6],
                payload[*pos + 7],
            ]);
            *pos += 8;
            Ok(v)
        }
        _ => Err(CompatError::Protocol(format!(
            "lenenc int: invalid first byte 0x{first:02x}"
        ))),
    }
}

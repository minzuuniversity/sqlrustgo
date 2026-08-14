//! MySQL wire-protocol client for `sqlrustgo-cli`.
//!
//! Implements the mysql_native_password auth handshake and COM_QUERY
//! message exchange over raw TCP. Adapted from `tests/common/mod.rs`
//! which is test-only and cannot be depended on at runtime.

use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const SCRAMBLE_LEN: usize = 20;
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

// Capability flags sent in HandshakeResponse41.
const CAP_LONG_PASSWORD: u32 = 0x0000_0001;
const CAP_PROTOCOL_41: u32 = 0x0000_0200;
const CAP_SECURE_CONNECTION: u32 = 0x0000_8000;
const CLIENT_CAPABILITIES: u32 = CAP_LONG_PASSWORD | CAP_PROTOCOL_41 | CAP_SECURE_CONNECTION;
const MAX_PACKET_SIZE: u32 = 16 * 1024 * 1024;
const CHARSET_UTF8: u8 = 33;

/// Result of a query execution.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Column names (empty for DDL/DML that has no result set).
    pub columns: Vec<String>,
    /// Rows, each row is a `Vec<String>` (NULL cells → empty string).
    pub rows: Vec<Vec<String>>,
    /// Number of rows returned.
    pub row_count: usize,
    /// Wall-clock duration of the query.
    pub duration: Duration,
}

/// Classification of the MySQL-compatible server we connected to.
///
/// The handshake `server_version` string is the only reliable way to
/// distinguish `sqlrustgo-mysql-server` from `mysqld` / `mariadbd`
/// without sending a probe query. The latter is what causes
/// Issue #4176 — REPL defaults to 127.0.0.1:3306 which collides with
/// system MySQL on dev machines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerKind {
    /// The server identifies itself as sqlrustgo (version string contains
    /// `sqlrustgo`).
    SqlRustGo(String),
    /// A non-sqlrustgo MySQL-compatible server (system MySQL, MariaDB,
    /// Percona, etc.). The string is the raw `server_version` from the
    /// handshake so callers can render it in diagnostics.
    Other(String),
    /// Handshake could not be parsed (truncated, wrong protocol, etc.).
    Unknown,
}

impl ServerKind {
    /// True iff this is a confirmed sqlrustgo server.
    pub fn is_sqlrustgo(&self) -> bool {
        matches!(self, ServerKind::SqlRustGo(_))
    }

    /// One-line diagnostic message for the case where REPL/CLI connected
    /// to a non-sqlrustgo server on the given port. Empty for sqlrustgo
    /// or unknown so we don't print false-positive warnings when the
    /// issue is something else (e.g. wrong password).
    pub fn diagnostic_for_port(&self, port: u16) -> String {
        match self {
            ServerKind::SqlRustGo(_) => String::new(),
            ServerKind::Other(ver) => format!(
                "warning: server on {port} identifies as `{ver}` — this looks like a \
                 non-sqlrustgo MySQL/MariaDB. Port {port} is the default for system MySQL. \
                 Start sqlrustgo-mysql-server (or pass --host/--port to point at the right \
                 server)."
            ),
            ServerKind::Unknown => String::new(),
        }
    }
}

/// Parse the server_version string from a HandshakeV10 packet payload.
///
/// Returns the raw server version (e.g. `"8.0.33"` or
/// `"8.0.33-sqlrustgo"`) — callers can inspect it to detect non-sqlrustgo
/// servers. Returns `Err` on protocol errors (wrong version byte,
/// missing NUL terminator, packet too short) so a corrupted stream
/// doesn't masquerade as a known MySQL.
pub fn parse_handshake_version(handshake: &[u8]) -> anyhow::Result<ServerKind> {
    // Minimum valid handshake: protocol(1) + version_at_least_1 + NUL(1)
    // + conn_id(4) + ... — anything shorter than 6 bytes is certainly
    // malformed (the test suite relies on this).
    if handshake.len() < 6 {
        return Err(anyhow::anyhow!(
            "handshake too short: {} bytes",
            handshake.len()
        ));
    }
    if handshake[0] != 0x0a {
        return Err(anyhow::anyhow!(
            "not a HandshakeV10: first byte = 0x{:02x}",
            handshake[0]
        ));
    }
    let v_end_rel = handshake[1..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow::anyhow!("server version not NUL-terminated"))?;
    let version = String::from_utf8_lossy(&handshake[1..1 + v_end_rel]).into_owned();
    let kind = if version.to_lowercase().contains("sqlrustgo") {
        ServerKind::SqlRustGo(version)
    } else {
        ServerKind::Other(version)
    };
    Ok(kind)
}

/// A MySQL wire-protocol client connected to a running server.
pub struct Client {
    stream: TcpStream,
    server_kind: ServerKind,
}

impl Client {
    /// Connect to a server at `host:port` using mysql_native_password auth.
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> anyhow::Result<Self> {
        let mut stream = TcpStream::connect((host, port))
            .map_err(|e| anyhow::anyhow!("connect {host}:{port}: {e}"))?;
        stream
            .set_read_timeout(Some(READ_TIMEOUT))
            .map_err(|e| anyhow::anyhow!("set_read_timeout: {e}"))?;
        stream
            .set_write_timeout(Some(WRITE_TIMEOUT))
            .map_err(|e| anyhow::anyhow!("set_write_timeout: {e}"))?;

        // 1) Read server's HandshakeV10 and extract scramble + version
        let handshake = read_packet(&mut stream)?;
        let server_kind = parse_handshake_version(&handshake).unwrap_or(ServerKind::Unknown);
        let scramble = parse_handshake(&handshake)?;

        // 2) Compute mysql_native_password auth response.
        //    V312-38 / Issue #4176: empty password must produce a
        //    zero-length auth-response (length byte = 0); MySQL protocol
        //    forbids sending the SHA1("") token for an empty password.
        let auth = native_password_auth(password.as_bytes(), &scramble);

        // 3) Send HandshakeResponse41
        let resp = build_handshake_response41(user, &auth)?;
        write_packet(&mut stream, 1, &resp)?;

        // 4) Read auth result (OK or ERR)
        let auth_resp = read_packet(&mut stream)?;
        check_ok_or_err(&auth_resp).map_err(|e| anyhow::anyhow!("auth failed: {e}"))?;

        Ok(Self {
            stream,
            server_kind,
        })
    }

    /// The server kind identified from the handshake. Useful for callers
    /// that want to warn the user when they accidentally connected to a
    /// non-sqlrustgo MySQL.
    pub fn server_kind(&self) -> &ServerKind {
        &self.server_kind
    }

    /// Execute a SQL statement that may return a result set (DDL, DML).
    /// Drains all response packets and returns Ok(()) on success.
    pub fn exec(&mut self, sql: &str) -> anyhow::Result<()> {
        let p = build_com_query(sql);
        write_packet(&mut self.stream, 0, &p)?;

        let first = read_packet(&mut self.stream)?;
        if first.is_empty() {
            return Err(anyhow::anyhow!("empty response"));
        }
        if first[0] == 0x00 {
            return Ok(()); // OK
        }
        if first[0] == 0xFF {
            return Err(anyhow::anyhow!("server error: {}", err_msg(&first)));
        }
        // Result set — drain column defs, rows, terminator
        let n = read_lenenc_int(&first, &mut 0).unwrap_or(0) as usize;
        for _ in 0..n {
            let _ = read_packet(&mut self.stream)?;
        }
        loop {
            let pkt = read_packet(&mut self.stream)?;
            if pkt.is_empty() {
                return Err(anyhow::anyhow!("unexpected empty packet"));
            }
            if pkt[0] == 0xFE && pkt.len() < 9 {
                break;
            }
            if pkt[0] == 0x00 {
                break;
            }
            if pkt[0] == 0xFF {
                return Err(anyhow::anyhow!("server error: {}", err_msg(&pkt)));
            }
        }
        Ok(())
    }

    /// Execute a SQL query and return the result set.
    pub fn query(&mut self, sql: &str) -> anyhow::Result<QueryResult> {
        let start = std::time::Instant::now();
        let p = build_com_query(sql);
        write_packet(&mut self.stream, 0, &p)?;

        // Read first response packet
        let first = read_packet(&mut self.stream)?;
        if first.is_empty() {
            return Err(anyhow::anyhow!("empty response"));
        }
        // OK: DDL/DML with no result set
        if first[0] == 0x00 {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                row_count: 0,
                duration: start.elapsed(),
            });
        }
        // ERR
        if first[0] == 0xFF {
            return Err(anyhow::anyhow!(
                "query `{sql}` returned ERR: {}",
                err_msg(&first)
            ));
        }
        // EOF (empty result set, no columns)
        if first[0] == 0xFE && first.len() < 9 {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                row_count: 0,
                duration: start.elapsed(),
            });
        }

        // Result set: first packet is column count (lenenc int)
        let mut pos = 0;
        let col_count = read_lenenc_int(&first, &mut pos)? as usize;

        // 2) Column definition packets — extract column names
        let mut columns = Vec::with_capacity(col_count);
        for _ in 0..col_count {
            let def = read_packet(&mut self.stream)?;
            columns.push(parse_column_name(&def));
        }

        // 3) Skip the intermediate EOF/OK that the server sends after
        //    column definitions (per MySQL protocol:
        //      col_count → col_defs… → INTERMEDIATE terminator
        //                  → rows…  → TRAILING terminator
        //    ). The sqlrustgo server always sends the intermediate one
        //    (even when DEPRECATE_EOF is set, it sends a 5-byte EOF
        //    packet). We must consume it before reading rows; otherwise
        //    we treat it as the end of the result set and misalign the
        //    stream — subsequent reads see the row packets as garbage
        //    and produce "row packet truncated" / "lenenc str: content
        //    oob" errors.
        let intermediate = read_packet(&mut self.stream)?;
        if intermediate.is_empty() {
            return Err(anyhow::anyhow!("unexpected empty intermediate packet"));
        }
        if intermediate[0] == 0xFF {
            return Err(anyhow::anyhow!(
                "ERR after column defs: {}",
                err_msg(&intermediate)
            ));
        }
        // intermediate[0] == 0xFE (EOF) or 0x00 (OK) — both are
        // valid intermediate terminators per the spec. Anything else
        // means the server skipped the intermediate step (not standard
        // but tolerated: treat that packet as the first row instead).
        let mut next_pkt = if intermediate[0] == 0xFE || intermediate[0] == 0x00 {
            read_packet(&mut self.stream)?
        } else {
            intermediate.clone()
        };

        // 4) Read rows until the trailing EOF/OK terminator.
        let mut rows: Vec<Vec<String>> = Vec::new();
        loop {
            if next_pkt.is_empty() {
                return Err(anyhow::anyhow!("unexpected empty row packet"));
            }
            // EOF terminator (DEPRECATE_EOF=0, short packet < 9 bytes)
            if next_pkt[0] == 0xFE && next_pkt.len() < 9 {
                break;
            }
            // OK terminator (DEPRECATE_EOF=1)
            if next_pkt[0] == 0x00 {
                break;
            }
            if next_pkt[0] == 0xFF {
                return Err(anyhow::anyhow!(
                    "ERR during result set: {}",
                    err_msg(&next_pkt)
                ));
            }
            // Row data
            let mut p = 0;
            let mut row = Vec::with_capacity(col_count);
            for _ in 0..col_count {
                if p >= next_pkt.len() {
                    return Err(anyhow::anyhow!("row packet truncated"));
                }
                if next_pkt[p] == 0xFB {
                    p += 1;
                    row.push(String::new());
                } else {
                    let s = read_lenenc_str(&next_pkt, &mut p)?;
                    row.push(s.to_string());
                }
            }
            rows.push(row);
            next_pkt = read_packet(&mut self.stream)?;
        }
        let row_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            row_count,
            duration: start.elapsed(),
        })
    }

    /// Send COM_QUIT to close the connection gracefully.
    pub fn quit(&mut self) -> anyhow::Result<()> {
        write_packet(&mut self.stream, 0, &[0x01])?; // COM_QUIT
        Ok(())
    }
}

// ─── Packet I/O ──────────────────────────────────────────────────────

fn read_packet(stream: &mut TcpStream) -> anyhow::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|e| anyhow::anyhow!("read packet header: {e}"))?;
    let len = u32::from_le_bytes(header) & 0x00FF_FFFF;
    let mut payload = vec![0u8; len as usize];
    stream
        .read_exact(&mut payload)
        .map_err(|e| anyhow::anyhow!("read packet payload (len={len}): {e}"))?;
    Ok(payload)
}

fn write_packet(stream: &mut TcpStream, seq: u8, payload: &[u8]) -> anyhow::Result<()> {
    let len = payload.len() as u32;
    let header = [len as u8, (len >> 8) as u8, (len >> 16) as u8, seq];
    stream
        .write_all(&header)
        .map_err(|e| anyhow::anyhow!("write header: {e}"))?;
    stream
        .write_all(payload)
        .map_err(|e| anyhow::anyhow!("write payload: {e}"))?;
    stream.flush().map_err(|e| anyhow::anyhow!("flush: {e}"))?;
    Ok(())
}

// ─── Handshake parsing ──────────────────────────────────────────────

/// Parse the server's HandshakeV10 packet and return the 20-byte
/// scramble (auth_plugin_data) used in mysql_native_password exchange.
fn parse_handshake(handshake: &[u8]) -> anyhow::Result<[u8; SCRAMBLE_LEN]> {
    if handshake.is_empty() || handshake[0] != 0x0a {
        return Err(anyhow::anyhow!(
            "not a HandshakeV10: first byte = 0x{:02x}",
            handshake.first().copied().unwrap_or(0)
        ));
    }
    let v_end = handshake[1..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow::anyhow!("server version not NUL-terminated"))?;
    let conn_id_off = 1 + v_end + 1;
    let scramble1_off = conn_id_off + 4;
    let scramble2_off = scramble1_off + 8 + 1 + 2 + 1 + 2 + 2 + 1 + 10;
    if handshake.len() < scramble2_off + SCRAMBLE_LEN {
        return Err(anyhow::anyhow!(
            "handshake too short: {} bytes (need at least {})",
            handshake.len(),
            scramble2_off + SCRAMBLE_LEN
        ));
    }
    let mut scramble = [0u8; SCRAMBLE_LEN];
    scramble[..8].copy_from_slice(&handshake[scramble1_off..scramble1_off + 8]);
    scramble[8..].copy_from_slice(&handshake[scramble2_off..scramble2_off + 12]);
    Ok(scramble)
}

/// mysql_native_password: SHA1(password) XOR SHA1(scramble + SHA1(SHA1(password)))
///
/// V312-38 / Issue #4176: per MySQL protocol, when `password` is empty the
/// client MUST send an empty `auth-response` (length byte = 0). Sending
/// `SHA1("") XOR SHA1(scramble + SHA1(SHA1("")))` instead makes servers
/// (notably system MySQL/MariaDB) respond with `Access denied for user
/// '<u>'@'<h>' (using password: YES)` because the 20-byte token does not
/// match what the server expects for a user with an empty password.
///
/// Returns a `Vec<u8>` (rather than `[u8; 20]`) so an empty password
/// yields a 0-length payload. Non-empty passwords still produce the
/// standard 20-byte XOR token.
pub(crate) fn native_password_auth(password: &[u8], scramble: &[u8; SCRAMBLE_LEN]) -> Vec<u8> {
    if password.is_empty() {
        // Per MySQL protocol: empty password → no auth-response bytes.
        return Vec::new();
    }
    let mut h1 = Sha1::new();
    h1.update(password);
    let sha1_pw = h1.finalize();

    let mut h2 = Sha1::new();
    h2.update(sha1_pw);
    let sha1_sha1_pw = h2.finalize();

    let mut h3 = Sha1::new();
    h3.update(scramble);
    h3.update(sha1_sha1_pw);
    let scrambled = h3.finalize();

    let mut out = [0u8; SCRAMBLE_LEN];
    for i in 0..SCRAMBLE_LEN {
        out[i] = sha1_pw[i] ^ scrambled[i];
    }
    out.to_vec()
}

/// Build the HandshakeResponse41 packet body for SECURE_CONNECTION auth.
/// The auth-response length byte is `auth_response.len()` as a single
/// u8, so an empty `auth_response` (empty password) correctly encodes
/// `0x00` as the length field followed by no token bytes.
pub(crate) fn build_handshake_response41(
    user: &str,
    auth_response: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let mut p = Vec::with_capacity(64 + user.len() + auth_response.len());
    p.extend_from_slice(&CLIENT_CAPABILITIES.to_le_bytes());
    p.extend_from_slice(&MAX_PACKET_SIZE.to_le_bytes());
    p.push(CHARSET_UTF8);
    p.extend_from_slice(&[0u8; 23]); // reserved
    p.extend_from_slice(user.as_bytes());
    p.push(0x00);
    if auth_response.len() > u8::MAX as usize {
        return Err(anyhow::anyhow!("auth response too long"));
    }
    p.push(auth_response.len() as u8);
    p.extend_from_slice(auth_response);
    Ok(p)
}

// ─── Packet building ────────────────────────────────────────────────

fn build_com_query(sql: &str) -> Vec<u8> {
    let mut p = Vec::with_capacity(1 + sql.len());
    p.push(0x03); // COM_QUERY
    p.extend_from_slice(sql.as_bytes());
    p
}

// ─── Response parsing ────────────────────────────────────────────────

fn check_ok_or_err(payload: &[u8]) -> Result<(), String> {
    if payload.is_empty() {
        return Err("empty response packet".into());
    }
    if payload[0] == 0x00 {
        Ok(())
    } else if payload[0] == 0xFF {
        Err(format!("server ERR: {}", err_msg(payload)))
    } else {
        Err(format!("unexpected response (first=0x{:02x})", payload[0]))
    }
}

/// Extract error message from an ERR packet (after the 9-byte header).
fn err_msg(pkt: &[u8]) -> String {
    if pkt.len() > 9 {
        String::from_utf8_lossy(&pkt[9..]).into_owned()
    } else {
        String::from_utf8_lossy(pkt).into_owned()
    }
}

/// Parse column name from a column definition packet (Catalog(EOF).def).
///
/// MySQL column def packet layout (COM_QUERY response):
///   lenenc catalog (always "def")
///   lenenc schema (db name)
///   lenenc table alias
///   lenenc table name
///   lenenc column alias
///   lenenc column name  ← we want this
///   lenenc fixed-length fields (always 0x0c)
///   2    character set
///   4    column length
///   1    column type
///   2    flags
///   1    decimals
///   2    reserved
fn parse_column_name(def: &[u8]) -> String {
    if def.is_empty() || def[0] == 0xFF {
        return String::new();
    }
    let mut pos = 0;
    // Skip: catalog, schema, table alias, table name (4 lenenc strings)
    for _ in 0..4 {
        if pos >= def.len() {
            return String::new();
        }
        if def[pos] == 0xFB {
            pos += 1;
        } else {
            let (_, n) = match try_read_lenenc_str(def, pos) {
                Some((s, n)) => (s, n),
                None => return String::new(),
            };
            pos = n;
        }
    }
    // Column name (the 5th lenenc string)
    if pos >= def.len() {
        return String::new();
    }
    if def[pos] == 0xFB {
        return String::new();
    }
    match try_read_lenenc_str(def, pos) {
        Some((name, _)) => name,
        None => String::new(),
    }
}

/// Read a length-encoded string; returns Err on bounds failure.
fn read_lenenc_str<'a>(payload: &'a [u8], pos: &mut usize) -> anyhow::Result<&'a str> {
    if *pos >= payload.len() {
        return Err(anyhow::anyhow!("lenenc str: out of bounds"));
    }
    let first = payload[*pos];
    *pos += 1;
    let len = match first {
        0xFB => return Ok(""),
        0xFC => {
            if *pos + 2 > payload.len() {
                return Err(anyhow::anyhow!("lenenc str: 2-byte len oob"));
            }
            let v = u16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
            *pos += 2;
            v as usize
        }
        0xFD => {
            if *pos + 3 > payload.len() {
                return Err(anyhow::anyhow!("lenenc str: 3-byte len oob"));
            }
            let v = payload[*pos] as usize
                | ((payload[*pos + 1] as usize) << 8)
                | ((payload[*pos + 2] as usize) << 16);
            *pos += 3;
            v
        }
        0xFE => {
            if *pos + 8 > payload.len() {
                return Err(anyhow::anyhow!("lenenc str: 8-byte len oob"));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[*pos..*pos + 8]);
            *pos += 8;
            u64::from_le_bytes(buf) as usize
        }
        n => n as usize,
    };
    if *pos + len > payload.len() {
        return Err(anyhow::anyhow!("lenenc str: content oob"));
    }
    let s = std::str::from_utf8(&payload[*pos..*pos + len])
        .map_err(|e| anyhow::anyhow!("lenenc str: invalid utf-8: {e}"))?;
    *pos += len;
    Ok(s)
}

/// Non-failing variant for column name parsing.
fn try_read_lenenc_str(payload: &[u8], pos: usize) -> Option<(String, usize)> {
    let mut p = pos;
    if p >= payload.len() {
        return None;
    }
    let first = payload[p];
    p += 1;
    let len = match first {
        0xFB => 0,
        0xFC => {
            if p + 2 > payload.len() {
                return None;
            }
            let v = u16::from_le_bytes([payload[p], payload[p + 1]]);
            p += 2;
            v as usize
        }
        0xFD => {
            if p + 3 > payload.len() {
                return None;
            }
            let v = payload[p] as usize
                | ((payload[p + 1] as usize) << 8)
                | ((payload[p + 2] as usize) << 16);
            p += 3;
            v
        }
        0xFE => {
            if p + 8 > payload.len() {
                return None;
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[p..p + 8]);
            p += 8;
            u64::from_le_bytes(buf) as usize
        }
        n => n as usize,
    };
    if p + len > payload.len() {
        return None;
    }
    let s = String::from_utf8_lossy(&payload[p..p + len]).into_owned();
    p += len;
    Some((s, p))
}

/// Read a length-encoded integer.
fn read_lenenc_int(payload: &[u8], pos: &mut usize) -> anyhow::Result<u64> {
    if *pos >= payload.len() {
        return Err(anyhow::anyhow!("lenenc int: out of bounds"));
    }
    let first = payload[*pos];
    *pos += 1;
    match first {
        0xFB => Ok(0),
        0xFC => {
            if *pos + 2 > payload.len() {
                return Err(anyhow::anyhow!("lenenc int: 2-byte int oob"));
            }
            let v = u16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
            *pos += 2;
            Ok(v as u64)
        }
        0xFD => {
            if *pos + 3 > payload.len() {
                return Err(anyhow::anyhow!("lenenc int: 3-byte int oob"));
            }
            let v = payload[*pos] as u64
                | ((payload[*pos + 1] as u64) << 8)
                | ((payload[*pos + 2] as u64) << 16);
            *pos += 3;
            Ok(v)
        }
        0xFE => {
            if *pos + 8 > payload.len() {
                return Err(anyhow::anyhow!("lenenc int: 8-byte int oob"));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[*pos..*pos + 8]);
            *pos += 8;
            Ok(u64::from_le_bytes(buf))
        }
        n => Ok(n as u64),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────
//
// V312-38 / Issue #4176 regression coverage: the empty-password case
// must produce a 0-length auth-response (length byte = 0, no token),
// not the bogus 20-byte SHA1("") token that system MySQL would reject
// with `(using password: YES)`.

#[cfg(test)]
mod tests {
    use super::*;

    /// Find the auth-response length byte in a HandshakeResponse41
    /// packet body. Layout (after our builder):
    ///   4   capabilities (LE u32)
    ///   4   max packet size (LE u32)
    ///   1   charset
    ///   23  reserved
    ///   N   username (then 0x00 NUL terminator)
    ///   1   auth-response length
    ///   M   auth-response bytes
    fn auth_response_length_byte(packet: &[u8]) -> u8 {
        // capabilities + max_packet + charset + reserved = 32 bytes,
        // then username (NUL-terminated), then 1 byte length.
        let user_start = 32;
        let nul = packet[user_start..]
            .iter()
            .position(|&b| b == 0)
            .expect("username NUL terminator");
        packet[user_start + nul + 1]
    }

    fn auth_response_bytes(packet: &[u8]) -> &[u8] {
        let user_start = 32;
        let nul = packet[user_start..]
            .iter()
            .position(|&b| b == 0)
            .expect("username NUL terminator");
        let len = packet[user_start + nul + 1] as usize;
        &packet[user_start + nul + 2..user_start + nul + 2 + len]
    }

    #[test]
    fn native_password_auth_empty_password_yields_zero_length() {
        // V312-38 / Issue #4176: empty password MUST yield a 0-length
        // auth-response per MySQL protocol.
        let scramble = [0xAAu8; SCRAMBLE_LEN];
        let auth = native_password_auth(b"", &scramble);
        assert_eq!(auth.len(), 0, "empty password must produce no token bytes");
    }

    #[test]
    fn native_password_auth_nonempty_password_yields_20_bytes() {
        // Sanity: non-empty password still produces a 20-byte XOR token.
        let scramble = [0xAAu8; SCRAMBLE_LEN];
        let auth = native_password_auth(b"hunter2", &scramble);
        assert_eq!(auth.len(), SCRAMBLE_LEN);
    }

    #[test]
    fn handshake_response_empty_password_encodes_length_zero() {
        // End-to-end: build_handshake_response41 with empty auth must
        // emit 0x00 as the auth-response length byte and no token.
        let scramble = [0xAAu8; SCRAMBLE_LEN];
        let auth = native_password_auth(b"", &scramble);
        let pkt = build_handshake_response41("root", &auth).expect("build");
        assert_eq!(auth_response_length_byte(&pkt), 0);
        assert_eq!(auth_response_bytes(&pkt).len(), 0);
    }

    #[test]
    fn handshake_response_nonempty_password_encodes_length_twenty() {
        // End-to-end: non-empty auth must emit 0x14 + 20 token bytes.
        let scramble = [0xAAu8; SCRAMBLE_LEN];
        let auth = native_password_auth(b"hunter2", &scramble);
        let pkt = build_handshake_response41("root", &auth).expect("build");
        assert_eq!(auth_response_length_byte(&pkt), SCRAMBLE_LEN as u8);
        assert_eq!(auth_response_bytes(&pkt).len(), SCRAMBLE_LEN);
        assert_eq!(auth_response_bytes(&pkt), auth.as_slice());
    }

    #[test]
    fn native_password_auth_is_stable_for_same_inputs() {
        // Determinism check: same password + scramble → same token.
        let scramble = [0x55u8; SCRAMBLE_LEN];
        let a = native_password_auth(b"hunter2", &scramble);
        let b = native_password_auth(b"hunter2", &scramble);
        assert_eq!(a, b);
        assert_ne!(a, vec![0u8; SCRAMBLE_LEN]);
    }
}

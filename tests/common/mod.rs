//! Shared test utilities for the canonical wire-protocol test surface.
//!
//! The `MySqlTestClient` is a tiny raw-protocol MySQL client used by
//! integration tests that need to drive SQL through the
//! [`sqlrustgo_mysql_server::testing::start_ephemeral`] harness.
//!
//! We deliberately use raw TCP instead of the `mysql` crate. The
//! `mysql` crate's default transport is partially TLS-aware (it tries
//! to negotiate SSL when the server advertises it, and fails on
//! servers that drop the SSL Request), and its URL parameter surface
//! shifts between major versions. A raw client gives us full control
//! over HandshakeResponse41 and avoids the upgrade path entirely.
//!
//! See:
//! - `openspec/changes/mysql-server-canonical-entry/specs/server-embedded-test-harness/spec.md`
//! - `openspec/changes/mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md`

#![allow(dead_code)]

use sha1::{Digest, Sha1};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// =========================================================================
// Capabilities we declare in HandshakeResponse41. Subset of
// `capability::SERVER_DEFAULT` from the server, chosen so the server
// takes the SECURE_CONNECTION auth path (1-byte length + 20-byte
// scramble) rather than the PLUGIN_AUTH_LENENC_CLIENT_DATA path.
// =========================================================================
const CAP_LONG_PASSWORD: u32 = 0x00000001;
const CAP_PROTOCOL_41: u32 = 0x00000200;
const CAP_SECURE_CONNECTION: u32 = 0x00008000;

const CLIENT_CAPABILITIES: u32 = CAP_LONG_PASSWORD | CAP_PROTOCOL_41 | CAP_SECURE_CONNECTION;
const MAX_PACKET_SIZE: u32 = 16 * 1024 * 1024;
const CHARSET_UTF8: u8 = 33;

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

const SCRAMBLE_LEN: usize = 20;

// =========================================================================
// Error type — kept private to this module to avoid leaking new
// dependencies into every test binary.
// =========================================================================
pub mod wire_err {
    use std::fmt;

    #[derive(Debug)]
    pub struct Error(pub String);
    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }
    impl std::error::Error for Error {}

    pub type Result<T> = std::result::Result<T, Error>;
    pub fn msg(s: impl Into<String>) -> Error {
        Error(s.into())
    }
}

// =========================================================================
// MySQL packet framing helpers.
// =========================================================================
fn read_packet(stream: &mut TcpStream) -> wire_err::Result<Vec<u8>> {
    let mut header = [0u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|e| wire_err::msg(format!("read packet header: {e}")))?;
    let len = u32::from_le_bytes(header) & 0x00FF_FFFF;
    let mut payload = vec![0u8; len as usize];
    stream
        .read_exact(&mut payload)
        .map_err(|e| wire_err::msg(format!("read packet payload (len={len}): {e}")))?;
    Ok(payload)
}

fn write_packet(stream: &mut TcpStream, seq: u8, payload: &[u8]) -> wire_err::Result<()> {
    if payload.len() > 0xFF_FFFF {
        return Err(wire_err::msg(format!(
            "packet payload too large: {}",
            payload.len()
        )));
    }
    let header = (payload.len() as u32) | ((seq as u32) << 24);
    stream
        .write_all(&header.to_le_bytes())
        .map_err(|e| wire_err::msg(format!("write packet header: {e}")))?;
    stream
        .write_all(payload)
        .map_err(|e| wire_err::msg(format!("write packet payload: {e}")))?;
    stream
        .flush()
        .map_err(|e| wire_err::msg(format!("flush: {e}")))?;
    Ok(())
}

/// Parse the server's HandshakeV10 packet and return the 20-byte
/// scramble (auth_plugin_data) used in the mysql_native_password
/// exchange.
///
/// Layout (per MySQL Reference Manual §14.3 "HandshakeV10"):
///   1  protocol_version (0x0a)
///   N  server_version (NUL-terminated)
///   4  connection_id
///   8  auth_plugin_data_part_1 (scramble[0..8])
///   1  filler (0x00)
///   2  capability_flags (lower 2 bytes)
///   1  character_set
///   2  status_flags
///   2  capability_flags (upper 2 bytes)
///   1  auth_plugin_data_len (only if PLUGIN_AUTH)
///   10 reserved (only if PLUGIN_AUTH)
///   12 auth_plugin_data_part_2 (scramble[8..20]) + 1 NUL
///   N  auth_plugin_name (NUL-terminated, only if PLUGIN_AUTH)
///
/// SQLRustGo's server (`SERVER_VERSION = "8.0.33-SQLRustGo"`,
/// 16 chars) emits a fixed 84-byte handshake, so we use known offsets
/// relative to the version NUL terminator.
fn parse_handshake(handshake: &[u8]) -> wire_err::Result<[u8; SCRAMBLE_LEN]> {
    if handshake.is_empty() || handshake[0] != 0x0a {
        return Err(wire_err::msg(format!(
            "not a HandshakeV10: first byte = 0x{:02x}",
            handshake.first().copied().unwrap_or(0)
        )));
    }
    let v_end = handshake[1..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| wire_err::msg("server version not NUL-terminated"))?;
    let conn_id_off = 1 + v_end + 1;
    let scramble1_off = conn_id_off + 4;
    let scramble2_off = scramble1_off + 8 + 1 + 2 + 1 + 2 + 2 + 1 + 10;
    if handshake.len() < scramble2_off + SCRAMBLE_LEN {
        return Err(wire_err::msg(format!(
            "handshake too short: {} bytes (need at least {})",
            handshake.len(),
            scramble2_off + SCRAMBLE_LEN
        )));
    }
    let mut scramble = [0u8; SCRAMBLE_LEN];
    scramble[..8].copy_from_slice(&handshake[scramble1_off..scramble1_off + 8]);
    scramble[8..].copy_from_slice(&handshake[scramble2_off..scramble2_off + 12]);
    Ok(scramble)
}

/// Compute the mysql_native_password auth response:
///   auth_response = SHA1(password) XOR SHA1(scramble + SHA1(SHA1(password)))
fn native_password_auth(password: &[u8], scramble: &[u8; SCRAMBLE_LEN]) -> [u8; SCRAMBLE_LEN] {
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

/// Build a HandshakeResponse41 packet body. We use the SECURE_CONNECTION
/// auth path (1-byte length + N-byte scramble) and skip both the
/// database name and the auth_plugin_name fields. The server's
/// `parse_handshake_response` only consults those fields when the
/// corresponding capability flag is set in the response.
fn build_handshake_response41(user: &str, auth_response: &[u8]) -> wire_err::Result<Vec<u8>> {
    let mut p = Vec::with_capacity(64 + user.len() + auth_response.len());

    p.extend_from_slice(&(CLIENT_CAPABILITIES as u32).to_le_bytes());
    p.extend_from_slice(&MAX_PACKET_SIZE.to_le_bytes());
    p.push(CHARSET_UTF8);
    p.extend_from_slice(&[0u8; 23]); // 23 reserved bytes

    p.extend_from_slice(user.as_bytes());
    p.push(0x00);

    // SECURE_CONNECTION auth: 1-byte length + N-byte scramble.
    if auth_response.len() > u8::MAX as usize {
        return Err(wire_err::msg(
            "auth response too long for SECURE_CONNECTION",
        ));
    }
    p.push(auth_response.len() as u8);
    p.extend_from_slice(auth_response);

    Ok(p)
}

fn build_com_query(sql: &str) -> Vec<u8> {
    let mut p = Vec::with_capacity(1 + sql.len());
    p.push(0x03); // COM_QUERY
    p.extend_from_slice(sql.as_bytes());
    p
}

fn build_com_quit() -> Vec<u8> {
    vec![0x01] // COM_QUIT
}

/// Inspect a server response packet: returns `Ok(())` for an OK packet
/// (first byte = 0x00) and `Err` for an ERR packet (first byte = 0xff)
/// or any other unexpected payload.
fn check_ok_or_err(seq_expected: u8, payload: &[u8]) -> wire_err::Result<()> {
    if payload.is_empty() {
        return Err(wire_err::msg("empty response packet"));
    }
    if payload[0] == 0x00 {
        Ok(())
    } else if payload[0] == 0xff {
        // ERR packet layout: 0xff + 2-byte error code + 1-byte '#' marker
        // + 5-byte SQL state + message
        let msg = if payload.len() > 9 {
            String::from_utf8_lossy(&payload[9..]).into_owned()
        } else {
            String::from_utf8_lossy(payload).into_owned()
        };
        Err(wire_err::msg(format!(
            "server ERR packet (seq={seq_expected}): {msg}"
        )))
    } else {
        Err(wire_err::msg(format!(
            "unexpected response (seq={seq_expected}, first=0x{:02x}): {}",
            payload[0],
            String::from_utf8_lossy(payload)
        )))
    }
}

/// Parse a length-encoded integer per the MySQL protocol (used for
/// column counts in the COM_QUERY result-set header).
fn read_lenenc_int(payload: &[u8], pos: &mut usize) -> wire_err::Result<u64> {
    if *pos >= payload.len() {
        return Err(wire_err::msg("lenenc int: out of bounds"));
    }
    let first = payload[*pos];
    *pos += 1;
    match first {
        0xFB => Ok(0), // NULL
        0xFC => {
            if *pos + 2 > payload.len() {
                return Err(wire_err::msg("lenenc int: 2-byte int oob"));
            }
            let v = u16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
            *pos += 2;
            Ok(v as u64)
        }
        0xFD => {
            if *pos + 3 > payload.len() {
                return Err(wire_err::msg("lenenc int: 3-byte int oob"));
            }
            let v = payload[*pos] as u64
                | ((payload[*pos + 1] as u64) << 8)
                | ((payload[*pos + 2] as u64) << 16);
            *pos += 3;
            Ok(v)
        }
        0xFE => {
            if *pos + 8 > payload.len() {
                return Err(wire_err::msg("lenenc int: 8-byte int oob"));
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[*pos..*pos + 8]);
            *pos += 8;
            Ok(u64::from_le_bytes(buf))
        }
        n if n < 0xFB => Ok(n as u64),
        b => Err(wire_err::msg(format!(
            "lenenc int: invalid marker 0x{b:02x}"
        ))),
    }
}

/// Read a length-encoded string (1/3/4-byte length prefix + payload).
fn read_lenenc_str<'a>(payload: &'a [u8], pos: &mut usize) -> wire_err::Result<&'a str> {
    let len = read_lenenc_int(payload, pos)? as usize;
    if *pos + len > payload.len() {
        return Err(wire_err::msg("lenenc str: oob"));
    }
    let s = std::str::from_utf8(&payload[*pos..*pos + len])
        .map_err(|e| wire_err::msg(format!("lenenc str: not utf-8: {e}")))?;
    *pos += len;
    Ok(s)
}

/// Read a NUL-terminated string from `payload` starting at `pos`.
/// Returns the string slice and the position immediately after the NUL.
fn read_cstr<'a>(payload: &'a [u8], pos: &mut usize) -> wire_err::Result<&'a str> {
    let start = *pos;
    let end = payload[start..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| wire_err::msg("cstr: not NUL-terminated"))?;
    let s = std::str::from_utf8(&payload[start..start + end])
        .map_err(|e| wire_err::msg(format!("cstr: not utf-8: {e}")))?;
    *pos = start + end + 1;
    Ok(s)
}

// =========================================================================
// Public client
// =========================================================================

/// A raw-protocol MySQL client bound to an [`EphemeralHandle`].
///
/// The handle is held until the client is dropped so the listener stays
/// alive for the duration of the test.
pub struct MySqlTestClient {
    pub handle: EphemeralHandle,
    stream: TcpStream,
    next_seq: u8,
}

impl MySqlTestClient {
    /// Start an ephemeral server on `127.0.0.1` and connect to it as
    /// the `tester` user (password = `tester`). The test harness
    /// pre-creates this user via its bootstrap callback.
    pub fn connect_default() -> wire_err::Result<Self> {
        let handle = start_ephemeral(EphemeralConfig::default())
            .map_err(|e| wire_err::msg(format!("start_ephemeral: {e}")))?;
        Self::connect_handle(handle)
    }

    /// Like [`connect_default`] but with a caller-supplied
    /// [`EphemeralConfig`]. Use this to opt out of the catalog
    /// bootstrap tables when the test asserts on a clean catalog.
    pub fn connect_with_config(config: EphemeralConfig) -> wire_err::Result<Self> {
        let handle =
            start_ephemeral(config).map_err(|e| wire_err::msg(format!("start_ephemeral: {e}")))?;
        Self::connect_handle(handle)
    }

    /// Connect to an already-running ephemeral server.
    pub fn connect_handle(handle: EphemeralHandle) -> wire_err::Result<Self> {
        let mut stream = TcpStream::connect(("127.0.0.1", handle.port))
            .map_err(|e| wire_err::msg(format!("tcp connect: {e}")))?;
        stream
            .set_read_timeout(Some(READ_TIMEOUT))
            .map_err(|e| wire_err::msg(format!("set_read_timeout: {e}")))?;
        stream
            .set_write_timeout(Some(WRITE_TIMEOUT))
            .map_err(|e| wire_err::msg(format!("set_write_timeout: {e}")))?;

        // 1) Read server's HandshakeV10
        let handshake = read_packet(&mut stream)?;
        let scramble = parse_handshake(&handshake)?;

        // 2) Compute mysql_native_password auth response
        let auth = native_password_auth(b"tester", &scramble);

        // 3) Send HandshakeResponse41 (sequence id = 1)
        let resp = build_handshake_response41("tester", &auth)?;
        write_packet(&mut stream, 1, &resp)?;

        // 4) Read OK or ERR (sequence id = 2)
        let auth_resp = read_packet(&mut stream)?;
        check_ok_or_err(2, &auth_resp)?;

        Ok(Self {
            handle,
            stream,
            next_seq: 0,
        })
    }

    /// Run a SQL statement that has no result set (DDL, DML, COMMIT).
    pub fn exec(&mut self, sql: &str) -> wire_err::Result<()> {
        let p = build_com_query(sql);
        write_packet(&mut self.stream, 0, &p)?;
        let resp = read_packet(&mut self.stream)?;
        check_ok_or_err(1, &resp)?;
        Ok(())
    }

    /// Run a SELECT and return the rows as `Vec<Vec<String>>` (each
    /// inner vector is one row, one cell per column). NULL cells are
    /// returned as the empty string. This is the lowest common
    /// denominator — tests that need typed results slice the cells
    /// themselves.
    pub fn query_rows(&mut self, sql: &str) -> wire_err::Result<Vec<Vec<String>>> {
        let p = build_com_query(sql);
        write_packet(&mut self.stream, 0, &p)?;

        // 1) Column count packet
        let col_count_pkt = read_packet(&mut self.stream)?;
        let mut pos = 0;
        let col_count = read_lenenc_int(&col_count_pkt, &mut pos)? as usize;

        // 2) Column definition packets — one per column.
        for _ in 0..col_count {
            let _ = read_packet(&mut self.stream)?;
        }

        // 3) EOF separator (when DEPRECATE_EOF=0, the server sends
        //    one EOF after all columns, before the row data).
        let sep = read_packet(&mut self.stream)?;
        if !sep.is_empty() && sep[0] == 0xFF {
            return Err(wire_err::msg(format!(
                "ERR after column defs: {}",
                String::from_utf8_lossy(&sep[3..])
            )));
        }

        // 4) Row packets until EOF/OK terminator.
        let mut rows = Vec::new();
        loop {
            let pkt = read_packet(&mut self.stream)?;
            if pkt.is_empty() {
                return Err(wire_err::msg("unexpected empty row packet"));
            }
            if pkt[0] == 0xFE && pkt.len() < 9 {
                // EOF terminator (DEPRECATE_EOF=0)
                break;
            }
            if pkt[0] == 0x00 {
                // OK terminator (DEPRECATE_EOF=1, or zero-row result)
                break;
            }
            if pkt[0] == 0xFF {
                return Err(wire_err::msg(format!(
                    "ERR during result set: {}",
                    String::from_utf8_lossy(&pkt[3..])
                )));
            }
            let mut p = 0;
            let mut row = Vec::with_capacity(col_count);
            for _ in 0..col_count {
                if p >= pkt.len() {
                    return Err(wire_err::msg("row packet truncated"));
                }
                if pkt[p] == 0xFB {
                    p += 1;
                    row.push(String::new());
                } else {
                    let s = read_lenenc_str(&pkt, &mut p)?;
                    row.push(s.to_string());
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Run a SELECT and return the first row's first column as `i64`.
    /// Used for `SELECT COUNT(*)` style assertions.
    pub fn query_one_i64(&mut self, sql: &str) -> wire_err::Result<i64> {
        let rows = self.query_rows(sql)?;
        if rows.is_empty() {
            return Err(wire_err::msg(format!(
                "query_one_i64 `{sql}`: returned 0 rows"
            )));
        }
        let cell = &rows[0][0];
        cell.parse::<i64>().map_err(|e| {
            wire_err::msg(format!(
                "query_one_i64 `{sql}`: cell `{cell}` is not i64: {e}"
            ))
        })
    }

    /// Send a COM_QUIT so the server closes the connection cleanly.
    /// Not strictly required — the handle's Drop impl will tear down
    /// the server side anyway — but it makes for tidier server logs
    /// during a long test run.
    pub fn quit(&mut self) -> wire_err::Result<()> {
        let p = build_com_quit();
        write_packet(&mut self.stream, 0, &p)?;
        Ok(())
    }
}

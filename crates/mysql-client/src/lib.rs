//! MySQL Wire Protocol Client
//!
//! Low-level MySQL wire protocol client providing:
//! - TCP connection + MySQL handshake
//! - mysql_native_password authentication
//! - COM_QUERY execution
//! - Result set parsing (text protocol)
//!
//! Compatible with `sqlrustgo-mysql-server` (MySQL 8.0 wire protocol).

use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

// ============================================================================
// Constants
// ============================================================================

const MAX_PACKET_SIZE: u32 = 0x00ffffff;
const SCRAMBLE_LENGTH: usize = 20;

mod packet_type {
    pub const COM_QUIT: u8 = 0x01;
    pub const COM_QUERY: u8 = 0x03;
    pub const COM_PING: u8 = 0x0e;
}

mod capability {
    pub const LONG_PASSWORD: u32 = 0x00000001;
    pub const FOUND_ROWS: u32 = 0x00000002;
    pub const LONG_FLAG: u32 = 0x00000004;
    pub const CONNECT_WITH_DB: u32 = 0x00000008;
    pub const PROTOCOL_41: u32 = 0x00000200;
    pub const TRANSACTIONS: u32 = 0x00002000;
    pub const SECURE_CONNECTION: u32 = 0x00008000;
    pub const PLUGIN_AUTH: u32 = 0x00080000;
    pub const PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 0x00200000;
    pub const DEPRECATE_EOF: u32 = 0x01000000;
}

/// Client capability flags (what this client supports)
const CLIENT_CAPABILITIES: u32 = {
    capability::LONG_PASSWORD
        | capability::FOUND_ROWS
        | capability::LONG_FLAG
        | capability::CONNECT_WITH_DB
        | capability::PROTOCOL_41
        | capability::TRANSACTIONS
        | capability::SECURE_CONNECTION
        | capability::PLUGIN_AUTH
        | capability::PLUGIN_AUTH_LENENC_CLIENT_DATA
        | capability::DEPRECATE_EOF
};

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug)]
pub enum MySqlClientError {
    Io(std::io::Error),
    Protocol(String),
    Auth(String),
    ServerError(u16, String),
    ConnectionClosed,
}

impl std::fmt::Display for MySqlClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MySqlClientError::Io(e) => write!(f, "IO error: {}", e),
            MySqlClientError::Protocol(s) => write!(f, "Protocol error: {}", s),
            MySqlClientError::Auth(s) => write!(f, "Auth error: {}", s),
            MySqlClientError::ServerError(code, msg) => {
                write!(f, "Server error {}: {}", code, msg)
            }
            MySqlClientError::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for MySqlClientError {}

impl From<std::io::Error> for MySqlClientError {
    fn from(e: std::io::Error) -> Self {
        MySqlClientError::Io(e)
    }
}

pub type MySqlResult<T> = Result<T, MySqlClientError>;

// ============================================================================
// Wire Protocol Packet
// ============================================================================

pub struct Packet {
    pub length: u32,
    pub sequence: u8,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Read a MySQL packet from a stream (4-byte header + payload).
    pub fn read_from<R: Read + ?Sized>(r: &mut R) -> MySqlResult<Self> {
        let mut header = [0u8; 4];
        r.read_exact(&mut header)?;
        let length = u32::from_le_bytes([header[0], header[1], header[2], 0]);
        let sequence = header[3];
        let mut payload = vec![0u8; length as usize];
        if length > 0 {
            r.read_exact(&mut payload)?;
        }
        Ok(Packet {
            length,
            sequence,
            payload,
        })
    }

    /// Write a MySQL packet to a stream.
    pub fn write_to<W: Write>(&self, w: &mut W) -> MySqlResult<()> {
        if self.length > MAX_PACKET_SIZE {
            return Err(MySqlClientError::Protocol(format!(
                "Packet too large: {} > {}",
                self.length, MAX_PACKET_SIZE
            )));
        }
        let header = [
            self.length as u8,
            (self.length >> 8) as u8,
            (self.length >> 16) as u8,
            self.sequence,
        ];
        w.write_all(&header)?;
        w.write_all(&self.payload)?;
        w.flush()?;
        Ok(())
    }

    pub fn new(seq: u8, payload: Vec<u8>) -> Self {
        let length = payload.len() as u32;
        Packet {
            length,
            sequence: seq,
            payload,
        }
    }
}

// ============================================================================
// Handshake parsing (server → client)
// ============================================================================

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub auth_plugin_data: [u8; SCRAMBLE_LENGTH],
    pub capability_flags: u32,
    pub character_set: u8,
    pub status_flags: u16,
    pub auth_plugin_name: String,
}

fn read_null_terminated(data: &[u8], offset: &mut usize) -> MySqlResult<String> {
    let start = *offset;
    while *offset < data.len() && data[*offset] != 0 {
        *offset += 1;
    }
    if *offset >= data.len() {
        return Err(MySqlClientError::Protocol(
            "Null terminator not found".to_string(),
        ));
    }
    let s = String::from_utf8_lossy(&data[start..*offset]).to_string();
    *offset += 1; // skip null
    Ok(s)
}

fn read_fixed_int(data: &[u8], offset: &mut usize, bytes: usize) -> u64 {
    let mut val: u64 = 0;
    for i in 0..bytes {
        val |= (data[*offset + i] as u64) << (i * 8);
    }
    *offset += bytes;
    val
}

/// Parse the initial HandshakeV10 packet from server.
pub fn parse_handshake(payload: &[u8]) -> MySqlResult<Handshake> {
    let mut off = 0;
    let protocol_version = payload[off];
    off += 1;
    if protocol_version != 0x0a {
        return Err(MySqlClientError::Protocol(format!(
            "Expected protocol 10, got {}",
            protocol_version
        )));
    }

    let server_version = read_null_terminated(payload, &mut off)?;
    let connection_id = read_fixed_int(payload, &mut off, 4) as u32;

    // auth-plugin-data-part-1: 8 bytes
    let mut auth_plugin_data = [0u8; SCRAMBLE_LENGTH];
    for i in 0..8 {
        auth_plugin_data[i] = payload[off + i];
    }
    off += 8;

    // filler (1 byte, 0x00)
    off += 1;

    // capability_flags lower 2 bytes
    let cap_lower = read_fixed_int(payload, &mut off, 2) as u32;

    // character_set (1 byte)
    let character_set = payload[off];
    off += 1;

    // status_flags (2 bytes)
    let status_flags = read_fixed_int(payload, &mut off, 2) as u16;

    // capability_flags upper 2 bytes
    let cap_upper = read_fixed_int(payload, &mut off, 2) as u32;
    let capability_flags = cap_lower | (cap_upper << 16);

    // auth-plugin-data-len (1 byte)
    let auth_plugin_data_len = payload[off];
    off += 1;

    // reserved (10 bytes)
    off += 10;

    // auth-plugin-data-part-2 (at least 12 bytes, padded with 0x00)
    let part2_len = (auth_plugin_data_len as usize).saturating_sub(8);
    let part2_len = part2_len.min(SCRAMBLE_LENGTH - 8);
    for i in 0..part2_len {
        if off + i < payload.len() {
            auth_plugin_data[8 + i] = payload[off + i];
        }
    }
    off += part2_len.max(12); // skip at least 12 bytes for part2

    // auth-plugin-name (null-terminated, only if CLIENT_PLUGIN_AUTH is set)
    let auth_plugin_name = if capability_flags & capability::PLUGIN_AUTH != 0 && off < payload.len()
    {
        read_null_terminated(payload, &mut off)?
    } else {
        "mysql_native_password".to_string()
    };

    Ok(Handshake {
        protocol_version,
        server_version,
        connection_id,
        auth_plugin_data,
        capability_flags,
        character_set,
        status_flags,
        auth_plugin_name,
    })
}

// ============================================================================
// Authentication: mysql_native_password
// ============================================================================

/// Compute mysql_native_password hash.
/// SHA1(password) XOR SHA1(scramble + SHA1(SHA1(password)))
fn native_password_hash(password: &str, scramble: &[u8; SCRAMBLE_LENGTH]) -> [u8; 20] {
    let hash_stage1 = Sha1::digest(password.as_bytes());
    let hash_stage2 = Sha1::digest(&hash_stage1);

    let mut sha = Sha1::new();
    sha.update(scramble);
    sha.update(&hash_stage2);
    let hash_result = sha.finalize();

    let mut result = [0u8; 20];
    for i in 0..20 {
        result[i] = hash_stage1[i] ^ hash_result[i];
    }
    result
}

/// Build HandshakeResponse41 packet (client → server after handshake).
fn build_handshake_response(
    seq: u8,
    username: &str,
    auth_response: &[u8],
    database: &str,
    auth_plugin: &str,
) -> Packet {
    let mut payload = Vec::new();

    // capability flags (4 bytes)
    payload.extend_from_slice(&CLIENT_CAPABILITIES.to_le_bytes());

    // max packet size (4 bytes)
    payload.extend_from_slice(&MAX_PACKET_SIZE.to_le_bytes());

    // character set (1 byte) — utf8mb4_general_ci = 45
    payload.push(45);

    // reserved (23 bytes of zeros)
    payload.extend_from_slice(&[0u8; 23]);

    // username (null-terminated)
    payload.extend_from_slice(username.as_bytes());
    payload.push(0);

    // auth-response (length-encoded)
    payload.push(auth_response.len() as u8);
    payload.extend_from_slice(auth_response);

    // database (null-terminated)
    if !database.is_empty() {
        payload.extend_from_slice(database.as_bytes());
        payload.push(0);
    }

    // auth-plugin-name (null-terminated)
    payload.extend_from_slice(auth_plugin.as_bytes());
    payload.push(0);

    Packet::new(seq, payload)
}

// ============================================================================
// Result set parsing
// ============================================================================

#[derive(Debug)]
pub struct ColumnDefinition {
    pub catalog: String,
    pub schema: String,
    pub table: String,
    pub org_table: String,
    pub name: String,
    pub org_name: String,
    pub character_set: u16,
    pub column_length: u32,
    pub column_type: u8,
    pub flags: u16,
    pub decimals: u8,
}

#[derive(Debug)]
pub enum ResultSet {
    /// Query returned rows (column definitions + rows)
    Select {
        columns: Vec<ColumnDefinition>,
        rows: Vec<Vec<String>>,
    },
    /// OK packet (non-SELECT: INSERT/UPDATE/DELETE)
    Ok {
        affected_rows: u64,
        last_insert_id: u64,
        status_flags: u16,
        warnings: u16,
        info: String,
    },
    /// Error packet
    Error {
        error_code: u16,
        sql_state: String,
        error_message: String,
    },
}

/// A prepared statement handle returned by `prepare()`.
/// Use `id` to call `execute_prepared()` or `close_statement()`.
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    pub id: u32,
    pub param_count: u16,
    pub column_count: u16,
}

/// Parse length-encoded integer (MySQL wire protocol).
fn parse_length_encoded_int(data: &[u8], offset: &mut usize) -> MySqlResult<u64> {
    if *offset >= data.len() {
        return Err(MySqlClientError::Protocol("Unexpected EOF".to_string()));
    }
    let first = data[*offset];
    *offset += 1;
    match first {
        0xfb => Ok(u64::MAX), // NULL
        0xfc => {
            let val = u16::from_le_bytes([data[*offset], data[*offset + 1]]) as u64;
            *offset += 2;
            Ok(val)
        }
        0xfd => {
            let val = data[*offset] as u64
                | (data[*offset + 1] as u64) << 8
                | (data[*offset + 2] as u64) << 16;
            *offset += 3;
            Ok(val)
        }
        0xfe => {
            let val = u64::from_le_bytes([
                data[*offset],
                data[*offset + 1],
                data[*offset + 2],
                data[*offset + 3],
                data[*offset + 4],
                data[*offset + 5],
                data[*offset + 6],
                data[*offset + 7],
            ]);
            *offset += 8;
            Ok(val)
        }
        _ => Ok(first as u64), // 0-250: direct value
    }
}

/// Parse a length-encoded string.
fn parse_length_encoded_string(data: &[u8], offset: &mut usize) -> MySqlResult<String> {
    let len = parse_length_encoded_int(data, offset)?;
    if len == u64::MAX || len == 0 {
        return Ok(String::new());
    }
    let len = len as usize;
    if *offset + len > data.len() {
        return Err(MySqlClientError::Protocol(
            "String exceeds packet".to_string(),
        ));
    }
    let s = String::from_utf8_lossy(&data[*offset..*offset + len]).to_string();
    *offset += len;
    Ok(s)
}

fn parse_column_definition(data: &[u8], offset: &mut usize) -> MySqlResult<ColumnDefinition> {
    let catalog = parse_length_encoded_string(data, offset)?;
    let schema = parse_length_encoded_string(data, offset)?;
    let table = parse_length_encoded_string(data, offset)?;
    let org_table = parse_length_encoded_string(data, offset)?;
    let name = parse_length_encoded_string(data, offset)?;
    let org_name = parse_length_encoded_string(data, offset)?;

    // fixed-length fields (length of following fields)
    let _length_of_fixed_fields = parse_length_encoded_int(data, offset)?;

    let character_set = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    let column_length = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    let column_type = data[*offset];
    *offset += 1;
    let flags = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    let decimals = data[*offset];
    *offset += 1;
    // filler (2 bytes)
    *offset += 2;

    Ok(ColumnDefinition {
        catalog,
        schema,
        table,
        org_table,
        name,
        org_name,
        character_set,
        column_length,
        column_type,
        flags,
        decimals,
    })
}

/// Parse a text-protocol row (length-encoded strings for each column).
fn parse_text_row(data: &[u8], offset: &mut usize, num_columns: usize) -> MySqlResult<Vec<String>> {
    let mut row = Vec::with_capacity(num_columns);
    for _ in 0..num_columns {
        if *offset >= data.len() {
            return Err(MySqlClientError::Protocol("Row data truncated".to_string()));
        }
        // Check for NULL (0xfb)
        if data[*offset] == 0xfb {
            row.push("NULL".to_string());
            *offset += 1;
        } else {
            let val = parse_length_encoded_string(data, offset)?;
            row.push(val);
        }
    }
    Ok(row)
}

/// Parse a result set from the stream after sending COM_QUERY.
pub fn parse_result_set(stream: &mut dyn Read, deprecate_eof: bool) -> MySqlResult<ResultSet> {
    let pkt = Packet::read_from(stream)?;

    // Check for error packet (first byte 0xff)
    if !pkt.payload.is_empty() && pkt.payload[0] == 0xff {
        let error_code = u16::from_le_bytes([pkt.payload[1], pkt.payload[2]]);
        let sql_state = if pkt.payload.len() > 5 {
            String::from_utf8_lossy(&pkt.payload[3..8]).to_string()
        } else {
            String::new()
        };
        let msg_start = if pkt.payload.len() > 8 { 8 } else { 3 };
        let error_message = if msg_start < pkt.payload.len() {
            String::from_utf8_lossy(&pkt.payload[msg_start..]).to_string()
        } else {
            String::new()
        };
        return Ok(ResultSet::Error {
            error_code,
            sql_state,
            error_message,
        });
    }

    // Check for OK packet (first byte 0x00 or 0xfe for OK/EOF)
    // In protocol with DEPRECATE_EOF, OK packet starts with 0x00 or 0xfe
    // when there are no rows (affected_rows response)
    if !pkt.payload.is_empty() && (pkt.payload[0] == 0x00 || pkt.payload[0] == 0xfe) {
        let mut off = 1;
        let affected_rows = parse_length_encoded_int(&pkt.payload, &mut off)?;
        let last_insert_id = parse_length_encoded_int(&pkt.payload, &mut off)?;
        let status_flags = if off + 2 <= pkt.payload.len() {
            u16::from_le_bytes([pkt.payload[off], pkt.payload[off + 1]])
        } else {
            0
        };
        off += 2;
        let warnings = if off + 2 <= pkt.payload.len() {
            u16::from_le_bytes([pkt.payload[off], pkt.payload[off + 1]])
        } else {
            0
        };
        off += 2;

        let info = if off < pkt.payload.len() {
            String::from_utf8_lossy(&pkt.payload[off..]).to_string()
        } else {
            String::new()
        };

        return Ok(ResultSet::Ok {
            affected_rows,
            last_insert_id,
            status_flags,
            warnings,
            info,
        });
    }

    // It's a result set: first packet is column count (length-encoded int)
    let mut off = 0;
    let column_count = parse_length_encoded_int(&pkt.payload, &mut off)? as usize;

    // Parse column definitions (one packet per column)
    let mut columns = Vec::with_capacity(column_count);
    for _ in 0..column_count {
        let col_pkt = Packet::read_from(stream)?;
        let mut col_off = 0;
        let col = parse_column_definition(&col_pkt.payload, &mut col_off)?;
        columns.push(col);
    }

    // Inter-record separator (EOF or OK packet). Discard.
    let _separator = Packet::read_from(stream)?;

    // Parse rows. Termination is:

    //   - Classic (DEPRECATE_EOF=0): EOF packet (0xFE, 5 bytes)
    //   - DEPRECATE_EOF=1: OK packet (0x00, 7 bytes)
    let mut rows = Vec::new();
    loop {
        let row_pkt = Packet::read_from(stream)?;
        let first = row_pkt.payload.first().copied();
        // Empty packet always terminates
        if row_pkt.payload.is_empty() {
            break;
        }
        // Terminated by EOF packet (classic) or OK packet (DEPRECATE_EOF)
        let is_eof = !deprecate_eof
            && first == Some(0xfe)
            && row_pkt.payload.len() < 9;
        let is_deprecate_eof = deprecate_eof
            && first == Some(0x00)
            && row_pkt.payload.len() <= 8;
        if is_eof || is_deprecate_eof {
            break;
        }
        let mut row_off = 0;
        let row = parse_text_row(&row_pkt.payload, &mut row_off, column_count)?;
        rows.push(row);
    }

    Ok(ResultSet::Select { columns, rows })
}

// ============================================================================
// High-level MySQL client
// ============================================================================

/// A MySQL wire-protocol connection.
pub struct MySqlConnection {
    stream: TcpStream,
    seq: u8,
    pub server_version: String,
}

impl MySqlConnection {
    /// Connect to a MySQL-compatible server and perform handshake + auth.
    pub fn connect(
        addr: &SocketAddr,
        user: &str,
        password: &str,
        database: &str,
    ) -> MySqlResult<Self> {
        let stream = TcpStream::connect_timeout(addr, Duration::from_secs(10))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        stream.set_nodelay(true)?;

        let mut conn = MySqlConnection {
            stream,
            seq: 0,
            server_version: String::new(),
        };

        // Read handshake from server
        let hs_pkt = Packet::read_from(&mut conn.stream)?;
        conn.seq = hs_pkt.sequence.wrapping_add(1);

        let handshake = parse_handshake(&hs_pkt.payload)?;
        conn.server_version = handshake.server_version.clone();

        // Compute auth response
        let auth_response = native_password_hash(password, &handshake.auth_plugin_data);

        // Send handshake response
        let resp_pkt = build_handshake_response(
            conn.seq,
            user,
            &auth_response,
            database,
            &handshake.auth_plugin_name,
        );
        conn.seq = resp_pkt.sequence.wrapping_add(1);
        resp_pkt.write_to(&mut conn.stream)?;

        // Read auth result
        let auth_result_pkt = Packet::read_from(&mut conn.stream)?;
        conn.seq = auth_result_pkt.sequence.wrapping_add(1);

        // Check for auth error
        if !auth_result_pkt.payload.is_empty() && auth_result_pkt.payload[0] == 0xff {
            let error_code =
                u16::from_le_bytes([auth_result_pkt.payload[1], auth_result_pkt.payload[2]]);
            let msg = if auth_result_pkt.payload.len() > 3 {
                String::from_utf8_lossy(&auth_result_pkt.payload[3..]).to_string()
            } else {
                String::new()
            };
            return Err(MySqlClientError::Auth(format!(
                "Auth failed ({}): {}",
                error_code, msg
            )));
        }

        Ok(conn)
    }

    /// Execute a SQL query via COM_QUERY and return the result.
    pub fn execute(&mut self, sql: &str) -> MySqlResult<ResultSet> {
        let mut payload = Vec::with_capacity(sql.len() + 1);
        payload.push(packet_type::COM_QUERY);
        payload.extend_from_slice(sql.as_bytes());

        let query_pkt = Packet::new(self.seq, payload);
        self.seq = query_pkt.sequence.wrapping_add(1);
        query_pkt.write_to(&mut self.stream)?;

        let result = parse_result_set(&mut self.stream, true)?;

        // Update seq from the last packet read (handled inside parse_result_set)
        // but we don't track it precisely there. For simplicity, reset seq.
        // In practice, multi-statement requires tracking; for single query OK.

        Ok(result)
    }

    /// Execute a multi-statement query via COM_QUERY.
    /// Each statement separated by `;` is executed independently.
    /// Returns the first parsed result set (limited multi-result support).
    pub fn execute_multi(&mut self, sql: &str) -> MySqlResult<Vec<ResultSet>> {
        // Multi-statement queries are sent as a single COM_QUERY packet.
        // For simplicity, we parse the first result set only.
        let mut payload = Vec::with_capacity(sql.len() + 1);
        payload.push(packet_type::COM_QUERY);
        payload.extend_from_slice(sql.as_bytes());

        let pkt = Packet::new(self.seq, payload);
        self.seq = pkt.sequence.wrapping_add(1);
        pkt.write_to(&mut self.stream)?;

        let first = parse_result_set(&mut self.stream, true)?;
        Ok(vec![first])
    }

     /// Ping the server.
    pub fn ping(&mut self) -> MySqlResult<()> {
        let pkt = Packet::new(self.seq, vec![packet_type::COM_PING]);
        self.seq = pkt.sequence.wrapping_add(1);
        pkt.write_to(&mut self.stream)?;
        let _resp = Packet::read_from(&mut self.stream)?;
        Ok(())
    }
    /// COM_STMT_PREPARE — prepare a statement
    /// Returns: (statement_id, param_count, column_count)
    pub fn prepare(&mut self, sql: &str) -> MySqlResult<PreparedStatement> {
        let mut payload = Vec::with_capacity(sql.len() + 1);
        payload.push(0x16); // COM_STMT_PREPARE
        payload.extend_from_slice(sql.as_bytes());

        let pkt = Packet::new(self.seq, payload);
        self.seq = pkt.sequence.wrapping_add(1);
        pkt.write_to(&mut self.stream)?;

        // Response: 1-byte status (0x00=OK, 0xFF=ERR)
        //           4-byte statement_id
        //           2-byte column_count
        //           2-byte param_count
        //           1-byte filler (0x00)
        //           2-byte warning_count
        let resp = Packet::read_from(&mut self.stream)?;
        self.seq = resp.sequence.wrapping_add(1);

        if resp.payload.is_empty() || resp.payload[0] == 0xff {
            let error_code = if resp.payload.len() > 2 {
                u16::from_le_bytes([resp.payload[1], resp.payload[2]])
            } else {
                0
            };
            let msg_start = 3.min(resp.payload.len());
            let msg = if msg_start < resp.payload.len() {
                String::from_utf8_lossy(&resp.payload[msg_start..]).to_string()
            } else {
                String::new()
            };
            return Err(MySqlClientError::Protocol(format!(
                "PREPARE failed ({}): {}",
                error_code, msg
            )));
        }

        if resp.payload.len() < 12 {
            return Err(MySqlClientError::Protocol(
                "PREPARE response too short".to_string(),
            ));
        }

        let stmt_id = u32::from_le_bytes([
            resp.payload[1],
            resp.payload[2],
            resp.payload[3],
            resp.payload[4],
        ]);
        let column_count = u16::from_le_bytes([resp.payload[5], resp.payload[6]]);
        let param_count = u16::from_le_bytes([resp.payload[7], resp.payload[8]]);

        // If there are parameters, the server sends parameter defs.
        // If there are columns, the server sends column defs.
        // For now, we just drain those packets.
        for _ in 0..(param_count + column_count) {
            let _ = Packet::read_from(&mut self.stream)?;
        }
        // The final packet is an EOF or DEPR_EOF terminator.
        let _ = Packet::read_from(&mut self.stream)?;

        Ok(PreparedStatement {
            id: stmt_id,
            param_count,
            column_count,
        })
    }

    /// COM_STMT_EXECUTE — execute a prepared statement
    /// Uses binary protocol parameters (typed as VARCHAR/text).
    pub fn execute_prepared(
        &mut self,
        stmt_id: u32,
        params: &[&str],
    ) -> MySqlResult<ResultSet> {
        let mut payload = Vec::new();
        payload.push(0x17); // COM_STMT_EXECUTE
        payload.extend_from_slice(&stmt_id.to_le_bytes());
        payload.push(0x00); // flags: CURSOR_TYPE_NONE
        payload.extend_from_slice(&1u32.to_le_bytes()); // iteration_count

        // NULL bitmap: ceil((param_count + 7) / 8) bytes, all zero
        let null_bitmap_len = (params.len() + 7) / 8;
        payload.extend_from_slice(&vec![0u8; null_bitmap_len]);

        // new_params_bound_flag = 1
        payload.push(0x01);

        // Parameter types (VARCHAR for all)
        for _ in 0..params.len() {
            payload.push(0xfd); // MYSQL_TYPE_VAR_STRING
        }

        // Parameter values (length-encoded strings)
        for p in params {
            let len = p.len();
            if len < 251 {
                payload.push(len as u8);
            } else {
                payload.push(0xfc);
                payload.extend_from_slice(&(len as u16).to_le_bytes());
            }
            payload.extend_from_slice(p.as_bytes());
        }

        let pkt = Packet::new(self.seq, payload);
        self.seq = pkt.sequence.wrapping_add(1);
        pkt.write_to(&mut self.stream)?;

        // Response: text or binary result set depending on server
        // We use parse_result_set for text protocol
        parse_result_set(&mut self.stream, true)
    }

    /// Ping the server.
    pub fn close(mut self) -> MySqlResult<()> {
        let pkt = Packet::new(self.seq, vec![packet_type::COM_QUIT]);
        pkt.write_to(&mut self.stream)?;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_password_hash() {
        let password = "tester";
        let scramble = [0u8; 20];
        let hash = native_password_hash(password, &scramble);
        assert_eq!(hash.len(), 20);
        // Verify deterministic
        let hash2 = native_password_hash(password, &scramble);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_parse_handshake() {
        // Minimal valid handshake payload
        let mut payload = Vec::new();
        payload.push(0x0a); // protocol version
        payload.extend_from_slice(b"8.0.33-Test\0"); // server version
        payload.extend_from_slice(&1u32.to_le_bytes()); // connection ID
        payload.extend_from_slice(b"12345678"); // auth-plugin-data-part-1 (8 bytes)
        payload.push(0x00); // filler
        payload.extend_from_slice(&0xffffu16.to_le_bytes()); // capability lower
        payload.push(45); // character set
        payload.extend_from_slice(&0u16.to_le_bytes()); // status flags
        payload.extend_from_slice(&0xffffu16.to_le_bytes()); // capability upper
        payload.push(20); // auth-plugin-data-len
        payload.extend_from_slice(&[0u8; 10]); // reserved
        payload.extend_from_slice(b"901234567890\0"); // auth-plugin-data-part-2 + null term
        payload.extend_from_slice(b"mysql_native_password\0");

        let hs = parse_handshake(&payload).unwrap();
        assert_eq!(hs.protocol_version, 0x0a);
        assert!(hs.server_version.contains("8.0.33"));
        assert_eq!(hs.connection_id, 1);
    }

    #[test]
    fn test_parse_length_encoded_int() {
        let mut off = 0;
        let data = [42u8];
        let val = parse_length_encoded_int(&data, &mut off).unwrap();
        assert_eq!(val, 42);

        let mut off = 0;
        let data = [0xfc, 0x10, 0x00];
        let val = parse_length_encoded_int(&data, &mut off).unwrap();
        assert_eq!(val, 16);
    }

    #[test]
    fn test_build_handshake_response() {
        let pkt =
            build_handshake_response(0, "testuser", &[1, 2, 3], "testdb", "mysql_native_password");
        assert_eq!(pkt.sequence, 0);
        assert!(pkt.payload.len() > 32);
    }
}

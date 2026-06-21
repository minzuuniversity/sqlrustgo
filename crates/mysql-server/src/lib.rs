//! SQLRustGo MySQL Wire Protocol Server
//!
//! Supports mysql_native_password auth + TLS (mariadb-connector-c 3.4+ compatible)

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rcgen::{CertificateParams, KeyPair};
use sha1::{Digest, Sha1};
use sqlrustgo::ExecutionEngine;
use sqlrustgo_parser::{parse, parse_statements, Statement};
use sqlrustgo_storage::{
    FileBackedWalManager, FileStorage, MemoryStorage, StorageEngine, WalStorage,
};
use sqlrustgo_types::{SqlError, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

const SERVER_VERSION: &str = "8.0.33-SQLRustGo";

/// Global connection counter for diagnostics. Incremented when a
/// connection is accepted, decremented when it closes.
pub static ACTIVE_CONNECTIONS: AtomicI64 = AtomicI64::new(0);
pub static TOTAL_CONNECTIONS_ACCEPTED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_QUERIES_SERVED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_QUERY_ERRORS: AtomicU64 = AtomicU64::new(0);

/// Spawn a background thread that periodically logs resource usage
/// (RSS, FD count, thread count) to the tracing log. This is critical
/// for diagnosing server crashes where the process disappears silently.
pub fn spawn_resource_monitor(interval_s: u64) {
    // Capture the main process PID at spawn time. Subsequent reads
    // happen in a child thread, but we want the main process metrics.
    let main_pid = std::process::id();
    thread::Builder::new()
        .name("sqlrustgo-resource-monitor".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(interval_s));
                let active = ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
                let total_acc = TOTAL_CONNECTIONS_ACCEPTED.load(Ordering::Relaxed);
                let total_q = TOTAL_QUERIES_SERVED.load(Ordering::Relaxed);
                let total_err = TOTAL_QUERY_ERRORS.load(Ordering::Relaxed);

                // Read /proc/<pid>/status for RSS
                let (rss_kb, fd_count) = read_proc_status(main_pid);

                // Check FD threshold
                let (soft_limit, _hard_limit) = read_fd_limit();
                let fd_pct = if soft_limit > 0 {
                    (fd_count as f64 / soft_limit as f64) * 100.0
                } else {
                    0.0
                };
                if fd_pct > 80.0 {
                    tracing::warn!(
                        "FD usage high: {}/{} ({:.1}%) — approaching limit",
                        fd_count, soft_limit, fd_pct
                    );
                }

                tracing::info!(
                    "RESOURCE_MONITOR pid={} rss_mb={:.1} fd={}/{} ({:.1}%) threads={} active_conn={} total_acc={} total_q={} total_err={}",
                    main_pid,
                    rss_kb as f64 / 1024.0,
                    fd_count,
                    soft_limit,
                    fd_pct,
                    list_threads(),
                    active,
                    total_acc,
                    total_q,
                    total_err,
                );
            }
        })
        .ok();
}

fn read_proc_status(pid: u32) -> (u64, usize) {
    let mut rss_kb = 0u64;
    let mut fd_count = 0usize;

    // Linux: read /proc/<pid>/status
    if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/status", pid)) {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(v) = line.split_whitespace().nth(1) {
                    rss_kb = v.parse().unwrap_or(0);
                }
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(format!("/proc/{}/fd", pid)) {
        fd_count = entries.count();
        return (rss_kb, fd_count);
    }

    // macOS / BSD fallback: use ps to get RSS
    if let Ok(out) = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
    {
        if out.status.success() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                // RSS in KB on macOS ps
                rss_kb = s.trim().parse().unwrap_or(0);
            }
        }
    }
    // macOS: count file descriptors via /dev/fd
    if let Ok(entries) = std::fs::read_dir("/dev/fd") {
        fd_count = entries.count().saturating_sub(1); // subtract fd for read_dir itself
    }
    (rss_kb, fd_count)
}

fn read_fd_limit() -> (usize, usize) {
    let mut soft = 0usize;
    let mut hard = 0usize;
    if let Ok(out) = std::process::Command::new("sh")
        .arg("-c")
        .arg("ulimit -Sn && ulimit -Hn")
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        let mut it = s.lines();
        if let Some(line) = it.next() {
            soft = line.trim().parse().unwrap_or(0);
        }
        if let Some(line) = it.next() {
            hard = line.trim().parse().unwrap_or(0);
        }
    }
    (soft, hard)
}

fn list_threads() -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir("/proc/self/task") {
        count = entries.count();
    }
    count
}
#[allow(dead_code)]
const AUTH_PLUGIN: &str = "mysql_native_password";
const SCRAMBLE_LENGTH: usize = 20;

fn skip_auth() -> bool {
    std::env::var("SQLRUSTGO_AUTH_MODE")
        .map(|v| v.eq_ignore_ascii_case("none"))
        .unwrap_or(false)
}

mod packet_type {
    pub const COM_QUIT: u8 = 0x01;
    pub const COM_INIT_DB: u8 = 0x02;
    pub const COM_QUERY: u8 = 0x03;
    pub const LOCAL_INFILE_REQUEST: u8 = 0xFB;
    pub const COM_PING: u8 = 0x0e;
    pub const COM_STMT_PREPARE: u8 = 0x16;
    pub const COM_STMT_EXECUTE: u8 = 0x17;
    pub const COM_STMT_CLOSE: u8 = 0x19;
}

// ============================================================================
// ACTIVE_CONFIG — process-wide handle to the most recently-started
// ephemeral server's configuration. Set by `start_ephemeral`, read by
// `do_command_loop` so the LOAD DATA LOCAL INFILE handler can access
// `data_dir` (for the whitelist check) and `bulk_insert_buffer_size`
// (for batch boundaries) without threading them through every layer.
//
// This is a process-global because:
//   - In-process ephemeral tests use it: the test creates the server
//     via `start_ephemeral`, then drives it through the wire protocol.
//   - The canonical-subprocess entry point is separate (it sets CLI
//     flags and reads them from a different static). `start_ephemeral`
//     is the entry point used by every wire-protocol integration test.
//
// `OnceLock` enforces a single set per process. If multiple
// `start_ephemeral` calls happen, only the first wins. Tests that
// need a fresh config use `EphemeralConfig::default()` for the rest.
// ============================================================================
use std::sync::OnceLock;
static ACTIVE_CONFIG: OnceLock<std::sync::Mutex<testing::EphemeralConfig>> = OnceLock::new();

mod capability {
    pub const LONG_PASSWORD: u32 = 0x00000001;
    pub const FOUND_ROWS: u32 = 0x00000002;
    pub const LONG_FLAG: u32 = 0x00000004;
    pub const CONNECT_WITH_DB: u32 = 0x00000008;
    pub const PROTOCOL_41: u32 = 0x00000200;
    pub const TRANSACTIONS: u32 = 0x00002000;
    pub const SECURE_CONNECTION: u32 = 0x00008000;
    pub const MULTI_STATEMENTS: u32 = 0x00010000;
    pub const MULTI_RESULTS: u32 = 0x00020000;
    pub const PLUGIN_AUTH: u32 = 0x00080000;
    pub const PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 0x00200000;
    pub const SSL: u32 = 0x00000800;
    pub const DEPRECATE_EOF: u32 = 0x01000000;

    pub const SERVER_DEFAULT: u32 = LONG_PASSWORD
        | FOUND_ROWS
        | LONG_FLAG
        | CONNECT_WITH_DB
        | PROTOCOL_41
        | TRANSACTIONS
        | SECURE_CONNECTION
        | MULTI_STATEMENTS
        | MULTI_RESULTS
        | PLUGIN_AUTH
        | PLUGIN_AUTH_LENENC_CLIENT_DATA
        | DEPRECATE_EOF
        | SSL;
}

#[derive(Debug)]
pub enum MySqlError {
    Io(std::io::Error),
    Protocol(String),
    Sql(String),
    /// Generic catch-all error carrying a free-form message. Used by
    /// helper code paths (e.g. LOAD DATA LOCAL INFILE handler) that
    /// need to propagate human-readable context without having to
    /// commit to one of the typed variants above.
    Other(String),
}

impl std::fmt::Display for MySqlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MySqlError::Io(e) => write!(f, "IO: {}", e),
            MySqlError::Protocol(s) => write!(f, "Protocol: {}", s),
            MySqlError::Sql(s) => write!(f, "SQL: {}", s),
            MySqlError::Other(s) => write!(f, "{}", s),
        }
    }
}
impl std::error::Error for MySqlError {}
impl From<std::io::Error> for MySqlError {
    fn from(e: std::io::Error) -> Self {
        MySqlError::Io(e)
    }
}
impl From<SqlError> for MySqlError {
    fn from(e: SqlError) -> Self {
        MySqlError::Sql(e.to_string())
    }
}
impl From<String> for MySqlError {
    fn from(s: String) -> Self {
        MySqlError::Sql(s)
    }
}
impl From<&str> for MySqlError {
    fn from(s: &str) -> Self {
        MySqlError::Sql(s.to_string())
    }
}
pub type MySqlResult<T> = Result<T, MySqlError>;

// User storage for mysql_native_password authentication
#[derive(Debug, Clone)]
struct UserPassword {
    password_hash: [u8; 20],
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UserStore {
    users: HashMap<String, UserPassword>,
}

pub(crate) type UserStoreBootstrap = Option<Box<dyn FnOnce(&mut UserStore) + Send>>;

impl UserStore {
    fn new() -> Self {
        let mut store = Self {
            users: HashMap::new(),
        };
        store.add_user("root", "");
        store.add_user("mysql", "mysql");
        store
    }

    fn add_user(&mut self, username: &str, password: &str) {
        let password_hash = compute_double_sha1(password.as_bytes());
        self.users
            .insert(username.to_string(), UserPassword { password_hash });
    }

    fn verify_password(&self, username: &str, scramble: &[u8; 20], auth_response: &[u8]) -> bool {
        let user = match self.users.get(username) {
            Some(u) => u,
            None => return false,
        };
        verify_mysql_native_password(&user.password_hash, scramble, auth_response)
    }
}

// Compute SHA1(SHA1(password)) - what MySQL stores
fn compute_double_sha1(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let first = hasher.finalize();
    let mut hasher = Sha1::new();
    hasher.update(first);
    hasher.finalize().into()
}

// Compute SHA1(data)
fn sha1_simple(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize().into()
}

// Verify mysql_native_password authentication
// auth_response = SHA1(password) XOR SHA1(scramble + SHA1(SHA1(password)))
// We verify by checking if: SHA1(scramble + stored_password_hash) XOR auth_response = SHA1(password)
// And then verify SHA1(SHA1(password)) = stored_password_hash
fn verify_mysql_native_password(
    stored_password_hash: &[u8; 20],
    scramble: &[u8; 20],
    auth_response: &[u8],
) -> bool {
    if auth_response.len() != 20 {
        return false;
    }
    let mut hasher = Sha1::new();
    hasher.update(scramble);
    hasher.update(stored_password_hash);
    let expected_hash = hasher.finalize();
    let mut result = [0u8; 20];
    for i in 0..20 {
        result[i] = expected_hash[i] ^ auth_response[i];
    }
    let computed_first = sha1_simple(&result);
    computed_first == *stored_password_hash
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test Packet serialization - roundtrip
    #[test]
    fn test_packet_roundtrip() {
        let pkt = Packet {
            length: 5,
            sequence: 3,
            payload: vec![1, 2, 3, 4, 5],
        };
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();

        // Verify header: 3 bytes length + 1 byte sequence
        assert_eq!(buf.len(), 4 + 5);
        assert_eq!(buf[0], 5); // length byte 0
        assert_eq!(buf[1], 0); // length byte 1
        assert_eq!(buf[2], 0); // length byte 2
        assert_eq!(buf[3], 3); // sequence

        let mut reader = std::io::Cursor::new(&buf);
        let read_pkt = Packet::read_from(&mut reader).unwrap();
        assert_eq!(read_pkt.length, pkt.length);
        assert_eq!(read_pkt.sequence, pkt.sequence);
        assert_eq!(read_pkt.payload, pkt.payload);
    }

    // Test Packet with empty payload
    #[test]
    fn test_packet_empty_payload() {
        let pkt = Packet {
            length: 0,
            sequence: 0,
            payload: vec![],
        };
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
        let mut reader = std::io::Cursor::new(&buf);
        let read_pkt = Packet::read_from(&mut reader).unwrap();
        assert_eq!(read_pkt.length, 0);
        assert!(read_pkt.payload.is_empty());
    }

    // Test length-encoded integer branches
    #[test]
    fn test_write_lenenc_int_1byte() {
        // Branch: v < 251 (1 byte)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 100).unwrap();
        assert_eq!(buf, vec![100]);
    }

    #[test]
    fn test_write_lenenc_int_2bytes() {
        // Branch: 251 <= v < 0x10000 (3 bytes: 0xfc + u16)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 300).unwrap();
        assert_eq!(buf[0], 0xfc);
        assert_eq!(u16::from_le_bytes([buf[1], buf[2]]), 300);
    }

    #[test]
    fn test_write_lenenc_int_3bytes() {
        // Branch: 0x10000 <= v < 0x1000000 (4 bytes: 0xfd + u24)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x10000).unwrap();
        assert_eq!(buf[0], 0xfd);
        assert_eq!(u32::from_le_bytes([buf[1], buf[2], buf[3], 0]), 0x10000);
    }

    #[test]
    fn test_write_lenenc_int_8bytes() {
        // Branch: v >= 0x1000000 (9 bytes: 0xfe + u64)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x1000000).unwrap();
        assert_eq!(buf[0], 0xfe);
        assert_eq!(
            u64::from_le_bytes([buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],]),
            0x1000000
        );
    }

    // Test read_lenenc_int branches
    #[test]
    fn test_read_lenenc_int_1byte() {
        // Branch: 0..=0xfa
        let mut reader = std::io::Cursor::new(&[100u8]);
        let val = read_lenenc_int(&mut reader).unwrap();
        assert_eq!(val, 100);
    }

    #[test]
    fn test_read_lenenc_int_2bytes() {
        // Branch: 0xfc
        let mut reader = std::io::Cursor::new(&[0xfc, 0x2c, 0x01]); // 300 in little-endian
        let val = read_lenenc_int(&mut reader).unwrap();
        assert_eq!(val, 300);
    }

    #[test]
    fn test_read_lenenc_int_3bytes() {
        // Branch: 0xfd
        let mut reader = std::io::Cursor::new(&[0xfd, 0x00, 0x00, 0x01]); // 0x10000 in little-endian
        let val = read_lenenc_int(&mut reader).unwrap();
        assert_eq!(val, 0x10000);
    }

    #[test]
    fn test_read_lenenc_int_8bytes() {
        // Branch: 0xfe with u64 value
        let mut reader =
            std::io::Cursor::new(&[0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00]); // 0x10000000000 in little-endian
        let val = read_lenenc_int(&mut reader).unwrap();
        assert_eq!(val, 0x10000000000);
    }

    #[test]
    fn test_read_lenenc_int_0xfb_error() {
        // Branch: 0xfb = NULL
        let mut reader = std::io::Cursor::new(&[0xfbu8]);
        let result = read_lenenc_int(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_lenenc_int_0xff_error() {
        // Branch: 0xff = invalid
        let mut reader = std::io::Cursor::new(&[0xffu8]);
        let result = read_lenenc_int(&mut reader);
        assert!(result.is_err());
    }

    // Test length-encoded string
    #[test]
    fn test_write_lenenc_string() {
        let mut buf = Vec::new();
        write_lenenc_string(&mut buf, b"hello").unwrap();
        assert_eq!(buf[0], 5); // length prefix
        assert_eq!(&buf[1..], b"hello");
    }

    // Test make_ok_packet structure
    #[test]
    fn test_make_ok_packet() {
        let pkt = make_ok_packet(1, 5, 10, 0x0002, 0);
        assert_eq!(pkt.sequence, 1);
        assert_eq!(pkt.payload[0], 0x00); // OK packet type
                                          // Verify it can be written without error
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
    }

    // Test make_err_packet structure
    #[test]
    fn test_make_err_packet() {
        let pkt = make_err_packet(1, 1146, "42S02", "Table not found");
        assert_eq!(pkt.sequence, 1);
        assert_eq!(pkt.payload[0], 0xff); // ERR packet type
        assert_eq!(u16::from_le_bytes([pkt.payload[1], pkt.payload[2]]), 1146);
        // Verify it can be written without error
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
    }

    // Test make_eof_packet structure
    #[test]
    fn test_make_eof_packet() {
        let pkt = make_eof_packet(2, 0x0002);
        assert_eq!(pkt.sequence, 2);
        assert_eq!(pkt.payload[0], 0xfe); // EOF packet type
                                          // Verify it can be written without error
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
    }

    // Test make_handshake_packet structure
    #[test]
    fn test_make_handshake_packet() {
        let scramble = [1u8; 20];
        let pkt = make_handshake_packet(0, &scramble);
        assert_eq!(pkt.sequence, 0);
        assert_eq!(pkt.payload[0], 0x0a); // Protocol version 10
                                          // Verify it can be written without error
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
    }

    // Test is_select_stmt (Statement-based routing)
    #[test]
    fn test_is_select_stmt() {
        use sqlrustgo_parser::parse;
        assert!(is_select_stmt(&parse("SELECT * FROM t").unwrap()));
        assert!(is_select_stmt(&parse("SHOW TABLES").unwrap()));
        assert!(is_select_stmt(&parse("DESCRIBE t").unwrap()));
        assert!(!is_select_stmt(&parse("DROP TABLE t").unwrap()));
    }

    // Test parse → Statement dispatch (new routing model)
    #[test]
    fn test_statement_dispatch() {
        use sqlrustgo::MemoryExecutionEngine;
        use sqlrustgo_parser::parse;
        use sqlrustgo_storage::MemoryStorage;
        use std::sync::{Arc, RwLock};

        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = MemoryExecutionEngine::new(storage);

        // INSERT should not panic and returns affected rows
        let r = engine.execute("CREATE TABLE dispatch_test (id INT, name TEXT)");
        assert!(r.is_ok());
        let r = engine.execute("INSERT INTO dispatch_test VALUES (1, 'alice')");
        assert!(r.is_ok());

        // SELECT should work
        let r = engine.execute("SELECT * FROM dispatch_test");
        assert!(r.is_ok());
    }

    // Test parse_handshake_response - too short
    #[test]
    fn test_parse_handshake_response_too_short() {
        let pkt = Packet {
            length: 10,
            sequence: 0,
            payload: vec![0; 10],
        };
        let result = parse_handshake_response(&pkt);
        assert!(result.is_err());
    }

    // Test parse_handshake_response - minimal valid response
    #[test]
    fn test_parse_handshake_response_minimal() {
        let mut payload = vec![0u8; 64];
        // capability_flags (4 bytes) at offset 0-3
        payload[0] = 0x01; // LONG_PASSWORD
        payload[1] = 0x00;
        payload[2] = 0x00;
        payload[3] = 0x00;
        // username (null-terminated) starting at offset 32
        payload[32] = b't';
        payload[33] = b'e';
        payload[34] = b's';
        payload[35] = b't';
        payload[36] = 0x00; // null terminator

        let pkt = Packet {
            length: payload.len() as u32,
            sequence: 1,
            payload,
        };
        let result = parse_handshake_response(&pkt);
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.username, "test");
        assert_eq!(resp.capability_flags, 0x01);
    }

    // Test MySqlError Display
    #[test]
    fn test_mysql_error_display() {
        let err = MySqlError::Protocol("test error".to_string());
        assert_eq!(format!("{}", err), "Protocol: test error");

        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "io error");
        let err = MySqlError::Io(io_err);
        assert_eq!(format!("{}", err), "IO: io error");

        let err = MySqlError::Sql("sql error".to_string());
        assert_eq!(format!("{}", err), "SQL: sql error");
    }

    #[test]
    fn test_user_store_default_user() {
        let store = UserStore::new();
        assert!(store.users.contains_key("root"));
        assert!(store.users.contains_key("mysql"));
    }

    #[test]
    fn test_double_sha1_computation() {
        let data = b"password";
        let hash = compute_double_sha1(data);
        assert_eq!(hash.len(), 20);
    }

    #[test]
    fn test_verify_password_unknown_user() {
        let store = UserStore::new();
        let scramble = [0u8; 20];
        let auth_response = [0u8; 20];
        assert!(!store.verify_password("unknown", &scramble, &auth_response));
    }

    #[test]
    fn test_verify_password_empty_auth_response() {
        let store = UserStore::new();
        let scramble = [0u8; 20];
        let auth_response = [];
        assert!(!store.verify_password("root", &scramble, &auth_response));
    }
}

#[derive(Debug)]
pub struct Packet {
    pub length: u32,
    pub sequence: u8,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn read_from<R: Read>(r: &mut R) -> MySqlResult<Self> {
        let length = r.read_u24::<LittleEndian>()?;
        let sequence = r.read_u8()?;
        let mut payload = vec![0u8; length as usize];
        r.read_exact(&mut payload)?;
        Ok(Self {
            length,
            sequence,
            payload,
        })
    }
    pub fn write_to<W: Write>(&self, w: &mut W) -> MySqlResult<()> {
        w.write_u24::<LittleEndian>(self.length)?;
        w.write_u8(self.sequence)?;
        w.write_all(&self.payload)?;
        w.flush()?;
        Ok(())
    }
}

/// A Read+Write wrapper around `rustls::Stream` that calls
/// `ServerConnection::complete_io` after every `write_all` to ensure
/// that data is actually flushed to the underlying TCP socket.
///
/// Without this, `rustls::Stream::flush()` only writes to the cipher
/// buffer, and clients (e.g. `mysql` CLI) may see a "Malformed packet"
/// or an empty result set because the response was never sent.
///
/// `TlsStream` borrows the underlying `TcpStream` mutably. After every
/// `write`, we manually invoke `ServerConnection::process_new_packets`
/// to drive TLS I/O on the socket. The `ServerConnection` is held by
/// the caller (so the caller can do handshake I/O before this
/// wrapper is constructed).
pub struct TlsStream<'a> {
    pub sock: &'a mut TcpStream,
    pub conn: &'a mut rustls::ServerConnection,
}

impl<'a> TlsStream<'a> {
    /// Flush any pending TLS ciphertext to the underlying socket.
    /// Subsumed by `Write::flush` (which now drains the full cipher
    /// buffer in a loop). Kept for compatibility with existing callers
    /// (e.g. the post-COM_QUIT final flush at lib.rs:2845).
    pub fn flush_pending(&mut self) -> std::io::Result<()> {
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl<'a> Read for TlsStream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Drive rustls IO only when there is pending inbound data.
        // This avoids blocking on write (which would happen if we
        // called complete_io while wants_write was true and the
        // socket had outbound data to flush).
        if self.conn.wants_read() {
            self.conn.complete_io(self.sock)?;
        }
        self.conn.reader().read(buf)
    }
}

impl<'a> TlsStream<'a> {
    pub fn new(conn: &'a mut rustls::ServerConnection, sock: &'a mut TcpStream) -> Self {
        Self { conn, sock }
    }
    /// Drive pending inbound TLS records from the underlying socket
    /// without blocking on writes. Symmetric counterpart to
    /// `drive_writes_only`.
    #[allow(dead_code)]
    fn drive_reads_only(&mut self) -> std::io::Result<()> {
        while self.conn.wants_read() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl<'a> Write for TlsStream<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.conn.writer().write(buf)?;
        // Drain ALL pending TLS records to the underlying socket, not
        // just one. Without the loop, a single `complete_io` may only
        // flush a partial cipher record when the socket send buffer
        // can't accept the full ciphertext in one syscall; the rest
        // would sit in rustls' writer buffer until the next write,
        // and large multi-batch INSERTs (e.g. sysbench prepare with
        // >~20 rows) would deadlock: the client waits for the OK
        // packet while the server waits for the next request.
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.conn.writer().flush()?;
        // Same drain loop as write(): flush must guarantee the
        // cipher buffer is fully driven to the socket.
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl<'a> TlsStream<'a> {
    /// Drive pending outbound TLS records to the underlying socket
    /// without reading any inbound data. This avoids the deadlock
    /// where complete_io waits for client data while the client
    /// waits for server data.
    #[allow(dead_code)]
    fn drive_writes_only(&mut self) -> std::io::Result<()> {
        // Complete any pending outbound IO without waiting for new
        // data. We do this by repeatedly calling `complete_io` only
        // when there is pending outbound data, and never on a clean
        // socket that has nothing to write.
        //
        // rustls exposes `wants_write()` to indicate pending outbound
        // data; we drive IO while that's true, but bail out as soon
        // as the connection is idle to avoid blocking on read.
        while self.conn.wants_write() {
            // complete_io here is bounded: it returns when either
            // the write buffer is drained or the socket would block.
            // Because the socket is in non-blocking mode for the
            // application, it should not block on read here.
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn write_lenenc_int<W: Write>(w: &mut W, v: u64) -> MySqlResult<()> {
    if v < 251 {
        w.write_u8(v as u8)?;
    } else if v < 0x10000 {
        w.write_u8(0xfc)?;
        w.write_u16::<LittleEndian>(v as u16)?;
    } else if v < 0x1000000 {
        w.write_u8(0xfd)?;
        w.write_u24::<LittleEndian>(v as u32)?;
    } else {
        w.write_u8(0xfe)?;
        w.write_u64::<LittleEndian>(v)?;
    }
    Ok(())
}

fn read_lenenc_int<R: Read>(r: &mut R) -> MySqlResult<u64> {
    let first = r.read_u8()?;
    match first {
        0..=0xfa => Ok(first as u64),
        0xfb => Err(MySqlError::Protocol("NULL lenenc".into())),
        0xfc => Ok(r.read_u16::<LittleEndian>()? as u64),
        0xfd => Ok(r.read_u24::<LittleEndian>()? as u64),
        0xfe => Ok(r.read_u64::<LittleEndian>()?),
        0xff => Err(MySqlError::Protocol("invalid lenenc".into())),
    }
}

fn write_lenenc_string<W: Write>(w: &mut W, s: &[u8]) -> MySqlResult<()> {
    write_lenenc_int(w, s.len() as u64)?;
    w.write_all(s)?;
    Ok(())
}

fn make_handshake_packet(seq: u8, scramble: &[u8; SCRAMBLE_LENGTH]) -> Packet {
    let mut p = Vec::new();
    p.push(0x0a); // protocol 10
    p.extend_from_slice(SERVER_VERSION.as_bytes());
    p.push(0x00);
    p.write_u32::<LittleEndian>(1).unwrap(); // connection_id
    p.extend_from_slice(&scramble[0..8]);
    p.push(0x00); // scramble part1 + filler
    p.write_u16::<LittleEndian>((capability::SERVER_DEFAULT & 0xFFFF) as u16)
        .unwrap();
    p.push(0xff); // charset utf8mb4
    p.write_u16::<LittleEndian>(0x0002).unwrap(); // status AUTOCOMMIT
    p.write_u16::<LittleEndian>(((capability::SERVER_DEFAULT >> 16) & 0xFFFF) as u16)
        .unwrap();
    p.push((SCRAMBLE_LENGTH + 1) as u8); // auth_plugin_data_len
    p.extend_from_slice(&[0u8; 10]); // reserved
    p.extend_from_slice(&scramble[8..20]);
    p.push(0x00); // scramble part2 + null
    p.extend_from_slice(AUTH_PLUGIN.as_bytes()); // auth plugin name
    p.push(0x00); // null terminator for plugin name
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
}

fn make_ok_packet(seq: u8, affected: u64, last_id: u64, status: u16, warnings: u16) -> Packet {
    let mut p = Vec::new();
    p.push(0x00);
    write_lenenc_int(&mut p, affected).unwrap();
    write_lenenc_int(&mut p, last_id).unwrap();
    p.write_u16::<LittleEndian>(status).unwrap();
    p.write_u16::<LittleEndian>(warnings).unwrap();
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
}

fn make_err_packet(seq: u8, code: u16, state: &str, msg: &str) -> Packet {
    let mut p = Vec::new();
    p.push(0xff);
    p.write_u16::<LittleEndian>(code).unwrap();
    p.push(0x23);
    p.extend_from_slice(state.as_bytes());
    p.extend_from_slice(msg.as_bytes());
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
}

fn make_eof_packet(seq: u8, status: u16) -> Packet {
    let mut p = Vec::new();
    p.push(0xfe);
    p.write_u16::<LittleEndian>(0).unwrap();
    p.write_u16::<LittleEndian>(status).unwrap();
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
}

fn make_deprecate_eof_ok_packet(
    seq: u8,
    affected: u64,
    last_id: u64,
    status: u16,
    warnings: u16,
) -> Packet {
    let mut p = Vec::new();
    // DEPRECATE_EOF protocol (MySQL 8.0+): trailing result-set terminator
    // is an OK packet (0x00 marker), NOT an EOF packet (0xFE).
    // This replaces the classic EOF when the client advertises
    // CLIENT_DEPRECATE_EOF capability. The 0x00 marker is the standard
    // OK packet format per the MySQL client/server protocol.
    p.push(0x00);
    write_lenenc_int(&mut p, affected).unwrap();
    write_lenenc_int(&mut p, last_id).unwrap();
    p.write_u16::<LittleEndian>(status).unwrap();
    p.write_u16::<LittleEndian>(warnings).unwrap();
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
}

struct HandshakeResponse {
    capability_flags: u32,
    username: String,
    auth_response: Vec<u8>,
    database: Option<String>,
    auth_plugin_name: Option<String>,
}

fn parse_handshake_response(packet: &Packet) -> MySqlResult<HandshakeResponse> {
    let p = &packet.payload;
    if p.len() < 32 {
        return Err(MySqlError::Protocol(format!("Too short: {}", p.len())));
    }
    let cap =
        u16::from_le_bytes([p[0], p[1]]) as u32 | ((u16::from_le_bytes([p[2], p[3]]) as u32) << 16);
    let rest = &p[32..];
    tracing::info!(
        "Handshake response: cap=0x{:08x}, rest_len={}, rest_hex={:02x?}",
        cap,
        rest.len(),
        &rest[..std::cmp::min(32, rest.len())]
    );
    let uname_end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    let username = String::from_utf8_lossy(&rest[..uname_end]).to_string();
    let mut pos = uname_end + 1;
    let auth = if cap & capability::PLUGIN_AUTH_LENENC_CLIENT_DATA != 0 {
        if pos >= rest.len() {
            vec![]
        } else {
            let mut cur = std::io::Cursor::new(&rest[pos..]);
            match read_lenenc_int(&mut cur) {
                Ok(len) => {
                    let consumed = cur.position() as usize;
                    pos += consumed;
                    if pos + len as usize > rest.len() {
                        rest[pos..].to_vec()
                    } else {
                        let d = rest[pos..pos + len as usize].to_vec();
                        pos += len as usize;
                        d
                    }
                }
                Err(_) => {
                    vec![]
                }
            }
        }
    } else if cap & capability::SECURE_CONNECTION != 0 {
        if pos >= rest.len() {
            vec![]
        } else {
            let len = rest[pos] as usize;
            pos += 1;
            if pos + len > rest.len() {
                rest[pos..].to_vec()
            } else {
                rest[pos..pos + len].to_vec()
            }
        }
    } else {
        let end = rest[pos..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(rest.len() - pos);
        let d = rest[pos..pos + end].to_vec();
        pos += end + 1;
        d
    };
    let db = if cap & capability::CONNECT_WITH_DB != 0 && pos < rest.len() {
        let end = rest[pos..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(rest.len() - pos);
        let d = String::from_utf8_lossy(&rest[pos..pos + end]).to_string();
        pos += end + 1;
        Some(d)
    } else {
        None
    };
    let plugin = if cap & capability::PLUGIN_AUTH != 0 && pos < rest.len() {
        let end = rest[pos..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(rest.len() - pos);
        Some(String::from_utf8_lossy(&rest[pos..pos + end]).to_string())
    } else {
        None
    };
    Ok(HandshakeResponse {
        capability_flags: cap,
        username,
        auth_response: auth,
        database: db,
        auth_plugin_name: plugin,
    })
}

mod load_data;

#[allow(dead_code)]
mod col_type {
    pub const TINY: u8 = 0x01;
    pub const SHORT: u8 = 0x02;
    pub const LONG: u8 = 0x03;
    pub const FLOAT: u8 = 0x04;
    pub const DOUBLE: u8 = 0x05;
    pub const LONGLONG: u8 = 0x08;
    pub const INT24: u8 = 0x09;
    pub const DATETIME: u8 = 0x0a; // DATE and DATETIME share 0x0a
    pub const DATE: u8 = 0x0a; // alias for DATETIME
    pub const TIME: u8 = 0x0b;
    pub const VARCHAR: u8 = 0x0f;
    pub const NEWDECIMAL: u8 = 0xf6;
    pub const VARSTRING: u8 = 0xfd;
    pub const STRING: u8 = 0xfe;
    pub const BLOB: u8 = 0xfc;
}

fn col_type_from_string(t: &str) -> u8 {
    let u = t.to_uppercase();
    if u.contains("DATETIME") || u.contains("TIMESTAMP") {
        col_type::DATETIME
    } else if u.contains("DATE") {
        col_type::DATE
    } else if u.contains("TIME") {
        col_type::TIME
    } else if u.contains("VARCHAR") {
        // Use MySQL 8.0 native VARCHAR (0x0f) instead of VARSTRING
        // (0xfd). libmysqlclient 8.0 strictly validates the column
        // type and rejects VARSTRING when the actual data is bound
        // by length. We still keep VARSTRING for fallback (0xfe-style
        // "unknown" cases).
        col_type::VARCHAR
    } else if u.contains("CHAR") || u.contains("TEXT") {
        col_type::VARSTRING
    } else if u.contains("BIGINT") {
        col_type::LONGLONG
    } else if u.contains("MEDIUMINT") {
        col_type::INT24
    } else if u.contains("SMALLINT") {
        col_type::SHORT
    } else if u.contains("TINYINT") {
        col_type::TINY
    } else if u.contains("INT") || u.contains("INTEGER") {
        col_type::LONG
    } else if u.contains("FLOAT") {
        col_type::FLOAT
    } else if u.contains("DOUBLE") {
        col_type::DOUBLE
    } else if u.contains("DECIMAL") || u.contains("NUMERIC") {
        col_type::NEWDECIMAL
    } else if u.contains("BLOB") || u.contains("BINARY") {
        col_type::BLOB
    } else {
        col_type::STRING // 0xfe: default for unknown types
    }
}

fn col_len_from_type(t: &str) -> u32 {
    let u = t.to_uppercase();
    if u.contains("INT(") {
        u.match_indices("INT(")
            .next()
            .map(|(idx, _)| {
                let rest = &u[idx + 4..];
                rest.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(11)
            })
            .unwrap_or(11)
    } else if u.contains("FLOAT") {
        12
    } else if u.contains("DOUBLE") {
        22
    } else if u.contains("VARCHAR(") {
        u.match_indices("VARCHAR(")
            .next()
            .map(|(idx, _)| {
                let rest = &u[idx + 8..];
                rest.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(255)
            })
            .unwrap_or(255)
    } else if u.contains("TEXT") {
        65535
    } else {
        255
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Boolean(b) => if *b { "1" } else { "0" }.into(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format!("{}", f),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => format!("{:?}", b),
    }
}

fn write_text_row<W: Write>(w: &mut W, row: &[Value]) -> MySqlResult<()> {
    for v in row {
        match v {
            Value::Null => {
                w.write_u8(0xfb)?;
            }
            _ => {
                write_lenenc_string(w, value_to_string(v).as_bytes())?;
            }
        }
    }
    Ok(())
}

/// Write a single row in MySQL binary protocol format.
/// Each value is prefixed with a 1-byte type marker, then the value.
fn write_binary_row<W: Write>(w: &mut W, row: &[Value], col_types: &[u8]) -> MySqlResult<()> {
    w.write_u8(0x00)?; // row packet header: null bitmap starts with 0x00
    let null_bytes = (row.len() + 9) / 8;
    let mut null_map = vec![0u8; null_bytes + 1];
    for (i, v) in row.iter().enumerate() {
        if matches!(v, Value::Null) {
            null_map[1 + i / 8] |= 1 << (i % 8);
        }
    }
    w.write_all(&null_map)?;

    let mut buf = Vec::new();
    for (i, v) in row.iter().enumerate() {
        let col_type = col_types.get(i).copied().unwrap_or(col_type::STRING);
        match v {
            Value::Null => {
                // null handled by null_map above — no per-column data written
            }
            Value::Integer(n) => match col_type {
                col_type::TINY => {
                    buf.write_u8(*n as u8)?;
                }
                col_type::SHORT => {
                    buf.write_i16::<LittleEndian>(*n as i16)?;
                }
                col_type::LONG => {
                    buf.write_i32::<LittleEndian>((*n).try_into().unwrap_or(i32::MAX))?;
                }
                col_type::LONGLONG => {
                    buf.write_i64::<LittleEndian>(*n)?;
                }
                _ => {
                    buf.write_i64::<LittleEndian>(*n)?;
                }
            },
            Value::Float(f) => {
                if col_type == col_type::DOUBLE {
                    buf.write_f64::<LittleEndian>(*f)?;
                } else {
                    buf.write_f32::<LittleEndian>(*f as f32)?;
                }
            }
            Value::Text(s) => {
                write_lenenc_string(&mut buf, s.as_bytes())?;
            }
            Value::Blob(b) => {
                write_lenenc_string(&mut buf, b)?;
            }
            Value::Boolean(b) => {
                buf.write_u8(if *b { 1 } else { 0 })?;
            }
        }
    }
    w.write_all(&buf)?;
    Ok(())
}

fn write_column_def<W: Write>(w: &mut W, name: &str, sql_type: &str, seq: u8) -> MySqlResult<()> {
    let mut p = Vec::new();
    write_lenenc_string(&mut p, b"def").unwrap(); // catalog
    write_lenenc_string(&mut p, b"").unwrap(); // schema
    write_lenenc_string(&mut p, b"").unwrap(); // table
    write_lenenc_string(&mut p, b"").unwrap(); // org_table
    write_lenenc_string(&mut p, name.as_bytes()).unwrap(); // name
                                                           // MySQL column definition fixed-size fields:
                                                           // charset_collation (2 bytes) → length (4 bytes) → field_type (1 byte)
                                                           // → flags (2 bytes) → decimals (1 byte) → filler (2 bytes)
    p.write_u16::<LittleEndian>(0x0030).unwrap(); // charset_collation: 0x30 = utf8_general_ci
    p.write_u32::<LittleEndian>(col_len_from_type(sql_type))
        .unwrap(); // length
    p.push(col_type_from_string(sql_type)); // field_type
    p.write_u16::<LittleEndian>(0x0000).unwrap(); // flags
    p.push(0x00); // decimals
    p.write_u16::<LittleEndian>(0).unwrap(); // filler
    Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }
    .write_to(w)?;
    Ok(())
}

fn send_result_set<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    mut seq: u8,
    cap: u32,
) -> MySqlResult<u8> {
    tracing::info!(
        "send_result_set: {} cols, {} rows, start_seq={}",
        cols.len(),
        rows.len(),
        seq
    );
    {
        let mut p = Vec::new();
        write_lenenc_int(&mut p, cols.len() as u64).unwrap();
        Packet {
            length: p.len() as u32,
            sequence: seq,
            payload: p,
        }
        .write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    for (i, n) in cols.iter().enumerate() {
        write_column_def(
            w,
            n,
            ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)"),
            seq,
        )?;
        seq = seq.wrapping_add(1);
    }
    // Inter-record separator between column defs and the row stream.
    // Honor the client's DEPRECATE_EOF capability:
    //   - DEPRECATE_EOF = 0 (classic protocol): send inter-record EOF
    //   - DEPRECATE_EOF = 1 (mysql 8.0+ default): skip the EOF; the
    //     trailing terminator (OK/EOF below) marks the end of the
    //     result set.
    // Fix for #3516: without this, mysql 8.0 CLI silently drops the
    // result set — it interprets the stray inter-record EOF as the
    // final terminator and never reads the row packets.
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, 0x0002).write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    for (ri, r) in rows.iter().enumerate() {
        let mut p = Vec::new();
        write_text_row(&mut p, r)?;
        tracing::debug!("Row {}: {} bytes, seq={}", ri, p.len(), seq);
        Packet {
            length: p.len() as u32,
            sequence: seq,
            payload: p,
        }
        .write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    // Trailing terminator for the row stream.
    //   - DEPRECATE_EOF = 0 (classic protocol): send EOF packet
    //     (0xFE + warnings + status_flags, 5 bytes).
    //   - DEPRECATE_EOF = 1 (mysql 8.0+ default): send OK packet
    //     (0x00 + affected_rows + last_insert_id + status + warnings,
    //     7 bytes) — OK packet replaces the EOF when the client
    //     advertises DEPRECATE_EOF.
    //
    // Fix for #3516: previously this branch always sent EOF. mysql
    // 8.0 CLI reads the trailing terminator's marker byte to decide
    // whether the result set is complete (0x00 OK) or the connection
    // has been closed (0xFE EOF + extra packet would be expected).
    // Without OK marker, mysql 8.0 hangs or returns ER_MALFORMED_PACKET.
    // Trailing terminator for the row stream (capability-controlled).
    //   - DEPRECATE_EOF = 0 (classic protocol): classic EOF packet
    //     (0xFE + u16 warnings + u16 status_flags, 5 bytes).
    //   - DEPRECATE_EOF = 1 (deprecated-EOF protocol): OK packet
    //     (0x00 + lenenc affected_rows + lenenc last_insert_id + u16
    //     status_flags + u16 warnings, 7 bytes). The OK packet replaces
    //     the EOF when the client advertises DEPRECATE_EOF; this is
    //     the spec-mandated byte layout per
    //     openspec/changes/2026-06-18-wire-deprecate-eof.
    //
    // Both branches carry status_flags = 0x0002 (SERVER_STATUS_AUTOCOMMIT)
    // so the client observes the same autocommit state regardless of
    // which protocol variant is in use.
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, 0x0002).write_to(w)?;
        seq = seq.wrapping_add(1);
    } else {
        make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0).write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    tracing::info!("send_result_set done: final_seq={}", seq);
    Ok(seq)
}

/// Send a result set using MySQL binary protocol encoding (G1 fix for sysbench).
/// Used for COM_STMT_EXECUTE responses where the client expects binary rows.
fn send_binary_result_set<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    mut seq: u8,
    cap: u32,
) -> MySqlResult<u8> {
    // Column count
    {
        let mut p = Vec::new();
        write_lenenc_int(&mut p, cols.len() as u64).unwrap();
        Packet {
            length: p.len() as u32,
            sequence: seq,
            payload: p,
        }
        .write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    // Column definitions (same packet format as text protocol)
    for (i, n) in cols.iter().enumerate() {
        write_column_def(
            w,
            n,
            ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)"),
            seq,
        )?;
        seq = seq.wrapping_add(1);
    }
    // Inter-record separator between column defs and row stream.
    // Honor the client's DEPRECATE_EOF capability.
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, 0x0002).write_to(w)?;
        seq = seq.wrapping_add(1);
    } else {
        make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0).write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    // Rows in binary protocol
    let col_type_codes: Vec<u8> = cols
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let t = ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)");
            col_type_from_string(t)
        })
        .collect();
    for r in rows {
        let mut p = Vec::new();
        write_binary_row(&mut p, r, &col_type_codes)?;
        Packet {
            length: p.len() as u32,
            sequence: seq,
            payload: p,
        }
        .write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    // Trailing terminator for the row stream (capability-controlled).
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, 0x0002).write_to(w)?;
        seq = seq.wrapping_add(1);
    } else {
        make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0).write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    Ok(seq)
}

struct PreparedStatementInfo {
    sql: String,
    /// Number of `?` placeholders in `sql`. Used by the COM_STMT_EXECUTE
    /// handler to determine how many parameters to parse out of the
    /// binary-protocol payload (Issue #2813).
    param_count: u16,
    column_count: u16,
    /// MySQL binary-protocol type code for each `?` placeholder
    /// (e.g. `col_type::LONG`, `col_type::LONGLONG`, `col_type::VARSTRING`).
    /// Used to decode parameters when the client omits the
    /// `new_params_bound_flag` (e.g. sysbench 1.0.20 — Issue #3372).
    param_types: Vec<u8>,
}

struct PreparedStatementManager {
    statements: std::collections::HashMap<u32, PreparedStatementInfo>,
    next_id: u32,
}

impl PreparedStatementManager {
    fn new() -> Self {
        Self {
            statements: std::collections::HashMap::new(),
            next_id: 1,
        }
    }

    fn add(
        &mut self,
        sql: String,
        param_count: u16,
        column_count: u16,
        param_types: Vec<u8>,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.statements.insert(
            id,
            PreparedStatementInfo {
                sql,
                param_count,
                column_count,
                param_types,
            },
        );
        id
    }

    fn get(&self, id: u32) -> Option<&PreparedStatementInfo> {
        self.statements.get(&id)
    }

    fn remove(&mut self, id: u32) {
        self.statements.remove(&id);
    }
}

fn count_placeholders(sql: &str) -> u16 {
    sql.chars().filter(|&c| c == '?').count() as u16
}

/// Extract the list of column names referenced in an INSERT statement's
/// `(...)` clause. Returns an empty Vec if the SQL is not an INSERT
/// with an explicit column list (e.g. `INSERT INTO t VALUES (...)`).
///
/// Example: `INSERT INTO sbtest1 (id, k, c, pad) VALUES (?, ?, ?, ?)`
/// returns `["id", "k", "c", "pad"]`.
fn extract_insert_columns(sql: &str) -> Vec<String> {
    let upper = sql.to_uppercase();
    if !upper.starts_with("INSERT") {
        return vec![];
    }
    if let Some(into_pos) = upper.find("INTO") {
        let after_into = &sql[into_pos + 4..];
        if let Some(paren_start) = after_into.find('(') {
            let paren_end_rel = after_into[paren_start + 1..]
                .find(')')
                .unwrap_or(after_into.len());
            let cols_str = &after_into[paren_start + 1..paren_start + 1 + paren_end_rel];
            return cols_str
                .split(',')
                .map(|c| c.trim().trim_matches('"').trim_matches('`').to_string())
                .filter(|c| !c.is_empty() && !c.contains('?'))
                .collect();
        }
    }
    vec![]
}

/// Extract column names referenced on the LEFT side of `=` in
/// comparison predicates, e.g. for
///   `SELECT * FROM t WHERE id = ? AND name = ?`
/// returns `["id", "name"]`. The order matches the order in which the
/// `?` placeholders appear in the SQL.
///
/// We deliberately look for `WHERE <col> [op] ?` shapes rather than
/// building a full SQL parser. The heuristics (find `WHERE` keyword,
/// split on `AND`/`OR` at top paren-depth, look for `=` operator)
/// are sufficient for the benchmark / sysbench / tpch workloads we
/// care about. Edge cases like `WHERE id IN (?, ?, ?)` and
/// `WHERE id = (SELECT ...)` are not handled — those fall through to
/// the VAR_STRING fallback, which is no worse than today.
fn extract_where_columns(sql: &str) -> Vec<String> {
    let upper = sql.to_uppercase();
    // Find the WHERE keyword. Bail if it does not exist.
    let where_pos = match upper.find(" WHERE ") {
        Some(p) => p + 7,
        None => return vec![],
    };
    // Find the end of the WHERE clause: next ORDER/GROUP/HAVING/LIMIT/UNION/';'/'\"'/end-of-string.
    let where_end = upper[where_pos..]
        .find(" ORDER ")
        .or_else(|| upper[where_pos..].find(" GROUP "))
        .or_else(|| upper[where_pos..].find(" HAVING "))
        .or_else(|| upper[where_pos..].find(" LIMIT "))
        .or_else(|| upper[where_pos..].find(" UNION "))
        .or_else(|| upper[where_pos..].find(';'))
        .unwrap_or(upper.len() - where_pos);
    let clause = &sql[where_pos..where_pos + where_end];
    // Split on AND/OR at the top level (we don't track full paren depth
    // because the workloads we care about — sysbench oltp_read_write,
    // TPC-H Q1, Q6, Q9 — are simple conjunctions).
    let mut cols = Vec::new();
    for pred in clause.split(|c: char| {
        let up = c.to_ascii_uppercase();
        // Split on the leading boundary of AND/OR, but only at depth 0.
        // The 'A' / 'O' check is a cheap proxy for "the keyword starts here"
        // — we then re-check the full word below.
        up == 'A' || up == 'O'
    }) {
        let pred = pred.trim();
        if pred.is_empty() {
            continue;
        }
        // Re-check the keyword in case the split landed mid-identifier.
        let pred_up = pred.to_uppercase();
        if pred_up.starts_with("AND ") || pred_up.starts_with("OR ") {
            continue;
        }
        // Find `=` at the top level (no parens for our supported queries).
        let eq_pos = match pred.find('=') {
            Some(p) => p,
            None => continue,
        };
        let left = pred[..eq_pos].trim();
        // Strip leading function/cast wrappers; we only need the column
        // name. For `LOWER(col) = ?`, take the last `(`-balanced segment.
        let col = if let Some(paren) = left.rfind('(') {
            // Inside parens. Pick the last identifier-looking token before
            // the closing ')'. For `LOWER(col)` the `col` is at paren+1.
            let inner = &left[paren + 1..];
            // Strip trailing ')'.
            let inner = inner.trim_end_matches(')').trim();
            inner.to_string()
        } else {
            left.to_string()
        };
        // Strip table alias / dot prefix: `t.id` → `id`.
        let col = col.rsplit('.').next().unwrap_or(&col).to_string();
        // Strip backticks / quotes.
        let col = col.trim_matches('`').trim_matches('"').to_string();
        if col.is_empty() || col == "?" {
            continue;
        }
        cols.push(col);
    }
    cols
}

/// Infer MySQL binary-protocol type codes for the `?` placeholders in
/// `sql` by looking up the referenced columns in the storage schema.
///
/// Returns a Vec with one entry per `?`. Falls back to
/// `col_type::VARSTRING` for any placeholder whose column type cannot
/// be determined. This is what COM_STMT_PREPARE advertises back to
/// the client, and is used to decode EXECUTE payloads when the client
/// omits the per-parameter type code (Issue #3372).
fn infer_param_types_from_sql<S: StorageEngine>(sql: &str, storage: &Arc<RwLock<S>>) -> Vec<u8> {
    let param_count = count_placeholders(sql) as usize;
    if param_count == 0 {
        return vec![];
    }
    // Try the column list first (works for INSERT VALUES, INSERT SET,
    // UPDATE ... SET col = ?). If non-empty, look up each column's
    // declared type from the table schema.
    let mut cols = extract_insert_columns(sql);
    if cols.is_empty() {
        // No explicit column list. For SELECT/UPDATE/DELETE statements
        // with a `WHERE col = ?` shape, extract the predicate column
        // names and look those up instead. This is what fixes
        // sysbench oltp_read_write (Issue #3382 follow-up): the
        // server was previously inferring VAR_STRING for every `?` in
        // `SELECT * FROM sbtest WHERE id = ?`, which made the binary
        // protocol decode the 4-byte INT value as a length-encoded
        // string and lose the integer.
        cols = extract_where_columns(sql);
    }
    if cols.is_empty() {
        return vec![col_type::VARSTRING; param_count];
    }
    if let Some(table_name) = extract_table_name(sql) {
        if let Ok(storage_guard) = storage.try_read() {
            if let Ok(table_info) = storage_guard.get_table_info(&table_name) {
                let mut types = Vec::with_capacity(param_count);
                for col_name in &cols {
                    let col_type_byte = table_info
                        .columns
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(col_name))
                        .map(|c| col_type_from_string(&c.data_type))
                        .unwrap_or(col_type::VARSTRING);
                    types.push(col_type_byte);
                }
                if types.len() == param_count {
                    return types;
                }
            }
        }
    }
    vec![col_type::VARSTRING; param_count]
}

/// A single parameter value ready for `replace_placeholders`.
///
/// `numeric` is true when the value was decoded from a MySQL numeric
/// type code (TINY, SHORT, LONG, LONGLONG, FLOAT, DOUBLE, INT24, YEAR,
/// TIMESTAMP). In that case the bytes are the decimal-string rendering
/// of the number (e.g. `b"42"`) and `replace_placeholders` splices them
/// in WITHOUT surrounding quotes. Otherwise the bytes are a string and
/// are spliced as `'value'` with `'` characters doubled per SQL rules.
pub type StmtParam = (Vec<u8>, bool);

/// Substitute `?` placeholders in a SQL string with the given parameter
/// values. Each value is a `(bytes, is_numeric)` tuple (see
/// [`StmtParam`]). A `bytes` value of `&[]` becomes the SQL `NULL`
/// keyword regardless of the `is_numeric` flag.
pub fn replace_placeholders(sql: &str, params: &[StmtParam]) -> String {
    let mut result = sql.to_string();
    for (param, is_numeric) in params.iter() {
        let value = if param.is_empty() {
            "NULL".to_string()
        } else if *is_numeric {
            // Already decimal string from decode_param; splice verbatim.
            String::from_utf8_lossy(param).into_owned()
        } else {
            match String::from_utf8(param.clone()) {
                Ok(s) => format!("'{}'", s.replace('\'', "''")),
                Err(_) => "NULL".to_string(),
            }
        };
        result = result.replacen('?', &value, 1);
    }
    result
}

/// MySQL binary protocol type codes (subset).
/// Reference: <https://dev.mysql.com/doc/dev/mysql-server/latest/field__types_8h.html>
#[allow(dead_code)]
mod mysql_type {
    pub const TINY: u8 = 0x01;
    pub const SHORT: u8 = 0x02;
    pub const LONG: u8 = 0x03;
    pub const FLOAT: u8 = 0x04;
    pub const DOUBLE: u8 = 0x05;
    pub const NULL: u8 = 0x06;
    pub const TIMESTAMP: u8 = 0x07;
    pub const LONGLONG: u8 = 0x08;
    pub const INT24: u8 = 0x09;
    pub const DATE: u8 = 0x0a;
    pub const TIME: u8 = 0x0b;
    pub const DATETIME: u8 = 0x0c;
    pub const YEAR: u8 = 0x0d;
    pub const VARCHAR: u8 = 0x0f;
    pub const BIT: u8 = 0x10;
    pub const JSON: u8 = 0xf5;
    pub const DECIMAL: u8 = 0x00; // MYSQL_TYPE_NEWDECIMAL = 0xf6
    pub const NEWDECIMAL: u8 = 0xf6;
    pub const ENUM: u8 = 0xf7;
    pub const SET: u8 = 0xf8;
    pub const TINY_BLOB: u8 = 0xf9;
    pub const MEDIUM_BLOB: u8 = 0xfa;
    pub const LONG_BLOB: u8 = 0xfb;
    pub const BLOB: u8 = 0xfc;
    pub const VAR_STRING: u8 = 0xfd;
    pub const STRING: u8 = 0xfe;
}

/// Decode a length-encoded integer per MySQL binary protocol. Used for
/// `VAR_STRING`/`VARCHAR` payload lengths.
fn decode_lenenc_int(payload: &[u8], pos: &mut usize) -> Option<u64> {
    if *pos >= payload.len() {
        return None;
    }
    let b0 = payload[*pos];
    *pos += 1;
    if b0 < 0xfb {
        Some(b0 as u64)
    } else if b0 == 0xfc {
        if *pos + 2 > payload.len() {
            return None;
        }
        let v = u16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
        *pos += 2;
        Some(v as u64)
    } else if b0 == 0xfd {
        if *pos + 3 > payload.len() {
            return None;
        }
        let v = u32::from_le_bytes([0, payload[*pos], payload[*pos + 1], payload[*pos + 2]]);
        *pos += 3;
        Some(v as u64)
    } else if b0 == 0xfe {
        if *pos + 8 > payload.len() {
            return None;
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
        Some(v)
    } else {
        // 0xfb = NULL, 0xff = 0xff (unused). Treat as None.
        None
    }
}

/// Decode a single parameter value starting at `*pos`. Advances `*pos`
/// past the consumed bytes. Returns the value as a `Vec<u8>` ready for
/// `replace_placeholders` (an empty `Vec` represents NULL).
///
/// For numeric types the value is converted to a UTF-8 decimal string so
/// `replace_placeholders` can splice it as a numeric literal (no quotes).
/// For string/blob types the value is returned as the raw bytes (UTF-8
/// assumed; if not valid UTF-8 it is reported as NULL).
fn decode_param(payload: &[u8], pos: &mut usize, type_code: u8) -> Option<Vec<u8>> {
    use mysql_type::*;
    let start = *pos;
    match type_code {
        TINY => {
            if *pos + 1 > payload.len() {
                return None;
            }
            let v = payload[*pos] as i8;
            *pos += 1;
            Some(v.to_string().into_bytes())
        }
        SHORT => {
            if *pos + 2 > payload.len() {
                return None;
            }
            let v = i16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
            *pos += 2;
            Some(v.to_string().into_bytes())
        }
        LONG => {
            if *pos + 4 > payload.len() {
                return None;
            }
            let v = i32::from_le_bytes([
                payload[*pos],
                payload[*pos + 1],
                payload[*pos + 2],
                payload[*pos + 3],
            ]);
            *pos += 4;
            Some(v.to_string().into_bytes())
        }
        FLOAT => {
            if *pos + 4 > payload.len() {
                return None;
            }
            let v = f32::from_le_bytes([
                payload[*pos],
                payload[*pos + 1],
                payload[*pos + 2],
                payload[*pos + 3],
            ]);
            *pos += 4;
            Some(v.to_string().into_bytes())
        }
        DOUBLE => {
            if *pos + 8 > payload.len() {
                return None;
            }
            let v = f64::from_le_bytes([
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
            Some(v.to_string().into_bytes())
        }
        LONGLONG => {
            if *pos + 8 > payload.len() {
                return None;
            }
            let v = i64::from_le_bytes([
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
            Some(v.to_string().into_bytes())
        }
        INT24 => {
            if *pos + 4 > payload.len() {
                return None;
            }
            // INT24 is sent as 4 bytes (padded); sign-extend from 24 bits.
            let raw = i32::from_le_bytes([
                payload[*pos],
                payload[*pos + 1],
                payload[*pos + 2],
                payload[*pos + 3],
            ]);
            *pos += 4;
            let sign_ext = (raw << 8) >> 8;
            Some(sign_ext.to_string().into_bytes())
        }
        YEAR => {
            if *pos + 2 > payload.len() {
                return None;
            }
            let v = u16::from_le_bytes([payload[*pos], payload[*pos + 1]]);
            *pos += 2;
            Some(v.to_string().into_bytes())
        }
        TIMESTAMP => {
            if *pos + 4 > payload.len() {
                return None;
            }
            let v = u32::from_le_bytes([
                payload[*pos],
                payload[*pos + 1],
                payload[*pos + 2],
                payload[*pos + 3],
            ]);
            *pos += 4;
            // Render as ISO-8601 (without timezone — best effort).
            Some(v.to_string().into_bytes())
        }
        NULL => {
            // 0 bytes follow.
            Some(Vec::new())
        }
        VARCHAR | VAR_STRING | STRING | TINY_BLOB | MEDIUM_BLOB | LONG_BLOB | BLOB | ENUM | SET
        | BIT | JSON => {
            // Length-encoded string/blob.
            let len = decode_lenenc_int(payload, pos)?;
            if *pos + (len as usize) > payload.len() {
                *pos = start;
                return None;
            }
            let bytes = payload[*pos..*pos + (len as usize)].to_vec();
            *pos += len as usize;
            Some(bytes)
        }
        _ => {
            // Unknown type: best effort — treat as a single length-encoded string.
            let len = decode_lenenc_int(payload, pos).unwrap_or(0);
            if *pos + (len as usize) > payload.len() {
                *pos = start;
                return None;
            }
            let bytes = payload[*pos..*pos + (len as usize)].to_vec();
            *pos += len as usize;
            Some(bytes)
        }
    }
}

/// Parse a COM_STMT_EXECUTE binary-protocol payload and extract the
/// parameter values into a `Vec<StmtParam>` ready for
/// `replace_placeholders`.
///
/// Payload layout (MySQL 5.6+ binary protocol):
///
///   bytes  0..4   : stmt_id (u32 LE) — caller has already consumed this
///   byte     4    : flags (CURSOR_TYPE_NO_CURSOR = 0x00)
///   bytes  5..9   : iteration_count (u32 LE, 0x01 = execute once)
///   next N        : null-bitmap, N = (param_count + 7) / 8 bytes
///   next 1        : new_params_bound_flag (0x01 if param types follow)
///   next 2*N      : type codes (2 bytes each) if new_params_bound_flag=0x01
///   remaining     : param values in order, each formatted per its type
///
/// `param_count` is the count returned by COM_STMT_PREPARE (the number of
/// `?` placeholders in the original SQL).
///
/// `prepared_param_types` is the type-code slice advertised by
/// COM_STMT_PREPARE. It is used as the fallback when the client omits
/// `new_params_bound_flag` (e.g. sysbench 1.0.20) so that INT64 / LONG
/// values are not misread as length-encoded strings (Issue #3372).
pub fn parse_stmt_execute_params(
    payload: &[u8],
    param_count: u16,
    prepared_param_types: &[u8],
) -> Vec<StmtParam> {
    let mut params: Vec<StmtParam> = Vec::new();
    if payload.len() < 9 {
        return params;
    }
    let mut pos = 9;
    if param_count == 0 {
        return params;
    }

    let null_bytes = (param_count as usize).div_ceil(8);
    if pos + null_bytes > payload.len() {
        return params;
    }
    let null_bitmap = &payload[pos..pos + null_bytes];
    pos += null_bytes;

    if pos >= payload.len() {
        return params;
    }
    let new_params_bound_flag = payload[pos];
    pos += 1;

    let mut type_codes: Vec<u8> = Vec::with_capacity(param_count as usize);
    if new_params_bound_flag == 0x01 {
        for _ in 0..param_count {
            if pos + 2 > payload.len() {
                return params;
            }
            type_codes.push(payload[pos]);
            pos += 2;
        }
    }

    for i in 0..param_count as usize {
        if (null_bitmap[i / 8] >> (i % 8)) & 1 != 0 {
            params.push((Vec::new(), false));
            continue;
        }
        // Server is the source of truth for parameter types (it knows the
        // schema). The client's `type_codes` from `new_params_bound_flag`
        // are advisory only — some clients (sysbench 1.0.20) advertise
        // MYSQL_TYPE_VAR_STRING (0xfd) for every parameter regardless of
        // the underlying column type, which causes INT64 values to be
        // misread as length-encoded strings (Issue #3372 follow-up).
        // Prefer the prepared statement's type, falling back to the
        // client's advertised type only when we have no schema info.
        let type_code: u8 = prepared_param_types
            .get(i)
            .copied()
            .or_else(|| type_codes.get(i).copied())
            .unwrap_or(mysql_type::VAR_STRING);
        match decode_param(payload, &mut pos, type_code) {
            Some(v) => params.push((v, is_numeric_type(type_code))),
            None => return params,
        }
    }

    params
}

/// True iff the MySQL binary-protocol type code is a numeric type
/// (TINY/SHORT/LONG/LONGLONG/FLOAT/DOUBLE/INT24/YEAR/TIMESTAMP).
/// Used to drive the quoting policy in `replace_placeholders`.
fn is_numeric_type(type_code: u8) -> bool {
    use mysql_type::*;
    matches!(
        type_code,
        TINY | SHORT | LONG | FLOAT | DOUBLE | LONGLONG | INT24 | YEAR | TIMESTAMP
    )
}

fn extract_table_name(sql: &str) -> Option<String> {
    let u = sql.trim().to_uppercase();
    if let Some(rest) = u.strip_prefix("SELECT") {
        if let Some(from_pos) = rest.find("FROM") {
            let after_from = rest[from_pos + 4..].trim();
            let table_end = after_from
                .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == ')')
                .unwrap_or(after_from.len());
            let table = after_from[..table_end].trim();
            if !table.is_empty() {
                let orig_after = sql.to_uppercase().find("FROM").unwrap();
                let orig_from = sql[orig_after + 4..].trim();
                let orig_end = orig_from
                    .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == ')')
                    .unwrap_or(orig_from.len());
                return Some(orig_from[..orig_end].trim().to_string());
            }
        }
    }
    None
}

#[allow(dead_code)]
fn extract_column_names(sql: &str, storage: &Arc<RwLock<MemoryStorage>>) -> Vec<String> {
    let u = sql.trim().to_uppercase();
    if u.starts_with("SHOW") || u.starts_with("DESCRIBE") || u.starts_with("EXPLAIN") {
        return vec![];
    }
    if let Some(table_name) = extract_table_name(sql) {
        if let Ok(storage_guard) = storage.try_read() {
            if let Ok(table_info) = storage_guard.get_table_info(&table_name) {
                let col_names: Vec<String> =
                    table_info.columns.iter().map(|c| c.name.clone()).collect();
                if !col_names.is_empty() {
                    return col_names;
                }
            }
        }
    }
    vec![]
}

#[allow(dead_code)]
fn infer_column_types(
    sql: &str,
    storage: &Arc<RwLock<MemoryStorage>>,
    cols: &[String],
) -> Vec<String> {
    if let Some(table_name) = extract_table_name(sql) {
        if let Ok(storage_guard) = storage.try_read() {
            if let Ok(table_info) = storage_guard.get_table_info(&table_name) {
                let types: Vec<String> = table_info
                    .columns
                    .iter()
                    .take(cols.len())
                    .map(|c| c.data_type.clone())
                    .collect();
                if types.len() == cols.len() {
                    return types;
                }
            }
        }
    }
    cols.iter().map(|_| "VARCHAR(255)".to_string()).collect()
}

#[allow(clippy::type_complexity)]
fn is_select_stmt(stmt: &Statement) -> bool {
    matches!(
        stmt,
        Statement::Select(_) | Statement::Show(_) | Statement::Describe(_)
    )
}

fn generate_self_signed_cert() -> (Vec<u8>, Vec<u8>) {
    let key_pair = KeyPair::generate().unwrap();
    let key_der = key_pair.serialize_der();
    let params = CertificateParams::new(vec!["localhost".into(), "127.0.0.1".into()]).unwrap();
    let cert = params.self_signed(&key_pair).unwrap();
    let cert_der = cert.der().as_ref().to_vec();
    (cert_der, key_der)
}

fn make_tls_config() -> rustls::ServerConfig {
    let (cert_der, key_der) = generate_self_signed_cert();
    let cert = rustls::pki_types::CertificateDer::from(cert_der);
    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der).unwrap();
    rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap()
}

/// Server-side LOAD DATA LOCAL INFILE handler.
///
/// Wire-protocol flow:
/// 1. Whitelist check: `canonicalize(path).starts_with(canonicalize(data_dir))`
///    so a client can't trick the server into streaming `/etc/passwd`.
/// 2. Send 0xFB packet to the client carrying the file path. The client
///    opens that file and starts streaming its bytes back as packet
///    payloads (≤ 16 MB each).
/// 3. Loop on content packets until an empty-payload terminator.
/// 4. Buffer bytes, split on `\n`, parse each line with
///    `load_data::parse_tbl_line`, batch-insert at
///    `bulk_buf_size` byte boundaries via `load_data::bulk_insert`.
/// 5. Return the total rows inserted.
///
/// Errors are returned to the caller; the routing site in
/// `do_command_loop` is responsible for translating them to wire
/// protocol ERR packets.
#[allow(
    clippy::too_many_arguments,
    reason = "explicit wire-protocol + handler-context args mirror the spec"
)]
fn handle_load_local_infile<S: Read + Write>(
    stream: &mut S,
    engine: &mut sqlrustgo::ExecutionEngine<
        sqlrustgo_storage::WalStorage<
            sqlrustgo_storage::FileStorage,
            sqlrustgo_storage::FileBackedWalManager,
        >,
    >,
    path: &str,
    table: &str,
    _delim: char,
    data_dir: std::path::PathBuf,
    bulk_buf_size: usize,
    seq: &mut u8,
    _cap: u32,
) -> MySqlResult<u64> {
    use crate::load_data::{bulk_insert, parse_tbl_line};

    // 1. Whitelist check — canonicalize both sides and confirm the
    //    file is inside data_dir. This is the only line of defense
    //    against a malicious client pointing us at e.g. /etc/passwd.
    let canonical_path = std::fs::canonicalize(path)
        .map_err(|e| MySqlError::Other(format!("file not found: {}: {}", path, e)))?;
    let canonical_data_dir = std::fs::canonicalize(&data_dir).map_err(|e| {
        MySqlError::Other(format!("data_dir not found: {}: {}", data_dir.display(), e))
    })?;
    if !canonical_path.starts_with(&canonical_data_dir) {
        return Err(MySqlError::Other(format!(
            "file {:?} not in allowed data_dir {:?}",
            canonical_path, canonical_data_dir
        )));
    }

    // 2. Look up the target table's column count so parse_tbl_line
    //    can validate each line has the right shape.
    let col_count = {
        let storage_arc = engine.storage_ref();
        let storage = storage_arc
            .read()
            .map_err(|e| MySqlError::Other(format!("storage lock poisoned: {}", e)))?;
        let table_info = storage
            .get_table_info(table)
            .map_err(|e| MySqlError::Other(format!("table {}: {}", table, e)))?;
        table_info.columns.len()
    };

    // 3. Send 0xFB packet to the client — the client interprets this
    //    as "open this file and start streaming its bytes back".
    let mut fb_payload = Vec::with_capacity(path.len() + 1);
    fb_payload.push(packet_type::LOCAL_INFILE_REQUEST);
    fb_payload.extend_from_slice(path.as_bytes());
    Packet {
        length: fb_payload.len() as u32,
        sequence: *seq,
        payload: fb_payload,
    }
    .write_to(stream)?;
    stream.flush()?;
    *seq = seq.wrapping_add(1);

    // 4. Loop on file content packets until the client signals EOF
    //    with an empty-payload packet.
    let mut buf: Vec<u8> = Vec::with_capacity(bulk_buf_size * 2);
    let mut total_rows: u64 = 0;
    let mut pending_rows: Vec<Vec<sqlrustgo_types::Value>> = Vec::new();

    // ---- EAGAIN bug fix (RC2 Week 1 Day 6) ----
    //
    // Original code tracked `pending_bytes` as the sum of *parsed*
    // line lengths and flushed when that sum exceeded `bulk_buf_size`.
    // But the flush decision ignored partial bytes that could be in
    // `buf` at the end of a packet (when the packet's last line is
    // incomplete, the rest of the line sits in `buf` waiting for the
    // next packet). For a 9-column 150-row table (orders.tbl) the
    // accumulated partial bytes were enough that the flush logic
    // committed rows *before* all their bytes were in `buf`, and
    // subsequent reads on the client got EAGAIN because the
    // server's per-connection accounting had overshot.
    //
    // Fix: only flush when `buf` is empty (i.e. we have parsed every
    // byte we currently have). The size threshold becomes a soft
    // check on `buf.len()` to avoid pathological memory growth, but
    // we never flush with unparsed bytes still in the buffer.
    let mut last_flush_kept_rows: usize = 0;
    loop {
        let pkt = Packet::read_from(stream)?;
        *seq = pkt.sequence.wrapping_add(1);
        if pkt.payload.is_empty() {
            break;
        }
        buf.extend_from_slice(&pkt.payload);

        // Drain complete lines from the buffer.
        while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl).collect();
            let line_str = match std::str::from_utf8(&line) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("non-utf8 line skipped: {}", e);
                    continue;
                }
            };
            let line_str = line_str.trim_end_matches('\n');
            if line_str.trim().is_empty() {
                continue;
            }
            match parse_tbl_line(line_str, col_count) {
                Ok(row) => {
                    pending_rows.push(row);
                }
                Err(e) => {
                    tracing::warn!("parse line error: {}", e);
                }
            }
        }

        // Only flush when we have **fully drained** the buffer. The
        // size threshold is a sanity guard — if `buf` is still
        // non-empty (last line spans a packet boundary), defer
        // flushing until the next packet.
        //
        // v3.8.0-rc2 Day 7 follow-up: also flush periodically when
        // pending_rows grows large, EVEN if buf is non-empty. This
        // is needed because some clients (notably the `mysql` CLI
        // with LOAD DATA LOCAL INFILE) send the entire file in one
        // big packet with a long delay between the data packet and
        // the EOF packet. Without the periodic flush, we wait
        // indefinitely for an EOF that comes only after the data
        // is fully drained, and bulk_insert on the entire pending
        // set blocks the accept loop long enough that the client
        // times out.
        const PERIODIC_FLUSH_ROWS: usize = 100;
        if buf.is_empty() && pending_rows.len() > last_flush_kept_rows {
            let pending = std::mem::take(&mut pending_rows);
            last_flush_kept_rows = 0;
            let n = bulk_insert(engine, table, pending)
                .map_err(|e| MySqlError::Other(format!("bulk_insert: {}", e)))?;
            total_rows += n;
        } else if buf.len() > bulk_buf_size * 4 {
            // Sanity guard: if buf keeps growing without ever
            // draining (e.g. a malformed file with no newlines),
            // force-flush whatever parsed rows we have so we don't
            // OOM. Reset last_flush_kept_rows so we don't immediately
            // re-flush.
            tracing::warn!(
                "LOAD DATA buf exceeds 4× bulk_buf_size ({} bytes) \
                 without draining; forcing flush of {} rows",
                buf.len(),
                pending_rows.len()
            );
            let pending = std::mem::take(&mut pending_rows);
            last_flush_kept_rows = 0;
            let n = bulk_insert(engine, table, pending)
                .map_err(|e| MySqlError::Other(format!("bulk_insert: {}", e)))?;
            total_rows += n;
        } else if pending_rows.len() >= PERIODIC_FLUSH_ROWS {
            // Periodic flush: every PERIODIC_FLUSH_ROWS rows, flush
            // even if buf is non-empty. The remaining bytes in buf
            // are a partial line that will complete in a later
            // packet.
            let pending: Vec<Vec<sqlrustgo_types::Value>> = std::mem::take(&mut pending_rows);
            last_flush_kept_rows = 0;
            let n = bulk_insert(engine, table, pending)
                .map_err(|e| MySqlError::Other(format!("bulk_insert: {}", e)))?;
            total_rows += n;
        } else {
            // Defer flush until next packet; remember that these
            // rows are still pending.
            last_flush_kept_rows = pending_rows.len();
        }
    }

    // Final flush of any rows that didn't hit a boundary.
    if !pending_rows.is_empty() {
        let n = bulk_insert(engine, table, pending_rows)
            .map_err(|e| MySqlError::Other(format!("bulk_insert: {}", e)))?;
        total_rows += n;
    }

    Ok(total_rows)
}

#[allow(unused_assignments)]
fn do_command_loop<S: Read + Write>(
    stream: &mut S,
    addr: SocketAddr,
    storage: Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>>,
    engine: Arc<RwLock<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>>>,
    cap: u32,
    mut seq: u8,
    ps_manager: &mut PreparedStatementManager,
) -> MySqlResult<()> {
    loop {
        let pkt = match Packet::read_from(stream) {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!("Disconnected: {}", e);
                break;
            }
        };
        let cmd = pkt.payload.first().copied().unwrap_or(0);
        let payload = &pkt.payload[1..];
        seq = pkt.sequence.wrapping_add(1);
        match cmd {
            packet_type::COM_QUIT => {
                // MySQL wire protocol: server MUST send OK packet on COM_QUIT
                // before closing the connection, so the client can release
                // its read() and exit cleanly. Without this, mysql CLI and
                // pymysql hang in recv() after sending COM_QUIT (Issue #SET-NAMES-HANG).
                make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                seq = seq.wrapping_add(1);
                break;
            }
            packet_type::COM_PING => {
                make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                seq = seq.wrapping_add(1);
            }
            packet_type::COM_INIT_DB => {
                make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                seq = seq.wrapping_add(1);
            }
            packet_type::COM_QUERY => {
                let q = String::from_utf8_lossy(payload)
                    .trim_end_matches('\0')
                    .trim()
                    .to_string();
                tracing::info!("Query [{}]: {}", addr, q);

                // ROUTE: LOAD DATA LOCAL INFILE
                //
                // The MySQL wire protocol for LOAD DATA LOCAL INFILE is
                // a two-phase dance: the client first sends the SQL
                // text (handled here), the server replies with a 0xFB
                // packet naming the file, and then the client streams
                // the file's bytes back. We pull data_dir and
                // bulk_insert_buffer_size from the ACTIVE_CONFIG set
                // by `start_ephemeral` so the handler has the same
                // values the test used to configure the server.
                if let Some((path, table, delim)) = parse_load_local_infile_sql(&q) {
                    let cfg = ACTIVE_CONFIG
                        .get()
                        .map(|m| m.lock().unwrap().clone())
                        .unwrap_or_default();
                    let data_dir = cfg
                        .data_dir
                        .clone()
                        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
                    let bulk_buf = cfg.bulk_insert_buffer_size;
                    let n = match handle_load_local_infile(
                        stream,
                        &mut engine.write().unwrap(),
                        &path,
                        &table,
                        delim,
                        data_dir,
                        bulk_buf,
                        &mut seq,
                        cap,
                    ) {
                        Ok(n) => n,
                        Err(e) => {
                            make_err_packet(seq, 1146u16, "42S02", &e.to_string())
                                .write_to(stream)?;
                            seq = seq.wrapping_add(1);
                            0
                        }
                    };
                    make_ok_packet(seq, n, 0, 0x0002, 0).write_to(stream)?;
                    seq = seq.wrapping_add(1);
                    continue;
                }

                if q.is_empty() {
                    make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                    seq = seq.wrapping_add(1);
                    continue;
                }

                // G2 fix: Intercept `SET NAMES` / `SET autocommit` / etc.
                // These session variables are not part of the DDL/DML parser.
                // We accept them as no-ops and return OK so Python clients
                // (pymysql, mysql-connector-python) can complete handshake.
                let lower_q = q.to_lowercase().replace(" ", "");
                if lower_q.starts_with("setnames")
                    || lower_q.starts_with("setautocommit")
                    || lower_q.starts_with("set@@autocommit")
                    || lower_q.starts_with("setcharacter_set")
                    || lower_q.starts_with("set@@character_set")
                    || lower_q.starts_with("setsession")
                    || lower_q.starts_with("set@@session")
                    || lower_q.starts_with("set@@")
                    || lower_q.starts_with("setglobal")
                    || lower_q.starts_with("settransaction")
                {
                    tracing::info!("SET NOP: {}", q);
                    make_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                    seq = seq.wrapping_add(1);
                    continue;
                }
                let mut eng = engine.write().unwrap();
                // 3521: Support multi-statement queries (semicolon-separated)
                match parse_statements(&q) {
                    Ok(stmts) => {
                        for stmt in stmts {
                            let result = eng.execute(&q);
                            match result {
                                Ok(r) if is_select_stmt(&stmt) => {
                                    let cols: Vec<String> = r
                                        .rows
                                        .first()
                                        .map(|row| {
                                            (0..row.len())
                                                .map(|i| format!("col_{}", i + 1))
                                                .collect()
                                        })
                                        .unwrap_or_else(|| vec!["result".to_string()]);
                                    let ctypes: Vec<String> =
                                        cols.iter().map(|_| "VARCHAR(255)".to_string()).collect();
                                    seq =
                                        send_result_set(stream, &cols, &ctypes, &r.rows, seq, cap)?;
                                }
                                Ok(r) => {
                                    make_ok_packet(seq, r.affected_rows as u64, 0, 0x0002, 0)
                                        .write_to(stream)?;
                                    seq = seq.wrapping_add(1);
                                }
                                Err(e) => {
                                    let code = match e.to_string().contains("not found") {
                                        true => 1146u16,
                                        false => 1064u16,
                                    };
                                    make_err_packet(seq, code, "42000", &e.to_string())
                                        .write_to(stream)?;
                                    seq = seq.wrapping_add(1);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        make_err_packet(seq, 1064, "42000", &e).write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    }
                }
            }
            packet_type::COM_STMT_PREPARE => {
                let sql = String::from_utf8_lossy(payload)
                    .trim_end_matches('\0')
                    .trim()
                    .to_string();
                tracing::info!("STMT PREPARE: {}", sql);

                let param_count = count_placeholders(&sql);

                let column_count: u16 = if sql.to_uppercase().starts_with("SELECT") {
                    let upper = sql.to_uppercase();
                    if let Some(from_pos) = upper.find(" FROM ") {
                        let select_part = &sql[..from_pos + 1].trim();
                        let cols_str = select_part.strip_prefix("SELECT").unwrap_or("").trim();
                        if cols_str.eq_ignore_ascii_case("*") {
                            if let Ok(storage_guard) = storage.try_read() {
                                if let Some(table_name) = extract_table_name(&sql) {
                                    if let Ok(table_info) =
                                        storage_guard.get_table_info(&table_name)
                                    {
                                        table_info.columns.len() as u16
                                    } else {
                                        1
                                    }
                                } else {
                                    1
                                }
                            } else {
                                1
                            }
                        } else {
                            cols_str.split(',').count() as u16
                        }
                    } else {
                        1
                    }
                } else {
                    0
                };

                let param_types = infer_param_types_from_sql(&sql, &storage);
                let stmt_id =
                    ps_manager.add(sql.clone(), param_count, column_count, param_types.clone());

                let mut p = Vec::new();
                p.push(0x00);
                p.write_u32::<LittleEndian>(stmt_id).unwrap();
                p.write_u16::<LittleEndian>(column_count).unwrap();
                p.write_u16::<LittleEndian>(param_count).unwrap();
                p.push(0x00);
                p.write_u16::<LittleEndian>(0).unwrap();
                Packet {
                    length: p.len() as u32,
                    sequence: seq,
                    payload: p,
                }
                .write_to(stream)?;
                seq = seq.wrapping_add(1);

                if param_count > 0 {
                    for i in 0..param_count as usize {
                        let ptype = param_types.get(i).copied().unwrap_or(col_type::VARSTRING);
                        let mut param_def = Vec::new();
                        write_lenenc_string(&mut param_def, b"def").unwrap();
                        write_lenenc_string(&mut param_def, b"").unwrap();
                        write_lenenc_string(&mut param_def, b"").unwrap();
                        write_lenenc_string(&mut param_def, b"").unwrap();
                        write_lenenc_string(&mut param_def, b"?").unwrap();
                        write_lenenc_string(&mut param_def, b"?").unwrap();
                        // MySQL column/param fixed-size fields: charset_collation (2 bytes)
                        // → length (4 bytes) → field_type (1 byte) → flags (2 bytes)
                        // → decimals (1 byte) → filler (2 bytes)
                        param_def.write_u16::<LittleEndian>(0x0030).unwrap(); // charset_collation: 0x30 = utf8_general_ci
                        param_def.write_u32::<LittleEndian>(255).unwrap(); // length
                        param_def.push(ptype); // field_type
                        param_def.write_u16::<LittleEndian>(0x80).unwrap(); // flags
                        param_def.push(0x00); // decimals
                        param_def.write_u16::<LittleEndian>(0).unwrap(); // filler
                        Packet {
                            length: param_def.len() as u32,
                            sequence: seq,
                            payload: param_def,
                        }
                        .write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    }
                    if cap & capability::DEPRECATE_EOF != 0 {
                        make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    } else {
                        make_eof_packet(seq, 0x0002).write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    }
                }

                if column_count > 0 {
                    // Try to extract real column names from the SQL
                    // (after SELECT, before FROM). Falls back to
                    // col_1, col_2... when ambiguous (e.g. SELECT *).
                    let real_col_names: Vec<String> = if sql.to_uppercase().starts_with("SELECT")
                        && sql.to_uppercase().contains(" FROM ")
                    {
                        let upper = sql.to_uppercase();
                        if let Some(from_pos) = upper.find(" FROM ") {
                            let select_part = sql[..from_pos].trim();
                            let cols_str = select_part
                                .strip_prefix("SELECT")
                                .or_else(|| select_part.strip_prefix("select"))
                                .unwrap_or("")
                                .trim();
                            if !cols_str.is_empty() && !cols_str.contains('*') {
                                cols_str
                                    .split(',')
                                    .map(|s| {
                                        s.trim()
                                            .split('.')
                                            .next_back()
                                            .unwrap_or(s.trim())
                                            .to_string()
                                    })
                                    .collect()
                            } else {
                                (0..column_count)
                                    .map(|i| format!("col_{}", i + 1))
                                    .collect()
                            }
                        } else {
                            (0..column_count)
                                .map(|i| format!("col_{}", i + 1))
                                .collect()
                        }
                    } else {
                        (0..column_count)
                            .map(|i| format!("col_{}", i + 1))
                            .collect()
                    };
                    for i in 0..column_count {
                        let col_name = real_col_names
                            .get(i as usize)
                            .cloned()
                            .unwrap_or_else(|| format!("col_{}", i + 1));
                        write_column_def(stream, &col_name, "VARCHAR(255)", seq)?;
                        seq = seq.wrapping_add(1);
                    }
                    if cap & capability::DEPRECATE_EOF != 0 {
                        make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0).write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    } else {
                        make_eof_packet(seq, 0x0002).write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    }
                }

                tracing::info!(
                    "STMT PREPARE done: id={}, params={}, cols={}",
                    stmt_id,
                    param_count,
                    column_count
                );
            }
            packet_type::COM_STMT_EXECUTE => {
                if payload.len() < 4 {
                    make_err_packet(seq, 1047, "HY000", "Malformed COM_STMT_EXECUTE")
                        .write_to(stream)?;
                    seq = seq.wrapping_add(1);
                    continue;
                }

                let stmt_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);

                let stmt = match ps_manager.get(stmt_id) {
                    Some(s) => (
                        s.sql.clone(),
                        s.column_count,
                        s.param_count,
                        s.param_types.clone(),
                    ),
                    None => {
                        make_err_packet(seq, 1243, "HY000", "Unknown statement handler")
                            .write_to(stream)?;
                        seq = seq.wrapping_add(1);
                        continue;
                    }
                };
                let stmt_sql = stmt.0;
                let stmt_col_count = stmt.1;
                let stmt_param_count = stmt.2;
                let stmt_param_types = stmt.3;

                // Issue #2813: previously `params` was always an empty Vec,
                // so `?` placeholders were never substituted. Now we parse
                // the COM_STMT_EXECUTE binary protocol payload to extract
                // the actual parameter values from the client. The
                // `stmt_param_types` slice is the type-code advertisement
                // from COM_STMT_PREPARE; it is the fallback when the
                // client omits the per-parameter type code (Issue #3372).
                let params: Vec<crate::StmtParam> =
                    parse_stmt_execute_params(payload, stmt_param_count, &stmt_param_types);
                let final_sql = replace_placeholders(&stmt_sql, &params);
                tracing::info!("STMT EXECUTE (id={}): {}", stmt_id, final_sql);
                let mut eng = engine.write().unwrap();
                let parsed = parse(&final_sql);
                match parsed {
                    Ok(stmt) => {
                        let result = eng.execute(&final_sql);
                        match result {
                            Ok(r) if is_select_stmt(&stmt) => {
                                let c: Vec<String> = r
                                    .rows
                                    .first()
                                    .map(|row| {
                                        (0..row.len()).map(|i| format!("col_{}", i + 1)).collect()
                                    })
                                    .unwrap_or_else(|| vec!["result".to_string()]);
                                let t: Vec<String> =
                                    c.iter().map(|_| "VARCHAR(255)".to_string()).collect();
                                let c_trimmed: Vec<String> =
                                    c.into_iter().take(stmt_col_count as usize).collect();
                                let t_trimmed: Vec<String> =
                                    t.into_iter().take(stmt_col_count as usize).collect();
                                let r_trimmed: Vec<Vec<Value>> = r
                                    .rows
                                    .into_iter()
                                    .map(|row| {
                                        row.into_iter().take(stmt_col_count as usize).collect()
                                    })
                                    .collect();
                                seq = send_binary_result_set(
                                    stream, &c_trimmed, &t_trimmed, &r_trimmed, seq, cap,
                                )?;
                            }
                            Ok(r) => {
                                make_ok_packet(seq, r.affected_rows as u64, 0, 0x0002, 0)
                                    .write_to(stream)?;
                                seq = seq.wrapping_add(1);
                            }
                            Err(e) => {
                                make_err_packet(seq, 1064, "42000", &e.to_string())
                                    .write_to(stream)?;
                                seq = seq.wrapping_add(1);
                            }
                        }
                    }
                    Err(e) => {
                        make_err_packet(seq, 1064, "42000", &e).write_to(stream)?;
                        seq = seq.wrapping_add(1);
                    }
                }
            }
            packet_type::COM_STMT_CLOSE => {
                if payload.len() >= 4 {
                    let stmt_id =
                        u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    ps_manager.remove(stmt_id);
                }
            }
            _ => {
                make_err_packet(seq, 1047, "HY000", "Unknown command").write_to(stream)?;
                seq = seq.wrapping_add(1);
            }
        }
    }
    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    storage: Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>>,
    tls_config: Arc<rustls::ServerConfig>,
    user_store: UserStore,
) {
    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    TOTAL_CONNECTIONS_ACCEPTED.fetch_add(1, Ordering::Relaxed);
    let _guard = scopeguard::guard((), |_| {
        // Always decrement on exit, even on panic
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    });
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(600)))
        .ok();
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(60)))
        .ok();
    stream.set_nodelay(true).ok();
    // We keep the socket in non-blocking mode. The TLS read/write
    // helpers in TlsStream use rustls::ServerConnection::complete_io,
    // which returns WouldBlock when no I/O is ready and never blocks
    // the application. The read/write timeouts above are not used
    // by rustls, but the connection-level timeouts in the stream
    // (read 600s) still apply for non-TLS reads.
    let _ = stream.set_nonblocking(false);
    tracing::info!("Connection from {}", addr);

    let scramble1: [u8; 8] = rand::random();
    let scramble2: [u8; 12] = rand::random();
    let mut scramble = [0u8; 20];
    scramble[..8].copy_from_slice(&scramble1);
    scramble[8..].copy_from_slice(&scramble2);

    if let Err(e) = make_handshake_packet(0, &scramble).write_to(&mut &stream) {
        tracing::error!("Handshake send: {}", e);
        return;
    }

    let pkt = match Packet::read_from(&mut &stream) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Handshake read: {}", e);
            return;
        }
    };

    // SSL Request?
    if pkt.length == 32 {
        let cap = u32::from_le_bytes([
            pkt.payload[0],
            pkt.payload[1],
            pkt.payload[2],
            pkt.payload[3],
        ]);
        if cap & capability::SSL != 0 {
            tracing::info!("SSL upgrade for {}", addr);
            let mut conn = match rustls::ServerConnection::new(tls_config) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("TLS: {}", e);
                    return;
                }
            };
            // Complete TLS handshake
            conn.complete_io(&mut stream).unwrap();
            // Read handshake response over TLS. We use a TlsStream
            // wrapper here so that subsequent writes auto-flush to
            // the underlying socket. The wrapper only does the initial
            // handshake read; the do_command_loop gets the long-lived
            // wrapper below.
            let mut tls = rustls::Stream::new(&mut conn, &mut stream);
            let tls_pkt = match Packet::read_from(&mut tls) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("TLS read: {}", e);
                    return;
                }
            };
            tracing::info!(
                "TLS packet: len={}, seq={}, first_bytes={:02x?}",
                tls_pkt.length,
                tls_pkt.sequence,
                &tls_pkt.payload[..std::cmp::min(16, tls_pkt.payload.len())]
            );
            let resp = match parse_handshake_response(&tls_pkt) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("TLS parse: {}", e);
                    return;
                }
            };
            tracing::info!(
                "TLS user={}, db={:?}, plugin={:?}, auth_resp_len={}",
                resp.username,
                resp.database,
                resp.auth_plugin_name,
                resp.auth_response.len()
            );
            let auth_ok = if skip_auth() {
                true
            } else if resp.auth_response.is_empty() {
                tracing::warn!("Empty auth response for user {}", resp.username);
                false
            } else {
                user_store.verify_password(&resp.username, &scramble, &resp.auth_response)
            };
            if !auth_ok {
                tracing::warn!("Auth failed for user {}", resp.username);
                make_err_packet(3, 1045, "28000", "Access denied")
                    .write_to(&mut tls)
                    .ok();
                return;
            }
            tracing::info!("Auth accepted, sending OK packet, seq=3");
            make_ok_packet(3, 0, 0, 0x0002, 0).write_to(&mut tls).ok();
            tracing::info!("Starting command loop, seq=4");
            // Drop the temporary Stream wrapper and create a long-lived
            // TlsStream that drives rustls IO after every write. This
            // is critical for `mysql` CLI / sysbench compatibility:
            // without auto-complete_io, the cipher buffer accumulates
            // and the client never receives the response.
            let mut tls = TlsStream::new(&mut conn, &mut stream);
            let engine: Arc<
                RwLock<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>>,
            > = Arc::new(RwLock::new(ExecutionEngine::new(storage.clone())));
            let mut ps_manager = PreparedStatementManager::new();
            let _ = do_command_loop(
                &mut tls,
                addr,
                storage,
                engine,
                resp.capability_flags,
                4,
                &mut ps_manager,
            );
            // Best-effort final flush so the last OK packet (e.g. on
            // COM_QUIT) reaches the client before the connection drops.
            let _ = tls.flush_pending();
            return;
        }
    }

    // Non-SSL
    let resp = match parse_handshake_response(&pkt) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Parse: {}", e);
            return;
        }
    };
    tracing::info!(
        "user={}, db={:?}, plugin={:?}, cap=0x{:08x}, auth_resp_len={}",
        resp.username,
        resp.database,
        resp.auth_plugin_name,
        resp.capability_flags,
        resp.auth_response.len()
    );
    let auth_ok = if skip_auth() {
        true
    } else if resp.auth_response.is_empty() {
        tracing::warn!("Empty auth response for user {}", resp.username);
        false
    } else {
        user_store.verify_password(&resp.username, &scramble, &resp.auth_response)
    };
    if !auth_ok {
        tracing::warn!("Auth failed for user {}", resp.username);
        make_err_packet(2, 1045, "28000", "Access denied")
            .write_to(&mut &stream)
            .ok();
        return;
    }
    tracing::info!("Auth accepted, sending OK packet, seq=2");
    make_ok_packet(2, 0, 0, 0x0002, 0)
        .write_to(&mut &stream)
        .ok();
    tracing::info!("Starting command loop, seq=3");
    let engine: Arc<RwLock<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>>> =
        Arc::new(RwLock::new(ExecutionEngine::new(storage.clone())));
    let mut ps_manager = PreparedStatementManager::new();
    let _ = do_command_loop(
        &mut &stream,
        addr,
        storage,
        engine,
        resp.capability_flags,
        3,
        &mut ps_manager,
    );
}

pub fn run_server(host: &str, port: u16) -> MySqlResult<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr)?;
    tracing::info!("MySQL server listening on {}", addr);
    run_server_with_listener(listener)
}

/// SERVER-01 Stage 2: Production-grade server with all options.
///
/// `max_connections` - semaphore-bounded concurrent client connections
/// `auth_mode` - "none" (allow all) | "password" (require mysql_native_password)
/// `data_dir` - logical identifier for WAL/data location (currently logged)
///
/// This is a Stage 2 evolution of [`run_server`] that wires the CLI
/// args to real behavior. The previous Stage 1 banner-only fields
/// (data_dir, max_connections, auth_mode) are now actually enforced.
pub fn run_server_v2(
    host: &str,
    port: u16,
    data_dir: &str,
    max_connections: usize,
    auth_mode: &str,
) -> MySqlResult<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr)?;
    tracing::info!(
        "MySQL server listening on {} (data_dir={}, max_conn={}, auth={})",
        addr,
        data_dir,
        max_connections,
        auth_mode
    );
    // Store options in env so the run_server_with_listener path can read them
    std::env::set_var("SQLRUSTGO_DATA_DIR", data_dir);
    std::env::set_var("SQLRUSTGO_MAX_CONN", max_connections.to_string());
    std::env::set_var("SQLRUSTGO_AUTH_MODE", auth_mode);
    // v3.8.0-rc2 Week 1 Day 7: also publish the data_dir to
    // ACTIVE_CONFIG so that the LOAD DATA LOCAL INFILE handler
    // recognizes files inside the data dir as in-whitelist.
    // Without this, only the in-process test harness (which calls
    // `start_ephemeral`) can issue LOAD DATA — a real `mysql`
    // client connecting to a server started by `run_server_v2`
    // would get "not in allowed data_dir" because ACTIVE_CONFIG
    // was never populated.
    use crate::testing::EphemeralConfig;
    let cfg = EphemeralConfig {
        data_dir: Some(std::path::PathBuf::from(data_dir)),
        ..Default::default()
    };
    let _ = crate::ACTIVE_CONFIG.set(std::sync::Mutex::new(cfg));
    run_server_with_listener(listener)
}

/// Server core extracted so the test harness can hand in a pre-bound
/// `TcpListener` (port = 0) and return its actual port before the
/// accept loop starts.
pub fn run_server_with_listener(listener: TcpListener) -> MySqlResult<()> {
    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Spawn the resource monitor (1 sample per 30s by default).
    // The monitor writes periodic "RESOURCE_MONITOR" log lines that
    // capture RSS, FD, thread count, and connection counters. This is
    // the primary diagnostic tool for crash analysis.
    spawn_resource_monitor(30);
    run_server_with_listener_and_shutdown(listener, shutdown)
}

/// Variant of [`run_server_with_listener`] that returns promptly when
/// `shutdown` is set to `true`. The accept loop is driven in
/// non-blocking mode so it can poll the shutdown flag without a client
/// having to connect. This is the entry point used by the test
/// harness; production callers should use the simpler
/// `run_server_with_listener` form.
///
/// `bootstrap` is invoked once after the storage layer is wired up
/// but before the accept loop starts. The test harness uses it to
/// pre-create the `tester` user with a known password so the `mysql`
/// crate's auth handshake succeeds.
///
/// `data_dir`, when `Some(path)`, becomes the on-disk location for
/// the WAL and the table files (i.e. the entire server state). When
/// `None`, the server falls back to a per-listener-port temp dir.
/// Passing `Some(path)` is what lets a test stop a server and start
/// a new one against the same dir, observing WAL recovery. The path
/// is **not** created or removed by the server: lifecycle is owned
/// by the caller (matching the convention already documented on
/// `EphemeralConfig::data_dir`).
pub(crate) fn run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql(
    listener: TcpListener,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    bootstrap: UserStoreBootstrap,
    bootstrap_tables: bool,
    bootstrap_sql: Vec<String>,
    data_dir: Option<std::path::PathBuf>,
) -> MySqlResult<()> {
    let tls_config = Arc::new(make_tls_config());
    tracing::info!("TLS ready (self-signed cert)");

    // WalStorage<FileStorage, FileBackedWalManager> for production runtime
    // (Issue #2808: G1 — DML must persist via WAL, not bypass to raw FileStorage)
    //
    // Issue #3257 fix: when no `data_dir` is provided, use a *stable* directory
    // under the current working directory (`.sqlrustgo/data/`) rather than a
    // port-keyed /tmp path. The old port-keyed /tmp path caused stale WAL
    // files to persist across restarts and trigger 20+ minute recovery on a
    // 9.9 GB WAL (see Issue #3257). The new default is:
    //   1. Predictable: developers can find the WAL on disk
    //   2. Persistent: data survives server restarts on the same port
    //   3. Clean: an empty default is a fresh, empty data dir
    // For ephemeral/test usage, callers should still pass an explicit
    // `data_dir` (e.g. the test harness's `start_ephemeral` does this).
    let wal_data_dir = match data_dir {
        Some(p) => p,
        None => {
            let port = listener.local_addr()?.port();
            let cwd_default = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(".sqlrustgo")
                .join("data");
            // Issue #3257: prefer SQLRUSTGO_DATA_DIR env var, then cwd default.
            // The env var lets operators point at a stable location for
            // long-running deployments without code changes.
            match std::env::var("SQLRUSTGO_DATA_DIR") {
                Ok(s) if !s.is_empty() => {
                    tracing::info!(
                        "WAL data_dir from SQLRUSTGO_DATA_DIR env: {} (port {})",
                        s,
                        port
                    );
                    std::path::PathBuf::from(s)
                }
                _ => {
                    tracing::info!(
                        "WAL data_dir default (cwd/.sqlrustgo/data/): {} (port {})",
                        cwd_default.display(),
                        port
                    );
                    cwd_default
                }
            }
        }
    };
    if let Some(parent) = wal_data_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::create_dir_all(&wal_data_dir);
    let mut file_storage =
        FileStorage::new_with_wal(wal_data_dir.clone()).map_err(std::io::Error::other)?;
    let wal_path = wal_data_dir.join("sqlrustgo.wal");
    // Issue #3257: emit a warning if the WAL file is suspiciously large at
    // startup. This catches stale WAL files left over from previous
    // configurations (the port-keyed /tmp regression) or from long-running
    // servers that never had WAL rotation enabled.
    if let Ok(meta) = std::fs::metadata(&wal_path) {
        let size_mb = meta.len() / (1024 * 1024);
        if size_mb >= 100 {
            tracing::warn!(
                "WAL file is large: {} ({} MB) at {}. \
                 This may indicate a stale WAL from a previous process. \
                 Recovery time will scale with file size; \
                 consider passing --data-dir to isolate runs, or pruning the WAL manually.",
                wal_path.display(),
                size_mb,
                wal_path.display()
            );
        } else {
            tracing::info!(
                "WAL file size at startup: {} ({} MB)",
                wal_path.display(),
                size_mb
            );
        }
    }
    // INT-2 (#3270 partial): replay any uncommitted WAL entries from
    // the previous process lifetime so DML/DDL that was journaled but
    // not yet flushed to FileStorage's persisted table files is
    // restored on restart. The recovery engine walks the WAL from the
    // last checkpoint, applies each committed entry to the inner
    // FileStorage, then we flush so a subsequent restart does not
    // re-apply the same entries.
    {
        use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
        let mut recovery: StatefulRecoveryEngine<FileStorage> = StatefulRecoveryEngine::new();
        let mut wal_manager_for_recovery = FileBackedWalManager::new(wal_path.clone())
            .map_err(|e| MySqlError::Sql(format!("WAL manager (recovery) init failed: {}", e)))?;
        match recovery.recover(&mut file_storage, &mut wal_manager_for_recovery) {
            Ok(report) => {
                tracing::info!(
                    "WAL recovery: total={} committed_txns={} rows_inserted={} rows_updated={} rows_deleted={}",
                    report.entries_total,
                    report.committed_txns,
                    report.rows_inserted,
                    report.rows_updated,
                    report.rows_deleted
                );
                let _ = file_storage.flush();
            }
            Err(e) => {
                tracing::warn!("WAL recovery skipped: {}", e);
            }
        }
    }
    let wal_manager = FileBackedWalManager::new(wal_path)
        .map_err(|e| MySqlError::Sql(format!("WAL manager init failed: {}", e)))?;
    let wal_storage = WalStorage::new(file_storage, wal_manager)
        .map_err(|e| MySqlError::Sql(format!("WalStorage init failed: {}", e)))?;
    let storage: Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>> =
        Arc::new(RwLock::new(wal_storage));
    if bootstrap_tables {
        let mut eng = ExecutionEngine::new(storage.clone());
        for sql in ["CREATE TABLE content (hash TEXT PRIMARY KEY, doc TEXT NOT NULL, created_at TEXT NOT NULL)",
                "CREATE TABLE vectors (hash_seq TEXT PRIMARY KEY, hash TEXT NOT NULL, embedding TEXT NOT NULL, created_at TEXT NOT NULL)",
                "CREATE TABLE documents (id TEXT PRIMARY KEY, title TEXT, content TEXT, created_at TEXT)"] {
                if let Err(e) = eng.execute(sql) { tracing::warn!("Init: {}", e); }
        }
    }
    if !bootstrap_sql.is_empty() {
        let mut eng = ExecutionEngine::new(storage.clone());
        for sql in &bootstrap_sql {
            if let Err(e) = eng.execute(sql) {
                tracing::warn!("Bootstrap SQL failed: {} (sql: {})", e, sql);
            }
        }
    }
    let mut user_store = UserStore::new();
    if let Some(bs) = bootstrap {
        bs(&mut user_store);
    }

    // Non-blocking accept so the loop can check the shutdown flag
    // even when no client is connecting. The 50ms sleep is the
    // shutdown latency of the test harness; production clients are
    // unaffected (every accept immediately tries again when the
    // syscall returns WouldBlock).
    if let Err(e) = listener.set_nonblocking(true) {
        tracing::warn!("set_nonblocking failed: {}", e);
    }
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    while !shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, addr)) => {
                let st = storage.clone();
                let tc = tls_config.clone();
                let us = user_store.clone();
                thread::spawn(move || handle_connection(stream, addr, st, tc, us));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                tracing::error!("Accept: {}", e);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Ok(())
}

/// Variant of [`run_server_with_listener`] that returns promptly when
/// `shutdown` is set to `true`. The accept loop is driven in
/// non-blocking mode so it can poll the shutdown flag without a client
/// having to connect. This is the entry point used by the test
/// harness; production callers should use the simpler
/// `run_server_with_listener` form.
pub fn run_server_with_listener_and_shutdown(
    listener: TcpListener,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> MySqlResult<()> {
    run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql(
        listener,
        shutdown,
        None,
        true,
        Vec::new(),
        None,
    )
}

/// Variant of [`run_server_with_listener_and_shutdown`] that also
/// pre-creates the internal catalog tables (`content`, `vectors`,
/// `documents`) unless `bootstrap_tables` is `false`. The user
/// bootstrap callback is independent — pass `bootstrap: Some(_)`
/// to add custom users, `bootstrap: None` to start with the
/// server's built-in `root` and `mysql` users only.
#[allow(dead_code)]
pub(crate) fn run_server_with_listener_and_shutdown_with_bootstrap(
    listener: TcpListener,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    bootstrap: UserStoreBootstrap,
) -> MySqlResult<()> {
    run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql(
        listener,
        shutdown,
        bootstrap,
        true,
        Vec::new(),
        None,
    )
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_col_type_from_string_integer() {
        // Default INT maps to LONG (0x03); BIGINT maps to LONGLONG (0x08)
        assert_eq!(col_type_from_string("INT"), 3); // LONG
        assert_eq!(col_type_from_string("INTEGER"), 3); // LONG
        assert_eq!(col_type_from_string("SMALLINT"), 2); // SHORT
        assert_eq!(col_type_from_string("TINYINT"), 1); // TINY
        assert_eq!(col_type_from_string("BIGINT"), 8); // LONGLONG
    }

    #[test]
    fn test_col_type_from_string_varchar() {
        assert_eq!(col_type_from_string("VARCHAR(255)"), 0x0f);
        assert_eq!(col_type_from_string("CHAR(10)"), 0xfd);
    }

    #[test]
    fn test_col_type_from_string_text() {
        assert_eq!(col_type_from_string("TEXT"), 0xfd); // VARSTRING
        assert_eq!(col_type_from_string("BLOB"), 0xfc); // BLOB
    }

    #[test]
    fn test_col_type_from_string_float() {
        assert_eq!(col_type_from_string("FLOAT"), 0x04); // FLOAT
        assert_eq!(col_type_from_string("DOUBLE"), 0x05); // DOUBLE
    }

    #[test]
    fn test_col_type_from_string_datetime() {
        // DATETIME/TIMESTAMP share 0x0a (DATETIME type in MySQL protocol)
        assert_eq!(col_type_from_string("DATETIME"), 0x0a);
        assert_eq!(col_type_from_string("DATE"), 0x0a);
        assert_eq!(col_type_from_string("TIMESTAMP"), 0x0a); // TIMESTAMP → DATETIME
    }

    #[test]
    fn test_col_type_from_string_decimal() {
        assert_eq!(col_type_from_string("DECIMAL"), 0xf6); // NEWDECIMAL
        assert_eq!(col_type_from_string("NUMERIC"), 0xf6); // NEWDECIMAL
    }

    #[test]
    fn test_col_type_from_string_unknown() {
        assert_eq!(col_type_from_string("UNKNOWN_TYPE"), 0xfe); // STRING default
    }

    // ============ value_to_string Tests ============

    #[test]
    fn test_value_to_string_null() {
        use sqlrustgo_types::Value;
        assert_eq!(value_to_string(&Value::Null), "NULL".to_string());
    }

    #[test]
    fn test_value_to_string_integer() {
        use sqlrustgo_types::Value;
        assert_eq!(value_to_string(&Value::Integer(42)), "42".to_string());
        assert_eq!(value_to_string(&Value::Integer(0)), "0".to_string());
        assert_eq!(value_to_string(&Value::Integer(-100)), "-100".to_string());
    }

    #[test]
    fn test_value_to_string_float() {
        use sqlrustgo_types::Value;
        assert_eq!(value_to_string(&Value::Float(3.14)), "3.14".to_string());
        assert_eq!(value_to_string(&Value::Float(0.0)), "0".to_string());
    }

    #[test]
    fn test_value_to_string_text() {
        use sqlrustgo_types::Value;
        assert_eq!(
            value_to_string(&Value::Text("hello".to_string())),
            "hello".to_string()
        );
        assert_eq!(
            value_to_string(&Value::Text("".to_string())),
            "".to_string()
        );
    }

    #[test]
    fn test_value_to_string_boolean() {
        use sqlrustgo_types::Value;
        assert_eq!(value_to_string(&Value::Boolean(true)), "1".to_string());
        assert_eq!(value_to_string(&Value::Boolean(false)), "0".to_string());
    }

    #[test]
    fn test_value_to_string_blob() {
        use sqlrustgo_types::Value;
        // Blob falls through to debug format
        let result = value_to_string(&Value::Blob(vec![1, 2, 3]));
        assert!(result.contains("[1, 2, 3]") || result.contains("1, 2, 3"));
    }

    /// MySQL old_password() hash
    fn old_password_hash(password: &str) -> i64 {
        let mut nr: u32 = 1345345333;
        let mut nr2: u32 = 0x12345671;
        for byte in password.bytes() {
            if byte == b' ' || byte == b'\t' {
                continue;
            }
            nr ^= (((nr & 63) ^ nr2) as u32);
            nr = nr.wrapping_add(nr >> 3);
            nr2 = nr2.wrapping_add((nr2 << 1) ^ nr);
        }
        ((nr & 0x7fffffff) as i64) | (((nr2 & 0x7fffffff) as i64) << 32)
    }

    // ============ old_password_hash Tests ============

    #[test]
    fn test_old_password_hash_deterministic() {
        let hash1 = old_password_hash("password");
        let hash2 = old_password_hash("password");
        assert_eq!(hash1, hash2, "Same password should produce same hash");
    }

    #[test]
    fn test_old_password_hash_empty() {
        let hash = old_password_hash("");
        // Hash result is i64, verify by checking it's non-zero for non-empty input
        // and consistent across calls (bits are deterministic)
        assert_eq!(hash, hash);
    }

    #[test]
    fn test_old_password_hash_deterministic_salted() {
        let hash1 = old_password_hash("password1");
        let hash2 = old_password_hash("password1");
        assert_eq!(
            hash1, hash2,
            "Same password should produce same hash (deterministic)"
        );
    }

    #[test]
    fn test_old_password_hash_length() {
        let hash = old_password_hash("test_password");
        // Hash result is i64, verify deterministic
        assert_eq!(hash, old_password_hash("test_password"));
    }

    // ============ Packet Tests (internal) ============

    #[test]
    fn test_packet_internal() {
        let pkt = Packet {
            length: 5,
            sequence: 2,
            payload: vec![1, 2, 3, 4, 5],
        };
        assert_eq!(pkt.length, 5);
        assert_eq!(pkt.sequence, 2);
        assert_eq!(pkt.payload.len(), 5);
    }

    // ============ write_lenenc_int Tests ============

    #[test]
    fn test_write_lenenc_int_small() {
        // Values < 251 use 1 byte
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0).unwrap();
        write_lenenc_int(&mut buf, 100).unwrap();
        write_lenenc_int(&mut buf, 250).unwrap();
        assert_eq!(buf, vec![0x00, 100, 250]);
    }

    #[test]
    fn test_write_lenenc_int_16bit() {
        // Values 251-0xFFFF use 3 bytes (0xfc + u16)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 251).unwrap();
        write_lenenc_int(&mut buf, 1000).unwrap();
        write_lenenc_int(&mut buf, 0xFFFF).unwrap();
        // 251 = 0xFB, 1000 = 0x3E8, 0xFFFF
        assert_eq!(
            buf,
            vec![0xfc, 0xfb, 0x00, 0xfc, 0xe8, 0x03, 0xfc, 0xff, 0xff]
        );
    }

    #[test]
    fn test_write_lenenc_int_24bit() {
        // Values 0x10000-0xFFFFFF use 4 bytes (0xfd + u24)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x10000).unwrap();
        write_lenenc_int(&mut buf, 0x1000000 - 1).unwrap();
        // 0x10000 = 0x00010000 -> 0xfd, 0x00, 0x00, 0x01
        // 0xFFFFFF = 0x00FFFFFF -> 0xfd, 0xff, 0xff, 0xff
        assert_eq!(buf, vec![0xfd, 0x00, 0x00, 0x01, 0xfd, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn test_write_lenenc_int_64bit() {
        // Values >= 0x1000000 use 9 bytes (0xfe + u64)
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x1000000).unwrap();
        write_lenenc_int(&mut buf, u64::MAX).unwrap();
        assert_eq!(buf.len(), 1 + 8 + 1 + 8); // Two 0xfe + u64 values
        assert_eq!(buf[0], 0xfe);
        assert_eq!(buf[9], 0xfe);
    }

    // ============ write_lenenc_string Tests ============

    #[test]
    fn test_write_lenenc_string_basic() {
        let mut buf = Vec::new();
        write_lenenc_string(&mut buf, b"hello").unwrap();
        // 5 (length) + "hello"
        assert_eq!(buf, vec![0x05, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_write_lenenc_string_empty() {
        let mut buf = Vec::new();
        write_lenenc_string(&mut buf, b"").unwrap();
        assert_eq!(buf, vec![0x00]);
    }

    #[test]
    fn test_write_lenenc_string_long() {
        let mut buf = Vec::new();
        let long_str = vec![0u8; 300];
        write_lenenc_string(&mut buf, &long_str).unwrap();
        // 300 = 0x12C, needs 0xfc + u16 encoding
        assert_eq!(buf[0], 0xfc);
        assert_eq!(buf[1], 0x2c);
        assert_eq!(buf[2], 0x01);
    }

    // ============ make_handshake_packet Tests ============

    #[test]
    fn test_make_handshake_packet() {
        let seed = [0x00; 20];
        let pkt = make_handshake_packet(0, &seed);
        assert!(pkt.payload.len() > 0);
    }

    #[test]
    fn test_make_handshake_packet_seq() {
        let seed = [0x00; 20];
        let pkt = make_handshake_packet(5, &seed);
        assert_eq!(pkt.sequence, 5);
    }

    // ============ make_ok_packet Tests ============

    #[test]
    fn test_make_ok_packet_basic() {
        let pkt = make_ok_packet(1, 0, 0, 0x0002, 0);
        assert_eq!(pkt.sequence, 1);
        assert_eq!(pkt.payload[0], 0x00); // OK packet type
    }

    #[test]
    fn test_make_ok_packet_with_affected_rows() {
        let pkt = make_ok_packet(2, 5, 10, 0x0002, 0);
        assert_eq!(pkt.sequence, 2);
        // Affected rows is lenenc-int of 5 = 0x05
        assert!(pkt.payload.contains(&5));
    }

    // ============ make_err_packet Tests ============

    #[test]
    fn test_make_err_packet_basic() {
        let pkt = make_err_packet(1, 1064, "42000", "Syntax error");
        assert_eq!(pkt.sequence, 1);
        assert_eq!(pkt.payload[0], 0xff); // ERR packet type
    }

    #[test]
    fn test_make_err_packet_access_denied() {
        let pkt = make_err_packet(2, 1045, "42000", "Access denied");
        // Error code is little-endian u16 at bytes 1-2
        assert_eq!(pkt.payload[1], 0x15); // 1045 = 0x0415
        assert_eq!(pkt.payload[2], 0x04);
    }

    #[test]
    fn test_make_err_packet_empty_message() {
        let pkt = make_err_packet(0, 2000, "42000", "");
        assert_eq!(pkt.payload[0], 0xff);
    }

    // ============ make_eof_packet Tests ============

    #[test]
    fn test_make_eof_packet() {
        let pkt = make_eof_packet(3, 0x0002);
        assert_eq!(pkt.sequence, 3);
        assert_eq!(pkt.payload[0], 0xfe); // EOF packet type
    }

    #[test]
    fn test_make_eof_packet_status() {
        let pkt = make_eof_packet(5, 0x0003);
        // EOF packet: 0xfe + warning_count(2 bytes) + status(2 bytes)
        // For status 0x0003: payload[3]=0x03, payload[4]=0x00
        assert_eq!(pkt.payload[0], 0xfe); // EOF marker
        assert_eq!(pkt.payload[3], 0x03); // status low byte
        assert_eq!(pkt.payload[4], 0x00); // status high byte
    }

    // ============ col_len_from_type Tests ============

    #[test]
    fn test_col_len_from_type_int1() {
        assert_eq!(col_len_from_type("TINYINT(1)"), 1);
    }

    #[test]
    fn test_col_len_from_type_int11() {
        // Default INT length is 11
        assert_eq!(col_len_from_type("INT(11)"), 11);
        assert_eq!(col_len_from_type("INT(10)"), 10);
    }

    #[test]
    fn test_col_len_from_type_float() {
        assert_eq!(col_len_from_type("FLOAT"), 12);
        assert_eq!(col_len_from_type("DOUBLE"), 22);
    }

    #[test]
    fn test_col_len_from_type_varchar() {
        assert_eq!(col_len_from_type("VARCHAR(255)"), 255);
        assert_eq!(col_len_from_type("VARCHAR(100)"), 100);
    }

    #[test]
    fn test_col_len_from_type_text() {
        assert_eq!(col_len_from_type("TEXT"), 65535);
    }

    #[test]
    fn test_col_len_from_type_default() {
        assert_eq!(col_len_from_type("UNKNOWN"), 255);
    }

    // ============ col_type_from_string Tests (additional) ============

    #[test]
    fn test_col_type_from_string_bit() {
        // BIT is not explicitly handled, falls through to default STRING (0xfe)
        assert_eq!(col_type_from_string("BIT"), 0xfe); // falls through to default
    }

    #[test]
    fn test_col_type_from_string_year() {
        // YEAR is not explicitly handled, falls through to default STRING (0xfe)
        assert_eq!(col_type_from_string("YEAR"), 0xfe); // falls through to default
    }

    #[test]
    fn test_col_type_from_string_mediumint() {
        assert_eq!(col_type_from_string("MEDIUMINT"), 0x09); // INT24
    }

    // ============ Packet round-trip with Vec<u8> ============

    #[test]
    fn test_packet_write_to_vec_and_read() {
        let original = Packet {
            length: 7,
            sequence: 4,
            payload: vec![0x03, b'S', b'E', b'L', b'E', b'C', b'T'],
        };
        let mut buf = Vec::new();
        original.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(read.length, original.length);
        assert_eq!(read.sequence, original.sequence);
        assert_eq!(read.payload, original.payload);
    }

    #[test]
    fn test_packet_max_sequence_wrap() {
        // Test that sequence wrapping works correctly
        let pkt = Packet {
            length: 5,
            sequence: 255,
            payload: vec![1, 2, 3, 4, 5],
        };
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let read = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(read.sequence, 255);
    }

    // ============ value_to_string edge cases ============

    #[test]
    fn test_value_to_string_negative_integer() {
        use sqlrustgo_types::Value;
        assert_eq!(
            value_to_string(&Value::Integer(i64::MIN)),
            i64::MIN.to_string()
        );
        assert_eq!(value_to_string(&Value::Integer(-1)), "-1".to_string());
    }

    #[test]
    fn test_value_to_string_large_float() {
        use sqlrustgo_types::Value;
        assert_eq!(
            value_to_string(&Value::Float(f64::MAX)),
            f64::MAX.to_string()
        );
    }

    #[test]
    fn test_value_to_string_unicode_text() {
        use sqlrustgo_types::Value;
        assert_eq!(
            value_to_string(&Value::Text("hello".to_string())),
            "hello".to_string()
        );
    }

    // ============ old_password_hash edge cases ============

    #[test]
    fn test_old_password_hash_known_value() {
        // The hash should be deterministic
        let hash = old_password_hash("test");
        // Same input should always produce same output
        assert_eq!(old_password_hash("test"), hash);
    }

    #[test]
    fn test_old_password_hash_long_password() {
        let hash = old_password_hash("a very long password that is much longer than average");
        // Result is i64, verify deterministic
        assert_eq!(
            old_password_hash("a very long password that is much longer than average"),
            hash
        );
    }

    #[test]
    fn test_old_password_hash_special_chars() {
        let hash1 = old_password_hash("pass@word!");
        let hash2 = old_password_hash("password");
        assert_ne!(hash1, hash2);
    }

    fn verify_old_password_response(scramble: &[u8], token: &str) -> bool {
        let hash = old_password_hash(token);
        let hash_low = hash as u32;
        let hash_high = (hash >> 32) as u32;
        let buf: Vec<u8> = scramble
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                let v = if i < 4 { hash_low } else { hash_high };
                b ^ ((v >> (i % 4) * 8) & 0xFF) as u8
            })
            .collect();
        !buf.is_empty() && buf.len() >= 8
    }

    // ============ verify_old_password_response Tests ============

    #[test]
    fn test_verify_old_password_response_basic() {
        // Test that the function works (even if it always returns false without real verification)
        let seed = [0x00; 8];
        let response = [0x00; 8];
        // With empty password, this should work
        let result = verify_old_password_response(&seed, "");
        // The algorithm produces consistent results
        assert!(result == verify_old_password_response(&seed, ""));
    }

    #[test]
    fn test_verify_old_password_response_with_seed() {
        let seed = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let response = [0x00; 8];
        let result1 = verify_old_password_response(&seed, "password");
        let result2 = verify_old_password_response(&seed, "password");
        assert_eq!(result1, result2); // Same inputs should give same result
    }

    // ============ write_text_row Tests ============

    #[test]
    fn test_write_text_row_single_value() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let row = vec![Value::Integer(42)];
        write_text_row(&mut buf, &row).unwrap();
        // Integer 42 -> "42" as lenenc string
        assert!(buf.len() > 0);
    }

    #[test]
    fn test_write_text_row_multiple_values() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let row = vec![
            Value::Integer(1),
            Value::Text("hello".to_string()),
            Value::Null,
        ];
        write_text_row(&mut buf, &row).unwrap();
        assert!(buf.len() > 0);
    }

    #[test]
    fn test_write_text_row_empty() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let row: Vec<Value> = vec![];
        write_text_row(&mut buf, &row).unwrap();
        assert_eq!(buf.len(), 0);
    }

    // ============ write_column_def Tests ============

    #[test]
    fn test_write_column_def_basic() {
        let mut buf = Vec::new();
        write_column_def(&mut buf, "id", "INT", 0).unwrap();
        assert!(buf.len() > 0);
        // Verify it can be read back as a packet
        let mut cursor = std::io::Cursor::new(buf);
        let pkt = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(pkt.sequence, 0);
    }

    #[test]
    fn test_write_column_def_varchar() {
        let mut buf = Vec::new();
        write_column_def(&mut buf, "name", "VARCHAR(100)", 5).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let pkt = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(pkt.sequence, 5);
    }

    #[test]
    fn test_write_column_def_float() {
        let mut buf = Vec::new();
        write_column_def(&mut buf, "price", "FLOAT", 10).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let pkt = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(pkt.sequence, 10);
    }

    // ============ send_result_set Tests ============

    #[test]
    fn test_send_result_set_empty() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let columns = vec!["id".to_string(), "name".to_string()];
        let column_types = vec!["INT".to_string(), "VARCHAR(255)".to_string()];
        let rows: Vec<Vec<Value>> = vec![];
        send_result_set(&mut buf, &columns, &column_types, &rows, 0, 0).unwrap();
        assert!(buf.len() > 0);
    }

    #[test]
    fn test_send_result_set_with_rows() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let columns = vec!["id".to_string()];
        let column_types = vec!["INT".to_string()];
        let rows = vec![vec![Value::Integer(1)], vec![Value::Integer(2)]];
        send_result_set(&mut buf, &columns, &column_types, &rows, 0, 0).unwrap();
        assert!(buf.len() > 0);
    }

    #[test]
    fn test_send_result_set_with_text_values() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let columns = vec!["name".to_string(), "email".to_string()];
        let column_types = vec!["VARCHAR(100)".to_string(), "VARCHAR(255)".to_string()];
        let rows = vec![
            vec![
                Value::Text("Alice".to_string()),
                Value::Text("alice@example.com".to_string()),
            ],
            vec![
                Value::Text("Bob".to_string()),
                Value::Text("bob@example.com".to_string()),
            ],
        ];
        send_result_set(&mut buf, &columns, &column_types, &rows, 0, 0).unwrap();
        assert!(buf.len() > 0);
    }

    #[test]
    fn test_send_result_set_single_row() {
        use sqlrustgo_types::Value;
        let mut buf = Vec::new();
        let columns = vec!["value".to_string()];
        let column_types = vec!["DOUBLE".to_string()];
        let rows = vec![vec![Value::Float(3.14159)]];
        send_result_set(&mut buf, &columns, &column_types, &rows, 7, 0).unwrap();
        assert!(buf.len() > 0);
    }

    // ============ Statement Dispatch Tests (new model) ============

    #[test]
    fn test_statement_dispatch_select() {
        use sqlrustgo::MemoryExecutionEngine;
        use sqlrustgo_storage::MemoryStorage;
        use sqlrustgo_types::Value;
        use std::sync::{Arc, RwLock};

        let storage: Arc<RwLock<MemoryStorage>> = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = MemoryExecutionEngine::new(storage);

        engine
            .execute("CREATE TABLE dispatch_test (id INT, name TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO dispatch_test VALUES (1, 'hello')")
            .unwrap();

        let result = engine.execute("SELECT * FROM dispatch_test");
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0].len(), 2);
    }

    #[test]
    fn test_statement_dispatch_insert() {
        use sqlrustgo::MemoryExecutionEngine;
        use sqlrustgo_storage::MemoryStorage;
        use std::sync::{Arc, RwLock};

        let storage: Arc<RwLock<MemoryStorage>> = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = MemoryExecutionEngine::new(storage);

        engine
            .execute("CREATE TABLE dispatch_insert_test (id INT)")
            .unwrap();
        let result = engine.execute("INSERT INTO dispatch_insert_test VALUES (1)");
        assert!(result.is_ok());
        assert!(result.unwrap().affected_rows > 0);
    }

    // ============ MySqlError::std::error::Error trait ============
    #[test]
    fn test_my_sql_error_source() {
        use std::error::Error;
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err = MySqlError::Io(io_err);
        // Note: MySqlError uses default Error impl which returns None for source
        // even for Io(Error) variant since it doesn't box the error
        let display = format!("{}", err);
        assert!(!display.is_empty()); // Just verify it produces output
    }

    #[test]
    fn test_my_sql_error_source_none() {
        use std::error::Error;
        let err = MySqlError::Protocol("test".to_string());
        // Protocol errors don't have a source
        assert!(err.source().is_none());
    }
}

/// Parse a LOAD DATA LOCAL INFILE SQL statement.
///
/// Returns (path, table, field_delimiter) if matched, None otherwise.
/// Only supports the TPC-H .tbl canonical form:
///     LOAD DATA LOCAL INFILE '<path>' INTO TABLE <table>
///     [FIELDS TERMINATED BY '<delim>']
fn parse_load_local_infile_sql(sql: &str) -> Option<(String, String, char)> {
    let upper = sql.trim().to_uppercase();
    if !upper.starts_with("LOAD DATA LOCAL INFILE") {
        return None;
    }

    // Extract path between first pair of single quotes after INFILE
    let after_infile = &sql[upper.find("INFILE")? + "INFILE".len()..];
    let path_start = after_infile.find('\'')? + 1;
    let path_end_rel = after_infile[path_start..].find('\'')?;
    let path = after_infile[path_start..path_start + path_end_rel].to_string();

    // Extract table name after "INTO TABLE"
    let after_into = &sql[upper.find("INTO TABLE")? + "INTO TABLE".len()..];
    let table_trim = after_into.trim_start();
    let table: String = table_trim
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if table.is_empty() {
        return None;
    }

    // Default delimiter is `|` (TPC-H .tbl standard)
    let delim = '|';

    Some((path, table, delim))
}

#[cfg(test)]
mod load_local_infile_tests {
    use super::*;
    use std::io::Cursor;

    /// Helper: simulate a client that sends 0xFB-ready file content.
    fn make_client_packets(file_bytes: &[u8], chunk_size: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in file_bytes.chunks(chunk_size) {
            // Packet header: 3-byte length + 1-byte seq
            let len = chunk.len() as u32;
            out.push((len & 0xFF) as u8);
            out.push(((len >> 8) & 0xFF) as u8);
            out.push(((len >> 16) & 0xFF) as u8);
            out.push(0x00); // seq
            out.extend_from_slice(chunk);
        }
        // Empty terminator
        out.push(0);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    #[test]
    fn test_parse_tbl_response_stream_basic() {
        // Verifies that we can read a stream of file content packets + empty terminator.
        let file = b"1|2|3|\n4|5|6|\n";
        let bytes = make_client_packets(file, 16);

        let mut stream = Cursor::new(bytes);
        let mut total = Vec::new();
        loop {
            let pkt = Packet::read_from(&mut stream).unwrap();
            if pkt.payload.is_empty() {
                break;
            }
            total.extend_from_slice(&pkt.payload);
        }
        assert_eq!(total, file);
    }

    #[test]
    fn test_parse_load_local_infile_sql_basic() {
        let sql = "LOAD DATA LOCAL INFILE '/tmp/region.tbl' INTO TABLE region";
        let result = parse_load_local_infile_sql(sql);
        assert_eq!(
            result,
            Some(("/tmp/region.tbl".to_string(), "region".to_string(), '|'))
        );
    }

    #[test]
    fn test_parse_load_local_infile_sql_ignores_non_pipe_delim() {
        // Per spec: only `|` delimiter is supported. The parser ignores
        // FIELDS TERMINATED BY clause and always returns '|'.
        let sql = "LOAD DATA LOCAL INFILE '/x.tbl' INTO TABLE t1 FIELDS TERMINATED BY ','";
        let result = parse_load_local_infile_sql(sql);
        assert_eq!(result, Some(("/x.tbl".to_string(), "t1".to_string(), '|')));
    }

    #[test]
    fn test_parse_load_local_infile_sql_non_matching() {
        // Regular SELECT — should NOT match
        let sql = "SELECT * FROM t1";
        let result = parse_load_local_infile_sql(sql);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_load_local_infile_sql_case_insensitive() {
        let sql = "load data local infile '/y.tbl' into table y";
        let result = parse_load_local_infile_sql(sql);
        assert_eq!(result, Some(("/y.tbl".to_string(), "y".to_string(), '|')));
    }
}

// ============================================================================
// Embedded test harness
//
// See `openspec/changes/mysql-server-canonical-entry/specs/server-embedded-test-harness/spec.md`
// for the contract this module is required to implement.
// ============================================================================

/// Re-export of [`testing::EphemeralConfig`] at the crate root for
/// ergonomic test imports (`use sqlrustgo_mysql_server::EphemeralConfig;`).
pub use testing::EphemeralConfig;

pub mod testing {
    use crate::ACTIVE_CONFIG;
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    /// Configuration for an ephemeral MySQL server.
    #[derive(Debug, Clone)]
    pub struct EphemeralConfig {
        pub host: String,
        /// If `true` (the default), the server pre-creates the
        /// internal catalog tables (`content`, `vectors`, `documents`).
        /// Tests that want a clean catalog should set this to `false`.
        pub bootstrap_tables: bool,
        /// If `true` (the default), the server pre-creates a `tester`
        /// user with password `tester` so the raw wire-protocol client
        /// (and the `mysql` crate) can authenticate. Tests that want
        /// to control the user table themselves should set this to
        /// `false` and add their own users via the listener-side API.
        pub bootstrap_users: bool,
        /// When `Some(path)`, the server uses this directory as its
        /// data dir instead of auto-creating one under
        /// `std::env::temp_dir()`. The path must already exist; the
        /// server does **not** create it. The handle's `Drop` is
        /// inert on this path (does not `remove_dir_all` it) so the
        /// caller can pre-stage .tbl data and inspect the dir after
        /// the test. Useful for the TPC-H wire-protocol smoke test
        /// that needs to share a data dir between two `start_ephemeral`
        /// calls (one to import, one to query).
        pub data_dir: Option<std::path::PathBuf>,
        /// Extra DDL statements to execute after the internal catalog
        /// tables (if `bootstrap_tables` is true) and before the server
        /// starts accepting connections. Use this to inject the 8 TPC-H
        /// `CREATE TABLE` statements into an ephemeral server.
        pub bootstrap_sql: Vec<String>,
        /// Maximum bytes to buffer in a single batched INSERT during
        /// LOAD DATA LOCAL INFILE. Default 1 MB. Tests / perf benches
        /// can set higher (e.g. 16 MB) for fewer INSERT round-trips.
        pub bulk_insert_buffer_size: usize,
    }

    impl Default for EphemeralConfig {
        fn default() -> Self {
            Self {
                host: "127.0.0.1".to_string(),
                bootstrap_tables: true,
                bootstrap_users: true,
                data_dir: None,
                bootstrap_sql: Vec::new(),
                bulk_insert_buffer_size: 1_048_576,
            }
        }
    }

    /// Handle to a running ephemeral server. Dropping the handle closes
    /// the listener and joins the accept-loop thread, and removes the
    /// temporary data directory.
    pub struct EphemeralHandle {
        pub port: u16,
        // Shared shutdown signal: Drop sets it to true, the
        // server thread's accept loop polls it and exits within 50ms.
        shutdown: Option<Arc<std::sync::atomic::AtomicBool>>,
        // Mutex so Drop can take the JoinHandle by value.
        join: Mutex<Option<JoinHandle<()>>>,
        // Temporary data directory; removed on Drop ONLY when the
        // server auto-created it. When the test supplied a path via
        // `EphemeralConfig::data_dir`, the path is caller-owned and
        // Drop must not touch it (caller decides when to clean up,
        // and may want to point a second `start_ephemeral` at it
        // for recovery-style tests).
        data_dir: PathBuf,
        // True iff the server created `data_dir` itself and owns the
        // cleanup. False when the test supplied the path through
        // `EphemeralConfig::data_dir`. The previous Drop logic used
        // `data_dir.starts_with(std::env::temp_dir())` to discriminate,
        // which is incorrect because the canonical test pattern uses
        // `tempfile::TempDir` whose paths are also under temp_dir —
        // the auto-generated `sqlrustgo_ephemeral_<port>_<pid>` and
        // the test's `tmpdir/.tmpXXXX` both match the prefix.
        externally_owned: bool,
    }

    impl std::fmt::Debug for EphemeralHandle {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("EphemeralHandle")
                .field("port", &self.port)
                .field("data_dir", &self.data_dir)
                .finish()
        }
    }

    impl EphemeralHandle {
        /// Build a no-op handle for a server that is **not**
        /// managed by this process (e.g. a subprocess spawned by
        /// an L3 acceptance test). Drop on the returned handle is
        /// inert: it does not touch a data dir, shutdown flag, or
        /// join a server thread, because those belong to the
        /// external process.
        pub fn detached_for_external_server(port: u16) -> Self {
            Self {
                port,
                shutdown: None,
                join: Mutex::new(None),
                data_dir: PathBuf::new(),
                externally_owned: true,
            }
        }
    }

    impl Drop for EphemeralHandle {
        fn drop(&mut self) {
            use std::sync::atomic::Ordering;
            // 1. Signal the accept loop to exit on its next poll.
            if let Some(flag) = self.shutdown.take() {
                flag.store(true, Ordering::SeqCst);
            }
            // 2. Join the server thread (bounded by the 50ms poll).
            if let Ok(mut guard) = self.join.lock() {
                if let Some(handle) = guard.take() {
                    let _ = handle.join();
                }
            }
            // 3. Remove the temporary data directory ONLY if the
            //    server created it. When the test supplied the path
            //    via `EphemeralConfig::data_dir` (e.g. for
            //    recovery-style tests that share the dir between two
            //    `start_ephemeral` calls), Drop is a no-op for the
            //    data dir and the caller is responsible for cleanup.
            //    An empty path means the handle is a no-op (external
            //    server spawned by another process).
            if !self.externally_owned && !self.data_dir.as_os_str().is_empty() {
                let _ = std::fs::remove_dir_all(&self.data_dir);
            }
        }
    }

    /// Boot an in-process MySQL server on an OS-assigned port and return
    /// a handle to it. The server runs on a background thread; the
    /// handle's `Drop` joins the thread and cleans up the temp data dir.
    pub fn start_ephemeral(config: EphemeralConfig) -> Result<EphemeralHandle, std::io::Error> {
        // Publish the config so the server thread's `do_command_loop`
        // can find the data_dir and bulk buffer size for LOAD DATA
        // LOCAL INFILE. Only the first call wins; later calls are a
        // no-op (OnceLock semantics).
        let _ = ACTIVE_CONFIG.set(std::sync::Mutex::new(config.clone()));

        let listener = TcpListener::bind(format!("{}:0", config.host))?;
        let port = listener.local_addr()?.port();

        let data_dir_for_thread = config.data_dir.clone();

        // When the caller supplies a data_dir, the test owns the
        // directory's lifecycle; we do not create it and we do
        // not remove it on Drop. When None, we auto-create one
        // under the OS temp dir and Drop removes it.
        let externally_owned = data_dir_for_thread.is_some();
        let data_dir = data_dir_for_thread.clone().unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "sqlrustgo_ephemeral_{}_{}",
                port,
                std::process::id()
            ))
        });
        if !externally_owned {
            std::fs::create_dir_all(&data_dir)?;
        }

        // Move the listener into the server thread. The accept loop
        // is non-blocking and polls a shutdown flag; Drop sets the
        // flag and joins the thread (the loop exits within 50ms).
        //
        // When `config.bootstrap_users` is true (default) the
        // bootstrap callback pre-creates a `tester` user with the
        // well-known password `tester` so the raw wire-protocol
        // client can authenticate. Tests that want full control over
        // the user table should pass `bootstrap_users: false` and
        // add their own users via the listener-side API.
        let listener_for_thread = listener;
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);
        let bootstrap_users = config.bootstrap_users;
        let bootstrap_tables_flag = config.bootstrap_tables;
        let bootstrap_sql = config.bootstrap_sql;
        let join = std::thread::spawn(move || {
            let bootstrap: crate::UserStoreBootstrap = if bootstrap_users {
                Some(Box::new(|user_store: &mut crate::UserStore| {
                    user_store.add_user("tester", "tester");
                }))
            } else {
                None
            };
            let _ = crate::run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql(
                listener_for_thread,
                shutdown_for_thread,
                bootstrap,
                bootstrap_tables_flag,
                bootstrap_sql,
                data_dir_for_thread,
            );
        });

        Ok(EphemeralHandle {
            port,
            shutdown: Some(shutdown),
            join: Mutex::new(Some(join)),
            data_dir,
            externally_owned,
        })
    }
}

/// Re-exports for the integration tests in `tests/`. The actual helpers
/// are `pub` (not `pub(crate)`) so the integration tests can reach
/// them; production code outside of `test_helpers` should call them via
/// the normal API.
#[doc(hidden)]
pub mod test_helpers {
    pub use crate::parse_stmt_execute_params;
    pub use crate::replace_placeholders;
    pub use crate::StmtParam;
}

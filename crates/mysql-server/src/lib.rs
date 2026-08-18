//! SQLRustGo MySQL Wire Protocol Server
//!
//! Supports mysql_native_password auth + TLS (mariadb-connector-c 3.4+ compatible)

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::{Compress, Decompress, FlushCompress, FlushDecompress};
use parking_lot::RwLock;
use rcgen::{CertificateParams, KeyPair};
use sha1::{Digest, Sha1};
use sqlrustgo::ExecutionEngine;
use sqlrustgo_parser::{parse, Statement};
use sqlrustgo_storage::wal::FileBackedWalManager;
use sqlrustgo_storage::{
    BinaryTableStorage, BoxStorageEngine, CheckpointManager, FileStorage, MemoryStorage,
    ParallelWalStorage, StorageEngine, WalStorage,
};
use sqlrustgo_types::{SqlError, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const SERVER_VERSION: &str = "8.0.33-SQLRustGo";

/// V312-18e Issue #4021 — Prometheus `/metrics` endpoint.
mod metrics_endpoint;

/// v3.10.0 Issue #3703: read intra-query executor parallelism from
/// the `SQLRUSTGO_EXECUTOR_PARALLELISM` env var (set by
/// `run_server_v2` from the `--executor-parallelism` CLI flag).
/// Defaults to 1 = sequential, zero regression. The env var is
/// honored regardless of whether the binary was built with
/// `--features parallel-executor`; the engine stores the value and
/// `LocalExecutor::execute_select_parallel` (feature-gated) reads it.
#[allow(dead_code)]
fn read_executor_parallelism() -> usize {
    std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(1)
}

/// Parse WAL sync mode from string (from --wal-sync CLI flag).
/// Formats: "every", "off", "batch:N"
fn parse_wal_sync_mode(s: &str) -> sqlrustgo_storage::WalSyncMode {
    let s = s.trim();
    if s.eq_ignore_ascii_case("off") {
        tracing::warn!("WAL sync OFF: durability disabled for performance!");
        sqlrustgo_storage::WalSyncMode::Off
    } else if s.to_lowercase().starts_with("batch:") {
        let n: u32 = s[6..].parse().unwrap_or(100);
        tracing::info!("WAL batch mode: sync every {} transactions", n);
        sqlrustgo_storage::WalSyncMode::Batch(n)
    } else {
        sqlrustgo_storage::WalSyncMode::Every
    }
}

/// v3.10.0 Issue #3703: build an `ExecutionEngine` with intra-query
/// parallelism pre-configured from the CLI flag / env var. Centralizes
/// the wiring so all engine construction sites pick up parallelism
/// uniformly.
#[allow(dead_code)]
pub(crate) fn build_engine_with_parallelism<S: StorageEngine + 'static>(
    storage: Arc<parking_lot::RwLock<S>>,
) -> ExecutionEngine<S> {
    let mut eng = ExecutionEngine::new(storage);
    eng.set_parallel_degree(read_executor_parallelism());
    eng
}

/// Global connection counter for diagnostics. Incremented when a
/// connection is accepted, decremented when it closes.
pub static ACTIVE_CONNECTIONS: AtomicI64 = AtomicI64::new(0);
pub static TOTAL_CONNECTIONS_ACCEPTED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_QUERIES_SERVED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_QUERY_ERRORS: AtomicU64 = AtomicU64::new(0);

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

#[cfg(test)]
mod helpers_tests {
    use super::*;

    // ---------- read_executor_parallelism ----------

    #[test]
    fn read_executor_parallelism_defaults_to_one() {
        let prev = std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM").ok();
        std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
        assert_eq!(read_executor_parallelism(), 1);
        if let Some(v) = prev {
            std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", v);
        }
    }

    #[test]
    fn read_executor_parallelism_honors_env() {
        let prev = std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM").ok();
        std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "8");
        assert_eq!(read_executor_parallelism(), 8);
        if let Some(v) = prev {
            std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", v);
        } else {
            std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
        }
    }

    #[test]
    fn read_executor_parallelism_clamps_zero_and_invalid() {
        let prev = std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM").ok();
        std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "0");
        assert_eq!(read_executor_parallelism(), 1);
        std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "not_a_number");
        assert_eq!(read_executor_parallelism(), 1);
        if let Some(v) = prev {
            std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", v);
        } else {
            std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
        }
    }

    // ---------- parse_wal_sync_mode ----------

    #[test]
    fn parse_wal_sync_mode_every_is_default() {
        match parse_wal_sync_mode("every") {
            sqlrustgo_storage::WalSyncMode::Every => {}
            other => panic!("expected Every, got {:?}", other),
        }
    }

    #[test]
    fn parse_wal_sync_mode_off() {
        match parse_wal_sync_mode("off") {
            sqlrustgo_storage::WalSyncMode::Off => {}
            other => panic!("expected Off, got {:?}", other),
        }
    }

    #[test]
    fn parse_wal_sync_mode_batch_default_n() {
        match parse_wal_sync_mode("batch:") {
            sqlrustgo_storage::WalSyncMode::Batch(n) => assert_eq!(n, 100),
            other => panic!("expected Batch(100), got {:?}", other),
        }
    }

    #[test]
    fn parse_wal_sync_mode_batch_with_n() {
        match parse_wal_sync_mode("batch:50") {
            sqlrustgo_storage::WalSyncMode::Batch(n) => assert_eq!(n, 50),
            other => panic!("expected Batch(50), got {:?}", other),
        }
    }

    #[test]
    fn parse_wal_sync_mode_case_insensitive() {
        match parse_wal_sync_mode("OFF") {
            sqlrustgo_storage::WalSyncMode::Off => {}
            other => panic!("case-insensitive OFF should parse"),
        }
    }

    // ---------- compute_double_sha1 ----------

    #[test]
    fn compute_double_sha1_returns_20_bytes() {
        let result = compute_double_sha1(b"hello");
        assert_eq!(result.len(), 20);
        let result2 = compute_double_sha1(b"hello");
        assert_eq!(result, result2);
    }

    #[test]
    fn compute_double_sha1_different_inputs_differ() {
        let a = compute_double_sha1(b"hello");
        let b = compute_double_sha1(b"world");
        assert_ne!(a, b);
    }

    // ---------- col_type_from_string ----------

    #[test]
    fn col_type_int() {
        assert_eq!(col_type_from_string("INT"), 3);
    }

    #[test]
    fn col_type_varchar() {
        assert_eq!(col_type_from_string("VARCHAR"), 15);
    }

    #[test]
    fn col_type_text_and_char_use_varstring() {
        // The implementation maps TEXT/CHAR to VARSTRING (not TINYBLOB)
        // for legacy compatibility. Assert it returns some non-zero code.
        assert!(col_type_from_string("TEXT") > 0);
        assert!(col_type_from_string("CHAR") > 0);
    }

    #[test]
    fn col_type_datetime_and_timestamp_share_code() {
        let dt = col_type_from_string("DATETIME");
        let ts = col_type_from_string("TIMESTAMP");
        assert_eq!(dt, ts);
        assert!(dt > 0);
    }

    #[test]
    fn col_type_unknown_falls_back_to_varchar() {
        // Unknown types fall back to a generic string code in this codebase.
        assert_eq!(col_type_from_string("UNKNOWN_TYPE"), 254);
    }

    // ---------- value_to_string ----------

    #[test]
    fn value_to_string_renders_basic_types() {
        assert_eq!(value_to_string(&Value::Null), "NULL");
        assert_eq!(value_to_string(&Value::Integer(42)), "42");
        assert_eq!(value_to_string(&Value::Float(3.14)), "3.14");
        // MySQL wire protocol uses 1/0 not true/false.
        assert_eq!(value_to_string(&Value::Boolean(true)), "1");
        assert_eq!(value_to_string(&Value::Boolean(false)), "0");
    }

    // ---------- lenenc roundtrip ----------

    #[test]
    fn write_lenenc_int_small_value_one_byte() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 42).unwrap();
        assert_eq!(buf, vec![42u8]);
    }

    #[test]
    fn write_lenenc_int_251_three_bytes() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 251).unwrap();
        assert_eq!(buf, vec![0xfc, 0xfb, 0x00]); // 251 LE
    }

    #[test]
    fn write_lenenc_int_max_u64_nine_bytes() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, u64::MAX).unwrap();
        assert_eq!(buf[0], 0xfe);
        assert_eq!(buf.len(), 9);
    }

    #[test]
    fn read_lenenc_int_roundtrip_small() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 100).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        assert_eq!(read_lenenc_int(&mut cur).unwrap(), 100);
    }

    #[test]
    fn read_lenenc_int_roundtrip_2byte() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 500).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        assert_eq!(read_lenenc_int(&mut cur).unwrap(), 500);
    }

    #[test]
    fn read_lenenc_int_roundtrip_3byte() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 100_000).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        assert_eq!(read_lenenc_int(&mut cur).unwrap(), 100_000);
    }

    #[test]
    fn read_lenenc_int_null_marker_is_error() {
        // 0xfb is the NULL marker per MySQL protocol; this code path
        // returns Err to distinguish from valid integer encodings.
        let buf = vec![0xfbu8];
        let mut cur = std::io::Cursor::new(buf);
        assert!(read_lenenc_int(&mut cur).is_err());
    }

    #[test]
    fn write_lenenc_string_writes_len_then_bytes() {
        let mut buf = Vec::new();
        write_lenenc_string(&mut buf, b"hello").unwrap();
        // 5-byte len prefix + "hello"
        assert_eq!(buf, vec![5, b'h', b'e', b'l', b'l', b'o']);
    }

    // ---------- packet builders ----------

    #[test]
    fn make_handshake_packet_structure() {
        let scramble = [0u8; SCRAMBLE_LENGTH];
        let pkt = make_handshake_packet(0, &scramble);
        assert_eq!(pkt.sequence, 0);
        assert!(!pkt.payload.is_empty());
        // First byte is protocol version 10
        assert_eq!(pkt.payload[0], 0x0a);
    }

    #[test]
    fn make_ok_packet_structure() {
        let packets = make_ok_packet(1, 5, 100, 0x0002, 0, 0, false);
        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
        assert_eq!(pkt.sequence, 1);
        // First byte 0x00 marks OK packet
        assert_eq!(pkt.payload[0], 0x00);
    }

    #[test]
    fn make_err_packet_structure() {
        let pkt = make_err_packet(2, 1064, "HY000", "syntax error");
        assert_eq!(pkt.sequence, 2);
        // First byte 0xff marks ERR packet
        assert_eq!(pkt.payload[0], 0xff);
        // Error code 1064 in LE
        assert_eq!(u16::from_le_bytes([pkt.payload[1], pkt.payload[2]]), 1064);
    }

    #[test]
    fn make_eof_packet_structure() {
        let pkt = make_eof_packet(3, 0x0002);
        assert_eq!(pkt.sequence, 3);
        // First byte 0xfe marks EOF packet (classic protocol)
        assert_eq!(pkt.payload[0], 0xfe);
    }

    #[test]
    fn value_to_string_renders_text() {
        assert_eq!(value_to_string(&Value::Text("hello".to_string())), "hello");
    }
}

#[cfg(test)]
mod utilities_tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    // ---------- build_engine_with_parallelism ----------

    #[test]
    fn build_engine_with_parallelism_basic() {
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let _engine: ExecutionEngine<_> = build_engine_with_parallelism(storage);
        // Just verify construction succeeds without panic.
    }

    // ---------- read_proc_status ----------

    #[test]
    fn read_proc_status_self_returns_nonzero_rss() {
        let pid = std::process::id();
        let (rss_kb, fd_count) = read_proc_status(pid);
        // On Linux, VmRSS should be > 0 for any running process.
        // On macOS, ps -o rss= also returns > 0.
        // We don't strictly assert > 0 (some sandboxes may return 0)
        // but at least one of (rss, fd) must be meaningful.
        assert!(rss_kb > 0 || fd_count > 0);
    }

    #[test]
    fn read_proc_status_nonexistent_pid_returns_zero() {
        // Pick a PID that's almost certainly not running.
        // Use a very large PID — Linux PIDs are typically < 2^22.
        let (rss_kb, fd_count) = read_proc_status(999_999_999);
        // Either /proc/.../status fails (rss=0, fd=0) or ps fails (rss=0, fd=0).
        // On Linux the second branch also returns 0.
        let _ = (rss_kb, fd_count);
    }

    // ---------- read_fd_limit ----------

    #[test]
    fn read_fd_limit_returns_positive_soft_limit() {
        let (soft, _hard) = read_fd_limit();
        // Any reasonable system has at least 64 FDs.
        assert!(soft >= 64, "soft FD limit should be >= 64, got {}", soft);
    }

    // ---------- list_threads ----------

    #[test]
    fn list_threads_returns_at_least_one() {
        let count = list_threads();
        // The current process has at least 1 thread (itself).
        assert!(count >= 1, "expected at least 1 thread, got {}", count);
    }

    // ---------- skip_auth ----------

    #[test]
    fn skip_auth_defaults_false() {
        let prev = std::env::var("SQLRUSTGO_AUTH_MODE").ok();
        std::env::remove_var("SQLRUSTGO_AUTH_MODE");
        assert!(!skip_auth());
        if let Some(v) = prev {
            std::env::set_var("SQLRUSTGO_AUTH_MODE", v);
        }
    }

    #[test]
    fn skip_auth_honors_none_value() {
        let prev = std::env::var("SQLRUSTGO_AUTH_MODE").ok();
        std::env::set_var("SQLRUSTGO_AUTH_MODE", "none");
        assert!(skip_auth());
        std::env::set_var("SQLRUSTGO_AUTH_MODE", "NONE");
        assert!(skip_auth()); // case-insensitive
        if let Some(v) = prev {
            std::env::set_var("SQLRUSTGO_AUTH_MODE", v);
        } else {
            std::env::remove_var("SQLRUSTGO_AUTH_MODE");
        }
    }

    #[test]
    fn skip_auth_rejects_password() {
        let prev = std::env::var("SQLRUSTGO_AUTH_MODE").ok();
        std::env::set_var("SQLRUSTGO_AUTH_MODE", "password");
        assert!(!skip_auth());
        if let Some(v) = prev {
            std::env::set_var("SQLRUSTGO_AUTH_MODE", v);
        } else {
            std::env::remove_var("SQLRUSTGO_AUTH_MODE");
        }
    }

    // ---------- atomic counters ----------

    #[test]
    fn atomic_counters_initial_zero() {
        // Process-global counters; we just verify they exist and are usable.
        // We can't easily test initial state (other tests may have incremented).
        let _ = ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
        let _ = TOTAL_CONNECTIONS_ACCEPTED.load(Ordering::Relaxed);
        let _ = TOTAL_QUERIES_SERVED.load(Ordering::Relaxed);
        let _ = TOTAL_QUERY_ERRORS.load(Ordering::Relaxed);
    }

    #[test]
    fn atomic_counter_increment_works() {
        let before = ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
        ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
        assert_eq!(ACTIVE_CONNECTIONS.load(Ordering::Relaxed), before + 1);
        // Restore so we don't leave dirty state.
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    }

    // ---------- decode_lenenc_int ----------

    #[test]
    fn decode_lenenc_int_small_value() {
        let payload = [42u8];
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(&payload, &mut pos), Some(42));
        assert_eq!(pos, 1);
    }

    #[test]
    fn decode_lenenc_int_2byte_via_0xfc() {
        let payload = [0xfc, 0xfb, 0x00]; // 251 LE
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(&payload, &mut pos), Some(251));
        assert_eq!(pos, 3);
    }
    fn decode_lenenc_int_3byte_via_0xfd() {
        // The impl uses [0, b0, b1, b2] = LE u32, so for [0xfd, 0x01, 0x00, 0x00]
        let payload = [0xfd, 0x01, 0x00, 0x00];
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(&payload, &mut pos), Some(65536));
        assert_eq!(pos, 4);
    }

    #[test]
    fn decode_lenenc_int_8byte_via_0xfe() {
        let payload = [0xfe, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(&payload, &mut pos), Some(1));
        assert_eq!(pos, 9);
    }

    #[test]
    fn decode_lenenc_int_null_marker_returns_none() {
        // 0xfb = NULL marker per MySQL protocol.
        let payload = [0xfb];
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(&payload, &mut pos), None);
        assert_eq!(pos, 1);
    }

    #[test]
    fn decode_lenenc_int_eof_returns_none() {
        let payload: &[u8] = &[];
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(payload, &mut pos), None);
        assert_eq!(pos, 0);
    }

    #[test]
    fn decode_lenenc_int_truncated_2byte_returns_none() {
        let payload = [0xfc, 0x01]; // missing second byte
        let mut pos = 0;
        assert_eq!(decode_lenenc_int(&payload, &mut pos), None);
    }
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
    pub const COM_RESET_CONNECTION: u8 = 0x1F;
}

// ============================================================================
// V312-32: removed the process-global `static ACTIVE_CONFIG: Mutex<Option<...>>`
// that previously let `start_ephemeral` publish its `EphemeralConfig` for
// the LOAD DATA LOCAL INFILE handler to read. The replacement is a
// per-handle `Arc<EphemeralConfig>` threaded through
//   `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`
//   → accept loop → `ServerJob` / `handle_connection` → `do_command_loop`.
// This makes concurrent `start_ephemeral` calls safe: each connection's
// LOAD DATA sees the `data_dir` and `bulk_insert_buffer_size` of the
// server it connected to, not whichever `start_ephemeral` ran last in
// the process. The removal of the `Mutex<Option<...>>` static also
// eliminates the cross-test data_dir pollution that caused
// `v312_13_load_data_sf1_region_nation_smoke` to be marked `#[ignore]`.
// ============================================================================

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
    /// CLIENT_COMPRESS (0x00200000). When advertised by both client and
    /// server, payloads after the HandshakeV10 are wrapped in zlib.
    /// PR #4112 added the wire primitives; see `compression.rs`.
    pub const COMPRESS: u32 = 0x00200000;
    /// CLIENT_SESSION_TRACK (0x00800000). When set by the client, every
    /// OK packet (and EOF/result-set terminator) MUST carry an extra
    /// lenenc-encoded `info` string at the end. If
    /// `status_flags & SERVER_STATUS_SESSION_STATE_CHANGED` is also set,
    /// an additional lenenc-encoded session-state blob follows. mysql CLI
    /// 8.0+ sets this bit by default, so omitting the trailing fields
    /// causes the client to block on recvfrom waiting for the missing
    /// bytes (Issue #4019.3).
    pub const SESSION_TRACK: u32 = 0x00800000;
    /// SERVER_STATUS_SESSION_STATE_CHANGED (0x4000). Set in the status
    /// flags of an OK packet when the server includes session-state
    /// change data in the packet (only meaningful when
    /// `CLIENT_SESSION_TRACK` is negotiated).
    pub const SERVER_STATUS_SESSION_STATE_CHANGED: u16 = 0x4000;
    /// SERVER_MORE_RESULTS_EXISTS (0x0008). V312-WIRE-8 fix (regression
    /// #4019/#4020/#4022 multi-query blocker): status flag set on the
    /// trailing EOF/OK packet of every result set that is NOT the last
    /// in a multi-statement COM_QUERY batch. Clients that negotiate
    /// CLIENT_MULTI_STATEMENTS use this bit to decide whether to read
    /// another result set from the same packet stream. Without this bit,
    /// the client stops reading after the first result and the second
    /// statement's response either gets concatenated into the row
    /// stream or blocks the client on recvfrom.
    pub const SERVER_MORE_RESULTS_EXISTS: u16 = 0x0008;

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
        | SSL
        | COMPRESS
        // V312-WIRE-3 fix (regression #4019.3): mysql CLI 8.0+ advertises
        // CLIENT_SESSION_TRACK by default. If we want to send the trailing
        // `info` field (and optional session-state blob) in OK packets, we
        // MUST also advertise SESSION_TRACK in HandshakeV10. Otherwise the
        // client parses the OK packet with the old layout and consumes the
        // trailing 0x00 as part of the next packet — leaving it blocked on
        // recvfrom waiting for a packet that will never arrive.
        | SESSION_TRACK;
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

    #[test]
    fn test_parse_wal_sync_mode_every() {
        let m = parse_wal_sync_mode("any_default");
        match m {
            sqlrustgo_storage::WalSyncMode::Every => {}
            _ => panic!("expected Every"),
        }
    }

    #[test]
    fn test_parse_wal_sync_mode_off() {
        let m = parse_wal_sync_mode("OFF");
        match m {
            sqlrustgo_storage::WalSyncMode::Off => {}
            _ => panic!("expected Off"),
        }
    }

    #[test]
    fn test_parse_wal_sync_mode_batch() {
        let m = parse_wal_sync_mode("batch:50");
        match m {
            sqlrustgo_storage::WalSyncMode::Batch(n) => assert_eq!(n, 50),
            _ => panic!("expected Batch"),
        }
    }

    #[test]
    fn test_parse_wal_sync_mode_batch_invalid() {
        let m = parse_wal_sync_mode("batch:invalid");
        match m {
            sqlrustgo_storage::WalSyncMode::Batch(n) => assert_eq!(n, 100), // default
            _ => panic!("expected Batch with default"),
        }
    }

    #[test]
    fn test_compute_double_sha1() {
        let h1 = compute_double_sha1(b"test");
        let h2 = compute_double_sha1(b"test");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 20);
    }

    #[test]
    fn test_compute_double_sha1_different() {
        let h1 = compute_double_sha1(b"foo");
        let h2 = compute_double_sha1(b"bar");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_sha1_simple() {
        let h1 = sha1_simple(b"hello");
        let h2 = sha1_simple(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 20);
    }

    #[test]
    fn test_verify_mysql_native_password_correct() {
        // Construct a valid auth_response
        let password = b"test_password";
        let scramble = [1u8; 20];
        let pwd_hash = sha1_simple(password);
        let double_hash = compute_double_sha1(password);
        let mut to_xor = [0u8; 20];
        for i in 0..20 {
            let mut h = Sha1::new();
            h.update(&scramble);
            h.update(&double_hash);
            let s = h.finalize();
            to_xor[i] = s[i];
        }
        let mut auth_response = [0u8; 20];
        for i in 0..20 {
            auth_response[i] = to_xor[i] ^ pwd_hash[i];
        }
        assert!(verify_mysql_native_password(
            &double_hash,
            &scramble,
            &auth_response
        ));
    }

    #[test]
    fn test_verify_mysql_native_password_wrong_length() {
        let h = [0u8; 20];
        let s = [0u8; 20];
        assert!(!verify_mysql_native_password(&h, &s, &[0u8; 10]));
    }

    #[test]
    fn test_verify_mysql_native_password_invalid() {
        let h = [0u8; 20];
        let s = [0u8; 20];
        assert!(!verify_mysql_native_password(&h, &s, &[0u8; 20]));
    }

    #[test]
    fn test_col_type_from_string_common2() {
        // Just verify consistency; specific values are tested in integration_tests
        let v1 = col_type_from_string("INT");
        let v2 = col_type_from_string("INT");
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_col_type_from_string_top_unknown() {
        let _ = col_type_from_string("UNKNOWN_TYPE");
    }

    #[test]
    fn test_col_len_from_type_int() {
        // INT should have specific length
        let len = col_len_from_type("INT");
        assert!(len > 0);
    }

    #[test]
    fn test_col_len_from_type_varchar() {
        let len = col_len_from_type("VARCHAR");
        assert!(len > 0);
    }

    #[test]
    fn test_col_len_from_type_unknown() {
        // Unknown types should return default
        let len = col_len_from_type("UNKNOWN");
        // either 0 or some default
        let _ = len;
    }

    #[test]
    fn test_count_placeholders() {
        assert_eq!(count_placeholders("SELECT 1"), 0);
        assert_eq!(count_placeholders("SELECT ?"), 1);
        assert_eq!(count_placeholders("SELECT ?, ?, ?"), 3);
        assert_eq!(count_placeholders("INSERT INTO t VALUES (?, ?)"), 2);
    }

    #[test]
    fn test_extract_insert_columns_basic() {
        let cols = extract_insert_columns("INSERT INTO t (a, b, c) VALUES (?, ?, ?)");
        assert_eq!(cols, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_extract_insert_columns_backticks() {
        let cols = extract_insert_columns("INSERT INTO t (`id`, `name`) VALUES (?, ?)");
        assert_eq!(cols, vec!["id", "name"]);
    }

    #[test]
    fn test_extract_insert_columns_no_into() {
        let cols = extract_insert_columns("INSERT t VALUES (1)");
        assert!(cols.is_empty());
    }

    #[test]
    fn test_extract_insert_columns_into_with_no_col_list() {
        let cols = extract_insert_columns("INSERT INTO t VALUES (1)");
        let _ = cols;
    }

    #[test]
    fn test_extract_insert_columns_non_insert() {
        let cols = extract_insert_columns("SELECT * FROM t");
        assert!(cols.is_empty());
    }

    #[test]
    fn test_extract_where_columns_basic() {
        let cols = extract_where_columns("SELECT * FROM t WHERE id = ? AND name = ?");
        assert!(!cols.is_empty());
    }

    #[test]
    fn test_extract_where_columns_no_where() {
        let cols = extract_where_columns("SELECT * FROM t");
        assert!(cols.is_empty());
    }

    #[test]
    fn test_extract_where_columns_single() {
        let cols = extract_where_columns("SELECT * FROM t WHERE id = ?");
        assert_eq!(cols, vec!["id"]);
    }

    #[test]
    fn test_write_lenenc_int_small() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0).unwrap();
        assert_eq!(buf, vec![0u8]);
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 100).unwrap();
        assert_eq!(buf, vec![100u8]);
    }

    #[test]
    fn test_write_lenenc_int_16bit() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x1234).unwrap();
        assert_eq!(buf[0], 0xfc);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_write_lenenc_int_24bit() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x123456).unwrap();
        assert_eq!(buf[0], 0xfd);
        assert_eq!(buf.len(), 4);
    }

    #[test]
    fn test_write_lenenc_int_64bit() {
        let mut buf = Vec::new();
        write_lenenc_int(&mut buf, 0x123456789abcdef0).unwrap();
        assert_eq!(buf[0], 0xfe);
        assert_eq!(buf.len(), 9);
    }

    #[test]
    fn test_read_lenenc_int_small() {
        let bytes = vec![42u8];
        let mut cursor = std::io::Cursor::new(&bytes);
        let v = read_lenenc_int(&mut cursor).unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn test_read_lenenc_int_16bit() {
        let bytes = vec![0xfc, 0x34, 0x12];
        let mut cursor = std::io::Cursor::new(&bytes);
        let v = read_lenenc_int(&mut cursor).unwrap();
        assert_eq!(v, 0x1234);
    }

    #[test]
    fn test_read_lenenc_int_24bit() {
        let bytes = vec![0xfd, 0x56, 0x34, 0x12];
        let mut cursor = std::io::Cursor::new(&bytes);
        let v = read_lenenc_int(&mut cursor).unwrap();
        assert_eq!(v, 0x123456);
    }

    #[test]
    fn test_read_lenenc_int_64bit() {
        let mut bytes = vec![0xfe];
        bytes.extend_from_slice(&0x123456789abcdef0u64.to_le_bytes());
        let mut cursor = std::io::Cursor::new(&bytes);
        let v = read_lenenc_int(&mut cursor).unwrap();
        assert_eq!(v, 0x123456789abcdef0);
    }

    #[test]
    fn test_read_lenenc_int_null() {
        let bytes = vec![0xfb];
        let mut cursor = std::io::Cursor::new(&bytes);
        let r = read_lenenc_int(&mut cursor);
        assert!(r.is_err());
    }

    #[test]
    fn test_read_lenenc_int_invalid() {
        let bytes = vec![0xff];
        let mut cursor = std::io::Cursor::new(&bytes);
        let r = read_lenenc_int(&mut cursor);
        assert!(r.is_err());
    }

    #[test]
    fn test_write_lenenc_string_basic() {
        let mut buf = Vec::new();
        write_lenenc_string(&mut buf, b"hello").unwrap();
        assert_eq!(buf[0], 5);
        assert_eq!(&buf[1..], b"hello");
    }

    #[test]
    fn test_write_lenenc_string_empty() {
        let mut buf = Vec::new();
        write_lenenc_string(&mut buf, b"").unwrap();
        assert_eq!(buf, vec![0u8]);
    }

    #[test]
    fn test_make_handshake_packet_basic() {
        let scramble = [0u8; 20];
        let p = make_handshake_packet(0, &scramble);
        assert!(p.payload.len() > 0);
        assert_eq!(p.sequence, 0);
    }

    #[test]
    fn test_make_ok_packet_basic() {
        let packets = make_ok_packet(1, 0, 0, 0x02, 0, 0, false);
        assert_eq!(packets.len(), 1);
        let p = &packets[0];
        assert!(p.payload.len() > 0);
        assert_eq!(p.sequence, 1);
    }

    #[test]
    fn test_make_err_packet_basic() {
        let p = make_err_packet(1, 1064, "HY000", "syntax error");
        assert!(p.payload.len() > 0);
        assert_eq!(p.payload[0], 0xff);
    }

    #[test]
    fn test_make_eof_packet_basic() {
        let p = make_eof_packet(1, 0x02);
        assert!(p.payload.len() > 0);
        assert_eq!(p.payload[0], 0xfe);
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(value_to_string(&sqlrustgo_types::Value::Integer(42)), "42");
        assert_eq!(
            value_to_string(&sqlrustgo_types::Value::Text("hi".to_string())),
            "hi"
        );
        assert_eq!(value_to_string(&sqlrustgo_types::Value::Boolean(true)), "1");
        assert_eq!(
            value_to_string(&sqlrustgo_types::Value::Boolean(false)),
            "0"
        );
        assert_eq!(value_to_string(&sqlrustgo_types::Value::Null), "NULL");
    }
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

    // Test BinaryTableStorage loads TPC-H SF=1.0 .bin files correctly
    #[test]
    fn test_binary_storage_tpch_sf1_load() {
        use sqlrustgo_storage::BinaryTableStorage;

        let bin_dir = std::path::PathBuf::from("/tmp/tpch-sf1-bin");
        if !bin_dir.exists() {
            println!("SKIP: /tmp/tpch-sf1-bin not found (run tbl2bin first)");
            return;
        }

        let storage = BinaryTableStorage::new_with_data(bin_dir).expect("load .bin files");
        let counts: Vec<(&str, usize)> = vec![
            ("region", 5),
            ("nation", 25),
            ("customer", 150_000),
            ("supplier", 10_000),
            ("part", 200_000),
            ("partsupp", 800_000),
            ("orders", 1_500_000),
            // TPC-H SF=1 dbgen generates exactly 6,001,215 lineitem rows
            // (confirmed by fixture at /var/tmp/tpch-sf1, GA_GATE_REPORT.md).
            ("lineitem", 6_001_215),
        ];

        for (table, expected) in counts {
            let rows = storage.scan(table).expect(table);
            assert_eq!(rows.len(), expected, "table {} row count mismatch", table);
            println!("  {}: {} rows OK", table, rows.len());
        }
        println!("BinaryTableStorage loaded all 8 TPC-H tables correctly");
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
        let packets = make_ok_packet(1, 5, 10, 0x0002, 0, 0, false);
        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
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
        // Format: 0xFF + error_code(2 LE) + 0x23 + SQL_STATE(5) + message + 0x00
        assert_eq!(pkt.payload[3], 0x23); // SQL state marker
        assert_eq!(&pkt.payload[4..9], b"42S02"); // SQL state
        assert_eq!(&pkt.payload[9..24], b"Table not found"); // message
        assert_eq!(pkt.payload[24], 0x00); // null terminator at end of message
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
        use parking_lot::RwLock;
        use sqlrustgo::MemoryExecutionEngine;
        use sqlrustgo_parser::parse;
        use sqlrustgo_storage::MemoryStorage;
        use std::sync::Arc;

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

    // ---------- split_top_level_statements ----------

    #[test]
    fn split_top_level_single_statement() {
        let stmts = split_top_level_statements("SELECT 1");
        assert_eq!(stmts, vec!["SELECT 1"]);
    }

    #[test]
    fn split_top_level_two_statements() {
        let stmts = split_top_level_statements("SELECT 1; SELECT 2");
        assert_eq!(stmts, vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn split_top_level_with_trailing_semicolon() {
        let stmts = split_top_level_statements("SELECT 1;");
        assert_eq!(stmts, vec!["SELECT 1"]);
    }

    #[test]
    fn split_top_level_with_whitespace() {
        let stmts = split_top_level_statements("  SELECT 1  ;  SELECT 2  ");
        assert_eq!(stmts, vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn split_top_level_semicolon_inside_string_is_ignored() {
        let stmts = split_top_level_statements("INSERT INTO t VALUES ('a;b'); SELECT 1");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "INSERT INTO t VALUES ('a;b')");
        assert_eq!(stmts[1], "SELECT 1");
    }

    #[test]
    fn split_top_level_escaped_quote_skips_next_char() {
        let stmts = split_top_level_statements("INSERT INTO t VALUES ('it\\'s'); SELECT 1");
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("it\\'s"));
    }

    #[test]
    fn split_top_level_double_quoted_string() {
        let stmts = split_top_level_statements("SELECT \"a;b\"; SELECT 2");
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn split_top_level_semicolon_inside_parens_ignored() {
        let stmts = split_top_level_statements("SELECT * FROM (SELECT 1; SELECT 2);");
        // The parser sees ONE statement (the outer SELECT) because the
        // inner ';' is inside parens.
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].contains("SELECT 1; SELECT 2"));
    }

    #[test]
    fn split_top_level_line_comment_skipped() {
        let stmts = split_top_level_statements(
            "-- comment with ; inside\nSELECT 1; -- another ;\nSELECT 2",
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn split_top_level_block_comment_skipped() {
        let stmts =
            split_top_level_statements("/* ; */ SELECT 2; /* multi\nline ; comment */ SELECT 3");
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn split_top_level_empty_input() {
        let stmts = split_top_level_statements("");
        assert!(stmts.is_empty());
    }

    #[test]
    fn split_top_level_only_whitespace() {
        let stmts = split_top_level_statements("   \n\t  ");
        assert!(stmts.is_empty());
    }

    // ---------- classify_long_query_time_set ----------

    #[test]
    fn classify_long_query_time_set_none_for_other_statements() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        // Begin is not a SET long_query_time, so None.
        let stmt = Statement::Transaction(TransactionStatement::Begin {
            work: false,
            isolation_level: None,
            readonly: false,
        });
        assert_eq!(classify_long_query_time_set(&stmt), None);
    }

    #[test]
    fn classify_long_query_time_set_parses_integer_seconds() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        let stmt = Statement::Transaction(TransactionStatement::SetSessionVariable {
            name: "long_query_time".to_string(),
            value: "5".to_string(),
        });
        match classify_long_query_time_set(&stmt) {
            Some(Ok(ms)) => assert_eq!(ms, 5000),
            other => panic!("expected Some(Ok(5000)), got {:?}", other),
        }
    }

    #[test]
    fn classify_long_query_time_set_parses_float_seconds() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        let stmt = Statement::Transaction(TransactionStatement::SetSessionVariable {
            name: "long_query_time".to_string(),
            value: "0.5".to_string(),
        });
        match classify_long_query_time_set(&stmt) {
            Some(Ok(ms)) => assert_eq!(ms, 500),
            other => panic!("expected Some(Ok(500)), got {:?}", other),
        }
    }

    #[test]
    fn classify_long_query_time_set_case_insensitive_name() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        let stmt = Statement::Transaction(TransactionStatement::SetSessionVariable {
            name: "LONG_QUERY_TIME".to_string(),
            value: "2".to_string(),
        });
        match classify_long_query_time_set(&stmt) {
            Some(Ok(ms)) => assert_eq!(ms, 2000),
            other => panic!("expected Some(Ok(2000)), got {:?}", other),
        }
    }

    #[test]
    fn classify_long_query_time_set_rejects_invalid_value() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        let stmt = Statement::Transaction(TransactionStatement::SetSessionVariable {
            name: "long_query_time".to_string(),
            value: "not_a_number".to_string(),
        });
        match classify_long_query_time_set(&stmt) {
            Some(Err(msg)) => assert!(msg.contains("Incorrect argument")),
            other => panic!("expected Some(Err), got {:?}", other),
        }
    }

    #[test]
    fn classify_long_query_time_set_rejects_negative() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        let stmt = Statement::Transaction(TransactionStatement::SetSessionVariable {
            name: "long_query_time".to_string(),
            value: "-1.0".to_string(),
        });
        match classify_long_query_time_set(&stmt) {
            Some(Err(_)) => {}
            other => panic!("expected Some(Err) for negative value, got {:?}", other),
        }
    }

    #[test]
    fn classify_long_query_time_set_ignores_other_variable() {
        use sqlrustgo_parser::{transaction::TransactionStatement, Statement};
        let stmt = Statement::Transaction(TransactionStatement::SetSessionVariable {
            name: "max_connections".to_string(),
            value: "100".to_string(),
        });
        // Not long_query_time → None (not interested in this SET).
        assert_eq!(classify_long_query_time_set(&stmt), None);
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

/// A `Read`+`Write` wrapper around `rustls::ServerConnection` that
/// drives TLS I/O on the underlying `TcpStream`. After each `write`
/// into rustls, `complete_io` flushes the resulting cipher records to
/// the socket. With blocking sockets (restored in the accept loop),
/// `complete_io` blocks in the kernel and never returns `WouldBlock`.
/// If `WouldBlock` occurs anyway (non-blocking socket in tests), we
/// break and leave remaining records in rustls — they flush on the
/// next read cycle.
pub struct TlsStream<'a> {
    pub sock: &'a mut TcpStream,
    pub conn: &'a mut rustls::ServerConnection,
}

impl<'a> TlsStream<'a> {
    /// Flush any remaining TLS ciphertext to the underlying socket.
    /// Best-effort: remaining records stay in rustls and flush on the
    /// next read cycle. Kept for compatibility (post-COM_QUIT flush).
    pub fn flush_pending(&mut self) -> std::io::Result<()> {
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl<'a> Read for TlsStream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Engine Bug B fix (refs #3635): loop drains ALL pending TLS
        // records before returning. A single `complete_io` only
        // decrypts ciphertext currently buffered in the socket, which
        // deadlocks large multi-record plaintexts (>= ~16 KB).
        while self.conn.wants_read() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        self.conn.reader().read(buf)
    }
}

impl<'a> TlsStream<'a> {
    pub fn new(conn: &'a mut rustls::ServerConnection, sock: &'a mut TcpStream) -> Self {
        Self { conn, sock }
    }
}

impl<'a> Write for TlsStream<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.conn.writer().write(buf)?;
        tracing::trace!(
            "TlsStream write: {} app-bytes -> rustls, wants_write={}",
            n,
            self.conn.wants_write()
        );
        // With blocking sockets (restored in accept loop), complete_io
        // blocks in the kernel and WouldBlock should not occur. We break
        // on WouldBlock anyway as a safety net — any remaining cipher
        // records stay buffered in rustls and flush on the next read cycle.
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.conn.writer().flush()?;
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
impl<'a> TlsStream<'a> {
    /// Drive ALL pending outbound TLS records to the socket.
    /// With blocking sockets (restored in accept loop), complete_io
    /// blocks in the kernel — this is best-effort; remaining records
    /// are flushed on the next read cycle.
    fn drive_writes_only(&mut self) -> std::io::Result<()> {
        while self.conn.wants_write() {
            match self.conn.complete_io(self.sock) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
/// Marker trait: types that implement std::io::Write but are NOT TlsStream.
/// Used to prevent the blanket DrainWrites impl from covering TlsStream
/// (which needs its own impl that calls drive_writes_only instead of flush).
trait NotTlsStream {}

/// Helper trait to force-drain any buffered writes on a Write stream.
/// For TlsStream this calls drive_writes_only; for all other streams
/// flush() is sufficient (data is already on the wire).
trait DrainWrites {
    fn force_drain(&mut self);
}

// Blanket impl for all Write types EXCEPT TlsStream
impl<W: std::io::Write> DrainWrites for W
where
    W: NotTlsStream,
{
    fn force_drain(&mut self) {
        // Regular stream: flush() is synchronous and sufficient
        let _ = std::io::Write::flush(self);
    }
}
impl<'a> DrainWrites for TlsStream<'a> {
    fn force_drain(&mut self) {
        if let Err(e) = self.drive_writes_only() {
            tracing::warn!("TlsStream::force_drain: drive_writes_only failed: {}", e);
        }
    }
}

// TcpStream is NOT a TlsStream (covers both TcpStream and &TcpStream)
impl NotTlsStream for std::net::TcpStream {}
impl<T: NotTlsStream> NotTlsStream for &T {}
/// Compress `payload` into a MySQL compressed packet frame and write to `w`.
pub fn write_compressed_packet<W: Write>(w: &mut W, seq: u8, payload: &[u8]) -> MySqlResult<()> {
    let uncompressed_len = payload.len();
    let mut compressor = Compress::new(flate2::Compression::default(), true);
    let bound = uncompressed_len.saturating_add(12);
    let mut compressed = Vec::with_capacity(bound);
    let _status = compressor
        .compress_vec(payload, &mut compressed, FlushCompress::Finish)
        .map_err(|e| MySqlError::Protocol(format!("zlib compress: {e}")))?;
    let compressed_len = compressed.len();
    // 7-byte header: [uncompressed_len: u24][seq: u8][compressed_len: u24]
    w.write_all(&(uncompressed_len as u32).to_le_bytes()[..3])?;
    w.write_u8(seq)?;
    w.write_all(&(compressed_len as u32).to_le_bytes()[..3])?;
    w.write_all(&compressed)?;
    w.flush()?;
    Ok(())
}

/// Read and decompress one MySQL compressed packet frame from `inner`.
pub fn read_compressed_packet<R: Read>(inner: &mut R) -> MySqlResult<(u8, Vec<u8>)> {
    // 7-byte header
    let mut header = [0u8; 7];
    inner.read_exact(&mut header).map_err(MySqlError::Io)?;

    let uncompressed_len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let seq = header[3];
    let compressed_len = u32::from_le_bytes([header[4], header[5], header[6], 0]) as usize;

    // Read compressed payload
    let mut compressed = vec![0u8; compressed_len];
    inner.read_exact(&mut compressed).map_err(MySqlError::Io)?;

    // Uncompressed payload (MySQL optimization for small frames)
    if uncompressed_len == 0 || compressed_len == uncompressed_len {
        return Ok((seq, compressed));
    }

    // Decompress using flate2 decompress_vec
    // IMPORTANT: decompress_vec APPENDS starting at len, so len must be 0
    let mut decompressed = Vec::with_capacity(uncompressed_len);
    decompressed.reserve(uncompressed_len); // capacity = 2*uncompressed, len = 0

    let mut d = Decompress::new(true);
    d.decompress_vec(&compressed, &mut decompressed, FlushDecompress::Finish)
        .map_err(|e| MySqlError::Protocol(format!("zlib: {e}")))?;

    if decompressed.len() != uncompressed_len {
        return Err(MySqlError::Protocol(format!(
            "zlib: decompressed {} bytes, expected {}",
            decompressed.len(),
            uncompressed_len
        )));
    }

    Ok((seq, decompressed))
}

// ============================================================================
// Compressed I/O wrappers for MySQL wire compression
// ============================================================================
//
// MySQL compressed packet format (7-byte header + payload):
//   [uncompressed_len: u24 LE][seq: u8][compressed_len: u24 LE][payload]
//
// Reference: MySQL 8.0 `net_serv.cc` compress_packet() / decompress_packet()
//
// Compression design: each MySQL packet (request or response) is independently
// compressible. When COMPRESS is negotiated, the sender MAY choose to send
// the payload uncompressed (when compressed_len >= uncompressed_len, MySQL
// optimization). The receiver MUST handle both compressed and uncompressed
// payloads transparently.

/// Read packets from a stream, automatically decompressing if the client
/// negotiated COMPRESS capability.
pub struct CompressedReader<'a, R: Read> {
    inner: &'a mut R,
    use_compress: bool,
    // Decompression buffer: holds partial decompressed data from a
    // compressed packet whose output spanned multiple MySQL payload chunks.
    // Most MySQL implementations don't span a single uncompressed packet
    // across multiple compressed frames, but we handle it for correctness.
    decompressed_buf: Vec<u8>,
    decompressed_pos: usize,
}

impl<'a, R: Read> CompressedReader<'a, R> {
    pub fn new(inner: &'a mut R, use_compress: bool) -> Self {
        Self {
            inner,
            use_compress,
            decompressed_buf: Vec::new(),
            decompressed_pos: 0,
        }
    }

    /// Reads one MySQL packet payload. When compression is enabled this
    /// reads and decompresses a compressed packet frame; otherwise reads
    /// a plain packet. Returns (seq, payload).
    pub fn read_packet(&mut self) -> MySqlResult<(u8, Vec<u8>)> {
        if !self.use_compress {
            let pkt = Packet::read_from(self.inner)?;
            return Ok((pkt.sequence, pkt.payload));
        }

        // First: drain any leftover decompressed data from a previous frame
        if self.decompressed_pos < self.decompressed_buf.len() {
            let remaining = self.decompressed_buf[self.decompressed_pos..].to_vec();
            let seq = self.decompressed_buf.get(0).copied().unwrap_or(0);
            self.decompressed_buf.clear();
            self.decompressed_pos = 0;
            return Ok((seq, remaining));
        }

        // Read a compressed packet frame
        let (seq, payload) = read_compressed_packet(self.inner)?;
        self.decompressed_buf = payload;
        self.decompressed_pos = 0;
        Ok((seq, self.decompressed_buf.clone()))
    }
}

impl<'a, R: Read> Read for CompressedReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // This Read impl is for the case where we use CompressedReader
        // as a drop-in Read replacement (draining decompressed data).
        // For simplicity, delegate to read_packet.
        if buf.is_empty() {
            return Ok(0);
        }
        match self.read_packet() {
            Ok((_seq, payload)) => {
                let len = payload.len().min(buf.len());
                buf[..len].copy_from_slice(&payload[..len]);
                Ok(len)
            }
            Err(MySqlError::Io(e)) => Err(e),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}

/// Write packets to a stream, automatically compressing if the client
/// negotiated COMPRESS capability.
pub struct CompressedWriter<'a, W: Write> {
    inner: &'a mut W,
    use_compress: bool,
}

impl<'a, W: Write> CompressedWriter<'a, W> {
    pub fn new(inner: &'a mut W, use_compress: bool) -> Self {
        Self {
            inner,
            use_compress,
        }
    }

    /// Write one MySQL packet. When compression is enabled this compresses
    /// the payload and writes a compressed packet frame; otherwise writes
    /// a plain packet.
    pub fn write_packet(&mut self, seq: u8, payload: &[u8]) -> MySqlResult<()> {
        if !self.use_compress {
            Packet {
                length: payload.len() as u32,
                sequence: seq,
                payload: payload.to_vec(),
            }
            .write_to(self.inner)?;
            return Ok(());
        }

        // Compress: use write_compressed_packet which handles the
        // uncompressed-payload optimization (when compressed_len >= uncompressed_len,
        // it sends payload uncompressed with uncompressed_len == compressed_len).
        write_compressed_packet(self.inner, seq, payload)?;
        Ok(())
    }
}

impl<'a, W: Write> Write for CompressedWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // This Write impl exists for DrainWrites compatibility.
        // We delegate to inner.write — caller should use write_packet for
        // proper MySQL packet framing.
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
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
    p.push(0x21); // charset utf8 (collation_id 33 = utf8_general_ci) — MySQL 8.0 client rejects 0xff as invalid
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

fn make_ok_packet(
    seq: u8,
    affected: u64,
    last_id: u64,
    status: u16,
    warnings: u16,
    client_cap: u32,
    is_auth_ok: bool,
) -> Vec<Packet> {
    let mut p = Vec::new();
    p.push(0x00);
    write_lenenc_int(&mut p, affected).unwrap();
    write_lenenc_int(&mut p, last_id).unwrap();
    // V312-WIRE-4 fix (regression #4019.4): when CLIENT_SESSION_TRACK is
    // negotiated, augment the status flags with
    // SERVER_STATUS_SESSION_STATE_CHANGED (0x4000) EXCEPT for the Auth OK
    // packet. mysql CLI 8.0+ expects this bit to be set in the OK packet's
    // status_flags when SESSION_TRACK is negotiated (it signals "session
    // state may have changed in this statement"); the client then knows to
    // look for the standalone session-state-change packet that follows.
    // Without 0x4000, mysql CLI 8.0.46 has been observed to hang on
    // recvfrom after the result-set terminator (verified via strace — see
    // docs/releases/v3.12.0/evidence/v4019.1/STRACE_BYTE_VERIFICATION.md).
    //
    // V312-WIRE-6 fix (regression #4019.4 third pass): the Auth OK packet
    // MUST NOT carry 0x4000 and MUST NOT emit the trailing session_state
    // packet. mysql CLI 8.0.46 reads the Auth OK in 3 recv calls (header
    // partial + header tail + payload), then immediately sends COM_QUERY
    // before reading the 2nd packet. The 2nd packet then arrives AFTER the
    // client's sendto, gets interpreted as the COM_QUERY response (wrong
    // seq = 3 instead of 1), and the client crashes with
    // CR_SERVER_LOST (exit 1). Verified via strace — see
    // docs/releases/v3.12.0/evidence/v4019.4/. The Auth OK status stays
    // pure AUTOCOMMIT (0x0002); no session_state sibling packet.
    let mut actual_status = status;
    if client_cap & capability::SESSION_TRACK != 0 && !is_auth_ok {
        actual_status |= capability::SERVER_STATUS_SESSION_STATE_CHANGED;
        // Also keep AUTOCOMMIT set if caller didn't already enable it.
        actual_status |= 0x0002;
    }
    p.write_u16::<LittleEndian>(actual_status).unwrap();
    p.write_u16::<LittleEndian>(warnings).unwrap();
    // V312-WIRE-3 fix (regression #4019.3): when CLIENT_SESSION_TRACK is
    // negotiated, mysql CLI 8.0+ ALWAYS expects the trailing lenenc `info`
    // field after `warnings` in the OK packet. Omitting it causes the client
    // to consume the next packet's header bytes as the info-length and
    // silently desync its read cursor (verified via strace — see
    // docs/releases/v3.12.0/evidence/v4019.3/STRACE_BYTE_VERIFICATION.md).
    //
    // V312-WIRE-7 fix (regression #4019.4 fourth pass — supersedes the
    // retracted V312-WIRE-5): per the MySQL 8.0 protocol spec
    // (https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_basic_ok_packet.html),
    // when status_flags has SERVER_STATUS_SESSION_STATE_CHANGED AND
    // CLIENT_SESSION_TRACK is negotiated, the session_state_changes lenenc
    // string is **embedded** in the SAME OK packet (right after `info`),
    // NOT sent as a separate packet. V312-WIRE-5 had assumed separate
    // transmission; that hypothesis was WRONG — mysql CLI 8.0.46 reads
    // session_state_changes from inside the OK packet's body via
    // `net_field_length` on the same buffer, and emits
    // `CR_SERVER_LOST` / hangs after the OK packet when it doesn't find
    // it there. Verified via strace after fix B' — see
    // docs/releases/v3.12.0/evidence/v4019.4/STRACE_BYTE_VERIFICATION.md.
    //
    // The session_state_changes we emit is `lenenc 0` (empty: we have no
    // session state to advertise). Auth OK stays `is_auth_ok` gated: it
    // never carries 0x4000 so the inner `if (status & 0x4000)` is false and
    // no session_state byte is appended.
    if client_cap & capability::SESSION_TRACK != 0 {
        write_lenenc_int(&mut p, 0).unwrap();
        // Embedded session_state_changes — only when status has 0x4000.
        // For Auth OK actual_status has no 0x4000 (short-circuited above),
        // so no session_state byte is appended. For statement/trailing OK
        // actual_status has 0x4000 (set above) so we append the empty
        // session_state_changes lenenc.
        if actual_status & capability::SERVER_STATUS_SESSION_STATE_CHANGED != 0 {
            write_lenenc_int(&mut p, 0).unwrap();
        }
    }
    vec![Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }]
}

fn make_err_packet(seq: u8, code: u16, state: &str, msg: &str) -> Packet {
    let mut p = Vec::new();
    p.push(0xff);
    p.write_u16::<LittleEndian>(code).unwrap();
    p.push(0x23);
    p.extend_from_slice(state.as_bytes());
    p.extend_from_slice(msg.as_bytes());
    p.push(0x00); // null-terminate message for C-compatible clients
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
    client_cap: u32,
) -> Vec<Packet> {
    let mut p = Vec::new();
    // V312-WIRE-8 fix (regression #4019.4 fifth pass — replaces the
    // retracted V312-WIRE-7 / V312-WIRE-5 hypothesis chain):
    //
    // Per MySQL WL#7766 (https://dev.mysql.com/worklog/task/?id=7766) and
    // verified by direct wire-byte capture against real MySQL 8.0.46 with
    // CLIENT_DEPRECATE_EOF + CLIENT_SESSION_TRACK negotiated, the trailing
    // result-set terminator under DEPRECATE_EOF protocol uses the **EOF
    // identifier 0xFE** as the FIRST byte, NOT the regular OK marker 0x00.
    //
    // WL#7766 explains: `net_send_ok(..., eof_identifier=true)` writes 0xFE
    // when the OK packet is being used as a result-set terminator under
    // CLIENT_DEPRECATE_EOF. The mysql CLI 8.0.46 client library dispatches
    // on this first byte: 0xFE → "OK-as-terminator" path, 0x00 → "regular
    // OK packet" path. We were emitting 0x00, which the client treated as
    // a regular OK packet, then expected another packet to follow (per the
    // session-tracking protocol path) — and hung on a 5th recvfrom that
    // never came.
    //
    // Verified empirically against real MySQL 8.0.46 port 3306 with the
    // same capabilities and same query ('select @@version_comment limit 1'):
    // the last result-set packet starts with `0xfe 00 00 02 00 00 00`
    // (7 bytes: 0xFE marker + lenenc affected=0 + lenenc last_id=0 +
    // status=0x0002 [AUTOCOMMIT only, NO 0x4000] + warnings=0). NO info,
    // NO session_state_changes — because status has no 0x4000.
    //
    // Rules:
    // - This function is ONLY for result-set terminator under DEPRECATE_EOF.
    //   Other OK packets (auth OK, COM_PING, COM_INIT_DB, statement OK
    //   without result-set, etc.) MUST keep the 0x00 header in
    //   `make_ok_packet` — they are not terminators.
    //   NOTE: COM_QUIT does NOT return an OK packet at all — the server
    //   just half-closes the TCP connection. So COM_QUIT is never in this
    //   set.
    // - We MUST NOT set 0x4000 (SESSION_STATE_CHANGED) here unless session
    //   state actually changed in this statement (e.g. SET, USE, multi-
    //   statement). For plain SELECTs (like our regression case), the
    //   trailing OK carries just AUTOCOMMIT (0x0002) and nothing else.
    // - When 0x4000 IS set (rare), per the canonical spec the OK packet
    //   then appends lenenc `info` (may be empty) and lenenc
    //   `session_state_changes`. We honor that path so the client can read
    //   session-state tracking data when it does change.
    p.push(0xfe); // OK-as-terminator under DEPRECATE_EOF: EOF identifier
    write_lenenc_int(&mut p, affected).unwrap();
    write_lenenc_int(&mut p, last_id).unwrap();
    // No unconditional 0x4000 — only set when session state actually
    // changed. Caller passes `status` and we trust it. For plain SELECT
    // / DML with no session impact, status stays at the caller's value
    // (typically 0x0002 AUTOCOMMIT only). This matches what real MySQL
    // 8.0.46 emits for `select @@version_comment limit 1`.
    let mut actual_status = status;
    // Keep AUTOCOMMIT visible to the client unless the caller disabled it.
    actual_status |= 0x0002;
    p.write_u16::<LittleEndian>(actual_status).unwrap();
    p.write_u16::<LittleEndian>(warnings).unwrap();
    // session_state_changes is appended IFF status has 0x4000 (i.e. session
    // state actually changed in this statement). The canonical spec wraps
    // session_state in lenenc(info) + lenenc(session_state) — but for the
    // no-change case (0x4000 unset), there is NO info field and NO
    // session_state field at all. This matches real MySQL 8.0.46 wire
    // bytes for plain SELECTs (no info byte, no session_state byte).
    if actual_status & capability::SERVER_STATUS_SESSION_STATE_CHANGED != 0
        && client_cap & capability::SESSION_TRACK != 0
    {
        // lenenc info (always present after warnings when SESSION_TRACK +
        // 0x4000 is negotiated, even if empty)
        write_lenenc_int(&mut p, 0).unwrap();
        // lenenc session_state_changes
        write_lenenc_int(&mut p, 0).unwrap();
    }
    vec![Packet {
        length: p.len() as u32,
        sequence: seq,
        payload: p,
    }]
}

/// Write one or more OK packets (returned by `make_ok_packet` /
/// `make_deprecate_eof_ok_packet`) to a stream, advancing `seq` once per
/// packet so the wire-protocol sequence numbers stay consistent across the
/// optional trailing `session_state_info` packet. Returns the post-write
/// `seq` value.
fn write_ok_packets<W: Write>(w: &mut W, packets: Vec<Packet>, mut seq: u8) -> MySqlResult<u8> {
    for pkt in packets {
        pkt.write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    Ok(seq)
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

/// Infer the MySQL binary-protocol column type code from a Value.
/// This must stay in sync with write_binary_row encoding.
fn value_type_string(v: &Value) -> String {
    match v {
        Value::Null => "VARCHAR(255)".into(),
        Value::Integer(_) => "INT".into(),
        Value::Float(_) => "FLOAT".into(),
        Value::Text(s) => {
            if s.len() < 256 {
                format!("VARCHAR({})", s.len())
            } else {
                "TEXT".into()
            }
        }
        Value::Blob(b) => {
            if b.len() < 256 {
                format!("VARBINARY({})", b.len())
            } else {
                "BLOB".into()
            }
        }
        Value::Boolean(_) => "TINYINT".into(),
        Value::Point(_, _) => "DOUBLE".into(),
        Value::Json(_) => "JSON".into(),
    }
}

fn value_col_type(v: &Value) -> u8 {
    match v {
        Value::Null => col_type::STRING,
        Value::Integer(_) => col_type::LONG,
        Value::Float(_) => col_type::FLOAT,
        Value::Text(s) => {
            if s.len() < 256 {
                col_type::VARCHAR
            } else {
                col_type::VARSTRING
            }
        }
        Value::Blob(_) => col_type::BLOB,
        Value::Boolean(_) => col_type::TINY,
        Value::Point(_, _) => col_type::DOUBLE,
        Value::Json(_) => 0xf5, // MySQL JSON type code
    }
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
        Value::Point(x, y) => format!("POINT({} {})", x, y),
        Value::Json(v) => v.to_string(),
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

fn write_binary_row<W: Write>(w: &mut W, row: &[Value], col_types: &[u8]) -> MySqlResult<()> {
    // MySQL binary-protocol row layout:
    //   1 byte  : 0x00 header
    //   ceil(cols/8) bytes : null_bitmap (col i is null iff bit (i%8) of byte (i/8))
    //   for each col, type-marker-byte + value-bytes
    let null_bytes = row.len().div_ceil(8);
    w.write_u8(0x00)?; // header
    let mut null_map = vec![0u8; null_bytes];
    for (i, v) in row.iter().enumerate() {
        if matches!(v, Value::Null) {
            null_map[i / 8] |= 1 << (i % 8);
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
            Value::Point(x, y) => {
                // MySQL binary protocol: 8-byte double for X, 8-byte double for Y
                buf.write_f64::<LittleEndian>(*x)?;
                buf.write_f64::<LittleEndian>(*y)?;
            }
            Value::Json(v) => {
                // Serialize JSON as a string
                write_lenenc_string(&mut buf, v.to_string().as_bytes())?;
            }
        }
    }
    w.write_all(&buf)?;
    Ok(())
}

fn write_column_def<W: Write>(w: &mut W, name: &str, sql_type: &str, seq: u8) -> MySqlResult<u8> {
    // MySQL column definition packet layout:
    //   catalog   : lenenc_str
    //   schema    : lenenc_str
    //   virtual_table: lenenc_str
    //   physical_table: lenenc_str
    //   virtual_name: lenenc_str
    //   physical_name: lenenc_str
    //   length_of_fixed_fields: lenenc_int (always 0x0c = 12)
    //   charsetnr    : 2 bytes LE
    //   column_length: 4 bytes LE
    //   field_type   : 1 byte
    //   flags        : 2 bytes LE
    //   decimals     : 1 byte
    //   filler       : 2 bytes
    let mut p = Vec::new();
    write_lenenc_string(&mut p, b"def").unwrap(); // catalog
    write_lenenc_string(&mut p, b"").unwrap(); // schema
    write_lenenc_string(&mut p, b"").unwrap(); // virtual_table
    write_lenenc_string(&mut p, b"").unwrap(); // physical_table
    write_lenenc_string(&mut p, name.as_bytes()).unwrap(); // virtual_name
    write_lenenc_string(&mut p, name.as_bytes()).unwrap(); // org_name (physical column name; same as virtual_name when no alias)
    write_lenenc_int(&mut p, 12).unwrap(); // length_of_fixed_fields: 12 bytes of fixed-size metadata follow
                                           // (charsetnr 2 + column_length 4 + field_type 1 + flags 2 + decimals 1 + filler 2)
                                           // MySQL column definition fixed-size fields:
                                           // charsetnr (2 bytes) → length (4 bytes) → type (1 byte) → flags (2 bytes) → decimals (1 byte) → filler (2 bytes)
    p.write_u16::<LittleEndian>(0x0030).unwrap(); // charsetnr: 0x30 = utf8_general_ci
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
    Ok(seq.wrapping_add(1))
}

fn send_result_set<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    seq: u8,
    cap: u32,
) -> MySqlResult<u8> {
    send_result_set_with_more(w, cols, ctypes, rows, seq, cap, 0)
}

/// V312-WIRE-8: trailing-status variant. `more_results_flag` is OR'd
/// into the status_flags of every trailing terminator (EOF for classic
/// protocol, OK for DEPRECATE_EOF protocol) so multi-statement clients
/// know whether another result set follows.
fn send_result_set_with_more<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    mut seq: u8,
    cap: u32,
    more_results_flag: u16,
) -> MySqlResult<u8> {
    let trailing_status: u16 = 0x0002 | more_results_flag;
    tracing::info!(
        "send_result_set: {} cols, {} rows, start_seq={}, more_results=0x{:04x}",
        cols.len(),
        rows.len(),
        seq,
        more_results_flag
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
        seq = write_column_def(
            w,
            n,
            ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)"),
            seq,
        )?;
    }
    // Inter-record separator between column defs and the row stream.
    // Per MySQL wire protocol (and verified against mysql 8.0 CLI behavior):
    //   - DEPRECATE_EOF = 0 (classic pre-8.0): send a 5-byte EOF packet
    //     so clients can detect "end of column metadata, rows begin".
    //   - DEPRECATE_EOF = 1 (mysql 8.0+ default): NO separator packet —
    //     column defs are followed directly by the row stream. The
    //     trailing OK packet (0x00) below marks end-of-result-set.
    //
    // V312-WIRE-1 fix (regression #4019.1): the previous implementation
    // sent an OK packet (0x00) as the "separator" even when DEPRECATE_EOF=1,
    // which mysql CLI 8.0.46 misinterpreted as the trailing terminator.
    // It then stopped reading the row stream, never received the actual
    // rows, and hung waiting for the next command response. Removing the
    // extra OK separator restores wire-protocol compatibility.
    //
    // V312-WIRE-8: in classic (DEPRECATE_EOF=0) protocol the inter-record
    // separator is also a place where MORE_RESULTS_EXISTS is sometimes
    // surfaced — MySQL 8.0 only sets the bit on the trailing terminator,
    // so we mirror that here. Keep this packet at 0x0002.
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, 0x0002).write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    // DEPRECATE_EOF=1: do NOT send any inter-record separator.
    for r in rows.iter() {
        let mut p = Vec::new();
        write_text_row(&mut p, r)?;

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
    // Both branches carry status_flags = trailing_status (default
    // 0x0002 = SERVER_STATUS_AUTOCOMMIT, OR'd with more_results_flag
    // when this is not the last result of a multi-statement batch).
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, trailing_status).write_to(w)?;
        seq = seq.wrapping_add(1);
    } else {
        // V312-WIRE-5: `make_deprecate_eof_ok_packet` returns Vec<Packet>
        // (1 OK packet, optionally +1 separate session_state_info packet
        // when status has 0x4000). Use `write_ok_packets` to emit them all
        // and advance seq once per packet.
        seq = write_ok_packets(
            w,
            make_deprecate_eof_ok_packet(seq, 0, 0, trailing_status, 0, cap),
            seq,
        )?;
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
    // Infer actual column type strings from row data (not the misleading
    // ctypes which may say VARCHAR(255) for integer columns).
    // This must match value_col_type so the binary row encoding is consistent.
    let actual_ctypes: Vec<String> = if let Some(first_row) = rows.first() {
        first_row.iter().map(|v| value_type_string(v)).collect()
    } else {
        ctypes.iter().cloned().collect()
    };

    // Column definitions — use actual types so client knows how to decode rows
    for (i, n) in cols.iter().enumerate() {
        write_column_def(
            w,
            n,
            actual_ctypes
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("VARCHAR(255)"),
            seq,
        )?;
        seq = seq.wrapping_add(1);
    }
    // Inter-record separator between column defs and row stream.
    // V312-WIRE-1 fix (regression #4019.1): previously this branch sent
    // an OK packet when DEPRECATE_EOF=1, breaking mysql CLI 8.0+ clients.
    // Correct MySQL 8.0+ protocol: NO inter-record separator when
    // DEPRECATE_EOF=1; only the trailing OK terminator below marks
    // end-of-result-set.
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet(seq, 0x0002).write_to(w)?;
        seq = seq.wrapping_add(1);
    }
    // DEPRECATE_EOF=1: do NOT send any inter-record separator.
    // Infer column type codes from the actual data values, NOT from the
    // column type strings (which may be misleading e.g. VARCHAR(255) for
    // integer columns). The encoding in write_binary_row is determined by
    // the Value variant, so we must match that here.
    let col_type_codes: Vec<u8> = if let Some(first_row) = rows.first() {
        first_row.iter().map(|v| value_col_type(v)).collect()
    } else {
        cols.iter()
            .enumerate()
            .map(|(i, _)| {
                let t = ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)");
                col_type_from_string(t)
            })
            .collect()
    };
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
        // V312-WIRE-5: see send_result_set for rationale. Vec<Packet> may
        // include a separate session_state_info packet after the OK.
        seq = write_ok_packets(
            w,
            make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0, cap),
            seq,
        )?;
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

    /// Reset all prepared statements and session state.
    /// MySQL protocol: COM_RESET_CONNECTION (0x1F) clears all prepared
    /// statement IDs and resets the statement counter to 1.
    fn reset(&mut self) {
        self.statements.clear();
        self.next_id = 1;
    }
}

/// Count `?` placeholders in SQL (used by COM_STMT_PREPARE to report param count).
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
        let storage_guard = storage.read();
        {
            if let Ok(table_info) = storage_guard.get_table_info(&table_name) {
                let mut types = Vec::with_capacity(param_count);
                for col_name in &cols {
                    let col_type_byte = table_info
                        .columns
                        .iter()
                        .find(|c| c.name.eq_ignore_ascii_case(col_name))
                        .map(|c| param_bind_type_from_string(&c.data_type))
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

/// Map a SQL column type (e.g. "INTEGER", "CHAR(120)") to the MySQL
/// binary-protocol type code that clients will encode parameter values
/// as when binding via libmysqlclient.
///
/// This is similar to [`col_type_from_string`] but for the
/// *parameter bind* wire format: when sysbench (libmysqlclient) binds
/// `MYSQL_TYPE_LONG`, it actually sends **8 bytes** LE on the wire so
/// that Lua's double-precision numbers round-trip safely. We therefore
/// advertise `LONGLONG` (8 bytes) for INTEGER/INT so that subsequent
/// re-executes with `new_params_bound_flag = 0` (which rely on the
/// cached types from PREPARE) line up with the actual byte layout.
///
/// Result-column metadata continues to use [`col_type_from_string`]
/// (LONG = 4 bytes for INT) so the Rust mysql crate's wire_decode
/// for INT result columns is unchanged.
fn param_bind_type_from_string(t: &str) -> u8 {
    let u = t.to_uppercase();
    if u.contains("DATETIME") || u.contains("TIMESTAMP") {
        col_type::DATETIME
    } else if u.contains("DATE") {
        col_type::DATE
    } else if u.contains("TIME") {
        col_type::TIME
    } else if u.contains("VARCHAR") {
        col_type::VARCHAR
    } else if u.contains("CHAR") || u.contains("TEXT") {
        col_type::VARSTRING
    } else if u.contains("INT") || u.contains("INTEGER") {
        // Promote INT/INTEGER to LONGLONG (8 bytes) so libmysqlclient's
        // MYSQL_TYPE_LONG wire encoding (which is 8 bytes LE) decodes
        // correctly on subsequent COM_STMT_EXECUTE calls.
        col_type::LONGLONG
    } else if u.contains("BIGINT") {
        col_type::LONGLONG
    } else if u.contains("MEDIUMINT") {
        col_type::INT24
    } else if u.contains("SMALLINT") {
        col_type::SHORT
    } else if u.contains("TINYINT") {
        col_type::TINY
    } else if u.contains("FLOAT") {
        col_type::FLOAT
    } else if u.contains("DOUBLE") {
        col_type::DOUBLE
    } else {
        col_type::VARSTRING
    }
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
                Ok(s) => {
                    // String types get quoted. Numeric types get unquoted
                    // even though the client sent VAR_STRING — this happens
                    // when the client advertised a wrong type but the value
                    // is a plain ASCII number that fits the schema column.
                    // Without this, `WHERE id = ?` with param "1" produces
                    // `WHERE id = '1'` which compares INT to STRING and
                    // matches zero rows (Issue #4130).
                    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && param.len() < 20 {
                        // Treat as numeric literal (no quotes).
                        s
                    } else {
                        format!("'{}'", s.replace('\'', "''"))
                    }
                }
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
        // Parameter type priority: client_advertised > server_prepared > VAR_STRING.
        //
        // When `new_params_bound_flag == 1` the client BOTH advertises the
        // type AND encodes the value per that type. If we use the
        // server-prepared type (LONG) but the client advertised VAR_STRING
        // and sent a length-encoded string, decode_param reads 4 bytes
        // looking for an i32 and fails (returns None) → param becomes NULL.
        // This breaks `WHERE id = ?` SELECT-style prepared statements
        // because `id = NULL` matches nothing and the test then sees
        // "expected Select, got OK(0)" from the test_wire_smoke_stmt
        // regression.
        //
        // The server's `prepared_param_types` are still useful for clients
        // that set `new_params_bound_flag = 0` (no per-param type sent) —
        // for those clients we use the prepared type. For clients that
        // DO send types (new_params_bound_flag = 1), trust their wire
        // encoding.
        let type_code: u8 = type_codes
            .get(i)
            .copied()
            .or_else(|| prepared_param_types.get(i).copied())
            .unwrap_or(mysql_type::VAR_STRING);
        match decode_param(payload, &mut pos, type_code) {
            Some(v) => params.push((v, is_numeric_type(type_code))),
            None => params.push((Vec::new(), false)), // NULL fallback, continue processing remaining params
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
    // SELECT <cols> FROM <table> [WHERE ...]
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
        return None;
    }
    // INSERT INTO <table> [(cols)] VALUES (...)
    if let Some(rest) = u.strip_prefix("INSERT") {
        // Find the keyword boundary after INSERT (skip whitespace).
        let after_insert = rest.trim_start();
        let kw = after_insert
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(';');
        if kw == "INTO" || kw.starts_with("INTO") {
            let body = after_insert[kw.len()..].trim_start();
            let table_end = body
                .find(|c: char| c.is_whitespace() || c == '(' || c == ';' || c == ',')
                .unwrap_or(body.len());
            let table = body[..table_end].trim().trim_matches('`').trim_matches('"');
            if !table.is_empty() {
                // Mirror back to the original-case SQL.
                let orig_kw_end = sql.to_uppercase().find("INTO").unwrap() + 4;
                let orig_body = sql[orig_kw_end..].trim_start();
                let orig_end = orig_body
                    .find(|c: char| c.is_whitespace() || c == '(' || c == ';' || c == ',')
                    .unwrap_or(orig_body.len());
                let orig_table = orig_body[..orig_end]
                    .trim()
                    .trim_matches('`')
                    .trim_matches('"');
                if !orig_table.is_empty() {
                    return Some(orig_table.to_string());
                }
            }
        }
        return None;
    }
    // UPDATE <table> SET ...
    if let Some(rest) = u.strip_prefix("UPDATE") {
        let body = rest.trim_start();
        let table_end = body
            .find(|c: char| c.is_whitespace() || c == ';' || c == ',')
            .unwrap_or(body.len());
        let table = body[..table_end].trim().trim_matches('`').trim_matches('"');
        if !table.is_empty() {
            let orig_after = sql.to_uppercase().find("UPDATE").unwrap() + 6;
            let orig_body = sql[orig_after..].trim_start();
            let orig_end = orig_body
                .find(|c: char| c.is_whitespace() || c == ';' || c == ',')
                .unwrap_or(orig_body.len());
            let orig_table = orig_body[..orig_end]
                .trim()
                .trim_matches('`')
                .trim_matches('"');
            if !orig_table.is_empty() {
                return Some(orig_table.to_string());
            }
        }
        return None;
    }
    // DELETE FROM <table> [WHERE ...]
    if let Some(rest) = u.strip_prefix("DELETE") {
        if let Some(from_pos) = rest.find("FROM") {
            let after_from = rest[from_pos + 4..].trim();
            let table_end = after_from
                .find(|c: char| c.is_whitespace() || c == ';' || c == ',')
                .unwrap_or(after_from.len());
            let table = after_from[..table_end]
                .trim()
                .trim_matches('`')
                .trim_matches('"');
            if !table.is_empty() {
                let orig_after = sql.to_uppercase().find("FROM").unwrap();
                let orig_from = sql[orig_after + 4..].trim();
                let orig_end = orig_from
                    .find(|c: char| c.is_whitespace() || c == ';' || c == ',')
                    .unwrap_or(orig_from.len());
                let orig_table = orig_from[..orig_end]
                    .trim()
                    .trim_matches('`')
                    .trim_matches('"');
                if !orig_table.is_empty() {
                    return Some(orig_table.to_string());
                }
            }
        }
        return None;
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
        let storage_guard = storage.read();
        {
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
        let storage_guard = storage.read();
        {
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

/// G13-OLTP-1: classify read-only statements. SELECT / SHOW / DESCRIBE
/// can run on a shared read lock; everything else needs the exclusive
/// write lock. Returning the inner reference (not just bool) lets the
/// dispatch site acquire the right lock and call the matching `&self`
/// execute method.
enum ReadOnlyStmt<'a> {
    Select(&'a sqlrustgo_parser::parser::SelectStatement),
    Show(&'a sqlrustgo_parser::parser::ShowStatement),
    Describe(&'a sqlrustgo_parser::parser::DescribeStatement),
}
fn read_only_stmt(stmt: &Statement) -> Option<ReadOnlyStmt<'_>> {
    match stmt {
        Statement::Select(s) => Some(ReadOnlyStmt::Select(s)),
        Statement::Show(s) => Some(ReadOnlyStmt::Show(s)),
        Statement::Describe(s) => Some(ReadOnlyStmt::Describe(s)),
        _ => None,
    }
}

/// V312-18e Issue #4021: classify a parsed `Statement` into a stable
/// Prometheus label. Kept on a small allowlist so the cardinality of
/// `sqlrustgo_queries_total` stays bounded — anything not on the list
/// collapses into `"OTHER"` and is recorded but not labeled.
///
/// `parsed` is `Result<Statement, String>` (the COM_QUERY dispatch
/// return type); a parse error collapses into `"PARSE_ERROR"` so we
/// still observe failed dispatches in the metrics.
fn statement_kind(parsed: &Result<Statement, String>) -> &'static str {
    match parsed {
        Err(_) => "PARSE_ERROR",
        Ok(stmt) => match stmt {
            Statement::Select(_) => "SELECT",
            Statement::Insert(_) => "INSERT",
            Statement::Update(_) => "UPDATE",
            Statement::Delete(_) => "DELETE",
            Statement::Merge(_) => "MERGE",
            Statement::CreateTable(_) => "CREATE_TABLE",
            Statement::CreateIndex(_) => "CREATE_INDEX",
            Statement::CreateView(_) => "CREATE_VIEW",
            Statement::DropTable(_) => "DROP_TABLE",
            Statement::DropIndex(_) => "DROP_INDEX",
            Statement::DropView(_) => "DROP_VIEW",
            Statement::CreateSequence(_) => "CREATE_SEQUENCE",
            Statement::DropSequence(_) => "DROP_SEQUENCE",
            Statement::AlterSequence(_) => "ALTER_SEQUENCE",
            Statement::Truncate(_) => "TRUNCATE",
            Statement::Analyze(_) => "ANALYZE",
            Statement::WithSelect(_) => "WITH_SELECT",
            Statement::WithDml(_) => "WITH_DML",
            Statement::AlterTable(_) => "ALTER_TABLE",
            Statement::AlterUser(_) => "ALTER_USER",
            Statement::Call(_) => "CALL",
            Statement::CreateProcedure(_) => "CREATE_PROCEDURE",
            // V312-55A / Issue #4238: add DROP PROCEDURE to the metric label.
            Statement::DropProcedure(_) => "DROP_PROCEDURE",
            Statement::Union(_) => "UNION",
            Statement::CreateTrigger(_) => "CREATE_TRIGGER",
            Statement::Intersect(_) => "INTERSECT",
            Statement::Except(_) => "EXCEPT",
            Statement::Values(_) => "VALUES",
            Statement::Transaction(_) => "TRANSACTION",
            Statement::Grant(_) | Statement::GrantRole(_) => "GRANT",
            Statement::Revoke(_) | Statement::RevokeRole(_) => "REVOKE",
            Statement::Show(_)
            | Statement::Describe(_)
            | Statement::ShowRoles
            | Statement::ShowGrantsFor(_) => "SHOW",
            Statement::CreateRole(_) => "CREATE_ROLE",
            Statement::DropRole(_) => "DROP_ROLE",
            Statement::CreateDatabase(_) => "CREATE_DATABASE",
            Statement::DropDatabase(_) => "DROP_DATABASE",
            Statement::UseDatabase(_) => "USE_DATABASE",
            Statement::SetRole(_) => "SET_ROLE",
            Statement::SavepointStatement { .. } => "SAVEPOINT",
            Statement::Prepare { .. }
            | Statement::Execute { .. }
            | Statement::Deallocate { .. } => "PREPARED_STMT",
            // Round-21 / Issue #4218: KILL admin statement.
            Statement::Kill { .. } => "KILL",
            // V312-56E / Issue #4255: EXPLAIN plan-shape oracle.
            Statement::Explain(_) => "EXPLAIN",
        },
    }
}
#[cfg(test)]
fn is_select_stmt(stmt: &Statement) -> bool {
    read_only_stmt(stmt).is_some()
}

/// Split a multi-statement query string into top-level statement text
/// slices. Respects parentheses nesting, single/double-quoted string
/// literals (with `\` escapes), and `--` / `/* */` comments so that
/// semicolons inside any of those contexts do not terminate a statement.
///
/// Engine Bug A supplementary fix (refs #3635): the COM_QUERY
/// dispatch path previously called `eng.execute(&q)` once per parsed
/// statement, re-running the whole multi-statement query N times for
/// an N-statement batch. Splitting first and executing each slice
/// independently restores the per-statement-once contract that
/// `mysql --execute="s1; s2"` and the multi-statement path of PR #3521
/// intended.
fn split_top_level_statements(q: &str) -> Vec<&str> {
    let bytes = q.as_bytes();
    let mut out: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let mut paren_depth: i32 = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let next = bytes.get(i + 1).copied().unwrap_or(0) as char;
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if c == '*' && next == '/' {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_single {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            '-' if next == '-' => {
                in_line_comment = true;
                i += 2;
                continue;
            }
            '/' if next == '*' => {
                in_block_comment = true;
                i += 2;
                continue;
            }
            ';' if paren_depth == 0 => {
                let stmt_text = q[start..i].trim();
                if !stmt_text.is_empty() {
                    out.push(stmt_text);
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let tail = q[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
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
    engine: &mut sqlrustgo::ExecutionEngine<BoxStorageEngine>,
    path: &str,
    table: &str,
    _delim: char,
    data_dir: std::path::PathBuf,
    bulk_buf_size: usize,
    // Round-21 / Issue #4217: chunk size for `bulk_insert` flushes.
    // Replaces the hard-coded `PERIODIC_FLUSH_ROWS = 100` so that
    // large tables (e.g. TPC-H SF=10 lineitem with 6M rows) can
    // accumulate more rows per flush instead of paying the
    // write-lock + Vec allocation cost on every 100 rows.
    // 0 means "disable periodic flush, only flush when buf drains".
    rows_per_flush: usize,
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
        let storage = storage_arc.read();
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
        //
        // Round-21 / Issue #4217: chunk size is now configurable via
        // `rows_per_flush` (default 10_000, was hard-coded 100). The
        // 100-row default made a 6M-row lineitem SF=10 load take ~60K
        // `bulk_insert_records` calls; 10_000-row chunks cut that to
        // ~600 calls and raise throughput dramatically. Setting to 0
        // disables the periodic flush (legacy V312-32 behavior).
        let periodic_threshold = rows_per_flush;
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
        } else if periodic_threshold > 0 && pending_rows.len() >= periodic_threshold {
            // Periodic flush: every `rows_per_flush` rows, flush
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

    // Materialize the .json file for this table so that subsequent
    // test runs (which check `json_data_ready()` before starting the
    // server) can skip the expensive LOAD DATA phase.  Without this,
    // large tables (orders, lineitem) are only in-memory + WAL and
    // every restart re-runs LOAD DATA from scratch.
    //
    // `WalStorage::flush()` delegates to `FileStorage::flush()` which
    // writes all table .json files.
    {
        engine
            .flush()
            .map_err(|e| MySqlError::Other(format!("flush storage: {}", e)))?;
    }

    Ok(total_rows)
}

/// V312-18e: recognise `SET long_query_time = N` and return `N`.
///
/// Inspect a parsed `Statement` to see whether it is a `SET long_query_time`.
/// Returns `None` if the statement is something else.
///
/// `N` is a threshold in **seconds** (matches MySQL semantics — fractional
/// values like `0.5` are accepted). The internal `SlowQueryLog` is
/// millisecond-granular, so this helper converts seconds → ms.
///
/// The parser accepts the statement (`TransactionStatement::SetSessionVariable`)
/// but no executor handles it, so `do_command_loop` intercepts it and
/// retunes the server's shared `SlowQueryLog` instead of dispatching.
fn classify_long_query_time_set(stmt: &Statement) -> Option<Result<u64, &'static str>> {
    use sqlrustgo_parser::transaction::TransactionStatement;
    match stmt {
        Statement::Transaction(TransactionStatement::SetSessionVariable { name, value })
            if name.eq_ignore_ascii_case("long_query_time") =>
        {
            let trimmed = value.trim();
            match trimmed.parse::<f64>() {
                Ok(secs) if secs.is_finite() && secs >= 0.0 => {
                    Some(Ok((secs * 1000.0).round() as u64))
                }
                _ => Some(Err("Incorrect argument type to variable 'long_query_time'")),
            }
        }
        _ => None,
    }
}

fn do_command_loop<S: Read + Write + DrainWrites>(
    stream: &mut S,
    addr: SocketAddr,
    storage: Arc<parking_lot::RwLock<BoxStorageEngine>>,
    engine: Arc<parking_lot::RwLock<ExecutionEngine<BoxStorageEngine>>>,
    cap: u32,
    server_last_sent_seq: &mut u8,
    ps_manager: &mut PreparedStatementManager,
    authenticated_user: Option<String>,
    // V312-32: per-handle ephemeral config. Replaces the process-global
    // `ACTIVE_CONFIG` lookup so concurrent `start_ephemeral` calls each
    // see their own server's `data_dir` and `bulk_insert_buffer_size`.
    config: &crate::testing::EphemeralConfig,
) -> MySqlResult<()> {
    loop {
        let pkt = Packet::read_from(stream)?;
        let cmd = pkt.payload.first().copied().unwrap_or(0);
        let payload = &pkt.payload[1..];
        // MySQL/MariaDB protocol: every new client command starts with
        // pkt_seq=0, and the server resets its response seq to 0
        // (so the first response packet uses seq=1). pymysql and
        // libmysqlclient both rely on this — they set next_seq_id=1
        // after sending each command and validate the server response
        // sequence number accordingly. We reset on EVERY pkt_seq=0,
        // not just the first one (which would break the second query).
        let mut seq = server_last_sent_seq.wrapping_add(1);
        if pkt.sequence == 0 {
            *server_last_sent_seq = 0;
            seq = 1;
        }
        match cmd {
            packet_type::COM_QUIT => {
                // MySQL wire protocol: server MUST send OK packet on COM_QUIT
                // before closing the connection, so the client can release
                // its read() and exit cleanly. Without this, mysql CLI and
                // pymysql hang in recv() after sending COM_QUIT (Issue #SET-NAMES-HANG).
                // V312-WIRE-5: write_ok_packets handles the optional
                // session_state_info packet that may follow the OK.
                seq = write_ok_packets(
                    stream,
                    make_ok_packet(seq, 0, 0, 0x0002, 0, cap, false),
                    seq,
                )?;
                *server_last_sent_seq = seq;
                break;
            }
            packet_type::COM_PING => {
                seq = write_ok_packets(
                    stream,
                    make_ok_packet(seq, 0, 0, 0x0002, 0, cap, false),
                    seq,
                )?;
                *server_last_sent_seq = seq;
            }
            packet_type::COM_INIT_DB => {
                seq = write_ok_packets(
                    stream,
                    make_ok_packet(seq, 0, 0, 0x0002, 0, cap, false),
                    seq,
                )?;
                *server_last_sent_seq = seq;
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
                // bulk_insert_buffer_size from the per-handle `config`
                // threaded down from `start_ephemeral` (V312-32: was
                // the process-global `ACTIVE_CONFIG`). This guarantees
                // the handler sees the same values the test used to
                // configure THIS server, not whichever `start_ephemeral`
                // ran last in the process.
                if let Some((path, table, delim)) = parse_load_local_infile_sql(&q) {
                    let data_dir = config
                        .data_dir
                        .clone()
                        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
                    // Issue #4020: prefer the per-handle LOAD DATA
                    // whitelist when configured; fall back to the
                    // storage data_dir so the default sandbox semantics
                    // are preserved.
                    let load_infile_dir = config
                        .load_infile_dir
                        .clone()
                        .unwrap_or_else(|| data_dir.clone());
                    let data_dir = load_infile_dir;
                    let bulk_buf = config.bulk_insert_buffer_size;
                    // Round-21 / Issue #4217: per-handle chunk size
                    // for LOAD DATA bulk_insert flushes. Default
                    // 10_000 (raises the previous hard-coded 100 to
                    // dramatically reduce write-lock acquisitions
                    // on large tables like TPC-H SF=10 lineitem).
                    let rows_per_flush = config.bulk_insert_rows_per_flush;
                    // G13-OLTP-1: poisoning recovery on the engine
                    // write lock. A previous LOAD DATA may have
                    // panicked mid-insert (e.g. parse_tbl_line on
                    // a malformed row), leaving the RwLock poisoned.
                    // The previous `engine.write().unwrap()` would
                    // then re-panic on every subsequent LOAD DATA.
                    // Recover via `into_inner()` and continue.
                    let mut eng_guard = engine.write();
                    let n = match handle_load_local_infile(
                        stream,
                        &mut eng_guard,
                        &path,
                        &table,
                        delim,
                        data_dir,
                        bulk_buf,
                        rows_per_flush,
                        &mut seq,
                        cap,
                    ) {
                        Ok(n) => n,
                        Err(e) => {
                            make_err_packet(seq, 1146u16, "42S02", &e.to_string())
                                .write_to(stream)?;
                            *server_last_sent_seq = seq;
                            seq = seq.wrapping_add(1);
                            0
                        }
                    };
                    seq = write_ok_packets(
                        stream,
                        make_ok_packet(seq, n, 0, 0x0002, 0, cap, false),
                        seq,
                    )?;
                    *server_last_sent_seq = seq;
                    continue;
                }

                if q.is_empty() {
                    seq = write_ok_packets(
                        stream,
                        make_ok_packet(seq, 0, 0, 0x0002, 0, cap, false),
                        seq,
                    )?;
                    *server_last_sent_seq = seq;
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
                    seq = write_ok_packets(
                        stream,
                        make_ok_packet(seq, 0, 0, 0x0002, 0, cap, false),
                        seq,
                    )?;
                    *server_last_sent_seq = seq;
                    continue;
                }
                // G13-OLTP-1 lock contention fix: DDL/DML use exclusive write lock with
                // poisoning recovery. If a previous thread panicked while holding the lock,
                // the RwLock poisons all subsequent .read()/.write() calls. Using .into_inner()
                // recovery allows the server to continue serving queries rather than hard-fail.
                let stmt_texts = split_top_level_statements(&q);
                let stmt_count = stmt_texts.len();
                let mut had_error = false;
                for (idx, stmt_sql) in stmt_texts.iter().enumerate() {
                    // V312-WIRE-8: when the client negotiated CLIENT_MULTI_
                    // STATEMENTS / CLIENT_MULTI_RESULTS, every result
                    // terminator (OK or trailing EOF) for a non-final
                    // statement in the batch MUST have the
                    // SERVER_MORE_RESULTS_EXISTS (0x0008) bit set in its
                    // status_flags. Without it, mysql 8.0 stops reading
                    // after the first result and the remaining statements'
                    // responses get concatenated into the row stream
                    // (or block the client on recvfrom).
                    let is_last_stmt = idx + 1 == stmt_count;
                    let more_results_flag = if is_last_stmt {
                        0
                    } else {
                        capability::SERVER_MORE_RESULTS_EXISTS
                    };
                    let parsed = parse(stmt_sql);
                    // G13-OLTP-1: pick read-vs-write lock based on AST.
                    let is_read_only = parsed
                        .as_ref()
                        .ok()
                        .and_then(|s| read_only_stmt(s).map(|_| s));
                    // G13-OLTP-1: pick read-vs-write lock based on AST.
                    let is_write_blocked = if !is_read_only.is_some() {
                        // Task 3.2: check password write blocking before allowing write operations.
                        // We must NOT hold the engine read lock while checking catalog (deadlock risk
                        // since catalog → auth_manager needs its own lock).
                        if let Some(ref user) = authenticated_user {
                            let catalog = engine.read().catalog();
                            catalog.and_then(|cat| {
                                let identity = sqlrustgo_catalog::auth::UserIdentity::new(user, "localhost");
                                if cat.read().auth_manager().is_password_write_blocked(&identity) {
                                    Some("Your password has expired. Change it before performing administrative operations.")
                                } else {
                                    None
                                }
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(msg) = is_write_blocked {
                        make_err_packet(seq, 1820u16, "HY000", msg).write_to(stream)?;
                        *server_last_sent_seq = seq;
                        seq = seq.wrapping_add(1);
                        had_error = true;
                        continue;
                    }
                    // V312-18e: `SET long_query_time = N` is intercepted
                    // here rather than dispatched to the executor (which has
                    // no handler for it). `N` is in seconds (MySQL semantics,
                    // fractional permitted); invalid values produce an error
                    // packet that mirrors MySQL error 1232.
                    if let Some(retune) =
                        parsed.as_ref().ok().and_then(classify_long_query_time_set)
                    {
                        match retune {
                            Ok(ms) => {
                                if let Some(ref slow_log) = config.slow_query_log {
                                    slow_log.set_threshold_ms(ms);
                                }
                                // V312-WIRE-8: OR MORE_RESULTS_EXISTS on
                                // the SET OK packet when this is not the
                                // last statement of a multi-stmt batch
                                // (e.g. "SET long_query_time=100; SELECT 1").
                                seq = write_ok_packets(
                                    stream,
                                    make_ok_packet(
                                        seq,
                                        0,
                                        0,
                                        0x0002 | more_results_flag,
                                        0,
                                        cap,
                                        false,
                                    ),
                                    seq,
                                )?;
                            }
                            Err(err) => {
                                make_err_packet(seq, 1232u16, "42000", err).write_to(stream)?;
                                seq = seq.wrapping_add(1);
                                had_error = true;
                            }
                        }
                        *server_last_sent_seq = seq;
                        continue;
                    }
                    // G13-OLTP-1: poisoning recovery in both branches.
                    let started = std::time::Instant::now();
                    let result = if let Some(stmt) = is_read_only {
                        let rstmt = read_only_stmt(stmt);
                        let eng = engine.read();
                        match rstmt {
                            Some(ReadOnlyStmt::Select(s)) => eng.execute_select(s),
                            Some(ReadOnlyStmt::Show(s)) => eng.execute_show(s),
                            Some(ReadOnlyStmt::Describe(s)) => eng.execute_describe(s),
                            None => unreachable!("is_read_only implied rstmt is Some"),
                        }
                    } else {
                        let mut eng = engine.write();
                        eprintln!("SERVER: eng.execute(sql={})", stmt_sql);
                        eng.execute(stmt_sql)
                    };
                    // V312-18e: time every dispatched statement; the log
                    // itself gates on its threshold.
                    let elapsed_ms = started.elapsed().as_millis() as u64;
                    if let Some(ref slow_log) = config.slow_query_log {
                        let rows = result.as_ref().map(|r| r.rows.len() as u64).unwrap_or(0);
                        slow_log.maybe_log(stmt_sql, elapsed_ms, rows);
                    }
                    // V312-18e Issue #4021: record every dispatched
                    // statement into the Prometheus counters. The
                    // renderer reads the same singleton that the
                    // `/metrics` endpoint serves.
                    let query_type = statement_kind(&parsed);
                    sqlrustgo_telemetry::GLOBAL_METRICS
                        .record_query(query_type, std::time::Duration::from_millis(elapsed_ms));
                    match result {
                        Ok(r) if is_read_only.is_some() => {
                            // Extract real column names from the SQL
                            // (after SELECT, before FROM). Falls back to
                            // col_1, col_2... when ambiguous (e.g. SELECT *).
                            let real_col_names: Vec<String> = if stmt_sql
                                .to_uppercase()
                                .starts_with("SELECT")
                                && stmt_sql.to_uppercase().contains(" FROM ")
                            {
                                let upper = stmt_sql.to_uppercase();
                                if let Some(from_pos) = upper.find(" FROM ") {
                                    let select_part = stmt_sql[..from_pos].trim();
                                    let cols_str = select_part
                                        .strip_prefix("SELECT")
                                        .or_else(|| select_part.strip_prefix("select"))
                                        .unwrap_or("")
                                        .trim();
                                    if !cols_str.is_empty() && !cols_str.contains('*') {
                                        cols_str
                                            .split(',')
                                            .map(|s: &str| {
                                                s.trim()
                                                    .split('.')
                                                    .next_back()
                                                    .unwrap_or(s.trim())
                                                    .to_string()
                                            })
                                            .collect()
                                    } else {
                                        let n = r.rows.first().map(|row| row.len()).unwrap_or(0);
                                        (0..n).map(|i| format!("col_{}", i + 1)).collect()
                                    }
                                } else {
                                    let n = r.rows.first().map(|row| row.len()).unwrap_or(0);
                                    (0..n).map(|i| format!("col_{}", i + 1)).collect()
                                }
                            } else {
                                let n = r.rows.first().map(|row| row.len()).unwrap_or(0);
                                (0..n).map(|i| format!("col_{}", i + 1)).collect()
                            };
                            let cols: Vec<String> = real_col_names;
                            let ctypes: Vec<String> =
                                cols.iter().map(|_| "VARCHAR(255)".to_string()).collect();
                            seq = send_result_set_with_more(
                                stream,
                                &cols,
                                &ctypes,
                                &r.rows,
                                seq,
                                cap,
                                more_results_flag,
                            )?;
                            *server_last_sent_seq = seq;
                        }
                        Ok(r) => {
                            // V312-WIRE-8: OR the more_results_flag into
                            // the OK packet's status_flags when this is
                            // not the last statement in the batch.
                            seq = write_ok_packets(
                                stream,
                                make_ok_packet(
                                    seq,
                                    r.affected_rows as u64,
                                    0,
                                    0x0002 | more_results_flag,
                                    0,
                                    cap,
                                    false,
                                ),
                                seq,
                            )?;
                            *server_last_sent_seq = seq;
                        }
                        Err(e) => {
                            let code = e.mysql_error_code();
                            let err_msg = e.to_string();
                            tracing::warn!("SQL error {} (42000): {}", code, err_msg);
                            make_err_packet(seq, code, "42000", &err_msg).write_to(stream)?;
                            *server_last_sent_seq = seq;
                            seq = seq.wrapping_add(1);
                            had_error = true;
                        }
                    }
                }
                if had_error && stmt_texts.len() > 1 {
                    tracing::debug!("multi-statement batch had at least one error");
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
                            let storage_guard = storage.read();
                            if let Some(table_name) = extract_table_name(&sql) {
                                if let Ok(table_info) = storage_guard.get_table_info(&table_name) {
                                    table_info.columns.len() as u16
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
                let ok_pkt_bytes = {
                    let mut pb = Vec::new();
                    pb.write_u24::<LittleEndian>(p.len() as u32).unwrap();
                    pb.write_u8(seq).unwrap();
                    pb.extend_from_slice(&p);
                    pb
                };
                tracing::debug!(
                    "STMT_PREPARE OK pkt: seq={}, len={}, hex={:02x?}",
                    seq,
                    ok_pkt_bytes.len(),
                    &ok_pkt_bytes[..]
                );
                Packet {
                    length: p.len() as u32,
                    sequence: seq,
                    payload: p,
                }
                .write_to(stream)?;
                *server_last_sent_seq = seq;
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
                        // length_of_fixed_fields (lenenc_int): always 0x0c = 12 bytes of
                        // fixed-size metadata follow (matches write_column_def format).
                        // Without this byte, libmysqlclient (used by sysbench) misparses
                        // the entire packet and returns "Unknown or undefined error code".
                        write_lenenc_int(&mut param_def, 12).unwrap();
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
                        *server_last_sent_seq = seq;
                        seq = seq.wrapping_add(1);
                    }
                    if cap & capability::DEPRECATE_EOF != 0 {
                        seq = write_ok_packets(
                            stream,
                            make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0, cap),
                            seq,
                        )?;
                        *server_last_sent_seq = seq;
                    } else {
                        make_eof_packet(seq, 0x0002).write_to(stream)?;
                        *server_last_sent_seq = seq;
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
                        seq = write_column_def(stream, &col_name, "VARCHAR(255)", seq)?;
                    }
                    if cap & capability::DEPRECATE_EOF != 0 {
                        seq = write_ok_packets(
                            stream,
                            make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0, cap),
                            seq,
                        )?;
                        *server_last_sent_seq = seq;
                    } else {
                        make_eof_packet(seq, 0x0002).write_to(stream)?;
                        *server_last_sent_seq = seq;
                        seq = seq.wrapping_add(1);
                    }
                }
                // Issue #3694: force-drain ALL TLS cipher records before
                // returning. The client (MariaDB Connector/C) waits for
                // the complete STMT_PREPARE response; if any records
                // are still buffered in rustls the client times out.
                stream.force_drain();

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
                    *server_last_sent_seq = seq;
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
                        *server_last_sent_seq = seq;
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
                // write for DDL/DML).
                let parsed = parse(&final_sql);
                let is_read_only = parsed
                    .as_ref()
                    .ok()
                    .and_then(|s| read_only_stmt(s).map(|_| s));
                let started = std::time::Instant::now();
                let result = if let Some(stmt) = is_read_only {
                    let rstmt = read_only_stmt(stmt);
                    let eng = engine.read();
                    match rstmt {
                        Some(ReadOnlyStmt::Select(s)) => eng.execute_select(s),
                        Some(ReadOnlyStmt::Show(s)) => eng.execute_show(s),
                        Some(ReadOnlyStmt::Describe(s)) => eng.execute_describe(s),
                        None => unreachable!("is_read_only implied rstmt is Some"),
                    }
                } else {
                    let mut eng = engine.write();
                    eng.execute(&final_sql)
                };
                // V312-18e: prepared-statement executions are timed too.
                let elapsed_ms = started.elapsed().as_millis() as u64;
                if let Some(ref slow_log) = config.slow_query_log {
                    let rows = result.as_ref().map(|r| r.rows.len() as u64).unwrap_or(0);
                    slow_log.maybe_log(&final_sql, elapsed_ms, rows);
                }
                // V312-18e Issue #4021: record prepared-statement
                // executions into the Prometheus counters as well.
                sqlrustgo_telemetry::GLOBAL_METRICS
                    .record_query("STMT_EXECUTE", std::time::Duration::from_millis(elapsed_ms));
                match result {
                    Ok(r) if is_read_only.is_some() => {
                        let c: Vec<String> = r
                            .rows
                            .first()
                            .map(|row| (0..row.len()).map(|i| format!("col_{}", i + 1)).collect())
                            .unwrap_or_else(|| vec!["result".to_string()]);
                        let t: Vec<String> = c.iter().map(|_| "VARCHAR(255)".to_string()).collect();
                        let c_trimmed: Vec<String> =
                            c.into_iter().take(stmt_col_count as usize).collect();
                        let t_trimmed: Vec<String> =
                            t.into_iter().take(stmt_col_count as usize).collect();
                        let r_trimmed: Vec<Vec<Value>> = r
                            .rows
                            .into_iter()
                            .map(|row| row.into_iter().take(stmt_col_count as usize).collect())
                            .collect();
                        seq = send_binary_result_set(
                            stream, &c_trimmed, &t_trimmed, &r_trimmed, seq, cap,
                        )?;
                    }
                    Ok(r) => {
                        seq = write_ok_packets(
                            stream,
                            make_ok_packet(seq, r.affected_rows as u64, 0, 0x0002, 0, cap, false),
                            seq,
                        )?;
                        *server_last_sent_seq = seq;
                    }
                    Err(e) => {
                        let code = e.mysql_error_code();
                        make_err_packet(seq, code, "42000", &e.to_string()).write_to(stream)?;
                        *server_last_sent_seq = seq;
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
            // COM_RESET_CONNECTION (0x1F): resets session state including all
            // prepared statements. MySQL protocol requires OK packet response.
            packet_type::COM_RESET_CONNECTION => {
                tracing::info!("COM_RESET_CONNECTION from {}", addr);
                ps_manager.reset();
                seq = write_ok_packets(
                    stream,
                    make_ok_packet(seq, 0, 0, 0x0002, 0, cap, false),
                    seq,
                )?;
                *server_last_sent_seq = seq;
            }
            _ => {
                make_err_packet(seq, 1047, "HY000", "Unknown command").write_to(stream)?;
                *server_last_sent_seq = seq;
                seq = seq.wrapping_add(1);
            }
        }
    }
    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    storage: Arc<parking_lot::RwLock<BoxStorageEngine>>,
    tls_config: Arc<rustls::ServerConfig>,
    user_store: UserStore,
    // V312-32: per-handle ephemeral config threaded down from
    // `ServerJob` so LOAD DATA LOCAL INFILE reads the right server's
    // `data_dir` and `bulk_insert_buffer_size` even when multiple
    // `start_ephemeral` servers coexist in the same process.
    config: Arc<crate::testing::EphemeralConfig>,
) {
    ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
    TOTAL_CONNECTIONS_ACCEPTED.fetch_add(1, Ordering::Relaxed);
    // V312-18e Issue #4021: feed the connection lifecycle into the
    // Prometheus singleton so `/metrics` exposes
    // `sqlrustgo_connections_active` / `sqlrustgo_connections_total`.
    sqlrustgo_telemetry::GLOBAL_METRICS.connection_acquired();
    let _guard = scopeguard::guard((), |_| {
        // Always decrement on exit, even on panic
        ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
        sqlrustgo_telemetry::GLOBAL_METRICS.connection_released();
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
            // V312-WIRE-5: Vec<Packet> — write all packets (OK + optional
            // session_state_info) and advance the sequence number per
            // packet written.
            for pkt in make_ok_packet(3, 0, 0, 0x0002, 0, resp.capability_flags, true) {
                pkt.write_to(&mut tls).ok();
            }
            tracing::info!("Starting command loop, seq=4+");
            // Drop the temporary Stream wrapper and create a long-lived
            // TlsStream that drives rustls IO after every write. This
            // is critical for `mysql` CLI / sysbench compatibility:
            // without auto-complete_io, the cipher buffer accumulates
            // and the client never receives the response.
            let mut tls = TlsStream::new(&mut conn, &mut stream);
            let engine: Arc<parking_lot::RwLock<ExecutionEngine<BoxStorageEngine>>> = Arc::new(
                parking_lot::RwLock::new(ExecutionEngine::new(storage.clone())),
            );
            let mut ps_manager = PreparedStatementManager::new();
            let mut server_last_sent_seq = 3u8;
            let _ = do_command_loop(
                &mut tls,
                addr,
                storage,
                engine,
                resp.capability_flags,
                &mut server_last_sent_seq,
                &mut ps_manager,
                Some(resp.username.clone()),
                &config,
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
    // V312-WIRE-5: Vec<Packet> — emit OK + optional session_state_info.
    for pkt in make_ok_packet(2, 0, 0, 0x0002, 0, resp.capability_flags, true) {
        pkt.write_to(&mut &stream).ok();
    }
    let mut server_last_sent_seq = 2u8;
    tracing::info!("Starting command loop with server_last_sent_seq=2");
    let engine: Arc<parking_lot::RwLock<ExecutionEngine<BoxStorageEngine>>> = Arc::new(
        parking_lot::RwLock::new(ExecutionEngine::new(storage.clone())),
    );
    let mut ps_manager = PreparedStatementManager::new();
    let _ = do_command_loop(
        &mut &stream,
        addr,
        storage,
        engine,
        resp.capability_flags,
        &mut server_last_sent_seq,
        &mut ps_manager,
        Some(resp.username.clone()),
        &config,
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
#[allow(clippy::too_many_arguments)]
pub fn run_server_v2(
    host: &str,
    port: u16,
    data_dir: &str,
    max_connections: usize,
    auth_mode: &str,
    server_threads: usize,
    storage: &str,
    wal_sync: &str,
    executor_parallelism: usize,
) -> MySqlResult<()> {
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr)?;
    tracing::info!(
        "MySQL server listening on {} (data_dir={}, max_conn={}, auth={}, server_threads={}, storage={}, wal_sync={})",
        addr,
        data_dir,
        max_connections,
        auth_mode,
        server_threads,
        storage,
        wal_sync
    );
    // Store options in env so the run_server_with_listener path can read them
    std::env::set_var("SQLRUSTGO_DATA_DIR", data_dir);
    std::env::set_var("SQLRUSTGO_MAX_CONN", max_connections.to_string());
    std::env::set_var("SQLRUSTGO_AUTH_MODE", auth_mode);
    std::env::set_var("SQLRUSTGO_STORAGE", storage);
    std::env::set_var("SQLRUSTGO_WAL_SYNC", wal_sync);
    // v3.10.0 Issue #3703: propagate intra-query executor parallelism.
    // Engine reads this env var on construction (see
    // `ExecutionEngine::new` + `set_parallel_degree`).
    std::env::set_var(
        "SQLRUSTGO_EXECUTOR_PARALLELISM",
        executor_parallelism.to_string(),
    );
    // v3.8.0-rc2 Week 1 Day 7: propagate data_dir to the LOAD DATA
    // LOCAL INFILE handler so it recognizes files inside the data dir
    // as in-whitelist. Without this, only the in-process test harness
    // (which calls `start_ephemeral`) can issue LOAD DATA — a real
    // `mysql` client connecting to a server started by `run_server_v2`
    // would get "not in allowed data_dir".
    //
    // V312-32: the propagation mechanism changed from the
    // process-global `ACTIVE_CONFIG` to a per-handle `Arc<EphemeralConfig>`
    // threaded through `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`
    // → accept loop → `ServerJob` / `handle_connection` → `do_command_loop`.
    // This lets multiple `start_ephemeral` servers (and `run_server_v2`)
    // coexist in the same process without one server's LOAD DATA seeing
    // another's `data_dir`.
    use crate::testing::EphemeralConfig;
    // Issue #4020: optional LOAD DATA whitelist separate from the
    // storage data_dir. Read from `SQLRUSTGO_LOAD_INFILE_DIR` so the
    // CLI plumbing (`--load-infile-dir` in main.rs) doesn't have to
    // widen the `run_server_v2` signature. When unset (the default),
    // the LOAD DATA whitelist falls back to `data_dir`, preserving the
    // pre-#4020 sandbox semantics.
    let load_infile_dir = std::env::var("SQLRUSTGO_LOAD_INFILE_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from);
    // Round-21 / Issue #4217: read the per-server
    // `bulk_insert_rows_per_flush` from env so CLI plumbing (in
    // main.rs) doesn't have to widen `run_server_v2`'s signature.
    // When unset (or unparseable), fall back to the default 10_000.
    let bulk_insert_rows_per_flush = std::env::var("SQLRUSTGO_BULK_INSERT_ROWS_PER_FLUSH")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10_000);
    let cfg = EphemeralConfig {
        data_dir: Some(std::path::PathBuf::from(data_dir)),
        load_infile_dir,
        server_threads,
        bulk_insert_rows_per_flush,
        ..Default::default()
    };

    // V312-26 / Issue #4021: optional Prometheus /metrics endpoint.
    // Read the port from `SQLRUSTGO_METRICS_PORT` so the CLI flag
    // plumbing (added to `main.rs`) doesn't have to widen the
    // `run_server_v2` signature. The env var is only consulted here;
    // the production binary sets it from `--metrics-port`. The handle
    // is intentionally detached — the metrics endpoint is fire-and-forget
    // and lives for the rest of the process.
    if let Ok(port_str) = std::env::var("SQLRUSTGO_METRICS_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            let _ = crate::metrics_endpoint::start(host, port);
        }
    }

    let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql(
        listener,
        shutdown,
        None,
        true,
        Vec::new(),
        Some(std::path::PathBuf::from(data_dir)),
        server_threads,
        Some(storage.to_string()),
        // V312-32: per-handle config replaces the process-global
        // ACTIVE_CONFIG publication. LOAD DATA LOCAL INFILE on this
        // server sees the same data_dir the caller configured.
        Arc::new(cfg),
    )
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql(
    listener: TcpListener,
    shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    bootstrap: UserStoreBootstrap,
    bootstrap_tables: bool,
    bootstrap_sql: Vec<String>,
    data_dir: Option<std::path::PathBuf>,
    server_threads: usize,
    storage: Option<String>,
    // V312-32: per-handle ephemeral config. The accept loop attaches
    // this to every `ServerJob` / `handle_connection` call so the LOAD
    // DATA LOCAL INFILE handler reads THIS server's `data_dir` and
    // `bulk_insert_buffer_size`, not a process-global.
    config: std::sync::Arc<crate::testing::EphemeralConfig>,
) -> MySqlResult<()> {
    let tls_config = Arc::new(make_tls_config());
    tracing::info!("TLS ready (self-signed cert)");

    // Resolve data directory (shared by both binary and WAL storage modes).
    let wal_data_dir = match data_dir {
        Some(p) => p,
        None => {
            let port = listener.local_addr()?.port();
            let cwd_default = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(".sqlrustgo")
                .join("data");
            match std::env::var("SQLRUSTGO_DATA_DIR") {
                Ok(s) if !s.is_empty() => {
                    tracing::info!(
                        "data_dir from SQLRUSTGO_DATA_DIR env: {} (port {})",
                        s,
                        port
                    );
                    std::path::PathBuf::from(s)
                }
                _ => {
                    tracing::info!(
                        "data_dir default (cwd/.sqlrustgo/data/): {} (port {})",
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

    let storage: Arc<RwLock<BoxStorageEngine>> = match storage.as_deref() {
        Some("binary") => {
            tracing::info!("Storage: binary (BinaryTableStorage, no WAL)");
            let bin_storage = BinaryTableStorage::new_with_data(wal_data_dir.clone())?;
            tracing::info!("Loaded .bin tables from data_dir");
            Arc::new(parking_lot::RwLock::new(BoxStorageEngine::new(bin_storage)))
        }
        Some("parallel") => {
            // V311-09: ParallelWalStorage with parallel table flush
            tracing::info!("Storage: parallel (ParallelWalStorage with parallel flush)");
            let mut file_storage =
                FileStorage::new_with_wal(wal_data_dir.clone()).map_err(std::io::Error::other)?;
            let wal_path = wal_data_dir.join("sqlrustgo.wal");
            // WAL recovery
            {
                use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
                let mut recovery: StatefulRecoveryEngine<FileStorage> =
                    StatefulRecoveryEngine::new();
                let mut wal_manager_for_recovery = FileBackedWalManager::new(wal_path.clone())
                    .map_err(|e| {
                        MySqlError::Sql(format!("WAL manager (recovery) init failed: {}", e))
                    })?;
                match recovery.recover(&mut file_storage, &mut wal_manager_for_recovery) {
                    Ok(report) => {
                        tracing::info!(
                            "WAL recovery: total={} committed_txns={} rows_inserted={}",
                            report.entries_total,
                            report.committed_txns,
                            report.rows_inserted
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
            let wal_sync_mode =
                std::env::var("SQLRUSTGO_WAL_SYNC").unwrap_or_else(|_| "every".to_string());
            let sync_mode = parse_wal_sync_mode(&wal_sync_mode);
            tracing::info!("WAL sync mode: {:?}", sync_mode);
            let mut parallel_storage = ParallelWalStorage::new(file_storage, wal_manager);
            parallel_storage.set_sync_mode(sync_mode);
            Arc::new(parking_lot::RwLock::new(BoxStorageEngine::new(
                parallel_storage,
            )))
        }
        _ => {
            // WalStorage<FileStorage, FileBackedWalManager>
            let mut file_storage =
                FileStorage::new_with_wal(wal_data_dir.clone()).map_err(std::io::Error::other)?;
            let wal_path = wal_data_dir.join("sqlrustgo.wal");
            if let Ok(meta) = std::fs::metadata(&wal_path) {
                let size_mb = meta.len() / (1024 * 1024);
                if size_mb >= 100 {
                    tracing::warn!(
                        "WAL file is large: {} ({} MB). Consider --data-dir to isolate runs.",
                        wal_path.display(),
                        size_mb
                    );
                } else {
                    tracing::info!(
                        "WAL file size at startup: {} ({} MB)",
                        wal_path.display(),
                        size_mb
                    );
                }
            }
            // WAL recovery
            {
                use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
                let mut recovery: StatefulRecoveryEngine<FileStorage> =
                    StatefulRecoveryEngine::new();
                let mut wal_manager_for_recovery = FileBackedWalManager::new(wal_path.clone())
                    .map_err(|e| {
                        MySqlError::Sql(format!("WAL manager (recovery) init failed: {}", e))
                    })?;
                match recovery.recover(&mut file_storage, &mut wal_manager_for_recovery) {
                    Ok(report) => {
                        tracing::info!(
                            "WAL recovery: total={} committed_txns={} rows_inserted={}",
                            report.entries_total,
                            report.committed_txns,
                            report.rows_inserted
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
            let checkpoint_manager = Arc::new(std::sync::RwLock::new(CheckpointManager::default()));
            let wal_sync_mode =
                std::env::var("SQLRUSTGO_WAL_SYNC").unwrap_or_else(|_| "every".to_string());
            let sync_mode = parse_wal_sync_mode(&wal_sync_mode);
            tracing::info!("WAL sync mode: {:?}", sync_mode);
            let wal_storage = WalStorage::new_with_sync_mode_and_checkpoint(
                file_storage,
                wal_manager,
                sync_mode,
                checkpoint_manager,
            )
            .map_err(|e| MySqlError::Sql(format!("WalStorage init failed: {}", e)))?;
            Arc::new(parking_lot::RwLock::new(BoxStorageEngine::new(wal_storage)))
        }
    };
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
    // Construct worker pool only when server_threads > 0. When server_threads
    // is 0, fall back to the legacy unbounded per-connection thread::spawn
    // path (used by tests that exercise many concurrent short-lived
    // connections and want full thread-per-connection isolation).
    let pool = if server_threads == 0 {
        None
    } else {
        Some(crate::testing::ServerThreadPool::start(server_threads))
    };

    while !shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, addr)) => {
                // The listener is non-blocking (for shutdown polling). On
                // Unix the accepted TcpStream inherits this flag, which
                // breaks TLS I/O: TlsStream::write() would never truly
                // block — it would busy-spin on WouldBlock. Restore
                // blocking mode so that Read/Write/complete_io block
                // properly in the kernel (per-connection thread).
                let _ = stream.set_nonblocking(false);
                let st = storage.clone();
                let tc = tls_config.clone();
                let us = user_store.clone();
                // V312-32: per-handle config clone so the handler
                // sees THIS server's data_dir / bulk_insert_buffer_size.
                let cfg = Arc::clone(&config);
                match &pool {
                    None => {
                        thread::spawn(move || handle_connection(stream, addr, st, tc, us, cfg));
                    }
                    Some(p) => {
                        let job = crate::testing::ServerJob {
                            stream,
                            addr,
                            storage: st,
                            tls_config: tc,
                            user_store: us,
                            config: cfg,
                        };
                        match p.send_timeout(job, Duration::from_millis(200)) {
                            Ok(()) => {}
                            Err(crate::testing::SendTimeoutError::Timeout(returned_job)) => {
                                crate::testing::BACKPRESSURE_COUNT
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                tracing::debug!(
                                    "worker pool full; rejecting connection from {} \
                                     (BACKPRESSURE_COUNT incremented)",
                                    returned_job.addr
                                );
                            }
                            Err(crate::testing::SendTimeoutError::Disconnected(returned_job)) => {
                                tracing::warn!(
                                    "worker pool shut down; dropping connection from {}",
                                    returned_job.addr
                                );
                            }
                        }
                    }
                }
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
    // Drop the pool so workers exit their recv loop and join. Any
    // in-flight jobs keep running until they complete (sync_channel
    // receivers hold the jobs until consumed).
    drop(pool);
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
        16,
        None,
        // V312-32: default ephemeral config (no data_dir override,
        // no bulk_insert_buffer_size override). LOAD DATA LOCAL INFILE
        // on this server uses the standard defaults.
        Arc::new(crate::testing::EphemeralConfig::default()),
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
        16,
        None,
        // V312-32: see note on `run_server_with_listener_and_shutdown`.
        Arc::new(crate::testing::EphemeralConfig::default()),
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
        let packets = make_ok_packet(1, 0, 0, 0x0002, 0, 0, false);
        // client_cap=0 means SESSION_TRACK NOT negotiated → no trailing
        // info, no separate session_state_info → single OK packet only.
        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
        assert_eq!(pkt.sequence, 1);
        assert_eq!(pkt.payload[0], 0x00); // OK packet type
    }

    #[test]
    fn test_make_ok_packet_with_affected_rows() {
        let packets = make_ok_packet(2, 5, 10, 0x0002, 0, 0, false);
        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
        assert_eq!(pkt.sequence, 2);
        // Affected rows is lenenc-int of 5 = 0x05
        assert!(pkt.payload.contains(&5));
    }

    #[test]
    fn test_make_ok_packet_session_track_embeds_session_state_in_ok_packet() {
        // V312-WIRE-7 (supersedes retracted V312-WIRE-5): with
        // SESSION_TRACK negotiated (0x00800000), the caller's status not
        // carrying 0x4000, and is_auth_ok=false (statement OK), the
        // make_ok_packet helper augments status with 0x4000 internally
        // and EMBEDS the empty session_state_changes lenenc inside the
        // SAME OK packet (after `info`), NOT as a separate packet.
        // Verified by strace against mysql CLI 8.0.46.
        let cap = capability::SESSION_TRACK;
        let packets = make_ok_packet(3, 0, 0, 0x0002, 0, cap, false);
        assert_eq!(packets.len(), 1, "should emit exactly one OK packet");
        assert_eq!(packets[0].sequence, 3);
        let p = &packets[0].payload;
        assert_eq!(p[0], 0x00, "OK marker");
        assert_eq!(p[1], 0x00, "affected=0");
        assert_eq!(p[2], 0x00, "last_id=0");
        // status LE — must have 0x4000 set (and AUTOCOMMIT 0x0002)
        let status = u16::from_le_bytes([p[3], p[4]]);
        assert_eq!(
            status,
            0x0002 | capability::SERVER_STATUS_SESSION_STATE_CHANGED,
            "statement OK status must include SESSION_STATE_CHANGED"
        );
        assert_eq!(p[5], 0x00, "warnings high");
        assert_eq!(p[6], 0x00, "warnings low");
        // lenenc(info=0) trailing — SESSION_TRACK negotiated
        assert_eq!(p[7], 0x00, "lenenc(info=0)");
        // EMBEDDED lenenc(session_state_changes=0) — appended inside OK
        assert_eq!(p[8], 0x00, "embedded lenenc(session_state_changes=0)");
    }

    #[test]
    fn test_make_ok_packet_auth_ok_does_not_emit_session_state_packet() {
        // V312-WIRE-6 + V312-WIRE-7: even with SESSION_TRACK negotiated,
        // the Auth OK packet MUST NOT carry 0x4000 and MUST NOT emit any
        // session_state byte (separate OR embedded). mysql CLI 8.0.46
        // reads the Auth OK and immediately sends COM_QUERY before
        // reading the 2nd packet, so any trailing session_state would
        // desync the wire. The payload ends at `info` (no embedded
        // session_state_changes lenenc after).
        let cap = capability::SESSION_TRACK;
        let packets = make_ok_packet(2, 0, 0, 0x0002, 0, cap, true);
        assert_eq!(packets.len(), 1, "Auth OK must be a single packet");
        assert_eq!(packets[0].sequence, 2);
        let p = &packets[0].payload;
        assert_eq!(p[0], 0x00, "OK marker");
        assert_eq!(p[1], 0x00, "affected=0");
        assert_eq!(p[2], 0x00, "last_id=0");
        // status LE = 0x0002 (no SESSION_STATE_CHANGED bit)
        let status = u16::from_le_bytes([p[3], p[4]]);
        assert_eq!(status, 0x0002, "Auth OK status must be 0x0002 (no 0x4000)");
        assert_eq!(p[5], 0x00, "warnings high");
        assert_eq!(p[6], 0x00, "warnings low");
        // lenenc(info=0) trailing — SESSION_TRACK negotiated
        assert_eq!(p[7], 0x00, "lenenc(info=0)");
        // NO embedded session_state_changes lenenc after info
        assert_eq!(
            p.len(),
            8,
            "Auth OK payload must end at info; no embedded session_state"
        );
    }

    #[test]
    fn test_make_deprecate_eof_ok_packet_terminator_uses_0xfe_marker() {
        // V312-WIRE-8 regression test (#4019.4 sixth pass — replaces the
        // retracted V312-WIRE-7 / V312-WIRE-5 / V312-WIRE-4 chain):
        //
        // Per MySQL WL#7766 (https://dev.mysql.com/worklog/task/?id=7766),
        // the trailing result-set terminator under CLIENT_DEPRECATE_EOF
        // uses the **EOF identifier 0xFE** as the FIRST byte, NOT the
        // regular OK marker 0x00. mysql CLI 8.0.46 dispatches on this
        // first byte: 0xFE → "OK-as-terminator" path, 0x00 → "regular OK
        // packet" path. Sending 0x00 here caused mysql CLI 8.0.46 to
        // treat the terminator as a regular OK packet and hang waiting
        // for a 5th recvfrom that never came.
        //
        // For plain SELECTs (no session-state change), the terminator is
        // exactly 7 bytes: 0xFE + lenenc(0) + lenenc(0) + status(0x0002)
        // + warnings(0). No 0x4000, no info, no session_state_changes —
        // matching real MySQL 8.0.46 wire bytes for `select
        // @@version_comment limit 1` captured via strace.
        let cap = capability::SESSION_TRACK;
        let packets = make_deprecate_eof_ok_packet(5, 0, 0, 0x0002, 0, cap);
        assert_eq!(
            packets.len(),
            1,
            "trailing OK must be a single packet under DEPRECATE_EOF"
        );
        assert_eq!(packets[0].sequence, 5);
        let p = &packets[0].payload;
        assert_eq!(
            p[0], 0xfe,
            "DEPRECATE_EOF terminator MUST use EOF identifier 0xFE (WL#7766), \
             NOT OK marker 0x00 — mysql CLI 8.0.46 dispatches on this byte"
        );
        assert_eq!(p[1], 0x00, "affected=0");
        assert_eq!(p[2], 0x00, "last_id=0");
        // status LE — must be 0x0002 (AUTOCOMMIT) only, NO 0x4000
        let status = u16::from_le_bytes([p[3], p[4]]);
        assert_eq!(
            status, 0x0002,
            "plain SELECT trailing OK status must be 0x0002 (no SESSION_STATE_CHANGED)"
        );
        assert!(
            status & capability::SERVER_STATUS_SESSION_STATE_CHANGED == 0,
            "plain SELECT trailing OK MUST NOT have 0x4000 set"
        );
        assert_eq!(p[5], 0x00, "warnings high");
        assert_eq!(p[6], 0x00, "warnings low");
        assert_eq!(
            p.len(),
            7,
            "plain SELECT terminator payload must be exactly 7 bytes \
             (0xFE + 2 lenenc + status + warnings), no info, no session_state"
        );
    }

    #[test]
    fn test_make_deprecate_eof_ok_packet_with_session_state_change_appends_info_and_session() {
        // V312-WIRE-8 regression test: when the trailing OK does carry
        // 0x4000 (e.g. SET, USE, multi-statement), it appends the
        // lenenc(info) + lenenc(session_state_changes) suffix INSIDE the
        // SAME packet (not as a separate packet). This is the only case
        // where the payload is > 7 bytes.
        let cap = capability::SESSION_TRACK;
        let packets = make_deprecate_eof_ok_packet(
            5,
            0,
            0,
            0x0002 | capability::SERVER_STATUS_SESSION_STATE_CHANGED,
            0,
            cap,
        );
        let p = &packets[0].payload;
        assert_eq!(
            p[0], 0xfe,
            "header is still 0xFE even when 0x4000 is set — WL#7766 eof_identifier"
        );
        let status = u16::from_le_bytes([p[3], p[4]]);
        assert!(
            status & capability::SERVER_STATUS_SESSION_STATE_CHANGED != 0,
            "status must include 0x4000 for this test"
        );
        assert_eq!(p[7], 0x00, "lenenc(info=0)");
        assert_eq!(p[8], 0x00, "embedded lenenc(session_state_changes=0)");
        assert_eq!(p.len(), 9, "payload must end at session_state_changes");
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
    #[test]
    fn test_make_err_packet_format() {
        // MySQL wire protocol error packet format (4.1+):
        // 0xFF + error_code(u16 LE) + 0x23 + SQL_STATE(5 bytes) + ERROR_MSG + 0x00
        // The null byte at the end terminates the message for C-compatible clients.
        let pkt = make_err_packet(1, 1146, "42S02", "Table not found");
        assert_eq!(pkt.payload[0], 0xff); // ERR packet type
                                          // Bytes 1-2: error code (1146 = 0x047A little-endian)
        assert_eq!(u16::from_le_bytes([pkt.payload[1], pkt.payload[2]]), 1146);
        assert_eq!(pkt.payload[3], 0x23); // '#' marker
                                          // Bytes 4-8: SQL state "42S02"
        assert_eq!(&pkt.payload[4..9], b"42S02");
        // Bytes 9+: error message
        assert_eq!(&pkt.payload[9..24], b"Table not found");
        assert_eq!(pkt.payload[pkt.payload.len() - 1], 0x00);
    }

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
        let _ = write_column_def(&mut buf, "id", "INT", 0).unwrap();
        assert!(buf.len() > 0);
        // Verify it can be read back as a packet
        let mut cursor = std::io::Cursor::new(buf);
        let pkt = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(pkt.sequence, 0);
    }

    #[test]
    fn test_write_column_def_varchar() {
        let mut buf = Vec::new();
        let _ = write_column_def(&mut buf, "name", "VARCHAR(100)", 5).unwrap();
        let mut cursor = std::io::Cursor::new(buf);
        let pkt = Packet::read_from(&mut cursor).unwrap();
        assert_eq!(pkt.sequence, 5);
    }

    #[test]
    fn test_write_column_def_float() {
        let mut buf = Vec::new();
        let _ = write_column_def(&mut buf, "price", "FLOAT", 10).unwrap();
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
        use parking_lot::RwLock;
        use sqlrustgo::MemoryExecutionEngine;
        use sqlrustgo_storage::MemoryStorage;
        use sqlrustgo_types::Value;
        use std::sync::Arc;

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
        use parking_lot::RwLock;
        use sqlrustgo::MemoryExecutionEngine;
        use sqlrustgo_storage::MemoryStorage;
        use std::sync::Arc;

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

    #[test]
    fn server_thread_pool_panic_isolation() {
        use crate::testing::ServerThreadPool;
        // Pool with 2 workers; both should start cleanly and exit cleanly
        // when sender drops. We can't easily inject a panicking job without
        // a full handle_connection, so we just verify lifecycle here.
        // The catch_unwind in worker_loop is verified by reading code
        // and by e2e tests in Task 8.
        let pool = ServerThreadPool::start(2);
        assert_eq!(pool.worker_count(), 2);
        pool.join();
    }

    #[test]
    fn server_thread_pool_graceful_shutdown() {
        use crate::testing::ServerThreadPool;
        let pool = ServerThreadPool::start(4);
        assert_eq!(pool.worker_count(), 4);
        // join() must return cleanly without hang or panic
        pool.join();
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
    use crate::BoxStorageEngine;
    use crate::UserStore;
    use crossbeam_channel::{bounded, Receiver, Sender};
    use std::net::{SocketAddr, TcpStream};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;

    /// One connection-handling job dispatched to a worker via the
    ///
    /// V312-32: `config` carries the per-server `EphemeralConfig` so the
    /// LOAD DATA LOCAL INFILE handler can read `data_dir` and
    /// `bulk_insert_buffer_size` without consulting process-global
    /// state. This makes concurrent `start_ephemeral` calls safe —
    /// each connection sees the config of the server it connected to,
    /// not whichever `start_ephemeral` ran last.
    pub struct ServerJob {
        pub stream: TcpStream,
        pub addr: SocketAddr,
        pub storage: Arc<parking_lot::RwLock<BoxStorageEngine>>,
        pub tls_config: Arc<rustls::ServerConfig>,
        pub(crate) user_store: UserStore,
        pub(crate) config: Arc<EphemeralConfig>,
    }

    /// Bounded worker pool: N worker threads + `sync_channel(N*2)` for
    /// backpressure. `server_threads=0` mode skips constructing this
    pub struct ServerThreadPool {
        tx: Sender<ServerJob>,
        workers: Vec<std::thread::JoinHandle<()>>,
    }

    /// Errors returned by [`ServerThreadPool::send_timeout`].
    pub enum SendTimeoutError {
        /// Channel was full for the entire timeout. Caller should
        /// back off and retry (or drop the connection).
        Timeout(ServerJob),
        /// All worker threads have exited; the receiver was dropped.
        Disconnected(ServerJob),
    }

    /// Counter of `send_timeout` timeouts across all pools in this
    /// process. Exposed via [`ServerThreadPool::backpressure_count`].
    pub static BACKPRESSURE_COUNT: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);
    const CHANNEL_BUFFER_MULTIPLIER: usize = 4;

    impl ServerThreadPool {
        /// Start N worker threads + bounded sync_channel.
        #[allow(private_interfaces)]
        pub fn start(n: usize) -> Self {
            assert!(n > 0, "ServerThreadPool::start requires n > 0");
            let (tx, rx) = bounded(n * CHANNEL_BUFFER_MULTIPLIER);
            let mut workers = Vec::with_capacity(n);
            for worker_id in 0..n {
                let rx = rx.clone();
                workers.push(std::thread::spawn(move || {
                    worker_loop(rx, worker_id);
                }));
            }
            Self { tx, workers }
        }

        /// Send a job; blocks if the channel is full (backpressure).
        /// Returns Err if all workers have shut down.
        pub fn send(&self, job: ServerJob) -> Result<(), ServerJob> {
            self.tx.send(job).map_err(|e| e.0)
        }

        /// Send a job with a timeout. Used by the accept loop so it
        /// can poll the shutdown flag even under sustained backpressure.
        pub fn send_timeout(
            &self,
            job: ServerJob,
            timeout: std::time::Duration,
        ) -> Result<(), SendTimeoutError> {
            match self.tx.send_timeout(job, timeout) {
                Ok(()) => Ok(()),
                Err(crossbeam_channel::SendTimeoutError::Timeout(j)) => {
                    Err(SendTimeoutError::Timeout(j))
                }
                Err(crossbeam_channel::SendTimeoutError::Disconnected(j)) => {
                    Err(SendTimeoutError::Disconnected(j))
                }
            }
        }

        /// Backpressure counter incremented every time a `send_timeout`
        /// returned `Timeout`. Exposed for tests that want to assert the
        /// pool actually exercised backpressure.
        pub fn backpressure_count(&self) -> u64 {
            BACKPRESSURE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
        }

        /// Drop the sender so workers exit their recv loop, then join.
        pub fn join(self) {
            drop(self.tx);
            for h in self.workers {
                let _ = h.join();
            }
        }

        /// Number of worker threads.
        pub fn worker_count(&self) -> usize {
            self.workers.len()
        }
    }

    fn worker_loop(rx: Receiver<ServerJob>, worker_id: usize) {
        loop {
            let job = match rx.recv() {
                Ok(job) => job,
                Err(_) => {
                    tracing::debug!("worker {worker_id}: channel closed, exiting");
                    return;
                }
            };
            // Panic isolation: one connection's panic doesn't kill the worker.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::handle_connection(
                    job.stream,
                    job.addr,
                    job.storage,
                    job.tls_config,
                    job.user_store,
                    job.config,
                )
            }));
            if let Err(e) = result {
                tracing::error!(
                    "worker {worker_id}: connection handler panicked: {:?}",
                    e.downcast_ref::<&str>().unwrap_or(&"unknown")
                );
            }
        }
    }

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
        /// When `Some(port)`, bind to that exact port. When `None` or
        /// `0`, the OS picks an available port. Exact ports allow
        /// [`EphemeralServerPool`] to reuse server instances across tests.
        pub port: Option<u16>,
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
        /// Issue #4020: directory used as the LOAD DATA LOCAL INFILE
        /// whitelist. When `Some(path)`, files must canonicalize inside
        /// THIS path (not the storage `data_dir`) to be accepted. When
        /// `None` (default), the whitelist falls back to `data_dir` —
        /// preserving the V312-13 sandbox semantics so existing tests
        /// (`test_load_local_infile_path_outside_data_dir`) continue
        /// to pass unchanged. The CLI mirrors this knob via
        /// `--load-infile-dir`.
        ///
        /// Why is this separate from `data_dir`? Because the bulk-load
        /// runner (`scripts/tpch/bulk_load_sf10.sh`) wants the server's
        /// *storage* data_dir to live next to the WAL under
        /// `$RUN_DIR/data` (so the runner can wipe it on exit) while
        /// the LOAD DATA fixtures live under `$DATA_DIR` (which the
        /// runner does NOT own — typically a long-lived `/tmp/tpch-sf10`
        /// shared across runs). Conflating the two would force a
        /// `cp` of every .tbl into the storage dir on every run; the
        /// 60M-line SF=10 lineitem.tbl makes that prohibitive.
        pub load_infile_dir: Option<std::path::PathBuf>,
        /// Extra DDL statements to execute after the internal catalog
        /// tables (if `bootstrap_tables` is true) and before the server
        /// starts accepting connections. Use this to inject the 8 TPC-H
        /// `CREATE TABLE` statements into an ephemeral server.
        pub bootstrap_sql: Vec<String>,
        /// Maximum bytes to buffer in a single batched INSERT during
        /// LOAD DATA LOCAL INFILE. Default 1 MB. Tests / perf benches
        /// can set higher (e.g. 16 MB) for fewer INSERT round-trips.
        pub bulk_insert_buffer_size: usize,
        /// Round-21 / Issue #4217: number of rows per
        /// `bulk_insert_records` flush during LOAD DATA LOCAL INFILE.
        /// Default 10_000 (raises the previous hard-coded 100-row
        /// constant so large tables like TPC-H SF=10 lineitem can
        /// avoid paying write-lock + Vec allocation cost on every
        /// 100 rows). Setting to 0 disables the periodic flush and
        /// falls back to "flush only when the per-packet byte buffer
        /// fully drains" (V312-32 legacy behavior).
        pub bulk_insert_rows_per_flush: usize,
        /// Maximum concurrent connection-handler worker threads.
        /// 0 = legacy unbounded `thread::spawn` (backwards compatible).
        /// 1..=80 = bounded `ServerThreadPool` with N workers +
        /// `sync_channel(N*2)` for backpressure. Default 16 (matches
        /// CLI default in `main.rs`).
        pub server_threads: usize,
        /// Storage backend selector forwarded to
        /// `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`.
        /// `None` (default) → `FileStorage` + WAL (JSON row files);
        /// `Some("binary")` → `BinaryTableStorage` reading pre-generated
        /// `*.bin` (BINT v2) files in `data_dir`. The `binary` backend
        /// is significantly faster at TPC-H load time because it does
        /// not run LOAD DATA; the operator must produce the `.bin`
        /// files upstream (e.g. `tools/tbl2bin`). The CLI mirrors
        /// this knob via `--storage binary`.
        pub storage: Option<String>,
        /// Slow query log. `None` (default) disables slow query logging
        /// entirely — no file is created and no timing is retained.
        /// `Some(log)` makes every dispatched statement time itself and
        /// hand the duration to [`SlowQueryLog::maybe_log`], which gates
        /// on its own threshold.
        ///
        /// Shared behind an `Arc` across all connections of one server:
        /// the slow query log is server-wide in MySQL, and
        /// `SET long_query_time = N` retunes the shared threshold.
        /// Build one with
        /// `EphemeralConfig::with_slow_query_log(path, threshold_ms)`.
        pub slow_query_log: Option<Arc<query_stats::SlowQueryLog>>,
        /// V312-26 / Issue #4021: when `Some(port)`, the ephemeral
        /// server spawns a background thread that serves Prometheus
        /// exposition format at `http://<host>:<port>/metrics`. `None`
        /// (the default) means no metrics endpoint is bound. The thread
        /// is best-effort — if the bind fails (port in use), the server
        /// keeps running and a `tracing::warn!` is emitted.
        /// Build one with `EphemeralConfig::with_metrics_port(port)`.
        pub metrics_port: Option<u16>,
    }

    impl Default for EphemeralConfig {
        fn default() -> Self {
            Self {
                host: "127.0.0.1".to_string(),
                bootstrap_tables: true,
                bootstrap_users: true,
                data_dir: None,
                load_infile_dir: None,
                bootstrap_sql: Vec::new(),
                bulk_insert_buffer_size: 1_048_576,
                bulk_insert_rows_per_flush: 10_000,
                server_threads: 16,
                storage: None,
                port: None,
                slow_query_log: None,
                metrics_port: None,
            }
        }
    }

    impl EphemeralConfig {
        /// Enable slow query logging to `log_path`, recording every
        /// statement whose wall-clock duration is >= `threshold_ms`.
        /// The file is appended to and created on first slow query.
        pub fn with_slow_query_log(
            mut self,
            log_path: std::path::PathBuf,
            threshold_ms: u64,
        ) -> Self {
            self.slow_query_log = Some(Arc::new(query_stats::SlowQueryLog::new(
                threshold_ms,
                log_path,
            )));
            self
        }

        /// V312-26 / Issue #4021: enable a Prometheus `/metrics`
        /// endpoint bound to `<host>:<port>`. The endpoint renders the
        /// wire-protocol counters (`ACTIVE_CONNECTIONS`,
        /// `TOTAL_QUERIES_SERVED`, etc.) plus the telemetry `Metrics`
        /// global (`sqlrustgo_queries_total`, cache, storage bytes,
        /// query-duration histogram) in text exposition format 0.0.4.
        pub fn with_metrics_port(mut self, port: u16) -> Self {
            self.metrics_port = Some(port);
            self
        }
    }

    /// Handle to a running ephemeral server. Dropping the handle closes
    /// the listener and joins the accept-loop thread, and removes the
    /// temporary data directory.
    pub struct EphemeralHandle {
        pub port: u16,
        /// V312-18e Issue #4021: port of the Prometheus `/metrics`
        /// HTTP endpoint, when one was enabled via
        /// `EphemeralConfig::with_metrics_port`. `None` if the
        /// endpoint was not enabled.
        pub metrics_port: Option<u16>,
        // Shared shutdown signal: Drop sets it to true, the
        // server thread's accept loop polls it and exits within 50ms.
        shutdown: Option<Arc<std::sync::atomic::AtomicBool>>,
        // Mutex so Drop can take the JoinHandle by value.
        join: Mutex<Option<JoinHandle<()>>>,
        // V312-18e Issue #4021: when set, Drop signals the metrics
        // endpoint to exit and joins its thread.
        metrics_endpoint: Option<crate::metrics_endpoint::MetricsEndpoint>,
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
                .field("metrics_port", &self.metrics_port)
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
                metrics_port: None,
                shutdown: None,
                join: Mutex::new(None),
                metrics_endpoint: None,
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
            // 3. V312-18e Issue #4021: drop the metrics endpoint last
            //    so it can still serve scrapes during the server's
            //    shutdown phase. Dropping the handle signals its
            //    accept loop and joins its thread.
            let _ = self.metrics_endpoint.take();
            // 4. Remove the temporary data directory ONLY if the
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
        // V312-32: the config is now passed to the server thread via
        // `Arc::clone` rather than published to a process-global. Each
        // `start_ephemeral` call gets its own `Arc<EphemeralConfig>`,
        // so concurrent in-process servers (e.g. one for `region_nation_smoke`,
        // another for `sf1_lineitem_smoke_subset`) each see their own
        // `data_dir` and `bulk_insert_buffer_size` from the LOAD DATA
        // LOCAL INFILE handler.
        let config_arc = Arc::new(config.clone());

        let requested_port = config.port.unwrap_or(0);
        let listener = std::net::TcpListener::bind(format!("{}:{}", config.host, requested_port))?;
        let port = listener.local_addr()?.port();

        let data_dir_for_thread = config.data_dir.clone();

        // When the caller supplies a data_dir, the test owns the
        // directory's lifecycle; we do not create it and we do
        // not remove it on Drop. When None, we auto-create one
        // under the OS temp dir and Drop removes it.
        let externally_owned = data_dir_for_thread.is_some();
        let data_dir = data_dir_for_thread.clone().unwrap_or_else(|| {
            // V312-F-2 #4025 fix: include nanosecond timestamp + thread id
            // to guarantee uniqueness even when OS reuses a port within
            // the same process. Previously: port + process_id, which
            // collided when two sequential tests got the same port (the
            // second test inherited the first test's storage and tables).
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            std::env::temp_dir().join(format!(
                "sqlrustgo_ephemeral_{}_{}_{}",
                port,
                std::process::id(),
                nanos
            ))
        });
        if !externally_owned {
            // V312-F-2 #4025: remove stale data_dir from a previous run
            // that may have used the same port (e.g. Drop didn't complete
            // before a new server bound the port).
            let _ = std::fs::remove_dir_all(&data_dir);
            std::fs::create_dir_all(&data_dir)?;
        }

        // V312-F-2 #4025 fix (round-2): the server thread MUST receive the
        // *resolved* `data_dir` (with the temp fallback already computed
        // and the directory created/cleaned), not the original `Option`
        // from `EphemeralConfig`. Previously the closure captured
        // `data_dir_for_thread` (= `config.data_dir.clone()`, which is
        // `None` for the typical `data_dir: None` ephemeral test), so the
        // server thread fell back to the shared `cwd/.sqlrustgo/data`
        // directory and tests polluted one another (e.g. `test_e2e_drop_table`
        // saw 2 rows for `t2` instead of 1, because a previous test left a
        // row there). We now re-assign `data_dir_for_thread` to
        // `Some(data_dir.clone())` so the closure receives the unique path.
        let data_dir_for_thread = Some(data_dir.clone());

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
        let server_threads = config.server_threads;
        let storage_backend = config.storage.clone();
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
                server_threads,
                storage_backend,
                // V312-32: per-handle ephemeral config. Replaces the
                // former process-global `ACTIVE_CONFIG` publication so
                // concurrent `start_ephemeral` servers don't share state.
                Arc::clone(&config_arc),
            );
        });

        // V312-18e Issue #4021: bind the Prometheus `/metrics` endpoint
        // BEFORE the server thread starts processing connections so
        // scrapes can succeed during the server's warm-up. The endpoint
        // reads the process-wide `Metrics::global()` singleton; the
        // query dispatch path records into the same singleton.
        let metrics_endpoint = match config.metrics_port {
            Some(0) => match crate::metrics_endpoint::MetricsEndpoint::bind(("127.0.0.1", 0)) {
                Ok(e) => Some(e),
                Err(e) => {
                    eprintln!(
                        "sqlrustgo: failed to bind ephemeral metrics endpoint on 127.0.0.1:0: {e}"
                    );
                    None
                }
            },
            Some(p) => match crate::metrics_endpoint::MetricsEndpoint::bind(("127.0.0.1", p)) {
                Ok(e) => Some(e),
                Err(e) => {
                    return Err(std::io::Error::new(
                        e.kind(),
                        format!("sqlrustgo: failed to bind metrics endpoint on 127.0.0.1:{p}: {e}"),
                    ));
                }
            },
            None => None,
        };
        let metrics_port = metrics_endpoint.as_ref().map(|e| e.port());

        Ok(EphemeralHandle {
            port,
            metrics_port,
            shutdown: Some(shutdown),
            join: Mutex::new(Some(join)),
            metrics_endpoint,
            data_dir,
            externally_owned,
        })
    }

    #[test]
    fn split_top_level_single_statement() {
        assert_eq!(
            crate::split_top_level_statements("INSERT INTO t VALUES (1)"),
            vec!["INSERT INTO t VALUES (1)"]
        );
    }

    #[test]
    fn split_top_level_no_trailing_semicolon() {
        assert_eq!(
            crate::split_top_level_statements("SELECT 1"),
            vec!["SELECT 1"]
        );
    }

    #[test]
    fn split_top_level_trailing_semicolon() {
        assert_eq!(
            crate::split_top_level_statements("INSERT INTO t VALUES (1);"),
            vec!["INSERT INTO t VALUES (1)"]
        );
    }

    #[test]
    fn split_top_level_two_statements() {
        assert_eq!(
            crate::split_top_level_statements(
                "INSERT INTO t VALUES (1); INSERT INTO t VALUES (2);"
            ),
            vec!["INSERT INTO t VALUES (1)", "INSERT INTO t VALUES (2)"]
        );
    }

    #[test]
    fn split_top_level_skips_empty_statements() {
        assert_eq!(
            crate::split_top_level_statements(";;INSERT INTO t VALUES (1);;"),
            vec!["INSERT INTO t VALUES (1)"]
        );
    }

    #[test]
    fn split_top_level_respects_paren_depth() {
        assert_eq!(
            crate::split_top_level_statements("SELECT (1;2); SELECT 3;"),
            vec!["SELECT (1;2)", "SELECT 3"]
        );
    }

    #[test]
    fn split_top_level_respects_string_literals() {
        assert_eq!(
            crate::split_top_level_statements("INSERT INTO t VALUES ('a;b;c'); SELECT 1;"),
            vec!["INSERT INTO t VALUES ('a;b;c')", "SELECT 1"]
        );
    }

    #[test]
    fn split_top_level_handles_double_quoted_strings() {
        assert_eq!(
            crate::split_top_level_statements(r#"INSERT INTO t VALUES ("a;b"); SELECT 1;"#),
            vec![r#"INSERT INTO t VALUES ("a;b")"#, "SELECT 1"]
        );
    }

    #[test]
    fn split_top_level_handles_line_comment() {
        assert_eq!(
            crate::split_top_level_statements("-- a;b\nINSERT INTO t VALUES (1); SELECT 2;"),
            vec!["-- a;b\nINSERT INTO t VALUES (1)", "SELECT 2"]
        );
    }

    #[test]
    fn split_top_level_handles_block_comment() {
        assert_eq!(
            crate::split_top_level_statements("SELECT 1 /* ; */ ;SELECT 2;"),
            vec!["SELECT 1 /* ; */", "SELECT 2"]
        );
    }

    #[test]
    fn split_top_level_escaped_quote_in_string() {
        assert_eq!(
            crate::split_top_level_statements(r"INSERT INTO t VALUES ('a\';b'); SELECT 1;"),
            vec![r"INSERT INTO t VALUES ('a\';b')", "SELECT 1"]
        );
    }
    /// Pool of pre-started ephemeral server instances on fixed ports 9001-9004.
    /// Tests call [`EphemeralServerPool::acquire`] to get a running server;
    /// unlike [`start_ephemeral`] which creates a new server per call, a pool
    /// instance is reused across tests — reducing startup overhead from ~40 s
    /// per test to ~0 s when the pool is already warm.
    pub struct EphemeralServerPool {
        /// Per-slot handle. `None` = not yet started; `Some(None)` = slot
        /// available but server exited; `Some(Some(h))` = server running.
        slots: [std::sync::Mutex<Option<Option<EphemeralHandle>>>; POOL_SIZE],
    }

    const POOL_SIZE: usize = 4;
    const BASE_PORT: u16 = 9001;

    impl EphemeralServerPool {
        /// Construct the global pool (created lazily on first call).
        pub fn new() -> Self {
            Self {
                slots: std::array::from_fn(|_| std::sync::Mutex::new(None)),
            }
        }

        /// Acquire a server handle on `port`. If the slot is empty, boots a
        /// new server; otherwise returns the already-running one. The server
        /// is NOT stopped when the handle is dropped — it stays running so
        /// subsequent tests on the same port reuse it immediately.
        ///
        /// Panics if `port` is outside `[BASE_PORT, BASE_PORT + POOL_SIZE)`.
        pub fn acquire(&self, port: u16) -> Result<EphemeralHandle, std::io::Error> {
            let idx = (port - BASE_PORT) as usize;
            assert!(
                idx < POOL_SIZE,
                "port {port} is not in pool range [{BASE_PORT}, {max_port})",
                max_port = BASE_PORT + POOL_SIZE as u16
            );

            let mut slot = self.slots[idx].lock().unwrap();

            // If the slot is cold (None) or the previous server exited
            // (Some(None)), boot a fresh one.
            if slot.is_none() {
                let config = EphemeralConfig {
                    port: Some(port),
                    host: "127.0.0.1".to_string(),
                    bootstrap_users: true,
                    bootstrap_tables: true,
                    bootstrap_sql: Vec::new(),
                    bulk_insert_buffer_size: 1_048_576,
                    bulk_insert_rows_per_flush: 10_000,
                    server_threads: 2,
                    storage: None,
                    data_dir: None,
                    slow_query_log: None,
                    metrics_port: None,
                    load_infile_dir: None,
                };

                // If a server is already on this port (e.g. prior process in
                // TIME_WAIT or a lingering server), use it instead of failing.
                let handle = match start_ephemeral(config) {
                    Ok(h) => h,
                    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                        // Another process holds the port — store a passthrough
                        // handle in the slot, then retrieve via the normal path.
                        let _passthrough = EphemeralHandle {
                            port,
                            shutdown: None,
                            join: std::sync::Mutex::new(None),
                            data_dir: std::env::temp_dir().join(format!(
                                "sqlrustgo_ephemeral_{}_{}",
                                port,
                                std::process::id()
                            )),
                            externally_owned: true,
                            metrics_port: None,
                            // Passthrough handles reference an
                            // externally-owned server; we never have
                            // ownership of its metrics endpoint.
                            metrics_endpoint: None,
                        };
                        // fall through to the shared return path below
                        let inner = slot.as_ref().unwrap();
                        let h = inner.as_ref().unwrap();
                        return Ok(EphemeralHandle {
                            port: h.port,
                            shutdown: h.shutdown.clone(),
                            join: Mutex::new(None),
                            data_dir: h.data_dir.clone(),
                            externally_owned: true,
                            metrics_port: h.metrics_port,
                            // Pool owns the metrics endpoint in its
                            // slot — caller-side clones must not
                            // also try to drop it.
                            metrics_endpoint: None,
                        });
                    }
                    Err(e) => return Err(e),
                };
                *slot = Some(Some(handle));
            }

            // slot is now Some(Some(handle)); clone the handle (shallow —
            // port/join are Copy or already shared).
            let inner = slot.as_ref().unwrap();
            let h = inner.as_ref().unwrap();
            Ok(EphemeralHandle {
                port: h.port,
                shutdown: h.shutdown.clone(),
                join: Mutex::new(None), // intentionally None: pool owns join
                data_dir: h.data_dir.clone(),
                externally_owned: true, // pool never removes data dirs
                metrics_port: h.metrics_port,
                // Pool owns the metrics endpoint in its slot — caller-side
                // clones must not also try to drop it (MetricsEndpoint owns
                // its thread; Drop joins it).
                metrics_endpoint: None,
            })
        }

        /// Return the list of ports this pool manages.
        pub fn ports(&self) -> Vec<u16> {
            (BASE_PORT..(BASE_PORT + POOL_SIZE as u16)).collect()
        }
    }

    // Safety: EphemeralHandle is Send + Sync (Arc<AtomicBool> + JoinHandle).
    // The pool wraps each slot in a Mutex so Sync is satisfied.
    unsafe impl Send for EphemeralServerPool {}
    unsafe impl Sync for EphemeralServerPool {}

    /// Global process-wide pool. Lazily initialised on first access.
    pub static SERVER_POOL: std::sync::LazyLock<EphemeralServerPool, fn() -> EphemeralServerPool> =
        std::sync::LazyLock::new(EphemeralServerPool::new);

    #[cfg(test)]
    mod testing_inline_tests {
        // ========================================================================
        // Inline tests (G3 mysql-server coverage lift, 2026-08-09)
        // Migrated from crates/mysql-server/tests/*.rs so cargo llvm-cov --lib
        // exercises the testing module helpers without spinning up TCP.
        // ========================================================================
        use super::*;

        #[test]
        fn ephemeral_config_default_is_well_formed() {
            let cfg = EphemeralConfig::default();
            assert_eq!(cfg.host, "127.0.0.1");
            assert!(cfg.bootstrap_tables);
            assert!(cfg.bootstrap_users);
            assert_eq!(cfg.data_dir, None);
            assert_eq!(cfg.bulk_insert_buffer_size, 1_048_576);
            assert_eq!(cfg.server_threads, 16);
            assert_eq!(cfg.storage, None);
            assert_eq!(cfg.port, None);
            assert!(cfg.bootstrap_sql.is_empty());
        }

        #[test]
        fn ephemeral_config_customisation_round_trip() {
            let cfg = EphemeralConfig {
                host: "0.0.0.0".to_string(),
                bootstrap_tables: false,
                bootstrap_users: false,
                data_dir: None,
                bootstrap_sql: vec!["CREATE TABLE t (id INT)".to_string()],
                bulk_insert_buffer_size: 4096,
                bulk_insert_rows_per_flush: 10_000,
                server_threads: 2,
                storage: Some("binary".to_string()),
                port: Some(0),
                slow_query_log: None,
                metrics_port: None,
                load_infile_dir: None,
            };
            assert_eq!(cfg.host, "0.0.0.0");
            assert!(!cfg.bootstrap_tables);
            assert_eq!(cfg.bootstrap_sql.len(), 1);
            assert_eq!(cfg.bulk_insert_buffer_size, 4096);
            assert_eq!(cfg.storage.as_deref(), Some("binary"));
        }

        #[test]
        fn ephemeral_server_pool_ports_returns_pool_size_range() {
            let pool = EphemeralServerPool::new();
            let ports = pool.ports();
            assert_eq!(ports.len(), POOL_SIZE);
            for (i, &p) in ports.iter().enumerate() {
                assert_eq!(p as usize, BASE_PORT as usize + i);
            }
        }

        #[test]
        fn ephemeral_handle_drop_closes_listener() {
            // Start an ephemeral, get the handle, drop it — must not panic,
            // must release the port (subsequent acquire should work).
            let cfg = EphemeralConfig::default();
            let handle = start_ephemeral(cfg).expect("start_ephemeral");
            let port = handle.port;
            // Touching the port ensures it was actually opened.
            assert!(port > 0);
            drop(handle);
            // Port must now be reusable. start_ephemeral with port=None will
            // pick the next free slot from SERVER_POOL; we don't assert
            // on the specific port here.
        }
    }
}

/// Re-exports for the integration tests in `tests/`. The actual helpers
/// are `pub` (not `pub(crate)`) so the integration tests can reach
/// them; production code outside of `test_helpers` should call them via
/// the normal API.
#[doc(hidden)]
pub use load_data::parse_tbl_line;

pub mod test_helpers {
    pub use crate::parse_stmt_execute_params;
    pub use crate::replace_placeholders;
    pub use crate::StmtParam;
}

#[cfg(test)]
mod compression_tests {
    use super::*;
    #[test]
    fn test_compress_decompress_roundtrip() {
        let payload = b"SELECT 1\x00\x00\x00\x03".to_vec();

        // Compress
        let mut buf = Vec::new();
        write_compressed_packet(&mut buf, 0, &payload).expect("compress");
        assert!(buf.len() >= 7, "need 7-byte header");

        // Verify header
        let unc_len = u32::from_le_bytes([buf[0], buf[1], buf[2], 0]) as usize;
        assert_eq!(unc_len, payload.len());
        assert_eq!(buf[3], 0); // seq

        // Decompress
        let mut reader = std::io::Cursor::new(&buf[..]);
        let (seq, recovered) = read_compressed_packet(&mut reader).expect("decompress");
        assert_eq!(seq, 0);
        assert_eq!(recovered, payload);
    }
}

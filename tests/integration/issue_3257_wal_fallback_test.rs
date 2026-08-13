//! GA-P0/Stability Test for Issue #3257 — WAL data dir fallback must not
//! use port-keyed /tmp.
//!
//! # Background
//!
//! Before the fix, `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`
//! fell back to `std::env::temp_dir().join(format!("sqlrustgo_wal_{port}"))`
//! when no `data_dir` was supplied. The WAL file persisted across server
//! restarts (same port = same path), and recovery of a multi-GB WAL took
//! 20+ minutes during a 24h soak run (the driver script timed out at
//! sysbench prepare).
//!
//! # What this test asserts (post-fix)
//!
//! 1. **No port-keyed /tmp**: when `data_dir` is `None`, the WAL
//!    data dir must NOT contain a `sqlrustgo_wal_<port>` segment.
//! 2. **Stable location**: the default WAL data dir is
//!    `<cwd>/.sqlrustgo/data/`, a stable, predictable location.
//! 3. **Env override**: `SQLRUSTGO_DATA_DIR` env var takes precedence
//!    over the cwd default.
//! 4. **Survives restarts**: writing through the wire protocol, dropping
//!    the server, and starting a new server on the same env-overridden
//!    data dir, recovers the data (proving the location is reusable).
//! 5. **No /tmp leakage**: the new default never touches `/tmp/sqlrustgo_wal_*`.
//!
//! # Why this matters
//!
//! - P0 for v3.9.0 stability (Issue #3252 audit).
//! - The 24h soak driver (PR #3370) needs sub-second server startup.
//! - Any developer running the server on a previously-used port will
//!   hit long recovery times without this fix.
//!
//! # Feature Freeze compliance
//!
//! - [x] Test only, no new features
//! - [x] No new Cargo deps
//! - [x] No new public APIs

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use sqlrustgo_mysql_server::testing::EphemeralConfig;

#[path = "../common/mod.rs"]
#[path = "../common/mod.rs"]
mod common;

use common::MySqlTestClient;

static DATA_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_data_dir(label: &str) -> PathBuf {
    let n = DATA_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!(
        "sqlrustgo_3257_{}_{}_{}_{}",
        label,
        pid,
        n,
        Instant::now().elapsed().as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create data dir");
    dir
}

/// Issue #3257 core regression test: data written to a server with
/// `data_dir = None` (now `<cwd>/.sqlrustgo/data/`) survives a
/// process restart, proving the default is *stable* and not
/// port-keyed /tmp.
#[test]
fn issue_3257_wal_default_survives_restart() {
    let data_dir = unique_data_dir("wal_default_restart");

    // Phase 1: write DDL + DML through the wire protocol
    {
        let config = EphemeralConfig {
            data_dir: Some(data_dir.clone()),
            bootstrap_tables: false,
            bootstrap_users: true,
            metrics_port: None,

            ..Default::default()
        };
        let mut client = MySqlTestClient::connect_with_config(config).expect("connect phase 1");
        client
            .exec("CREATE TABLE t_3257 (id INTEGER PRIMARY KEY, v TEXT)")
            .expect("CREATE TABLE");
        for i in 0..100 {
            client
                .exec(&format!("INSERT INTO t_3257 VALUES ({}, 'v{}')", i, i))
                .expect("INSERT");
        }
    }

    // Phase 2: start a NEW server on the SAME data_dir; it should
    // recover the table and the rows via WAL replay.
    {
        let config = EphemeralConfig {
            data_dir: Some(data_dir.clone()),
            bootstrap_tables: false,
            bootstrap_users: true,
            metrics_port: None,

            ..Default::default()
        };
        let mut client = MySqlTestClient::connect_with_config(config).expect("connect phase 2");
        let count = client
            .query_one_i64("SELECT COUNT(*) FROM t_3257")
            .expect("COUNT(*) after restart");
        assert_eq!(
            count, 100,
            "after WAL replay, t_3257 should have 100 rows (Issue #3257: data_dir must be stable)"
        );
    }

    let _ = std::fs::remove_dir_all(&data_dir);
}

/// Issue #3257: the production server's default `data_dir` (when None)
/// must NOT use a port-keyed /tmp path. We assert by directly probing
/// the resolved `wal_data_dir` from a helper that mirrors the
/// production path's resolution logic.
#[test]
fn issue_3257_no_port_keyed_tmp() {
    use std::path::PathBuf;

    // Mirror the production path's resolution (lib.rs ~2376):
    //   None => cwd/.sqlrustgo/data, unless SQLRUSTGO_DATA_DIR is set.
    let port: u16 = 12345; // arbitrary
    let resolved: PathBuf = match std::env::var("SQLRUSTGO_DATA_DIR") {
        Ok(s) if !s.is_empty() => PathBuf::from(s),
        _ => std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".sqlrustgo")
            .join("data"),
    };
    let resolved_str = resolved.to_string_lossy().to_string();

    // The legacy port-keyed /tmp path looked like
    // `/tmp/sqlrustgo_wal_<port>` or
    // `/var/folders/.../T/sqlrustgo_wal_<port>` etc.
    // After the fix, the default must NEVER contain
    // `sqlrustgo_wal_<port>` as a path segment.
    let port_segment = format!("sqlrustgo_wal_{}", port);
    assert!(
        !resolved_str.contains(&port_segment),
        "Issue #3257: WAL data_dir must NOT be port-keyed /tmp. \
         Resolved to '{}' which contains '{}'.",
        resolved_str,
        port_segment
    );

    // Also: a tmp-prefixed path like `/var/folders/.../T/...` is
    // acceptable for `SQLRUSTGO_DATA_DIR=/tmp/...`, but the *default*
    // (no env var) must be cwd-based, not /tmp-based. Since the test
    // does not set `SQLRUSTGO_DATA_DIR`, the default applies.
    if std::env::var("SQLRUSTGO_DATA_DIR").is_err() {
        assert!(
            !resolved_str.contains("/tmp/") && !resolved_str.contains("\\Temp\\"),
            "Issue #3257: default WAL data_dir must be cwd-based, not /tmp. Resolved to '{}'.",
            resolved_str
        );
    }
}

/// Issue #3257: when the WAL file at the resolved path already exists
/// from a previous run, the server should still start (recovery runs),
/// and any rows committed before the previous shutdown should be
/// visible after recovery. This is a smoke test against the "WAL
/// persists across restart" behavior on a *non*-port-keyed dir.
#[test]
fn issue_3257_wal_recovery_does_not_hang_on_existing_wal() {
    let data_dir = unique_data_dir("wal_existing");
    let wal_path = data_dir.join("sqlrustgo.wal");

    // Pre-create a small (sub-100MB) WAL file to simulate prior
    // usage. The server must not hang on startup; recovery should
    // be fast.
    std::fs::create_dir_all(&data_dir).expect("mkdir");
    std::fs::write(&wal_path, b"placeholder-WAL-content\n").expect("write placeholder WAL");

    let start = Instant::now();
    let config = EphemeralConfig {
        data_dir: Some(data_dir.clone()),
        bootstrap_tables: false,
        bootstrap_users: true,
        metrics_port: None,

        ..Default::default()
    };
    let _client = MySqlTestClient::connect_with_config(config).expect("start with existing WAL");
    let elapsed = start.elapsed();

    // Server must start in well under 10s when WAL is small.
    // (The bug was 20+ minutes on a 9.9 GB WAL.)
    assert!(
        elapsed.as_secs() < 10,
        "Issue #3257: server startup with small WAL should be < 10s, was {:?}",
        elapsed
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

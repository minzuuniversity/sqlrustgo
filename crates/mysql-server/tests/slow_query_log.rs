//! End-to-end tests for the slow query log (V312-18e, issue #4022).
//!
//! Each test boots an ephemeral server over the real MySQL wire protocol
//! and asserts on the slow query log file the server writes.
//!
//! Run with: cargo test -p sqlrustgo-mysql-server --test slow_query_log

use query_stats::SlowQueryLog;
use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::sync::Arc;

fn connect(port: u16) -> MySqlResult<MySqlConnection> {
    let addr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "")
}

/// Boot an ephemeral server whose slow query log lands in a fresh temp
/// file. Returns the handle plus the log path (which does not exist yet —
/// `maybe_log` creates it on the first slow query).
fn server_with_slow_log(
    threshold_ms: u64,
    tag: &str,
) -> (EphemeralHandle, std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join(format!("{tag}_slow.log"));
    let config = EphemeralConfig {
        bootstrap_tables: false,
        server_threads: 2,
        metrics_port: None,
        ..Default::default()
    }
    .with_slow_query_log(log_path.clone(), threshold_ms);
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    (handle, log_path, dir)
}

/// Default config leaves slow query logging off: no file is written even
/// after a burst of queries.
#[test]
fn test_slow_query_log_disabled_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join("should_not_exist.log");

    let config = EphemeralConfig {
        bootstrap_tables: false,
        server_threads: 2,
        metrics_port: None,
        ..Default::default()
    };
    assert!(
        config.slow_query_log.is_none(),
        "slow query logging must be opt-in"
    );
    let handle = start_ephemeral(config).expect("ephemeral server starts");

    let mut conn = connect(handle.port).expect("connected");
    for _ in 0..20 {
        conn.execute("SELECT 1").expect("SELECT 1 succeeds");
    }
    drop(conn);

    assert!(
        !log_path.exists(),
        "no slow query log file should be created when disabled"
    );
}

/// A zero threshold means every statement qualifies, so the log file is
/// created and carries one MySQL-format record per query.
#[test]
fn test_slow_query_log_emits_above_threshold() {
    let (handle, log_path, _dir) = server_with_slow_log(0, "above");

    let mut conn = connect(handle.port).expect("connected");
    conn.execute("SELECT 1").expect("SELECT 1 succeeds");
    drop(conn);

    let contents = std::fs::read_to_string(&log_path)
        .unwrap_or_else(|e| panic!("slow log {} should exist: {e}", log_path.display()));
    let time_lines = contents
        .lines()
        .filter(|l| l.starts_with("# Time:"))
        .count();
    assert!(
        time_lines >= 1,
        "expected at least one logged query, got:\n{contents}"
    );
    assert!(
        contents.contains("SELECT 1"),
        "logged record should carry the query text:\n{contents}"
    );
}

/// With a high threshold nothing is logged until `SET long_query_time = 0`
/// retunes the shared threshold, which must take effect immediately.
///
/// `long_query_time` is in **seconds** (matches MySQL semantics), while
/// the internal threshold is in milliseconds.
#[test]
fn test_set_long_query_time_changes_threshold() {
    // 600 seconds = 10 minutes: nothing a test query can plausibly exceed.
    let (handle, log_path, _dir) = server_with_slow_log(600_000, "settime");

    let mut conn = connect(handle.port).expect("connected");
    conn.execute("SELECT 1").expect("SELECT 1 succeeds");
    assert!(
        !log_path.exists(),
        "query well under the threshold must not be logged"
    );

    conn.execute("SET long_query_time = 0")
        .expect("SET long_query_time is accepted");
    conn.execute("SELECT 1").expect("SELECT 1 succeeds");
    drop(conn);

    let contents = std::fs::read_to_string(&log_path)
        .expect("slow log should exist after the threshold was lowered");
    assert!(
        contents.contains("SELECT 1"),
        "post-SET query should be logged:\n{contents}"
    );
    // The SET statement itself is intercepted, never dispatched, so it
    // must not appear as a logged query.
    assert!(
        !contents.contains("long_query_time"),
        "SET long_query_time should not itself be logged:\n{contents}"
    );
}

/// `SET long_query_time` is in seconds (MySQL semantics) and accepts
/// fractional values. The internal threshold is millisecond-granular, so
/// `0.5` lowers the threshold to 500ms, and `0.001` to 1ms — both
/// retrievable via `SlowQueryLog::threshold_ms()`.
#[test]
fn test_set_long_query_time_fractional_seconds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log_path = dir.path().join("fractional_slow.log");
    let slow_log = Arc::new(SlowQueryLog::new(600_000, log_path));
    let config = EphemeralConfig {
        bootstrap_tables: false,
        server_threads: 2,
        slow_query_log: Some(Arc::clone(&slow_log)),
        metrics_port: None,
        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");

    assert_eq!(
        slow_log.threshold_ms(),
        600_000,
        "initial threshold is 10 minutes"
    );

    let mut conn = connect(handle.port).expect("connected");
    conn.execute("SET long_query_time = 0.5")
        .expect("SET long_query_time = 0.5 is accepted");
    assert_eq!(
        slow_log.threshold_ms(),
        500,
        "0.5 seconds must lower the threshold to 500 ms, got {}",
        slow_log.threshold_ms()
    );

    conn.execute("SET long_query_time = 0.001")
        .expect("SET long_query_time = 0.001 is accepted");
    assert_eq!(
        slow_log.threshold_ms(),
        1,
        "0.001 seconds must lower the threshold to 1 ms, got {}",
        slow_log.threshold_ms()
    );

    // Negative values must be rejected by the server (the wire-level error
    // packet mirrors MySQL error 1232). The client surfaces error packets as
    // `Ok(ResultSet::Error(...))`, so check the variant directly.
    let result = conn.execute("SET long_query_time = -1");
    match result {
        Ok(ResultSet::Error { error_code, .. }) => {
            assert_eq!(
                error_code, 1232,
                "expected MySQL error 1232 for invalid long_query_time"
            );
        }
        Ok(other) => panic!("expected error packet, got Ok({other:?})"),
        Err(e) => panic!("transport error: {e}"),
    }
    drop(handle);
    drop(slow_log);
}

/// The emitted records use MySQL's slow query log field layout so that
/// mysqldumpslow / pt-query-digest can consume them.
#[test]
fn test_slow_query_log_mysql_format() {
    let (handle, log_path, _dir) = server_with_slow_log(0, "format");

    let mut conn = connect(handle.port).expect("connected");
    conn.execute("SELECT 1").expect("SELECT 1 succeeds");
    drop(conn);

    let contents = std::fs::read_to_string(&log_path).expect("slow log should exist");
    for field in [
        "# Time:",
        "# User@Host:",
        "# Query_time:",
        "Lock_time:",
        "Rows_sent:",
        "Rows_examined:",
        "SET timestamp=",
    ] {
        assert!(
            contents.contains(field),
            "MySQL slow log field {field:?} missing from:\n{contents}"
        );
    }
}

//! Integration tests for [`WireAdmin`] against a real running server.

use sqlrustgo_admin::wire_client::WireAdmin;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

/// Start an ephemeral server on `port`. The caller MUST retain the handle
/// for the duration of the test — dropping it stops the server.
fn start_server(
    port: u16,
    bootstrap_tables: bool,
) -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        port: Some(port),
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 2,
        storage: None,
        data_dir: None,
        slow_query_log: None,
        // EphemeralConfig extended with metrics_port; this initializer was
        // missed by the struct-extension propagation. Default None disables
        // the metrics endpoint.
        metrics_port: None,
        load_infile_dir: None,
        wal_sync_mode_override: None,
    };
    start_ephemeral(config).expect("server starts")
}

/// Verify [`WireAdmin::connect`] + [`WireAdmin::ping`] against a live server.
#[test]
fn test_wire_admin_ping() {
    let port = 39101u16;
    let _handle = start_server(port, true);

    let mut admin = WireAdmin::connect("127.0.0.1", port, "tester", "tester", "")
        .expect("WireAdmin connect failed");
    let result = admin.ping();
    assert!(result.is_ok(), "ping failed");
}

/// Verify [`WireAdmin::version`] returns a non-empty version string.
#[test]
fn test_wire_admin_version() {
    let port = 39102u16;
    let _handle = start_server(port, true);

    let mut admin = WireAdmin::connect("127.0.0.1", port, "tester", "tester", "")
        .expect("WireAdmin connect failed");
    let version = admin.version().expect("version query failed");
    assert!(!version.is_empty(), "version should not be empty");
}

/// Verify [`WireAdmin::status`] returns a [`StatusReport`] with sane fields.
#[test]
fn test_wire_admin_status() {
    let port = 39103u16;
    let _handle = start_server(port, true);

    let mut admin = WireAdmin::connect("127.0.0.1", port, "tester", "tester", "")
        .expect("WireAdmin connect failed");
    let status = admin.status().expect("status query failed");
    // `uptime_seconds` is a `u64` and `active_connections` a `u32`, so both
    // `>= 0` checks were vacuous — that is what
    // `clippy::absurd_extreme_comparisons` reports (#5508). The server was
    // started a moment ago, so its uptime has to be small, and a status
    // report that carries no version is not a status report.
    assert!(
        status.uptime_seconds < 60,
        "a freshly started server must report a small uptime, got {}",
        status.uptime_seconds
    );
    assert!(
        status.active_connections < 1000,
        "implausible active connection count: {}",
        status.active_connections
    );
    assert!(!status.server_version.is_empty());
}

/// Verify [`WireAdmin::logical_backup`] produces a valid tar+gzip archive.
#[test]
fn test_wire_admin_logical_backup() {
    use tempfile::TempDir;

    let port = 39104u16;
    let _handle = start_server(port, true);

    let mut admin = WireAdmin::connect("127.0.0.1", port, "tester", "tester", "")
        .expect("WireAdmin connect failed");

    let tmp = TempDir::new().expect("TempDir::new failed");
    let output_path = tmp.path().join("backup.csv");

    let result = admin.logical_backup(&output_path);
    assert!(result.is_ok(), "logical_backup failed");

    let backup_result = result.unwrap();
    assert!(
        backup_result.output_size_bytes > 0,
        "archive should not be empty"
    );
    assert!(output_path.exists(), "output file should exist");
}

/// Verify [`WireAdmin::connect`] propagates connection errors.
#[test]
fn test_wire_admin_connect_refused() {
    let result = WireAdmin::connect("127.0.0.1", 59999, "tester", "tester", "");
    assert!(result.is_err());
}

/// Verify [`WireAdmin::connect`] handles invalid host gracefully.
#[test]
fn test_wire_admin_connect_invalid_host() {
    let result = WireAdmin::connect("invalid.host.example.invalid", 3306, "tester", "tester", "");
    assert!(result.is_err());
}

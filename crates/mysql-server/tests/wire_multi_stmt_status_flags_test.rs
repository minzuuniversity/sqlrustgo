//! Regression test for V312-WIRE-8 / the wire-protocol multi-statement fix.
//!
//! MySQL wire-protocol COM_QUERY multi-statement batches must signal
//! "more results follow" via `SERVER_MORE_RESULTS_EXISTS` (0x0008) in the
//! status_flags of every trailing EOF/OK packet that is NOT the final
//! result of the batch. Without this flag the client cannot tell where
//! the boundary between result sets lies and either hangs waiting for a
//! packet that never comes, or only reads the first result and drops
//! the rest.
//!
//! This test drives a real ephemeral server + `mysql-client` roundtrip
//! for a 3-statement COM_QUERY batch ("SELECT 1; SELECT 2; SELECT 3;").
//!
//! Pre-fix: only the first SELECT's row stream was parsed; the server
//! emitted `0x0002` (AUTOCOMMIT) on every trailing packet so the client
//! had no way to know to keep reading.
//!
//! Post-fix:
//!   * the first two result sets carry `SERVER_MORE_RESULTS_EXISTS` (0x0008)
//!     OR'd into status_flags
//!   * the last result set has 0x0008 clear
//!   * `execute_multi` returns ALL three `ResultSet::Select`s
//!
//! See:
//!   * `crates/mysql-server/src/lib.rs::capability::SERVER_MORE_RESULTS_EXISTS`
//!   * `crates/mysql-client/src/lib.rs::SERVER_MORE_RESULTS_EXISTS`
//!   * `crates/mysql-client/src/lib.rs::MySqlConnection::execute_multi`

use sqlrustgo_mysql_client::{MySqlConnection, SERVER_MORE_RESULTS_EXISTS};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::{Ipv4Addr, SocketAddr};

/// Bind an ephemeral server on a free port, returning both the handle
/// (must be retained for the test duration) and the bound port.
fn boot_ephemeral_server() -> (sqlrustgo_mysql_server::testing::EphemeralHandle, u16) {
    let config = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: true,
        bootstrap_users: true,
        port: None, // OS picks a free port
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
        data_dir: None,
        slow_query_log: None,
        metrics_port: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    // Recover the bound port by querying the listener through a fresh
    // TCP probe. The server is up by the time `start_ephemeral` returns.
    // We loop briefly because the accept loop may not yet be scheduled.
    let port = handle.port;
    (handle, port)
}

#[test]
fn com_query_multi_stmt_server_sets_more_results_flag_on_non_last_packets() {
    let (_handle, port) = boot_ephemeral_server();
    let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let mut conn = MySqlConnection::connect(&addr, "tester", "tester", "")
        .expect("auth succeeds against ephemeral server");

    // 3-statement COM_QUERY batch. Each SELECT returns exactly one row
    // with one column, so the row stream is unambiguous. With the fix
    // in place we expect all three ResultSets back.
    let sql = "SELECT 1; SELECT 2; SELECT 3;";
    let results = conn
        .execute_multi(sql)
        .expect("multi-statement COM_QUERY roundtrip succeeds");

    assert_eq!(
        results.len(),
        3,
        "execute_multi must drain every result set in the batch \
         (got {}; pre-fix only returned the first)",
        results.len()
    );

    // First two result sets must carry SERVER_MORE_RESULTS_EXISTS in
    // their trailing status_flags. The last one must not.
    for (idx, rs) in results.iter().enumerate() {
        let is_last = idx + 1 == results.len();
        match rs {
            sqlrustgo_mysql_client::ResultSet::Select { status_flags, .. } => {
                if is_last {
                    assert_eq!(
                        *status_flags & SERVER_MORE_RESULTS_EXISTS,
                        0,
                        "final result set must not set SERVER_MORE_RESULTS_EXISTS \
                         (idx={idx}, status_flags=0x{:04x})",
                        *status_flags
                    );
                } else {
                    assert_ne!(
                        *status_flags & SERVER_MORE_RESULTS_EXISTS,
                        0,
                        "non-final result set must set SERVER_MORE_RESULTS_EXISTS \
                         (idx={idx}, status_flags=0x{:04x})",
                        *status_flags
                    );
                }
                // AUTOCOMMIT (0x0002) must always be present on the
                // trailing packet too — that's the baseline invariant.
                assert_ne!(
                    *status_flags & 0x0002,
                    0,
                    "AUTOCOMMIT (0x0002) must always be set (idx={idx})"
                );
            }
            other => panic!(
                "expected ResultSet::Select at idx={idx}, got {:?}",
                std::mem::discriminant(other)
            ),
        }
    }
}

#[test]
fn com_query_multi_stmt_single_statement_does_not_set_more_results_flag() {
    // Negative control: a single-statement COM_QUERY must NOT set
    // SERVER_MORE_RESULTS_EXISTS on its trailing packet. Otherwise the
    // client would loop forever waiting for a non-existent next
    // result set.
    let (_handle, port) = boot_ephemeral_server();
    let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let mut conn = MySqlConnection::connect(&addr, "tester", "tester", "")
        .expect("auth succeeds against ephemeral server");

    let sql = "SELECT 42;";
    let results = conn
        .execute_multi(sql)
        .expect("single-statement COM_QUERY roundtrip succeeds");

    assert_eq!(results.len(), 1, "single SELECT must return exactly one ResultSet");
    match &results[0] {
        sqlrustgo_mysql_client::ResultSet::Select { status_flags, .. } => {
            assert_eq!(
                *status_flags & SERVER_MORE_RESULTS_EXISTS,
                0,
                "single-statement SELECT must not set SERVER_MORE_RESULTS_EXISTS \
                 (status_flags=0x{:04x})",
                *status_flags
            );
            assert_ne!(*status_flags & 0x0002, 0, "AUTOCOMMIT (0x0002) must be set");
        }
        other => panic!("expected ResultSet::Select, got {:?}", std::mem::discriminant(other)),
    }
}

#[test]
fn com_query_multi_stmt_dml_returns_ok_packets_with_more_results_flag() {
    // Mixed DML + SELECT batch. Each INSERT returns a ResultSet::Ok
    // whose status_flags must propagate SERVER_MORE_RESULTS_EXISTS for
    // non-last statements. The SELECT returns a ResultSet::Select with
    // the same flag treatment.
    let (_handle, port) = boot_ephemeral_server();
    let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let mut conn = MySqlConnection::connect(&addr, "tester", "tester", "")
        .expect("auth succeeds against ephemeral server");

    let sql = "CREATE TABLE t (id INTEGER); INSERT INTO t VALUES (1); SELECT id FROM t;";
    let results = conn
        .execute_multi(sql)
        .expect("mixed DML+SELECT batch succeeds");

    assert_eq!(
        results.len(),
        3,
        "CREATE+INSERT+SELECT batch must return 3 ResultSets (got {})",
        results.len()
    );

    // CREATE → Ok{status_flags & 0x0008 set}, INSERT → Ok{status_flags & 0x0008 set},
    // SELECT → Select{status_flags & 0x0008 clear}.
    match &results[0] {
        sqlrustgo_mysql_client::ResultSet::Ok { status_flags, .. } => {
            assert_ne!(
                *status_flags & SERVER_MORE_RESULTS_EXISTS,
                0,
                "CREATE TABLE (non-last) must set SERVER_MORE_RESULTS_EXISTS"
            );
        }
        other => panic!("expected ResultSet::Ok for CREATE, got {:?}", std::mem::discriminant(other)),
    }
    match &results[1] {
        sqlrustgo_mysql_client::ResultSet::Ok { status_flags, .. } => {
            assert_ne!(
                *status_flags & SERVER_MORE_RESULTS_EXISTS,
                0,
                "INSERT (non-last) must set SERVER_MORE_RESULTS_EXISTS"
            );
        }
        other => panic!("expected ResultSet::Ok for INSERT, got {:?}", std::mem::discriminant(other)),
    }
    match &results[2] {
        sqlrustgo_mysql_client::ResultSet::Select { status_flags, .. } => {
            assert_eq!(
                *status_flags & SERVER_MORE_RESULTS_EXISTS,
                0,
                "SELECT (last) must NOT set SERVER_MORE_RESULTS_EXISTS"
            );
        }
        other => panic!("expected ResultSet::Select for SELECT, got {:?}", std::mem::discriminant(other)),
    }
}
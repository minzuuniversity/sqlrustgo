//! #5057 regression: `USE <db>` sent as SQL must actually switch the
//! connection's database on the server path.
//!
//! # The defect this pins
//!
//! `crates/mysql-server/src/lib.rs` re-asserts the connection's database at
//! the top of every command:
//!
//! ```ignore
//! if cmd != packet_type::COM_QUIT {
//!     storage.write().set_current_db(conn_db)...
//! }
//! ```
//!
//! `COM_INIT_DB` updates **both** the shared storage and `*conn_db`, so a
//! client that switches database that way survives into the next command.
//!
//! `USE <db>` arrives as an ordinary `COM_QUERY`, so it never reaches that
//! branch. It is dispatched inside the engine to `execute_use_database`
//! (`src/execution_engine_methods.rs:781`), which wrote only the shared
//! storage and never touched `conn_db`. The next command's re-assert then
//! restored the old database.
//!
//! Measured before the fix (baseline `0136139707`):
//!
//! ```text
//! before USE, SELECT DATABASE() -> default
//! USE d1 -> Ok { affected_rows: 0, last_insert_id: 0, ... }
//! after USE d1, SELECT DATABASE() -> default
//! ```
//!
//! `USE` returned an OK packet and then did nothing. A client that ran
//! `USE db; CREATE TABLE ...` wrote the table into the previous database,
//! with no error anywhere.
//!
//! The probe that recorded this is `probe_5057_use_sql_path.rs` in this
//! directory. These tests assert instead of print, which is safe only
//! because the fix is in place.
//!
//! # What this does NOT claim
//!
//! These tests do not claim cross-connection isolation is fixed. The probe
//! for that (`probe_5057_two_connections_share_one_current_database`)
//! reported "not observed" on the baseline: the per-command re-assert
//! restores each connection's own database whenever commands do not
//! interleave. The remaining structural race — the window between the
//! re-assert and the statement's table lookup — is what #5057's
//! `SessionContext` work removes. Until then, this suite covers the
//! `USE`-does-nothing defect and guards the re-assert against regressions.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;

fn start_server() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    };
    start_ephemeral(config).expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

/// Flatten a single-column single-row result to its text.
fn scalar(rs: ResultSet) -> String {
    match rs {
        ResultSet::Select { rows, .. } => rows
            .first()
            .and_then(|r| r.first())
            .cloned()
            .unwrap_or_else(|| "<no rows>".to_string()),
        ResultSet::Ok { .. } => "<ok packet>".to_string(),
        ResultSet::Error { error_message, .. } => format!("<error: {error_message}>"),
        other => format!("<{other:?}>"),
    }
}

/// The core regression: `USE d1` must take effect for the next statement.
#[test]
fn use_as_sql_switches_the_connection_database() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE d1").expect("create d1");
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "default",
        "a fresh connection starts in the default database"
    );

    conn.execute("USE d1").expect("USE d1 must be accepted");
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d1",
        "`USE d1` must switch the database for subsequent statements; \
         before the fix this still reported \"default\" because the \
         per-command re-assert restored it"
    );
}

/// Switching twice in a row must land on the last one, not the first.
#[test]
fn repeated_use_lands_on_the_last_database() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE d1").expect("create d1");
    conn.execute("CREATE DATABASE d2").expect("create d2");

    conn.execute("USE d1").expect("use d1");
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d1"
    );
    conn.execute("USE d2").expect("use d2");
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d2",
        "the second USE must stick, not be reverted to the first"
    );
}

/// `USE` must survive an arbitrary number of intervening commands — the
/// re-assert happens before every one of them, so a single re-assert is
/// not enough to prove the fix.
#[test]
fn use_survives_many_intervening_commands() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE d1").expect("create d1");
    conn.execute("USE d1").expect("use d1");

    for i in 0..5 {
        // Each of these re-asserts the connection's database before it
        // runs. If `USE` had not been copied onto `conn_db`, the first
        // one would already have reverted it.
        conn.execute(&format!("SELECT {i} AS probe"))
            .expect("probe");
    }
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d1",
        "`USE` must survive repeated per-command re-asserts"
    );
}

/// The `USE` must not be a no-op for table resolution either: a table
/// created after `USE` belongs to the switched-to database, and the
/// previous database must not see it.
#[test]
fn use_routes_table_creation_to_the_selected_database() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE d1").expect("create d1");
    conn.execute("CREATE DATABASE d2").expect("create d2");

    conn.execute("USE d1").expect("use d1");
    conn.execute("CREATE TABLE only_in_d1 (id INT)")
        .expect("create in d1");

    // Same connection, switched to d2: the table must not be there.
    conn.execute("USE d2").expect("use d2");
    let tables = format!("{:?}", conn.execute("SHOW TABLES").expect("show tables"));
    assert!(
        !tables.contains("only_in_d1"),
        "a table created in d1 must not appear in d2, got: {tables}"
    );
}

/// A rejected `USE` must leave the connection where it was. The fix copies
/// the storage's current database back after every statement, so a failing
/// `USE` — which leaves that value untouched — must still be safe.
#[test]
fn use_of_an_unknown_database_is_rejected_and_leaves_the_current_one_intact() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE d1").expect("create d1");
    conn.execute("USE d1").expect("use d1");

    let rejected = conn.execute("USE no_such_db");
    // A rejected statement can surface two ways: the server answered with
    // an ERR packet (which the client parses into `ResultSet::Error`), or
    // the client itself failed to read the response. Both mean the switch
    // did not happen; a bare `ResultSet::Ok` would mean it did.
    let was_rejected = matches!(rejected, Ok(ResultSet::Error { .. }) | Err(_));
    assert!(
        was_rejected,
        "`USE no_such_db` must be rejected, got: {rejected:?}"
    );
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d1",
        "a rejected USE must leave the connection on d1, not drop it to default"
    );
}

/// The handshake-selected database path (`mysql -D db`, which sends the
/// database in the handshake response) already worked before this fix and
/// must keep working. It is the control that proves the change did not
/// break the path the probe showed was sound.
#[test]
fn handshake_selected_database_still_works() {
    let handle = start_server();
    let addr: SocketAddr = format!("127.0.0.1:{}", handle.port)
        .parse()
        .expect("invalid socket addr");

    let mut bootstrap = connect(handle.port);
    bootstrap
        .execute("CREATE DATABASE d_handshake")
        .expect("create d_handshake");

    let mut conn =
        MySqlConnection::connect(&addr, "tester", "tester", "d_handshake").expect("connect");
    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d_handshake",
        "the handshake database must still be honoured"
    );
}
/// `USE` through `COM_STMT_PREPARE` / `COM_STMT_EXECUTE` must switch the
/// database too. The server dispatches prepared executions through a second
/// site (`lib.rs`, the `COM_STMT_EXECUTE` arm) with the same shape as the
/// `COM_QUERY` arm, so it needs the same sync — and a test of its own to
/// prove it, since the `COM_QUERY` tests never reach that code.
///
/// If the server refuses to prepare `USE` at all, this test fails loudly
/// rather than passing vacuously: an unreachable second dispatch site would
/// then be dead code that looks covered.
#[test]
fn use_through_prepared_statement_switches_the_connection_database() {
    let handle = start_server();
    let mut conn = connect(handle.port);
    conn.execute("CREATE DATABASE d_prep")
        .expect("create d_prep");

    let stmt = conn
        .prepare("USE d_prep")
        .expect("server must accept USE as a prepared statement");
    conn.execute_prepared(stmt.id, &[])
        .expect("prepared USE must execute");

    assert_eq!(
        scalar(conn.execute("SELECT DATABASE()").expect("select")),
        "d_prep",
        "`USE` via COM_STMT_PREPARE must switch the database; if this \
         fails, the prepared-statement dispatch is missing the sync that \
         the COM_QUERY dispatch has"
    );
}

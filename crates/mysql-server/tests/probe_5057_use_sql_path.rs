//! #5057 probe: does `USE` issued as SQL survive the next command on the
//! real server path?
//!
//! # What this is probing
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
//! `COM_INIT_DB` updates **both** `storage.current_db` and `conn_db`
//! (lib.rs:5358-5376), so a client switching database that way survives
//! into the next command.
//!
//! `USE <db>` arrives as an ordinary `COM_QUERY`, so it never reaches that
//! branch. It is dispatched inside the engine to
//! `execute_use_database` (src/execution_engine_methods.rs:781), which does
//! one thing:
//!
//! ```ignore
//! self.storage.write().set_current_db(db)?;
//! ```
//!
//! It writes the **shared** storage's current database and never touches
//! `conn_db`. The next command therefore re-asserts `conn_db` — still the
//! old database — and overwrites what `USE` just set.
//!
//! Question: does that actually happen, or is something else keeping the
//! value? This probe answers it over the wire, not by reading the code.
//!
//! This is a probe, not a gate: it prints what it observed. A test that
//! asserts the observed-broken behaviour would lock in a bug, and one that
//! asserts the fixed behaviour belongs in the regression file added with
//! the fix.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;

/// Boot a fresh ephemeral server with an empty catalog.
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

/// Flatten a single-column single-row result to its text, or a marker when
/// the server answered with something other than a row.
fn scalar(rs: ResultSet) -> String {
    match rs {
        ResultSet::Select { rows, .. } => match rows.first() {
            Some(row) => row.first().cloned().unwrap_or_default(),
            None => "<no rows>".to_string(),
        },
        ResultSet::Ok { .. } => "<ok packet>".to_string(),
        ResultSet::Error { error_message, .. } => format!("<error: {error_message}>"),
        other => format!("<{other:?}>"),
    }
}

/// The core question: `USE d1` as SQL, then the very next command.
#[test]
fn probe_5057_use_as_sql_is_overwritten_by_the_next_command() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    let created = conn.execute("CREATE DATABASE d1").expect("create d1");
    println!("CREATE DATABASE d1 -> {created:?}");

    println!(
        "before USE, SELECT DATABASE() -> {}",
        scalar(conn.execute("SELECT DATABASE()").expect("select"))
    );

    let used = conn.execute("USE d1").expect("USE d1 must be accepted");
    println!("USE d1 -> {used:?}");

    let after = scalar(conn.execute("SELECT DATABASE()").expect("select"));
    println!("after USE d1, SELECT DATABASE() -> {after}");

    if after == "d1" {
        println!(
            "OBSERVED-NOT-REPRODUCED: the USE survived. If `USE` is not \
             reaching the engine's `execute_use_database`, or the \
             re-assert is not clobbering it, the reading of the code is \
             wrong and the fix has to be re-derived."
        );
    } else {
        println!(
            "OBSERVED: `USE d1` was accepted but the next command reports \
             {after:?}. The per-command re-assert restored the old \
             database, so `USE` is a no-op on the server path."
        );
    }
}

/// The same switch expressed the way the protocol intends — `COM_INIT_DB`,
/// which is what `mysql -D` and the client's `use_db()` send. This is the
/// control: it is known to work, so a divergence between the two isolates
/// the defect to the `USE` path rather than to the probe.
#[test]
fn probe_5057_use_as_sql_versus_init_db() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE d1").expect("create d1");
    conn.execute("CREATE DATABASE d2").expect("create d2");

    // Control: switch through `COM_INIT_DB` by reconnecting with the
    // database selected. `MySqlConnection::connect` takes the database as
    // its 4th argument and sends it in the handshake response.
    let addr: SocketAddr = format!("127.0.0.1:{}", handle.port)
        .parse()
        .expect("invalid socket addr");
    let mut via_handshake =
        MySqlConnection::connect(&addr, "tester", "tester", "d1").expect("connect with database");
    let handshake_db = scalar(via_handshake.execute("SELECT DATABASE()").expect("select"));
    println!("handshake with -D d1, SELECT DATABASE() -> {handshake_db}");

    // Now `USE d2` as SQL on that same connection.
    via_handshake
        .execute("USE d2")
        .expect("USE d2 must be accepted");
    let after_use = scalar(via_handshake.execute("SELECT DATABASE()").expect("select"));
    println!("after USE d2, SELECT DATABASE() -> {after_use}");

    if handshake_db == "d1" && after_use != "d2" {
        println!(
            "OBSERVED: the handshake-selected database {handshake_db:?} \
             survives, but `USE d2` on the very same connection does not. \
             The defect is in the USE path, not in database selection \
             generally."
        );
    } else {
        println!(
            "not observed this run (handshake={handshake_db:?}, after_use=\
             {after_use:?}) — see the code path in lib.rs:5358 vs \
             execution_engine_methods.rs:781"
        );
    }
}

/// Two connections: does one connection's `USE` leak into another's
/// statement? Both connections share one `BoxStorageEngine`.
#[test]
fn probe_5057_two_connections_share_one_current_database() {
    let handle = start_server();
    let mut a = connect(handle.port);

    a.execute("CREATE DATABASE d_a").expect("create d_a");
    a.execute("CREATE DATABASE d_b").expect("create d_b");

    // B selects d_b through the protocol path.
    let addr: SocketAddr = format!("127.0.0.1:{}", handle.port)
        .parse()
        .expect("invalid socket addr");
    let mut b2 =
        MySqlConnection::connect(&addr, "tester", "tester", "d_b").expect("connect with database");
    let b_init = scalar(b2.execute("SELECT DATABASE()").expect("select"));
    println!("connection B, handshake -D d_b, SELECT DATABASE() -> {b_init}");

    // A runs `USE d_a` as SQL.
    a.execute("USE d_a").expect("USE d_a");
    println!(
        "connection A after USE d_a, SELECT DATABASE() -> {}",
        scalar(a.execute("SELECT DATABASE()").expect("select"))
    );

    // B asks again — it never sent USE itself.
    let b_after = scalar(b2.execute("SELECT DATABASE()").expect("select"));
    println!("connection B now reports SELECT DATABASE() -> {b_after}");

    if b_init == "d_b" && b_after != "d_b" {
        println!(
            "OBSERVED: B's database changed without B sending anything. \
             The current database is one shared value on the storage, so \
             A's `USE` moved it out from under B."
        );
    } else {
        println!(
            "not observed this run (b_init={b_init:?}, b_after=\
             {b_after:?}) — the per-command re-assert may have restored it"
        );
    }
}

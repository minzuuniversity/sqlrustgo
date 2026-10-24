//! #5462 / #5463 — wire-level regression: GROUP BY key resolution.
//!
//! # What the issues reported
//!
//! **#5463** — `GROUP BY` on a column that does not exist was not rejected by
//! the binder (only the SELECT list had that check). The invalid statement
//! reached the executor, where the row-width assertion panicked:
//!
//! ```text
//! thread 'mysql-server-worker-15' panicked at crates/executor/src/executor.rs:70:9:
//! row width 19 does not match the 1 declared columns
//! ```
//!
//! The panic killed the **server thread**, so after that EVERY statement on
//! that connection failed with
//! `read packet header: failed to fill whole buffer` — a one-statement
//! DoS against the whole server.
//!
//! **#5462** — `GROUP BY <select-list alias>` was executed as if the alias
//! were a string constant: every row produced the same group key, the whole
//! input collapsed into ONE group, and **no error was raised**.
//!
//! # What this file establishes
//!
//! Over the real wire protocol, on a real server:
//!
//! * the bad statement comes back as an **ERR packet** (not a panic, not a
//!   dropped connection);
//! * the **same connection** still works afterwards — this is the DoS check:
//!   the failure mode of #5463 was the *next* statement failing too;
//! * `GROUP BY <alias>` groups by the alias's **defining expression**, not by
//!   the alias text.
//!
//! The engine-level counterpart of these assertions lives in
//! `src/engine_group_by_binder_tests.rs`; this file is the wire-level half.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};

fn start() -> EphemeralHandle {
    start_ephemeral(EphemeralConfig {
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
    })
    .expect("ephemeral server starts")
}

fn connect(h: &EphemeralHandle) -> MySqlConnection {
    let addr = format!("127.0.0.1:{}", h.port).parse().expect("addr");
    MySqlConnection::connect(&addr, "root", "", "test").expect("connect")
}

/// The table from the issue reports: three `l_returnflag` values, two years.
fn seed(c: &mut MySqlConnection) {
    c.execute("CREATE DATABASE gb5462").expect("create db");
    c.execute("USE gb5462").expect("use");
    c.execute("CREATE TABLE lineitem (l_orderkey INT, l_returnflag TEXT, l_shipdate TEXT)")
        .expect("create");
    c.execute(
        "INSERT INTO lineitem VALUES \
         (1,'A','1995-01-01'),(2,'A','1995-06-01'),(3,'N','1996-01-01'),\
         (4,'N','1996-06-01'),(5,'R','1996-06-01')",
    )
    .expect("insert");
}

/// Result rows as `col,col` strings, sorted — the assertions must not depend
/// on the server's row order.
fn rows_of(c: &mut MySqlConnection, sql: &str) -> Vec<String> {
    match c.execute(sql) {
        Ok(ResultSet::Select { rows, .. }) => {
            let mut out: Vec<String> = rows.iter().map(|r| r.join(",")).collect();
            out.sort();
            out
        }
        Ok(other) => panic!("{sql}: expected a result set, got {other:?}"),
        Err(e) => panic!("{sql}: query failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// #5463 — the bad statement must be an ERR packet, and the server must live.
// ---------------------------------------------------------------------------

#[test]
fn group_by_unknown_column_returns_an_error_packet_and_the_server_survives() {
    let h = start();
    let mut c = connect(&h);
    seed(&mut c);

    // The statement from the issue.
    match c.execute("SELECT COUNT(*) FROM lineitem GROUP BY zzz_bogus_col") {
        Ok(ResultSet::Error {
            error_code,
            error_message,
            ..
        }) => {
            assert!(
                error_message.contains("Binder error: column 'zzz_bogus_col' not found in schema"),
                "the error must name the unresolved column, got {error_code}: {error_message}"
            );
        }
        Ok(other) => panic!("expected an ERR packet, got {other:?}"),
        Err(e) => panic!(
            "the server must answer with an ERR packet and keep the connection \
             open; a transport error here is exactly the #5463 DoS symptom: {e}"
        ),
    }

    // The DoS check: the NEXT statement on the SAME connection must still
    // work. Before the fix the server thread had panicked and every
    // statement on the connection failed with
    // "read packet header: failed to fill whole buffer".
    let after = rows_of(&mut c, "SELECT COUNT(*) FROM lineitem");
    assert_eq!(
        after,
        vec!["5"],
        "the server must survive a rejected GROUP BY and answer the next query"
    );
}

#[test]
fn group_by_unknown_column_does_not_take_down_other_connections() {
    let h = start();
    let mut bad_conn = connect(&h);
    let mut other_conn = connect(&h);
    seed(&mut bad_conn);
    // The seeded table lives in the `gb5462` database; the second connection
    // starts in `test`.
    other_conn.execute("USE gb5462").expect("use");

    assert!(matches!(
        bad_conn.execute("SELECT COUNT(*) FROM lineitem GROUP BY zzz_bogus_col"),
        Ok(ResultSet::Error { .. })
    ));

    // A second, independent connection must be unaffected.
    let rows = rows_of(&mut other_conn, "SELECT COUNT(*) FROM lineitem");
    assert_eq!(rows, vec!["5"], "other connections must be unaffected");
}

// ---------------------------------------------------------------------------
// #5462 — GROUP BY <alias> groups by the alias's defining expression.
// ---------------------------------------------------------------------------

#[test]
fn group_by_alias_of_a_column_groups_by_the_column() {
    let h = start();
    let mut c = connect(&h);
    seed(&mut c);

    let rows = rows_of(
        &mut c,
        "SELECT l_returnflag AS lf, COUNT(*) FROM lineitem GROUP BY lf",
    );
    assert_eq!(
        rows,
        vec!["A,2", "N,2", "R,1"],
        "GROUP BY <alias> must group by the underlying column's value, not by \
         the alias text (which would collapse everything into one group)"
    );
}

#[test]
fn group_by_alias_of_an_expression_groups_by_the_expression() {
    let h = start();
    let mut c = connect(&h);
    seed(&mut c);

    let rows = rows_of(
        &mut c,
        "SELECT EXTRACT(YEAR FROM l_shipdate) AS ly, COUNT(*) FROM lineitem GROUP BY ly",
    );
    assert_eq!(
        rows,
        vec!["1995,2", "1996,3"],
        "GROUP BY <alias of an expression> must group by the expression's value"
    );
}

#[test]
fn group_by_real_column_is_unchanged() {
    let h = start();
    let mut c = connect(&h);
    seed(&mut c);

    let rows = rows_of(
        &mut c,
        "SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag",
    );
    assert_eq!(
        rows,
        vec!["A,2", "N,2", "R,1"],
        "GROUP BY <real column> must keep grouping by the column's value"
    );
}

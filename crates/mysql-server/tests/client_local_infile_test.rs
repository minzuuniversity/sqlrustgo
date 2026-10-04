//! End-to-end LOCAL INFILE round trip: real server, real client.
//!
//! The unit tests in `crates/mysql-client/tests/local_infile_test.rs`
//! prove the client frames the 0xFB exchange correctly against a script.
//! These tests prove the *other* half: that a real
//! `sqlrustgo-mysql-server` accepts what the client sends, the rows land,
//! and the connection is still usable afterwards.
//!
//! Before the fix there was no client-side handler at all, so every one
//! of these aborted with "capacity overflow" inside the client before a
//! single byte was uploaded.

use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

/// Start a server whose LOAD DATA whitelist is `dir`, so the test can
/// place an input file there and know the whitelist check will pass.
fn start_server(dir: &std::path::Path) -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        data_dir: Some(dir.to_path_buf()),
        load_infile_dir: Some(dir.to_path_buf()),
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    };
    start_ephemeral(config).expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlResult<MySqlConnection> {
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "")
}

fn rows_of(rs: ResultSet) -> Vec<Vec<String>> {
    match rs {
        ResultSet::Select { rows, .. } => rows,
        ResultSet::Ok { .. } => panic!("expected a result set, got Ok"),
        ResultSet::Error {
            error_code,
            error_message,
            ..
        } => panic!("server error {}: {}", error_code, error_message),
    }
}

fn count(conn: &mut MySqlConnection, table: &str) -> u64 {
    let rs = conn
        .execute(&format!("SELECT COUNT(*) FROM {}", table))
        .expect("count query");
    let rows = rows_of(rs);
    assert_eq!(rows.len(), 1, "COUNT(*) must return exactly one row");
    rows[0][0].parse::<u64>().expect("count is numeric")
}

/// The baseline: `execute` reads the server-named path off local disk,
/// exactly like the `mysql` CLI, and the rows land.
#[test]
fn local_infile_uploads_from_disk_and_rows_land() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("orders.tbl");
    std::fs::write(&file, b"1|alpha\n2|beta\n3|gamma\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        file.to_str().unwrap()
    );
    let rs = conn.execute(&sql).expect("load data");

    match rs {
        ResultSet::Ok {
            affected_rows,
            warnings,
            ..
        } => {
            assert_eq!(affected_rows, 3, "all 3 rows must be reported loaded");
            assert_eq!(warnings, 0, "a clean file must report no warnings");
        }
        other => panic!("expected Ok, got {:?}", other),
    }
    assert_eq!(
        count(&mut conn, "t"),
        3,
        "rows must actually be in the table"
    );
}

/// The connection must survive the extra round trip. After the client
/// sends content + terminator packets, the server's OK continues a
/// sequence counter the client also has to advance; if that bookkeeping
/// is wrong, the next command desyncs and hangs or misparses.
#[test]
fn connection_is_usable_after_the_upload() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = dir.path().join("first.tbl");
    std::fs::write(&first, b"1|alpha\n2|beta\n").expect("write input");
    let second = dir.path().join("second.tbl");
    std::fs::write(&second, b"3|gamma\n4|delta\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    // Distinct ids: the first column is the row key, so re-loading the
    // same file would overwrite rather than append.
    let load = |conn: &mut MySqlConnection, file: &std::path::Path| {
        let sql = format!(
            "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
            file.to_str().unwrap()
        );
        conn.execute(&sql).expect("load data")
    };

    load(&mut conn, &first);
    assert_eq!(count(&mut conn, "t"), 2, "first load must land");

    load(&mut conn, &second);
    assert_eq!(
        count(&mut conn, "t"),
        4,
        "second load on the same connection must land"
    );

    load(&mut conn, &first);
    assert_eq!(
        count(&mut conn, "t"),
        4,
        "re-loading the same ids overwrites (first column is the row key), \
         so the count must stay flat rather than grow"
    );

    conn.execute("INSERT INTO t VALUES (99, 'after')")
        .expect("insert");
    let rows = rows_of(
        conn.execute("SELECT name FROM t WHERE id = 99")
            .expect("select"),
    );
    assert_eq!(rows, vec![vec!["after".to_string()]]);
}

/// #4941's server-side accounting, observed end to end: a file with
/// unparseable rows reports a warning count instead of a clean success.
#[test]
fn skipped_rows_surface_as_warnings_through_the_client() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("mixed.tbl");
    // Table has 2 columns, so "MALFORMED" (1 field) cannot be parsed.
    std::fs::write(&file, b"1|alpha\nMALFORMED\n2|beta\n3\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        file.to_str().unwrap()
    );
    let rs = conn.execute(&sql).expect("load data");

    match rs {
        ResultSet::Ok {
            affected_rows,
            warnings,
            ..
        } => {
            assert_eq!(affected_rows, 2, "only the 2 well-formed rows load");
            assert_eq!(
                warnings, 2,
                "both dropped rows must be reported as warnings, not hidden"
            );
        }
        other => panic!("expected Ok, got {:?}", other),
    }
    assert_eq!(
        count(&mut conn, "t"),
        2,
        "the table must hold what was loaded"
    );
}

/// `load_data_local` sends caller-supplied bytes even though the server
/// names a path. If the client secretly re-read the named file, the row
/// counts below would be the file's, not the buffer's — so this pins
/// which bytes actually crossed the wire.
#[test]
fn load_data_local_streams_supplied_bytes_not_the_named_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("on_disk.tbl");
    // The file on disk has 4 rows. We will claim 3 different ones.
    std::fs::write(&file, b"1|a\n2|b\n3|c\n4|d\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        file.to_str().unwrap()
    );
    let rs = conn
        .load_data_local(&sql, b"10|x\n20|y\n30|z\n")
        .expect("load data");

    match rs {
        ResultSet::Ok { affected_rows, .. } => assert_eq!(affected_rows, 3),
        other => panic!("expected Ok, got {:?}", other),
    }
    assert_eq!(count(&mut conn, "t"), 3);

    let rows = rows_of(
        conn.execute("SELECT name FROM t ORDER BY id")
            .expect("select"),
    );
    let names: Vec<String> = rows.into_iter().map(|r| r[0].clone()).collect();
    assert_eq!(
        names,
        vec!["x".to_string(), "y".to_string(), "z".to_string()],
        "the buffered bytes must be what landed, not the on-disk rows"
    );
}

/// A path outside the whitelist is refused by the server. The client must
/// surface that as a `ResultSet::Error` and must NOT have silently
/// succeeded — and the table must stay empty.
///
/// The tail of this test is the load-bearing part: the rejection used to
/// be followed by a second OK packet, which left the client's stream one
/// packet ahead of the server's. Every later command on the connection
/// then read the *previous* response, and the client eventually rendered
/// a "row" whose value was the literal bytes of the next query.
#[test]
fn path_outside_the_whitelist_is_rejected_by_the_server() {
    let allowed = tempfile::tempdir().expect("allowed dir");
    let outside = tempfile::tempdir().expect("outside dir");
    let file = outside.path().join("secret.tbl");
    std::fs::write(&file, b"1|leak\n").expect("write input");
    let inside = allowed.path().join("fine.tbl");
    std::fs::write(&inside, b"2|fine\n").expect("write input");

    let handle = start_server(allowed.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    let bad_sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        file.to_str().unwrap()
    );
    let good_sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        inside.to_str().unwrap()
    );

    let rs = conn
        .execute(&bad_sql)
        .expect("load data must return a result, not a transport error");
    match rs {
        ResultSet::Error {
            error_code,
            error_message,
            ..
        } => {
            assert_eq!(error_code, 1146, "ER_TABLEACCESS_DENIED_ERROR");
            assert!(
                error_message.contains("not in allowed data_dir"),
                "error must explain the whitelist rejection, got: {}",
                error_message
            );
        }
        other => panic!("server must reject the path, got {:?}", other),
    }

    // Exactly one response packet was sent, so the next command's reply
    // must be the next thing on the wire.
    assert_eq!(count(&mut conn, "t"), 0, "nothing may be loaded");

    // A well-formed load right after the rejection must still work.
    let rs = conn.execute(&good_sql).expect("load after rejection");
    match rs {
        ResultSet::Ok { affected_rows, .. } => assert_eq!(affected_rows, 1),
        other => panic!("expected Ok, got {:?}", other),
    }

    let rows = rows_of(conn.execute("SELECT name FROM t").expect("select"));
    assert_eq!(
        rows,
        vec![vec!["fine".to_string()]],
        "only the whitelisted file's row may be present"
    );
}

/// A path that does not exist is rejected by the **server**, before it
/// ever sends a 0xFB request. The whitelist + `canonicalize` check in
/// `handle_load_local_infile` runs first, so the client is never asked
/// for bytes it cannot read. This pins that ordering: a client-side read
/// error is not reachable through a missing file, and the caller must
/// see a server error rather than a hang or a silent success.
#[test]
fn missing_local_file_is_rejected_by_the_server_before_the_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("does_not_exist.tbl");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        missing.to_str().unwrap()
    );
    let rs = conn
        .execute(&sql)
        .expect("must return a result, not a transport error");

    match rs {
        ResultSet::Error { error_message, .. } => assert!(
            error_message.contains("file not found"),
            "error must say the file is missing, got: {}",
            error_message
        ),
        other => panic!("expected Error, got {:?}", other),
    }
    assert_eq!(count(&mut conn, "t"), 0, "nothing may be loaded");
    // And the connection must still be usable — this is the same path
    // that used to leave a stale OK in the stream.
    conn.execute("INSERT INTO t VALUES (5, 'still works')")
        .expect("insert");
    assert_eq!(count(&mut conn, "t"), 1);
}

/// A payload larger than one wire packet must survive the split/reassemble
/// round trip. Uses a modest file (well under 16 MB) but enough rows to
/// span the server's own internal buffering boundaries.
#[test]
fn multi_chunk_payload_loads_every_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("big.tbl");
    let mut body = String::new();
    for i in 0..20_000 {
        body.push_str(&format!("{}|row{}\n", i, i));
    }
    std::fs::write(&file, body.as_bytes()).expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        file.to_str().unwrap()
    );
    let rs = conn.execute(&sql).expect("load data");
    match rs {
        ResultSet::Ok {
            affected_rows,
            warnings,
            ..
        } => {
            assert_eq!(affected_rows, 20_000, "every row must be loaded");
            assert_eq!(warnings, 0, "no rows may be dropped");
        }
        other => panic!("expected Ok, got {:?}", other),
    }
    assert_eq!(count(&mut conn, "t"), 20_000);
}

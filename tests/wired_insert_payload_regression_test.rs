//! Regression tests for wired-SOAK Bug #1 + #2 (Issue #3265 / wired-soak-4-bugs.md).
//!
//! Bug #1: COM_QUERY payload truncation — `mysql` CLI sends
//!   `INSERT INTO region VALUES (1, 'Africa')` as a COM_QUERY, but
//!   the server only saw the truncated prefix `INSERT INTO region VALUES`
//!   causing "Expected `(` after VALUES".
//!
//! Bug #2: sysbench prepare bulk INSERT drops the connection with
//!   "Lost connection to MySQL server during query" (error 2013).
//!   Same root cause: large multi-row INSERT payloads not handled
//!   correctly end-to-end.
//!
//! Both root causes are addressed by the protocol fixes merged in
//! PR #3652 (3 protocol bugs) and Engine Bug A/B fixes in PR #3637 / #3638.
//! These tests verify the regressions do not re-appear.
//!
//! Engine Bug A: `split_top_level_statements()` (PR #3638) — multi-stmt
//!   batches execute each statement independently.
//! Engine Bug B: TLS drain loop (PR #3637) — multi-record TLS payloads
//!   of ~16 KB+ are read completely.
//! PR #3652: charset 0x21, column def fill byte, sequence reset on every
//!   new command.
//!
//! Each test uses a real `sqlrustgo-mysql-server` ephemeral handle and
//! connects via the MySQL wire protocol.

mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

/// Bug #1: A single COM_QUERY INSERT with a VALUES clause that has
/// non-trivial content (string literal, integer) must round-trip the
/// payload without truncation. Originally the server dropped everything
/// after the keyword `VALUES`, surfacing as:
///     ERROR 1064 (42000): Expected `(` after VALUES
#[test]
fn regression_insert_single_row_values_payload_not_truncated() {
    let mut client = start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
        .expect("ephemeral handle + client connect");

    let _ = client.exec("DROP TABLE IF EXISTS region");
    let r = client.exec("CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT)");
    assert!(r.is_ok(), "CREATE TABLE: {:?}", r.err());

    // The exact INSERT from the bug report.
    let r = client.exec("INSERT INTO region VALUES (1, 'Africa')");
    assert!(r.is_ok(), "INSERT single row: {:?}", r.err());

    let rows = client
        .query_rows("SELECT r_regionkey, r_name FROM region")
        .expect("SELECT region");
    assert_eq!(rows.len(), 1, "row count mismatch");
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[0][1], "Africa");

    let _ = client.exec("DROP TABLE region");
}

/// Bug #1 extended: 5 single-row INSERTs with different value patterns
/// (int, string, NULL) — verifies the parser sees the full payload each
/// time, not just the prefix.
#[test]
fn regression_insert_single_row_various_types() {
    let mut client = start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
        .expect("ephemeral handle + client connect");

    let _ = client.exec("DROP TABLE IF EXISTS mix");
    client
        .exec("CREATE TABLE mix (id INTEGER PRIMARY KEY, label TEXT, score INTEGER)")
        .expect("CREATE TABLE mix");

    let inserts = [
        "INSERT INTO mix VALUES (1, 'alpha', 100)",
        "INSERT INTO mix VALUES (2, 'beta', NULL)",
        "INSERT INTO mix VALUES (3, '', 0)",
        "INSERT INTO mix VALUES (4, 'gamma delta', 42)",
        "INSERT INTO mix VALUES (5, 'with ''quote'' inside', 7)",
    ];
    for sql in inserts {
        let r = client.exec(sql);
        assert!(r.is_ok(), "INSERT failed for `{}`: {:?}", sql, r.err());
    }

    let count = client
        .query_rows("SELECT COUNT(*) FROM mix")
        .expect("count");
    assert_eq!(count[0][0], "5", "all 5 INSERTs must land");
    let _ = client.exec("DROP TABLE mix");
}

/// Bug #2: Large multi-row INSERT (the sysbench prepare pattern) must
/// not drop the connection. Reproduced by building a 200-row INSERT
/// string and posting it as a single COM_QUERY. The original bug
/// surfaced as "Lost connection to MySQL server during query" mid-statement.
#[test]
fn regression_insert_multi_row_200_rows_no_lost_connection() {
    let mut client = start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
        .expect("ephemeral handle + client connect");

    let _ = client.exec("DROP TABLE IF EXISTS big");
    client
        .exec("CREATE TABLE big (id INTEGER PRIMARY KEY, payload TEXT)")
        .expect("CREATE TABLE big");

    // Build a single INSERT with 200 rows (≈ 5–10 KB, well past any
    // single-packet limit boundary).
    let n: i64 = 200;
    let mut sql = String::with_capacity(64 * n as usize);
    sql.push_str("INSERT INTO big VALUES ");
    for i in 0..n {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("({}, 'row_{}_padded_text')", i, i));
    }

    let r = client.exec(&sql);
    assert!(r.is_ok(), "200-row INSERT: {:?}", r.err());

    let count = client
        .query_rows("SELECT COUNT(*) FROM big")
        .expect("count");
    assert_eq!(count[0][0].parse::<i64>().unwrap(), n);

    // Connection must still be alive after the big INSERT.
    // Use query_rows since `SELECT 1` returns a result set (1 row × 1 col).
    let rows = client
        .query_rows("SELECT 1")
        .expect("SELECT 1 after bulk INSERT");
    assert_eq!(rows.len(), 1, "SELECT 1 must return 1 row");

    let _ = client.exec("DROP TABLE big");
}

/// Bug #2 extended: 1000-row single INSERT (≈ 25 KB), simulating the
/// sysbench `--table-size=1000` prepare step. Verifies that the wire
/// layer (TLS drain, multi-packet framing) and the executor handle the
/// payload end-to-end without disconnecting.
#[test]
fn regression_insert_multi_row_1000_rows_25kb() {
    let mut client = start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
        .expect("ephemeral handle + client connect");

    let _ = client.exec("DROP TABLE IF EXISTS big1k");
    client
        .exec("CREATE TABLE big1k (id INTEGER PRIMARY KEY, payload TEXT)")
        .expect("CREATE TABLE big1k");

    let n: i64 = 1000;
    // Pad each row's payload string so total SQL exceeds the 16KB
    // TLS record boundary (`({}, 'v{}_<24 chars>')` ≈ 38 B/row × 1000 ≈ 38 KB).
    let mut sql = String::with_capacity(48 * n as usize);
    sql.push_str("INSERT INTO big1k VALUES ");
    for i in 0..n {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("({}, 'v{}_payload_padding_text_xx')", i, i));
    }
    let payload_bytes = sql.len();
    assert!(
        payload_bytes > 16 * 1024,
        "payload must exceed 16KB TLS record boundary (was {} bytes)",
        payload_bytes
    );

    let r = client.exec(&sql);
    assert!(
        r.is_ok(),
        "1000-row INSERT ({} bytes): {:?}",
        payload_bytes,
        r.err()
    );

    let count = client
        .query_rows("SELECT COUNT(*) FROM big1k")
        .expect("count");
    assert_eq!(count[0][0].parse::<i64>().unwrap(), n);
    let _ = client.exec("DROP TABLE big1k");
}

/// Composite smoke: exercise the exact scenarios from the bug report
/// back-to-back in one connection. This is the closest reproduction
/// of the wired-SOAK failure mode (sysbench prepare → "Lost connection").
#[test]
fn regression_sysbench_prepare_pattern_composite() {
    let mut client = start_ephemeral(EphemeralConfig::default())
        .ok()
        .and_then(|h| MySqlTestClient::connect_handle(h).ok())
        .expect("ephemeral handle + client connect");

    let _ = client.exec("DROP TABLE IF EXISTS sbtest");
    client
        .exec("CREATE TABLE sbtest (id INTEGER PRIMARY KEY, k INTEGER, c TEXT, pad TEXT)")
        .expect("CREATE TABLE sbtest");

    // sysbench prepare inserts 1 row at a time, but each is a multi-column
    // INSERT that goes over the wire protocol. Loop 500 times to stress.
    for i in 0..500i64 {
        let sql = format!(
            "INSERT INTO sbtest VALUES ({}, {}, 'comment text {}', 'pad pad pad pad {}')",
            i, i, i, i
        );
        let r = client.exec(&sql);
        assert!(
            r.is_ok(),
            "sysbench-pattern INSERT at row {}: {:?}",
            i,
            r.err()
        );
    }

    let count = client
        .query_rows("SELECT COUNT(*) FROM sbtest")
        .expect("count");
    assert_eq!(count[0][0].parse::<i64>().unwrap(), 500);
    let _ = client.exec("DROP TABLE sbtest");
}

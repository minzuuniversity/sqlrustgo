//! LIMIT clause tests (TDD demonstration) — driven through the
//! MySQL wire protocol via the embedded `start_ephemeral`
//! harness.
//!
//! **Issue**: `src/execution_engine.rs` did not handle LIMIT in
//! SELECT statements (fixed before this migration).
//!
//! **Phase 2b migration**: rewritten on top of `MySqlTestClient`
//! to drive SQL through the canonical entry point.

mod common;

use common::MySqlTestClient;

fn setup() -> MySqlTestClient {
    let mut client =
        MySqlTestClient::connect_default().expect("ephemeral server + raw client should come up");
    client
        .exec("CREATE TABLE t1 (id INTEGER)")
        .expect("CREATE TABLE");
    for i in 1..=10 {
        client
            .exec(&format!("INSERT INTO t1 VALUES ({i})"))
            .expect("INSERT");
    }
    client
}

#[test]
fn test_select_limit() {
    let mut client = setup();
    let rows = client
        .query_rows("SELECT * FROM t1 LIMIT 3")
        .expect("SELECT with LIMIT should succeed");
    assert_eq!(rows.len(), 3, "LIMIT 3 should return exactly 3 rows");
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[1][0], "2");
    assert_eq!(rows[2][0], "3");
}

#[test]
fn test_select_limit_offset() {
    let mut client = setup();
    let rows = client
        .query_rows("SELECT * FROM t1 LIMIT 3 OFFSET 5")
        .expect("SELECT with LIMIT OFFSET should succeed");
    assert_eq!(rows.len(), 3, "LIMIT 3 OFFSET 5 should return 3 rows");
    assert_eq!(rows[0][0], "6");
    assert_eq!(rows[1][0], "7");
    assert_eq!(rows[2][0], "8");
}

#[test]
fn test_select_limit_zero() {
    let mut client =
        MySqlTestClient::connect_default().expect("ephemeral server + raw client should come up");
    client.exec("CREATE TABLE t1 (id INTEGER)").unwrap();
    client.exec("INSERT INTO t1 VALUES (1)").unwrap();
    client.exec("INSERT INTO t1 VALUES (2)").unwrap();

    let rows = client
        .query_rows("SELECT * FROM t1 LIMIT 0")
        .expect("LIMIT 0 should succeed");
    assert_eq!(rows.len(), 0);
}

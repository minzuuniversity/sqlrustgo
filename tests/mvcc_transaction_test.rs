//! MVCC transaction tests — driven through the MySQL wire
//! protocol via the embedded `start_ephemeral` harness.
//!
//! **Phase 2b migration**: rewritten on top of `MySqlTestClient`
//! to drive SQL through the canonical entry point.

mod common;

use common::MySqlTestClient;

fn setup() -> MySqlTestClient {
    let mut client =
        MySqlTestClient::connect_default().expect("ephemeral server + raw client should come up");
    client
        .exec("CREATE TABLE t (id INTEGER, value INTEGER)")
        .expect("CREATE TABLE");
    client
        .exec("INSERT INTO t VALUES (1, 100)")
        .expect("INSERT");
    client
}

#[test]
fn test_begin_commit_transaction() {
    let mut client = setup();

    client.exec("BEGIN").expect("BEGIN should succeed");
    client
        .exec("UPDATE t SET value = 200 WHERE id = 1")
        .expect("UPDATE");
    client.exec("COMMIT").expect("COMMIT should succeed");

    let rows = client
        .query_rows("SELECT id, value FROM t WHERE id = 1")
        .expect("SELECT");
    assert_eq!(rows[0][1], "200");
}

#[test]
fn test_begin_rollback_transaction() {
    let mut client = setup();

    client.exec("BEGIN").expect("BEGIN should succeed");
    client
        .exec("UPDATE t SET value = 999 WHERE id = 1")
        .expect("UPDATE");
    client.exec("ROLLBACK").expect("ROLLBACK should succeed");
}

#[test]
fn test_begin_serializable() {
    let mut client = setup();

    client
        .exec("BEGIN SERIALIZABLE")
        .expect("BEGIN SERIALIZABLE should succeed");

    let rows = client
        .query_rows("SELECT id, value FROM t WHERE id = 1")
        .expect("SELECT");
    assert_eq!(rows[0][1], "100");

    client.exec("COMMIT").expect("COMMIT");
}

#[test]
fn test_set_transaction_isolation() {
    let mut client = setup();

    client
        .exec("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .expect("SET TRANSACTION ISOLATION LEVEL should succeed");
    client
        .exec("BEGIN")
        .expect("BEGIN should succeed after SET");
    client.exec("COMMIT").expect("COMMIT");
}

#[test]
fn test_start_transaction() {
    let mut client = setup();

    client
        .exec("START TRANSACTION")
        .expect("START TRANSACTION should succeed");
    client
        .exec("UPDATE t SET value = 300 WHERE id = 1")
        .expect("UPDATE");
    client.exec("COMMIT").expect("COMMIT should succeed");

    let rows = client
        .query_rows("SELECT id, value FROM t WHERE id = 1")
        .expect("SELECT");
    assert_eq!(rows[0][1], "300");
}

#[test]
fn test_start_transaction_serializable() {
    let mut client = setup();

    client
        .exec("START TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .expect("START TRANSACTION ISOLATION LEVEL SERIALIZABLE should succeed");

    let rows = client
        .query_rows("SELECT id, value FROM t WHERE id = 1")
        .expect("SELECT");
    assert_eq!(rows[0][1], "100");

    client.exec("COMMIT").expect("COMMIT");
}

//! RED→GREEN smoke test for the raw MySQL wire-protocol client.
//!
//! Validates the canonical wire-protocol test surface from
//! `openspec/changes/mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md`
//! by performing a full CREATE → INSERT → SELECT round trip through
//! the embedded server.

mod common;

use common::MySqlTestClient;

#[test]
fn test_wire_protocol_create_insert_select_round_trip() {
    let mut client = MySqlTestClient::connect_default()
        .expect("ephemeral server + raw-protocol client should come up");

    client
        .exec("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .expect("CREATE TABLE should succeed");
    client
        .exec("INSERT INTO t VALUES (1, 'hello')")
        .expect("first INSERT should succeed");
    client
        .exec("INSERT INTO t VALUES (2, 'world')")
        .expect("second INSERT should succeed");

    let count = client
        .query_one_i64("SELECT COUNT(*) FROM t")
        .expect("SELECT COUNT(*) should succeed");
    assert_eq!(count, 2, "two rows should be visible to the wire client");

    let rows = client
        .query_rows("SELECT id, v FROM t ORDER BY id")
        .expect("SELECT should succeed");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[0][1], "hello");
    assert_eq!(rows[1][0], "2");
    assert_eq!(rows[1][1], "world");

    client.quit().ok();
}

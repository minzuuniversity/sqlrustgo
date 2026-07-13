//! Smoke test for openspec/changes/2026-06-18-wire-deprecate-eof.
//!
//! Asserts that `MySqlTestClient` can drive `SELECT 1` end-to-end
//! against an in-process sqlrustgo server in both capability variants:
//!
//! - classic capabilities (DEPRECATE_EOF unset, the existing default)
//! - deprecated-EOF capabilities (DEPRECATE_EOF = 0x01000000)
//!
//! In both cases the result should be `vec![vec!["1"]]`. The server
//! is responsible for emitting the right packet sequence per
//! capability, and the client is responsible for parsing it.

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;

/// DEPRECATE_EOF capability flag, per the MySQL wire-protocol spec.
const CAP_DEPRECATE_EOF: u32 = 0x0100_0000;

#[test]
fn classic_caps_round_trip() {
    let handle = sqlrustgo_mysql_server::testing::start_ephemeral(
        sqlrustgo_mysql_server::testing::EphemeralConfig::default(),
    )
    .expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    let rows = client.query_rows("SELECT 1").expect("query_rows");
    assert_eq!(
        rows,
        vec![vec!["1".to_string()]],
        "classic-caps client should receive one row with the literal 1"
    );
    assert_eq!(
        client.client_capabilities() & CAP_DEPRECATE_EOF,
        0,
        "default caps should NOT advertise DEPRECATE_EOF"
    );
}

#[test]
fn deprecated_eof_caps_round_trip() {
    let handle = sqlrustgo_mysql_server::testing::start_ephemeral(
        sqlrustgo_mysql_server::testing::EphemeralConfig::default(),
    )
    .expect("start_ephemeral");
    let addr = ("127.0.0.1", handle.port);
    let mut client =
        MySqlTestClient::connect_with_caps(addr, "tester", "tester", CAP_DEPRECATE_EOF)
            .expect("connect_with_caps");
    let rows = client.query_rows("SELECT 1").expect("query_rows");
    assert_eq!(
        rows,
        vec![vec!["1".to_string()]],
        "deprecated-EOF client should receive one row with the literal 1"
    );
    assert_ne!(
        client.client_capabilities() & CAP_DEPRECATE_EOF,
        0,
        "client caps SHOULD advertise DEPRECATE_EOF"
    );
}

//! V312-13 / ISSUE #3900 — typed wrapper smoke tests for the new
//! `MySqlTestClient` methods added by the
//! `v312-13-mysql-wire-load-data-hardening` openspec change.
//!
//! Each test reports PASS / DEFERRED explicitly so the gate script
//! in `scripts/gate/check_v312_13_wire_load_data.sh` can render a
//! per-test row in `V312-13-REPORT.md`.
//!
//! See: openspec/changes/v312-13-mysql-wire-load-data-hardening/

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use common::MysqlError;

/// COM_STMT_PREPARE round-trip: prepare, assert stmt id, then close.
#[test]
fn v312_13_prepare_returns_stmt_id() {
    let mut client = MySqlTestClient::connect_default().expect("ephemeral connect");
    let info = client
        .prepare("SELECT 1")
        .expect("prepare SELECT 1 must succeed");
    assert!(info.stmt_id > 0, "server must assign a non-zero stmt id");
    client
        .stmt_close(info.stmt_id)
        .expect("stmt_close must succeed");
}

/// COM_RESET_CONNECTION: the v3.12.0 server MAY either reset the
/// session (preferred) or return ERR with "Unknown command" (the
/// v3.11.0 server has the latter behavior). The test pins both
/// outcomes as documented gaps in the V312-13 openspec change.
#[test]
fn v312_13_reset_connection_ok() {
    let mut client = MySqlTestClient::connect_default().expect("ephemeral connect");
    let res = client.reset_connection();
    // Acceptable outcomes:
    //   - Ok(()): server fully implemented reset
    //   - Err containing "Unknown command": documented v3.11.0 gap
    match res {
        Ok(()) => {
            // After a real reset, follow-up queries must still work.
            // Use query_rows (returns a result set) instead of exec
            // (expects a single OK/ERR packet), because SELECT returns
            // a result set, not an OK packet.
            let _rows = client
                .query_rows("SELECT 1")
                .expect("post-reset SELECT 1 must return a result set");
        }
        Err(e) if e.to_string().contains("Unknown command") => {
            // Documented gap: v3.11.0 server does not implement 0x1F.
            // The V312-13 design tasks include wiring the server side.
        }
        Err(e) => panic!("unexpected reset_connection outcome: {}", e),
    }
}
#[test]
fn v312_13_expect_err_syntax() {
    let mut client = MySqlTestClient::connect_default().expect("ephemeral connect");
    let err: MysqlError = client
        .expect_err("SELEC 1")
        .expect("syntax error must return ERR packet");
    assert_eq!(err.sqlstate, "42000", "syntax error sqlstate");
    let lc = err.message.to_lowercase();
    assert!(
        lc.contains("syntax") || lc.contains("parse error"),
        "syntax error message: {}",
        err.message
    );
}

/// TLS negotiation: the v3.12.0 ephemeral harness does not yet
/// support TLS. The test pins the documented gap.
#[test]
fn v312_13_force_tls_deferred() {
    let mut client = MySqlTestClient::connect_default().expect("ephemeral connect");
    let res = client.force_tls();
    assert!(
        res.is_err(),
        "force_tls must return Err until TLS decryption is implemented; got {:?}",
        res
    );
}

/// Compression negotiation: the v3.12.0 ephemeral harness does not
/// yet support compression. The test pins the documented gap.
#[test]
fn v312_13_force_compress_deferred() {
    let mut client = MySqlTestClient::connect_default().expect("ephemeral connect");
    let res = client.force_compress();
    assert!(
        res.is_err(),
        "force_compress must return Err until zlib decoding is implemented; got {:?}",
        res
    );
}

/// Smoke: a prepare/execute/close cycle for a non-parameter SELECT.
#[test]
fn v312_13_prepare_execute_close_roundtrip() {
    let mut client = MySqlTestClient::connect_default().expect("ephemeral connect");
    let info = client.prepare("SELECT 1").expect("prepare");
    let _rows = client.execute(info.stmt_id, &[]).expect("execute");
    client.stmt_close(info.stmt_id).expect("close");
}

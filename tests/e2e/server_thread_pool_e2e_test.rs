//! ServerThreadPool end-to-end behavior tests
//!
//! Verifies:
//! 1. server_threads=16 (default) — MySQL handshake + simple SELECT succeeds
//! 2. server_threads=0 (legacy unbounded) — doesn't regress
//! 3. server_threads=1 (single worker) — doesn't regress
//!
//! Note: multi-client concurrency tests are limited by EphemeralHandle not
//! being Clone; single-connection handshake tests are sufficient to validate
//! the accept loop + worker pool code paths. Real concurrency stress is in
//! scripts/stability/run_wired_soak.sh (Task 11/12).
//!
//! Refs: docs/superpowers/specs/2026-06-26-soak-1h-server-threads-design.md §6.3

#[path = "../common/mod.rs"]
mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn make_cfg(server_threads: usize) -> EphemeralConfig {
    let tmp = tempfile::TempDir::new().expect("TempDir");
    EphemeralConfig {
        data_dir: Some(tmp.path().to_path_buf()),
        bootstrap_tables: false,
        bootstrap_users: true,
        server_threads,
        ..Default::default()
    }
}

#[test]
fn e2e_server_threads_16_handshake_succeeds() {
    let handle = start_ephemeral(make_cfg(16)).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    let result = client.query_rows("SELECT 1");
    assert!(
        result.is_ok(),
        "SELECT 1 with server_threads=16 should succeed; got {:?}",
        result
    );
}

#[test]
fn e2e_server_threads_0_legacy_handshake_succeeds() {
    let handle = start_ephemeral(make_cfg(0)).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    let result = client.query_rows("SELECT 1");
    assert!(
        result.is_ok(),
        "SELECT 1 with server_threads=0 should succeed; got {:?}",
        result
    );
}

#[test]
fn e2e_server_threads_1_single_worker_handshake_succeeds() {
    let handle = start_ephemeral(make_cfg(1)).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    let result = client.query_rows("SELECT 1");
    assert!(
        result.is_ok(),
        "SELECT 1 with server_threads=1 should succeed; got {:?}",
        result
    );
}

#[test]
fn e2e_server_threads_2_handshake_succeeds() {
    let handle = start_ephemeral(make_cfg(2)).expect("start_ephemeral");
    let mut client = MySqlTestClient::connect_handle(handle).expect("connect");
    let result = client.query_rows("SELECT 1");
    assert!(
        result.is_ok(),
        "SELECT 1 with server_threads=2 should succeed; got {:?}",
        result
    );
}

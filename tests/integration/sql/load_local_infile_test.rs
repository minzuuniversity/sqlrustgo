//! LOAD DATA LOCAL INFILE — wire-protocol integration tests.
//!
//! See docs/superpowers/specs/2026-06-04-load-data-local-infile-design.md.
//!
//! V312-32: these 5 tests still share a single ephemeral server (see
//! [`SHARED`]) for convenience, but the LOAD DATA handler no longer
//! reads a process-global `ACTIVE_CONFIG`. Each connection sees its
//! server's own `data_dir` via a per-handle `Arc<EphemeralConfig>`
//! threaded from `start_ephemeral` through `handle_connection` to
//! `do_command_loop`. The shared-server pattern is preserved for
//! startup-cost reasons; tests that need independent data_dirs can
//! use their own `start_ephemeral` calls safely. Each loading test
//! uses a unique table name so they remain safe to run in parallel.

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tempfile::TempDir;

const TPC_H_REGION_SCHEMA: &str = "CREATE TABLE region ( \
    r_regionkey INTEGER, \
    r_name TEXT, \
    r_comment TEXT)";

const TPC_H_NATION_SCHEMA: &str = "CREATE TABLE nation ( \
    n_nationkey INTEGER, \
    n_name TEXT, \
    n_regionkey INTEGER, \
    n_comment TEXT)";

struct SharedServer {
    _handle: EphemeralHandle,
    data_dir: TempDir,
    port: u16,
}

static SHARED: OnceLock<SharedServer> = OnceLock::new();

/// Initialize the shared server on first call. V312-32: the per-handle
/// `Arc<EphemeralConfig>` makes this purely a startup-cost optimization
/// rather than a correctness requirement — each connection now sees
/// the right server's `data_dir` regardless of how many `start_ephemeral`
/// calls precede it.
fn shared() -> &'static SharedServer {
    SHARED.get_or_init(|| {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let config = EphemeralConfig {
            data_dir: Some(tmp.path().to_path_buf()),
            bootstrap_tables: false,
            bootstrap_users: true,                metrics_port: None,

            ..Default::default()
        };
        let handle = start_ephemeral(config).expect("start_ephemeral");
        let port = handle.port;
        SharedServer {
            _handle: handle,
            data_dir: tmp,
            port,
        }
    })
}

/// Open a fresh client connection to the shared server. The returned
/// client owns a no-op ("detached") handle, so dropping it does not
/// tear down the shared server.
fn connect_shared() -> MySqlTestClient {
    let port = shared().port;
    MySqlTestClient::connect_at(("127.0.0.1", port), "tester", "tester").expect("connect")
}

fn data_dir() -> &'static Path {
    shared().data_dir.path()
}

#[test]
fn test_load_local_infile_basic() {
    let mut client = connect_shared();
    // Unique table name so this test is independent of any other
    // test that may load into `region`.
    client
        .exec("CREATE TABLE region_basic (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)")
        .expect("create region_basic");
    let tbl: PathBuf = data_dir().join("region_basic.tbl");
    let mut f = fs::File::create(&tbl).unwrap();
    writeln!(f, "0|AFRICA|lar deposits.|").unwrap();
    writeln!(f, "1|AMERICA|hs use ironic.|").unwrap();
    drop(f);

    let n = client
        .load_local_infile(&tbl, "region_basic")
        .expect("load_local_infile");
    assert_eq!(n, 2);

    let count = client
        .query_one_i64("SELECT COUNT(*) FROM region_basic")
        .expect("count");
    assert_eq!(count, 2);
}

#[test]
fn test_load_local_infile_full_tpch_sf01_region() {
    let mut client = connect_shared();
    // Unique table name keeps this test independent of the other
    // region-loading test.
    client
        .exec("CREATE TABLE region_sf01 (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)")
        .expect("create region_sf01");
    let tbl: PathBuf = data_dir().join("region_sf01.tbl");
    let mut f = fs::File::create(&tbl).unwrap();
    for (k, name) in ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"]
        .iter()
        .enumerate()
    {
        writeln!(f, "{}|{}|comment for {}|", k, name, name).unwrap();
    }
    drop(f);

    let n = client
        .load_local_infile(&tbl, "region_sf01")
        .expect("load_local_infile");
    assert_eq!(n, 5);
}

#[test]
fn test_load_local_infile_full_tpch_sf01_nation() {
    let mut client = connect_shared();
    client.exec(TPC_H_NATION_SCHEMA).expect("create nation");
    let tbl: PathBuf = data_dir().join("nation.tbl");
    let mut f = fs::File::create(&tbl).unwrap();
    for k in 0..25 {
        writeln!(f, "{}|NATION_{}|{}|comment {}|", k, k, k % 5, k).unwrap();
    }
    drop(f);

    let n = client
        .load_local_infile(&tbl, "nation")
        .expect("load_local_infile");
    assert_eq!(n, 25);
}

#[test]
fn test_load_local_infile_path_outside_data_dir() {
    // Create a file OUTSIDE the data_dir. The server's whitelist
    // check should reject the LOAD DATA with "not in allowed data_dir".
    let outside = tempfile::tempdir().unwrap();
    let bad = outside.path().join("secret.tbl");
    fs::write(&bad, "0|AFRICA|secret|\n").unwrap();

    let mut client = connect_shared();
    client.exec(TPC_H_REGION_SCHEMA).expect("create region");

    let result = client.load_local_infile(&bad, "region");
    assert!(result.is_err(), "expected error for outside-data-dir path");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not in allowed"), "unexpected error: {}", err);
}

#[test]
fn test_load_local_infile_client_refuses() {
    // A missing file should produce an error: the server fails
    // canonicalize() and returns ERR before sending 0xFB. (A future
    // enhancement could add a client variant that sends an empty
    // packet on 0xFB to test the "refuses" path; for now we cover
    // the missing-file path which the existing test client supports.)
    let mut client = connect_shared();
    let bad: PathBuf = data_dir().join("nonexistent.tbl");

    client.exec(TPC_H_REGION_SCHEMA).expect("create region");

    let result = client.load_local_infile(&bad, "region");
    assert!(result.is_err(), "expected error for missing file");
}

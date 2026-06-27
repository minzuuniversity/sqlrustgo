//! TPC-H wire-protocol smoke — Phase 2d Track 2.
//!
//! Spawns the canonical `sqlrustgo-mysql-server` binary as a
//! subprocess, connects to it via the raw MySQL wire-protocol
//! client (`MySqlTestClient`), and runs a hand-computed TPC-H
//! query whose result is then compared against the expected
//! value. This proves the TPC-H SQL round-trips through the
//! production code path (auth → COM_QUERY → do_command_loop →
//! engine → result-set serialization → client).
//!
//! Unlike `tpch_gate_test.rs` (which uses
//! `ExecutionEngine::new(MemoryStorage)` directly), this test
//! exercises the canonical binary's full stack.
//!
//! Refs: `docs/audit/analysis/2026-06-04-tpch-test-design.md`
//! (Track 2 — wire-protocol TPC-H smoke).

mod common;

use common::MySqlTestClient;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Locate the workspace root from `CARGO_MANIFEST_DIR`.
fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn canonical_binary() -> std::path::PathBuf {
    let mut p = workspace_root();
    p.push("target");
    p.push("debug");
    p.push("sqlrustgo-mysql-server");
    p
}

struct SubprocessHandle {
    child: Child,
    #[allow(dead_code)]
    port: u16,
}

impl Drop for SubprocessHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_canonical_with_client() -> (SubprocessHandle, MySqlTestClient) {
    let bin = canonical_binary();
    assert!(
        bin.exists(),
        "canonical binary not found at {bin:?}; run `cargo build -p sqlrustgo-mysql-server`"
    );

    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe port");
    let port = probe.local_addr().expect("probe local_addr").port();
    drop(probe);

    let child = Command::new(&bin)
        .arg("serve")
        .args(["--host", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .args(["--log-level", "warn"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sqlrustgo-mysql-server serve");

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    while TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_err() {
        if std::time::Instant::now() >= deadline {
            panic!("canonical server did not become reachable on 127.0.0.1:{port} within 5s");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let client = MySqlTestClient::connect_at(("127.0.0.1", port), "root", "")
        .expect("MySqlTestClient should connect to canonical subprocess server");
    (SubprocessHandle { child, port }, client)
}

const TPCH_TINY_DDL: &[&str] = &[
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice INTEGER, l_discount INTEGER, l_tax INTEGER, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT)",
];

const TPCH_TINY_DML: &[&str] = &[
    "INSERT INTO lineitem VALUES (1, 1, 1, 1, 10, 1000, 0, 0, 'R', 'F', '1998-09-01')",
    "INSERT INTO lineitem VALUES (2, 1, 1, 1, 20, 2000, 0, 0, 'A', 'F', '1994-06-15')",
];

#[test]
fn tpch_smoke_count_via_wire() {
    let (_server, mut c) = spawn_canonical_with_client();
    for ddl in TPCH_TINY_DDL {
        c.exec(ddl).expect("DDL over wire should succeed");
    }
    for dml in TPCH_TINY_DML {
        c.exec(dml).expect("DML over wire should succeed");
    }
    let count = c
        .query_one_i64("SELECT COUNT(*) FROM lineitem")
        .expect("COUNT over wire should succeed");
    assert_eq!(count, 2, "COUNT(*) over 2 seeded rows should return 2");
}

#[test]
fn tpch_smoke_sum_via_wire() {
    let (_server, mut c) = spawn_canonical_with_client();
    for ddl in TPCH_TINY_DDL {
        c.exec(ddl).expect("DDL over wire should succeed");
    }
    for dml in TPCH_TINY_DML {
        c.exec(dml).expect("DML over wire should succeed");
    }
    let sum = c
        .query_one_i64("SELECT SUM(l_quantity) FROM lineitem")
        .expect("SUM over wire should succeed");
    assert_eq!(
        sum, 30,
        "SUM(l_quantity) over {TPCH_TINY_DML:?} should be 30"
    );
}

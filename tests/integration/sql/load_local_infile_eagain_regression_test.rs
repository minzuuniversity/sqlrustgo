//! Regression test for `LOAD DATA LOCAL INFILE` EAGAIN bug (RC2 Week 1 Day 6).
//!
//! # Background
//!
//! Issue: server `handle_load_local_infile` in
//! `crates/mysql-server/src/lib.rs` reliably returned
//! `EAGAIN (os error 11)` on the client side when loading tables with
//! 9+ columns and 150+ rows. The boundary: orders.tbl (9 cols, 150
//! rows) and lineitem.tbl (16 cols, 501 rows) crashed; smaller
//! tables (region=5, nation=25, supplier=10, customer=15, part=20,
//! partsupp=80) loaded fine.
//!
//! # Root cause
//!
//! Original code tracked `pending_bytes` as the sum of *parsed* line
//! lengths and flushed when that sum exceeded `bulk_buf_size`. But
//! when a packet ended mid-line (the last line was incomplete), the
//! rest of the line sat in `buf` waiting for the next packet. The
//! `pending_bytes >= bulk_buf_size` check then triggered a flush
//! *before* all the bytes for the current rows were in `buf`, and
//! subsequent client reads got EAGAIN.
//!
//! # Fix
//!
//! Defer the flush decision until `buf` is fully drained (i.e. no
//! partial line remains). Add a sanity guard for malformed input
//! that grows `buf` without ever draining.
//!
//! # This test
//!
//! Loads all 8 TPC-H SF=0.001 tables through the wire protocol and
//! asserts that **all** tables load to exactly the expected row
//! counts. Before the fix, this would panic at `orders.tbl` with
//! "EAGAIN"; after the fix, all 8 tables load in one go.

#[path = "../../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tempfile::TempDir;

const TABLES_AND_PATHS: &[(&str, &str)] = &[
    ("region", "region.tbl"),
    ("nation", "nation.tbl"),
    ("supplier", "supplier.tbl"),
    ("customer", "customer.tbl"),
    ("part", "part.tbl"),
    ("partsupp", "partsupp.tbl"),
    ("orders", "orders.tbl"),
    ("lineitem", "lineitem.tbl"),
];

const EXPECTED_COUNTS: &[(&str, usize)] = &[
    ("region", 5),
    ("nation", 25),
    ("supplier", 10),
    ("customer", 15),
    ("part", 20),
    ("partsupp", 80),
    ("orders", 150),
    ("lineitem", 501),
];

const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

struct SharedServer {
    _handle: EphemeralHandle,
    port: u16,
}

static SHARED: OnceLock<SharedServer> = OnceLock::new();

/// Initialize a shared ephemeral server. data_dir points at the
/// checked-in `tests/data/tpch-sf001/` fixture.
fn shared() -> &'static SharedServer {
    SHARED.get_or_init(|| {
        let data_dir = PathBuf::from("tests/data/tpch-sf001");
        if !data_dir.exists() {
            // Defer to test body which checks the path.
            let tmp = TempDir::new().expect("tempdir");
            return SharedServer {
                _handle: start_ephemeral(EphemeralConfig {
                    data_dir: Some(tmp.path().to_path_buf()),
                    bootstrap_tables: false,
                    bootstrap_users: true,                                metrics_port: None,

                    ..Default::default()
                })
                .expect("start_ephemeral"),
                port: 0, // not used if data dir missing
            };
        }
        let config = EphemeralConfig {
            data_dir: Some(data_dir),
            bootstrap_tables: false,
            bootstrap_users: true,                metrics_port: None,

            ..Default::default()
        };
        let handle = start_ephemeral(config).expect("start_ephemeral");
        let port = handle.port;
        SharedServer {
            _handle: handle,
            port,
        }
    })
}

fn connect_shared() -> MySqlTestClient {
    let port = shared().port;
    MySqlTestClient::connect_at(("127.0.0.1", port), "tester", "tester").expect("connect")
}

#[test]
fn test_load_local_infile_eagain_regression() {
    // Sanity: data dir must exist for this test to be meaningful.
    let data_dir = PathBuf::from("tests/data/tpch-sf001");
    if !data_dir.exists() {
        eprintln!(
            "[SKIP] data dir not found: {}\n\
             This test needs tests/data/tpch-sf001/ to be checked out.",
            data_dir.display()
        );
        return;
    }
    // Check that all 8 .tbl files exist.
    for (_tbl, rel) in TABLES_AND_PATHS {
        let p = data_dir.join(rel);
        if !p.exists() {
            eprintln!("[SKIP] missing .tbl file: {}", p.display());
            return;
        }
        let meta = fs::metadata(&p).expect("stat");
        eprintln!("  {}: {} bytes", rel, meta.len());
    }

    let mut client = connect_shared();

    // Create all 8 schemas.
    for ddl in SCHEMA_DDL {
        client.exec(ddl).expect("DDL");
    }

    // Load all 8 tables via LOAD DATA LOCAL INFILE. **This is where
    // the EAGAIN bug used to fire on orders.tbl (9 cols, 150 rows)
    // and lineitem.tbl (16 cols, 614 rows)**.
    for (table, rel_path) in TABLES_AND_PATHS {
        let path = data_dir.join(rel_path);
        let n = client
            .load_local_infile(&path, table)
            .unwrap_or_else(|e| panic!("load_local_infile {}: {}", table, e));
        let expected = EXPECTED_COUNTS
            .iter()
            .find(|(t, _)| *t == *table)
            .map(|(_, c)| *c)
            .unwrap();
        assert_eq!(
            n as usize, expected,
            "{}: expected {} rows, loaded {}",
            table, expected, n
        );
        eprintln!("  {}: {} rows (expected {})", table, n, expected);
    }

    // Sanity: aggregate over the largest table.
    let cnt: i64 = client
        .query_one_i64("SELECT COUNT(*) FROM lineitem")
        .expect("count lineitem");
    assert_eq!(cnt, 501, "lineitem row count mismatch");
}

//! TPC-H 22/22 wire-protocol round-trip test on SF=0.1 fixture.
//!
//! Companion to `tests/tpch_22_queries_wire_test.rs` (SF=0.001).
//! Extends wire-protocol coverage from 501 to 60000 lineitem rows,
//! exercising the EAGAIN-bug-fixed path (PR-3128, RC2 Week 1 Day 6)
//! at canonical TPC-H SF=0.1 scale.
//!
//! Asserts: 8 tables load via LOAD DATA LOCAL INFILE with correct row
//! counts, all 22 queries run to completion over the wire (0 panic,
//! 0 error packet), and row-count + first-3-row values match the
//! per-query SQLite baselines at `tests/data/tpch-sf01/expected/`.

mod common;

use common::MySqlTestClient;
use serde_json::Value as JsonValue;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

const DATA_DIR: &str = "tests/data/tpch-sf01";
const EXPECTED_DIR: &str = "tests/data/tpch-sf01/expected";

const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL, PRIMARY KEY (l_orderkey, l_linenumber))",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

const EXPECTED_LOAD_COUNTS: &[(&str, u64)] = &[
    ("region", 5),
    ("nation", 25),
    ("supplier", 100),
    ("customer", 1500),
    ("part", 2000),
    ("partsupp", 8000),
    ("orders", 15000),
    ("lineitem", 60000),
];

struct SharedServer {
    _handle: EphemeralHandle,
    port: u16,
}

static SHARED: OnceLock<SharedServer> = OnceLock::new();

fn shared() -> &'static SharedServer {
    SHARED.get_or_init(|| {
        let config = EphemeralConfig {
            data_dir: Some(PathBuf::from(DATA_DIR)),
            bootstrap_tables: false,
            bootstrap_users: true,
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

fn load_tbl(client: &mut MySqlTestClient, name: &str) -> u64 {
    let path = PathBuf::from(DATA_DIR).join(format!("{}.tbl", name));
    client
        .load_local_infile(&path, name)
        .unwrap_or_else(|e| panic!("load {}: {}", name, e))
}

fn load_queries() -> Vec<(u8, String)> {
    let queries_dir = PathBuf::from("queries");
    let mut qs: Vec<(u8, String)> = (1..=22)
        .map(|n| {
            let p = queries_dir.join(format!("q{}.sql", n));
            let sql = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
            (n, sql.trim_end_matches(';').trim().to_string())
        })
        .collect();
    qs.sort_by_key(|(n, _)| *n);
    qs
}

fn read_baseline(qnum: u8) -> Option<(u64, Vec<Vec<String>>)> {
    let p = PathBuf::from(EXPECTED_DIR).join(format!("Q{}_sf01_baseline.json", qnum));
    if !p.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&p).ok()?;
    let v: JsonValue = serde_json::from_str(&content).ok()?;
    let rc = v["row_count"].as_u64()?;
    let rows: Vec<Vec<String>> = v["first_3_rows"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|row| {
                    row.as_array()
                        .map(|cells| {
                            cells
                                .iter()
                                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    Some((rc, rows))
}

#[test]
fn test_tpch_sf01_22_queries_wire_roundtrip() {
    let data_dir = PathBuf::from(DATA_DIR);
    if !data_dir.exists() {
        eprintln!("[SKIP] data dir not found: {}", data_dir.display());
        return;
    }
    for tbl in TABLES {
        let p = data_dir.join(format!("{}.tbl", tbl));
        if !p.exists() {
            eprintln!("[SKIP] missing .tbl: {}", p.display());
            return;
        }
    }

    let mut client = connect_shared();
    client
        .set_timeouts(Duration::from_secs(300), Duration::from_secs(300))
        .expect("set_timeouts");

    eprintln!("[1/3] Creating 8 schemas");
    for ddl in SCHEMA_DDL {
        client.exec(ddl).expect("DDL");
    }

    eprintln!("[2/3] Loading 8 tables via LOAD DATA LOCAL INFILE (SF=0.1)");
    eprintln!("       (exercises the EAGAIN-bug-fixed path at full 60K-row scale)");
    for (tbl, expected) in EXPECTED_LOAD_COUNTS {
        let n = load_tbl(&mut client, tbl);
        assert_eq!(n, *expected, "{} row count mismatch", tbl);
        eprintln!("  {}: {} rows", tbl, n);
    }

    eprintln!("[3/3] Running 22 TPC-H queries over the wire");
    let queries = load_queries();
    assert_eq!(queries.len(), 22, "must have exactly 22 queries");

    let mut pass = 0;
    let mut fail = 0;
    let mut skipped = 0;
    let mut fail_details: Vec<String> = Vec::new();

    for (qnum, sql) in &queries {
        match client.query_rows(sql) {
            Ok(rows) => {
                let actual_rc = rows.len() as u64;
                match read_baseline(*qnum) {
                    Some((expected_rc, expected_rows)) => {
                        if actual_rc != expected_rc {
                            fail += 1;
                            fail_details.push(format!(
                                "Q{}: rc mismatch actual={} expected={}",
                                qnum, actual_rc, expected_rc
                            ));
                            continue;
                        }
                        if expected_rows.is_empty() {
                            pass += 1;
                            eprintln!("  Q{}: OK (rc={}, empty result)", qnum, actual_rc);
                            continue;
                        }
                        let compare_n = expected_rows.len().min(3);
                        // Compare cell-by-cell with FP tolerance.
                        // Differences that may fail strict string
                        // match but are cell-value-equal: float
                        // formatting (146193 vs 146193.0), full
                        // IEEE-754 precision (2941.6499999999996 vs
                        // 2941.65), int vs float display (1 vs
                        // 1.00000000).
                        let mut actual_first3: Vec<Vec<String>> =
                            rows.iter().take(compare_n).cloned().collect();
                        let mut expected_first3: Vec<Vec<String>> =
                            expected_rows.iter().take(compare_n).cloned().collect();
                        actual_first3.sort();
                        expected_first3.sort();
                        let cell_eq = |a: &str, b: &str| -> bool {
                            if a == b {
                                return true;
                            }
                            match (a.parse::<f64>(), b.parse::<f64>()) {
                                (Ok(av), Ok(bv)) => {
                                    let abs = (av - bv).abs();
                                    let rel = if bv != 0.0 { abs / bv.abs() } else { abs };
                                    abs <= 1e-3 || rel <= 1e-6
                                }
                                _ => false,
                            }
                        };
                        let rows_eq = |a: &[String], b: &[String]| -> bool {
                            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| cell_eq(x, y))
                        };
                        if !actual_first3
                            .iter()
                            .zip(expected_first3.iter())
                            .all(|(a, e)| rows_eq(a, e))
                        {
                            fail += 1;
                            eprintln!("[Q{} ACTUAL sorted]", qnum);
                            for r in &actual_first3 {
                                eprintln!("  {:?}", r);
                            }
                            eprintln!("[Q{} EXPECTED sorted]", qnum);
                            for r in &expected_first3 {
                                eprintln!("  {:?}", r);
                            }
                            fail_details.push(format!(
                                "Q{}: first 3 rows differ (sorted, FP tolerant)",
                                qnum
                            ));
                            continue;
                        }
                        pass += 1;
                        eprintln!("  Q{}: OK (rc={}, first3 match)", qnum, actual_rc);
                    }
                    None => {
                        skipped += 1;
                        eprintln!("  Q{}: ran (no baseline, rc={})", qnum, actual_rc);
                    }
                }
            }
            Err(e) => {
                fail += 1;
                fail_details.push(format!("Q{}: wire exec error: {}", qnum, e));
            }
        }
    }

    eprintln!(
        "\n=== TPC-H SF=0.1 22/22 Wire Round-Trip ===\n  Pass: {}\n  Skip (no baseline): {}\n  Fail: {}\n",
        pass, skipped, fail
    );
    if !fail_details.is_empty() {
        eprintln!("\nFailures:");
        for d in &fail_details {
            eprintln!("  {}", d);
        }
    }
    if fail > 0 {
        eprintln!(
            "\n[NOTE] {} failures above. Sprint 5 v11 fixed the Q17 + Q18 cell-value\n             bugs (see docs/audit/status/2026-06-09-sprint5-v11-cell-fixes.md).\n             Any remaining FAILs are real engine bugs, not wire-protocol issues.",
            fail
        );
    }
    assert_eq!(fail, 0, "wire protocol or engine bugs: {} failures", fail);
    assert_eq!(pass, 22, "expected 22 PASS, got {}", pass);
}

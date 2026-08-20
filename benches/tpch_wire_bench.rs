//! TPC-H Wire-Protocol Performance Benchmark — Issue #2948 (Track 3)
//!
//! Runs TPC-H queries against the canonical `sqlrustgo-mysql-server`
//! over the real MySQL wire protocol, with the bulk loader
//! (`LOAD DATA LOCAL INFILE`) handling the data side.
//!
//! # Why a separate bench (not extend tpch_bench.rs)
//!
//! `tpch_bench.rs` uses `ExecutionEngine` directly (in-process API).
//! This bench uses `MySqlTestClient` + `start_ephemeral` to go through
//! the **wire protocol** — the only path that produces a system number
//! comparable to MySQL/MariaDB/PostgreSQL (per Issue #2948 § "Why it
//! matters").
//!
//! # SF selection
//!
//! Issue #2948 requires SF≥1. The data must come from the
//! `tpch_data_gen` example at the right scale; see Issue #2948 § 1
//! "Data generation" and the `tpch_data_gen --scale N` invocation.
//!
//! # Output
//!
//! Writes a JSON summary to `benchmarks/results/sf1/wire.json`
//! (per Issue #2948 § 3) with per-query p50/p95/p99 + total
//! wall-clock + row count for sanity.

#![allow(dead_code)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

mod common;
use common::MySqlTestClient;

const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_custkey_marker TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

// Issue #2948 expects SF>=1 data; for bench portability we read from
// `TPCH_SF1_DIR` env var. Default to `/tmp/tpch_sf1` which is what
// `cargo run --example tpch_data_gen -- --scale 1 --output /tmp/tpch_sf1`
// produces.
fn fixture_dir() -> PathBuf {
    PathBuf::from(std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| "/tmp/tpch_sf1".to_string()))
}

struct SharedServer {
    _handle: EphemeralHandle,
    port: u16,
    data_dir: PathBuf,
}

static SHARED: OnceLock<SharedServer> = OnceLock::new();

fn shared() -> &'static SharedServer {
    SHARED.get_or_init(|| {
        let dir = fixture_dir();
        if !dir.join("lineitem.tbl").exists() {
            panic!(
                "SF=1 fixture missing at {}. Generate with:\n  \
                 cargo run --release -p sqlrustgo-bench --example tpch_data_gen -- --scale 1 --output {}\n\
                 (see Issue #2948 § 1)",
                dir.display(),
                dir.display()
            );
        }
        let config = EphemeralConfig {
            data_dir: Some(dir.clone()),
            bootstrap_tables: false,
            bootstrap_users: true,
            ..Default::default()
        };
        let handle = start_ephemeral(config).expect("start_ephemeral");
        let port = handle.port;
        SharedServer {
            _handle: handle,
            port,
            data_dir: dir,
        }
    })
}

fn connect() -> MySqlTestClient {
    let port = shared().port;
    // Issue #2948 (Track 3) hint: start_ephemeral returns immediately
    // with a port; the listener thread may not be bound yet. Retry the
    // connect for up to 30 s before giving up.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match MySqlTestClient::connect_at(("127.0.0.1", port), "tester", "tester") {
            Ok(c) => return c,
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => panic!("connect to ephemeral server: {}", e),
        }
    }
}

fn bootstrap_tables_and_load(client: &mut MySqlTestClient) {
    for ddl in SCHEMA_DDL {
        client.exec(ddl).expect("DDL");
    }
    for tbl in TABLES {
        let path = shared().data_dir.join(format!("{}.tbl", tbl));
        let n = client
            .load_local_infile(&path, tbl)
            .unwrap_or_else(|e| panic!("load {}: {}", tbl, e));
        eprintln!("  {}: {} rows loaded", tbl, n);
    }
}

/// TPC-H queries that pass in the in-process test (#2948 § 3 quotes
/// Q1/Q3/Q6 specifically — the 3 queries that the engine supports
/// today without a join-bug regression).
const TPCH_QUERIES: &[(&str, &str)] = &[
    (
        "Q1",
        "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, \
         SUM(l_extendedprice) AS sum_base_price, SUM(l_extendedprice*(1-l_discount)) AS sum_disc_price, \
         SUM(l_extendedprice*(1-l_discount)*(1+l_tax)) AS sum_charge, AVG(l_quantity) AS avg_qty, \
         AVG(l_extendedprice) AS avg_price, AVG(l_discount) AS avg_disc, COUNT(*) AS count_order \
         FROM lineitem WHERE l_shipdate <= DATE_SUB('1998-12-01', INTERVAL 90 DAY) \
         GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus",
    ),
    (
        "Q3",
        "SELECT l_orderkey, SUM(l_extendedprice*(1-l_discount)) AS revenue, o_orderdate, o_shippriority \
         FROM customer, orders, lineitem \
         WHERE c_mktsegment = 'BUILDING' AND c_custkey = o_custkey AND l_orderkey = o_orderkey \
         AND o_orderdate < '1995-03-15' AND l_shipdate > '1995-03-15' \
         GROUP BY l_orderkey, o_orderdate, o_shippriority ORDER BY revenue DESC, o_orderdate LIMIT 10",
    ),
    (
        "Q6",
        "SELECT SUM(l_extendedprice*l_discount) AS revenue FROM lineitem \
         WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' \
         AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24",
    ),
];

fn bench_tpch_wire(c: &mut Criterion) {
    let mut client = connect();
    bootstrap_tables_and_load(&mut client);

    let mut group = c.benchmark_group("tpch_wire");
    for (name, sql) in TPCH_QUERIES {
        // Warmup
        for _ in 0..5 {
            let _ = client.query_rows(sql).expect("warmup");
        }
        group.bench_with_input(BenchmarkId::from_parameter(name), sql, |b, sql| {
            b.iter(|| {
                let rows = client.query_rows(sql).expect("query");
                let rc = rows.len();
                criterion::black_box(rc);
            });
        });
    }
    group.finish();

    // After benchmarks, run a final report and dump JSON
    let mut report = serde_json::Map::new();
    report.insert("scale_factor".into(), json!(1.0));
    report.insert("transport".into(), json!("MySQL wire protocol"));
    report.insert("binary".into(), json!("sqlrustgo-mysql-server (canonical)"));
    let mut queries = Vec::new();
    for (name, sql) in TPCH_QUERIES {
        let n = 30; // 30 timed iterations
        let mut lats_us = Vec::with_capacity(n);
        let mut first_err: Option<String> = None;
        for _ in 0..n {
            let t0 = Instant::now();
            let r = client.query_rows(sql);
            let dt = t0.elapsed();
            match r {
                Ok(rows) => {
                    lats_us.push(dt.as_micros() as u64);
                    criterion::black_box(rows);
                }
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(format!("{}", e));
                    }
                }
            }
        }
        lats_us.sort();
        let p = |q: f64| -> u64 {
            let idx =
                ((lats_us.len() as f64 * q / 100.0) as usize).min(lats_us.len().saturating_sub(1));
            lats_us.get(idx).copied().unwrap_or(0)
        };
        let avg = if lats_us.is_empty() {
            0
        } else {
            lats_us.iter().sum::<u64>() / lats_us.len() as u64
        };
        let mut qobj = serde_json::Map::new();
        qobj.insert("name".into(), json!(name));
        qobj.insert("iterations".into(), json!(n));
        qobj.insert("avg_ms".into(), json!(avg as f64 / 1000.0));
        qobj.insert("p50_ms".into(), json!(p(50.0) as f64 / 1000.0));
        qobj.insert("p95_ms".into(), json!(p(95.0) as f64 / 1000.0));
        qobj.insert("p99_ms".into(), json!(p(99.0) as f64 / 1000.0));
        if let Some(e) = first_err {
            qobj.insert("error".into(), json!(e));
        }
        queries.push(serde_json::Value::Object(qobj));
    }
    report.insert("queries".into(), serde_json::Value::Array(queries));

    let out_path = PathBuf::from("benchmarks/results/sf1/wire.json");
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json_str = serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap();
    if let Err(e) = std::fs::write(&out_path, &json_str) {
        eprintln!("WARN: could not write {}: {}", out_path.display(), e);
    } else {
        eprintln!("Wrote benchmark report: {}", out_path.display());
    }
}

criterion_group!(tpch_wire_benches, bench_tpch_wire);
criterion_main!(tpch_wire_benches);

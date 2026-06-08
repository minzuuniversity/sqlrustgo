//! TPC-H 22/22 in-process on SF=0.1 fixture, compared against the
//! SQLite baseline at `tests/data/tpch-sf01/expected/Q{N}_sf01_baseline.json`.
//!
//! Usage:
//!   cargo test --test tpch_sf01_22_vs_sqlite -- --nocapture

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const DATA_DIR: &str = "tests/data/tpch-sf01";
const QUERIES_DIR: &str = "queries";
const EXPECTED_DIR: &str = "tests/data/tpch-sf01/expected";

const SCHEMA_SQL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

fn lookup_col_types(table: &str) -> Vec<&'static str> {
    let ddl = SCHEMA_SQL.iter().find(|s| s.contains(table)).expect("ddl");
    let start = ddl.find('(').unwrap() + 1;
    let end = ddl.rfind(')').unwrap();
    let inner = &ddl[start..end];
    inner
        .split(',')
        .map(|c| {
            let c = c.trim();
            if c.eq_ignore_ascii_case("PRIMARY KEY") || c.contains("PRIMARY KEY") {
                ""
            } else {
                let tokens: Vec<&str> = c.split_whitespace().collect();
                if tokens.len() >= 2 { tokens[1] } else { "" }
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn load_tbl(storage: &Arc<RwLock<MemoryStorage>>, tbl: &str) -> usize {
    let path = format!("{}/{}.tbl", DATA_DIR, tbl);
    let content = fs::read_to_string(&path).expect(&format!("read {}", path));
    let types = lookup_col_types(tbl);
    let mut n = 0usize;
    for line in content.lines() {
        if line.is_empty() { continue; }
        let fields: Vec<&str> = line.split('|').collect();
        let mut vals: Vec<SqlValue> = Vec::new();
        for (i, f) in fields.iter().enumerate() {
            let t = types.get(i).copied().unwrap_or("TEXT");
            vals.push(match t.to_uppercase().as_str() {
                "INTEGER" => f.parse::<i64>().map(SqlValue::Integer).unwrap_or(SqlValue::Null),
                "REAL" => f.parse::<f64>().map(SqlValue::Float).unwrap_or(SqlValue::Null),
                _ => SqlValue::Text(f.to_string()),
            });
        }
        let row: Vec<SqlValue> = vals;
        storage.write().unwrap().insert(tbl, vec![row]).expect("insert");
        n += 1;
    }
    n
}

fn build_engine() -> (ExecutionEngine<MemoryStorage>, Arc<RwLock<MemoryStorage>>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    for ddl in SCHEMA_SQL {
        ExecutionEngine::new(storage.clone()).execute(ddl).expect("ddl");
    }
    let engine = ExecutionEngine::new(storage.clone());
    (engine, storage)
}

#[test]
fn tpch_sf01_22_vs_sqlite() {
    if !PathBuf::from(DATA_DIR).exists() {
        panic!("fixture {} not present; skipping", DATA_DIR);
    }
    eprintln!("=== Loading SF=0.1 fixture from {} ===", DATA_DIR);
    let (mut engine, storage) = build_engine();
    for tbl in TABLES {
        let n = load_tbl(&storage, tbl);
        eprintln!("  {}: {} rows", tbl, n);
    }
    eprintln!();
    eprintln!("=== Running 22 TPC-H queries vs SQLite baseline ===");
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut skip = 0usize;
    for n in 1..=22usize {
        let sql_path = format!("{}/q{}.sql", QUERIES_DIR, n);
        let sql = fs::read_to_string(&sql_path).expect(&sql_path);
        let expected_path = format!("{}/Q{}_sf01_baseline.json", EXPECTED_DIR, n);
        let expected_rc: Option<usize> = if PathBuf::from(&expected_path).exists() {
            let s = fs::read_to_string(&expected_path).unwrap();
            let v: serde_json::Value = serde_json::from_str(&s).expect("json");
            Some(v["row_count"].as_u64().unwrap() as usize)
        } else {
            None
        };
        let t0 = std::time::Instant::now();
        let res = engine.execute(&sql);
        let elapsed = t0.elapsed();
        match res {
            Ok(r) => {
                let status = match expected_rc {
                    Some(er) if er == r.rows.len() => {
                        pass += 1;
                        format!("PASS (rc={}, expected={})", r.rows.len(), er)
                    }
                    Some(er) => {
                        fail += 1;
                        format!("FAIL (rc={}, expected={})", r.rows.len(), er)
                    }
                    None => {
                        skip += 1;
                        format!("SKIP (no baseline, rc={})", r.rows.len())
                    }
                };
                eprintln!("  Q{:2}: {} in {:?}", n, status, elapsed);
            }
            Err(e) => {
                fail += 1;
                eprintln!("  Q{:2}: ERROR ({}) in {:?}", n, e, elapsed);
            }
        }
    }
    eprintln!();
    eprintln!("=== Summary: pass={} fail={} skip={} ===", pass, fail, skip);
    assert_eq!(fail, 0, "{} queries fail vs SQLite baseline", fail);
}

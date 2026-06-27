//! TPC-H Single-Query Runner (Sprint 5 harness v2 adapter).
//!
//! Reads SQL from stdin or --query, runs against sqlrustgo engine
//! with /tmp/tpch_sf01_v2/ data, outputs JSON for harness v2.
//!
//! Usage:
//!   # Single query
//!   cargo run --example tpch_run_query -- --query Q6
//!
//!   # SQL from stdin
//!   echo "SELECT count(*) FROM lineitem" | cargo run --example tpch_run_query
//!
//!   # With timeout
//!   cargo run --example tpch_run_query -- --query Q3 --timeout 10
//!
//! Output JSON:
//!   {"engine":"sqlrustgo","query":"Q6","rows":[["6076.9327"]],"row_count":1,"duration_ms":35,"error":null,"timed_out":false}

use serde::{Deserialize, Serialize};
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
struct RunResult {
    engine: String,
    query: String,
    rows: Vec<Vec<String>>,
    row_count: usize,
    duration_ms: u128,
    error: Option<String>,
    timed_out: bool,
}

const SCHEMA_SQL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER, c_name TEXT, c_address TEXT, c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT)",
    "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

fn load_data(engine: &mut ExecutionEngine<MemoryStorage>, data_dir: &str) {
    use std::fs;
    for ddl in SCHEMA_SQL {
        let _ = engine.execute(ddl);
    }
    for table in TABLES {
        let path = format!("{}/{}.tbl", data_dir, table);
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let line_trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = line_trimmed.split('|').collect();
            let mut vals: Vec<String> = Vec::new();
            for c in &cols {
                if c.parse::<i64>().is_ok() || c.parse::<f64>().is_ok() {
                    vals.push(c.to_string());
                } else {
                    let esc = c.replace('\'', "''");
                    vals.push(format!("'{}'", esc));
                }
            }
            let sql = format!("INSERT INTO {} VALUES ({})", table, vals.join(","));
            let _ = engine.execute(&sql);
        }
    }
}

fn format_value(v: &sqlrustgo::Value) -> String {
    use sqlrustgo::Value::*;
    match v {
        Null => String::new(),
        Integer(n) => n.to_string(),
        Float(f) => f.to_string(),
        Text(s) => s.clone(),
        Blob(b) => format!("{:?}", b),
        Boolean(b) => b.to_string(),
    }
}

fn run_query(sql: &str, query_name: &str, data_dir: &str, timeout_sec: u64) -> RunResult {
    let start = Instant::now();
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    load_data(&mut engine, data_dir);
    let load_ms = start.elapsed().as_millis();

    let exec_start = Instant::now();
    let result = engine.execute(sql);
    let exec_ms = exec_start.elapsed().as_millis();

    let total_ms = start.elapsed().as_millis();
    let _ = load_ms; // suppress unused warning

    let timed_out = total_ms > (timeout_sec as u128) * 1000;

    match result {
        Ok(exec) => RunResult {
            engine: "sqlrustgo".to_string(),
            query: query_name.to_string(),
            rows: exec
                .rows
                .iter()
                .map(|r| r.iter().map(format_value).collect())
                .collect(),
            row_count: exec.rows.len(),
            duration_ms: exec_ms,
            error: None,
            timed_out,
        },
        Err(e) => RunResult {
            engine: "sqlrustgo".to_string(),
            query: query_name.to_string(),
            rows: vec![],
            row_count: 0,
            duration_ms: exec_ms,
            error: Some(e.to_string()),
            timed_out,
        },
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut data_dir = "/tmp/tpch_sf01_v2".to_string();
    let mut query_name = "Q?".to_string();
    let mut query_sql: Option<String> = None;
    let mut timeout_sec: u64 = 15;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" => {
                data_dir = args[i + 1].clone();
                i += 2;
            }
            "--query" => {
                query_name = args[i + 1].clone();
                let path = format!("queries/{}.sql", args[i + 1].to_lowercase());
                query_sql = Some(
                    std::fs::read_to_string(&path)
                        .unwrap_or_default()
                        .trim()
                        .trim_end_matches(';')
                        .to_string(),
                );
                i += 2;
            }
            "--sql" => {
                query_sql = Some(args[i + 1].clone());
                i += 2;
            }
            "--timeout" => {
                timeout_sec = args[i + 1].parse().unwrap_or(15);
                i += 2;
            }
            "--help" | "-h" => {
                eprintln!("Usage: tpch_run_query --query Q6 [--data-dir DIR] [--timeout SEC]");
                eprintln!(
                    "       tpch_run_query --sql 'SELECT ...' [--data-dir DIR] [--timeout SEC]"
                );
                eprintln!("       echo 'SELECT ...' | tpch_run_query");
                std::process::exit(0);
            }
            _ => {
                i += 1;
            }
        }
    }

    let sql = match query_sql {
        Some(s) if !s.is_empty() => s,
        _ => {
            // Read from stdin
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap();
            query_name = "stdin".to_string();
            buf.trim().to_string()
        }
    };

    let result = run_query(&sql, &query_name, &data_dir, timeout_sec);
    println!("{}", serde_json::to_string(&result).unwrap());
    if let Some(e) = &result.error {
        eprintln!("[error] {}", e);
        std::process::exit(1);
    }
}

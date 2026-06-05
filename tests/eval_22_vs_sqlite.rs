//! 全面评估: in-process engine 跑 sf001 + 22 query, 跟 SQLite baseline JSON 对比 row_count
//!
//! 这是 Issue #2977 真"完成度"评估 — 不只是 "not crash" 而是 "row_count 对不对".
//! 目的: 给李哥看 TPC-H 22 线程的真正状态 (in-process 跟 SQLite 的差距).

use serde_json::Value as JsonValue;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const FIXTURE_DIR: &str = "tests/data/tpch-sf001";
const QUERIES_DIR: &str = "queries";
const EXPECTED_DIR: &str = "tests/data/tpch-sf001/expected";

const DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_regionkey INTEGER, n_name TEXT, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_nationkey INTEGER, s_name TEXT, s_address TEXT, s_phone TEXT, s_acctbal INTEGER, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER, c_nationkey INTEGER, c_name TEXT, c_address TEXT, c_phone TEXT, c_acctbal INTEGER, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice INTEGER, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost INTEGER, ps_comment TEXT)",
    "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice INTEGER, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice INTEGER, l_discount INTEGER, l_tax INTEGER, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
];

const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

fn lookup_col_types(table: &str) -> Vec<&'static str> {
    for ddl in DDL {
        if let Some(rest) = ddl.strip_prefix("CREATE TABLE ") {
            if let Some(open) = rest.find('(') {
                let cols_str = &rest[open + 1..rest.len() - 1];
                if rest[..open].trim() != table {
                    continue;
                }
                return cols_str
                    .split(',')
                    .map(|c| {
                        let p: Vec<&str> = c.trim().split_whitespace().collect();
                        if p.len() >= 2 {
                            p[1]
                        } else {
                            "TEXT"
                        }
                    })
                    .collect();
            }
        }
    }
    vec![]
}

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for d in DDL {
        engine.execute(d).expect("DDL");
    }
    let base = PathBuf::from(FIXTURE_DIR);
    for tbl in TABLES {
        let path = base.join(format!("{}.tbl", tbl));
        let content = fs::read_to_string(&path).expect("read .tbl");
        let col_types = lookup_col_types(tbl);
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            let trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = trimmed.split('|').collect();
            let vals: Vec<String> = cols
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let ty = col_types.get(i).copied().unwrap_or("TEXT");
                    if ty == "INTEGER" || ty == "REAL" {
                        s.to_string()
                    } else {
                        format!("'{}'", s.replace('\'', "''"))
                    }
                })
                .collect();
            let sql = format!("INSERT INTO {} VALUES ({})", tbl, vals.join(","));
            engine.execute(&sql).expect("insert");
        }
    }
    engine
}

fn read_q_sql(qnum: u32) -> String {
    let path = PathBuf::from(QUERIES_DIR).join(format!("q{}.sql", qnum));
    fs::read_to_string(&path)
        .expect("read q*.sql")
        .trim()
        .trim_end_matches(';')
        .to_string()
}

fn expected_row_count(qnum: u32) -> Option<i64> {
    let path = PathBuf::from(EXPECTED_DIR).join(format!("Q{}_three_way.json", qnum));
    let text = fs::read_to_string(&path).expect("read json");
    let json: JsonValue = serde_json::from_str(&text).expect("parse json");
    json.get("consensus_row_count").and_then(|v| v.as_i64())
}

#[test]
fn eval_22_in_process_vs_sqlite() {
    let mut engine = make_engine();
    let mut passed = 0u32;
    let mut mismatched = 0u32;
    let mut err = 0u32;
    let mut summary: Vec<String> = Vec::new();

    for q in 1u32..=22 {
        let sql = read_q_sql(q);
        let expected = expected_row_count(q);
        match engine.execute(&sql) {
            Ok(r) => {
                if let Some(exp) = expected {
                    if (r.rows.len() as i64) == exp {
                        passed += 1;
                        summary.push(format!(
                            "Q{:2}: PASS  actual={} expected={}",
                            q,
                            r.rows.len(),
                            exp
                        ));
                    } else {
                        mismatched += 1;
                        summary.push(format!(
                            "Q{:2}: MISMATCH  actual={} expected={}",
                            q,
                            r.rows.len(),
                            exp
                        ));
                    }
                } else {
                    summary.push(format!("Q{:2}: OK  rows={} (no expected)", q, r.rows.len()));
                }
            }
            Err(e) => {
                err += 1;
                summary.push(format!("Q{:2}: ERR  {}", q, e));
            }
        }
    }

    println!("\n=== TPC-H 22 in-process vs SQLite baseline (sf001) ===");
    for line in &summary {
        println!("  {}", line);
    }
    println!(
        "\nTotals: {} matched, {} mismatched, {} errors, 22 total",
        passed, mismatched, err
    );

    // Don't fail the test — this is a diagnostic, not a gate.
}

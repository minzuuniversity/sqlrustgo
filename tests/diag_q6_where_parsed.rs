//! Q6 exact reproduction - run Q6 SQL from queries/q6.sql verbatim
//!
//! To determine whether the issue is in:
//! - WHERE clause parsing
//! - WHERE clause evaluation (eval_predicate)
//! - BETWEEN operator
//! - Compare logic for dates

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const FIXTURE: &str = "/home/openclaw/sqlrustgo-tpch/data";

fn load_lineitem(storage: &Arc<RwLock<MemoryStorage>>) -> usize {
    let path = format!("{}/lineitem.tbl", FIXTURE);
    let content = fs::read_to_string(&path).expect("read");
    const BATCH_SIZE: usize = 10000;
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split('|').collect();
        if values.len() < 16 {
            continue;
        }
        let record: Vec<SqlValue> = values[..16]
            .iter()
            .map(|v| {
                let s = v.trim();
                if s.is_empty() {
                    SqlValue::Null
                } else if let Ok(i) = s.parse::<i64>() {
                    SqlValue::Integer(i)
                } else if let Ok(f) = s.parse::<f64>() {
                    SqlValue::Float(f)
                } else {
                    SqlValue::Text(s.to_string())
                }
            })
            .collect();
        batch.push(record);
        if batch.len() >= BATCH_SIZE {
            let mut s = storage.write().unwrap();
            let _ = s.insert("lineitem", batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write().unwrap();
        let _ = s.insert("lineitem", batch.clone());
        count += batch.len();
    }
    count
}

#[test]
fn diag_q6_where_parsed() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
             l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, \
             l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
             l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, \
             l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, \
             PRIMARY KEY (l_orderkey, l_linenumber))",
        )
        .unwrap();
    let n = load_lineitem(&storage);
    eprintln!("loaded {} lineitem rows", n);

    // Test 1: simple WHERE with date string
    let r1 = engine
        .execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate = '1994-01-01'")
        .unwrap();
    eprintln!("shipdate = '1994-01-01' count = {}", r1.rows[0][0]);

    let r2 = engine
        .execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01'")
        .unwrap();
    eprintln!("shipdate >= '1994-01-01' count = {}", r2.rows[0][0]);

    let r3 = engine
        .execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate < '1995-01-01'")
        .unwrap();
    eprintln!("shipdate < '1995-01-01' count = {}", r3.rows[0][0]);

    // Test 2: between
    let r4 = engine
        .execute("SELECT COUNT(*) FROM lineitem WHERE l_discount BETWEEN 0.06 AND 0.08")
        .unwrap();
    eprintln!("discount BETWEEN 0.06 AND 0.08 count = {}", r4.rows[0][0]);

    // Test 3: l_shipdate value at row 0
    let r5 = engine
        .execute("SELECT l_shipdate FROM lineitem LIMIT 3")
        .unwrap();
    eprintln!("First 3 shipdates:");
    for row in r5.rows {
        eprintln!("  {:?}", row);
    }

    // Test 4: actual range
    let r6 = engine
        .execute("SELECT MIN(l_shipdate), MAX(l_shipdate) FROM lineitem")
        .unwrap();
    eprintln!("shipdate range: {:?}", r6.rows);
}

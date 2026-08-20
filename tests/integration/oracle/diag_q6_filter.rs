//! Q6 filter diagnostic
//!
//! Step-by-step check whether WHERE filters actually work for
//! 1-table queries in current engine.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::Arc;

const FIXTURE: &str = match option_env!("TPCH_DATA_DIR") {
    Some(p) => p,
    None => "tests/data/tpch-sf01",
};

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
            let mut s = storage.write();
            let _ = s.insert("lineitem", batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write();
        let _ = s.insert("lineitem", batch.clone());
        count += batch.len();
    }
    count
}

#[test]
fn diag_q6_filter() {
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

    // Step 1: total count
    let r1 = engine.execute("SELECT COUNT(*) FROM lineitem").unwrap();
    let total = match &r1.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("Step 1: total lineitem rows = {}", total);

    // Step 2: count after only shipdate filter
    let r2 = engine
        .execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01'")
        .unwrap();
    let shipdate_count = match &r2.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("Step 2: after shipdate filter = {}", shipdate_count);

    // Step 3: shipdate + discount BETWEEN
    let r3 = engine.execute(
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08"
    ).unwrap();
    let both_count = match &r3.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("Step 3: shipdate + discount = {}", both_count);

    // Step 4: shipdate + discount + quantity
    let r4 = engine.execute(
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08 AND l_quantity < 25"
    ).unwrap();
    let all_count = match &r4.rows[0][0] {
        SqlValue::Integer(n) => *n,
        v => panic!("unexpected: {:?}", v),
    };
    eprintln!("Step 4: shipdate + discount + quantity = {}", all_count);
    eprintln!("  expected from Q6: 1");

    // Step 5: full Q6 with SUM
    let r5 = engine.execute("SELECT SUM(l_extendedprice * l_discount) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08 AND l_quantity < 25").unwrap();
    eprintln!("Step 5: full Q6 SUM = {:?}", r5.rows);
}

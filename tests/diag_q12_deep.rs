//! Q12 deeper isolation

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::{Arc, RwLock};

const FIXTURE: &str = "/home/openclaw/sqlrustgo-tpch/data";

fn load(storage: &Arc<RwLock<MemoryStorage>>, tbl: &str, ncols: usize) -> usize {
    let path = format!("{}/{}.tbl", FIXTURE, tbl);
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
        if values.len() < ncols {
            continue;
        }
        let record: Vec<SqlValue> = values[..ncols]
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
            let _ = s.insert(tbl, batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write().unwrap();
        let _ = s.insert(tbl, batch.clone());
        count += batch.len();
    }
    count
}

#[test]
fn diag_q12_deep() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))").unwrap();
    load(&storage, "lineitem", 16);

    // IN clause test
    let r1 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP')").unwrap();
    eprintln!("shipmode IN: {:?}", r1.rows);

    // < test on dates
    let r2 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_commitdate < l_receiptdate").unwrap();
    eprintln!("commit < receipt: {:?}", r2.rows);

    // AND chain
    let r3 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate").unwrap();
    eprintln!("shipmode IN AND commit < receipt: {:?}", r3.rows);

    // More filters
    let r4 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate").unwrap();
    eprintln!("+ ship < commit: {:?}", r4.rows);

    let r5 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'").unwrap();
    eprintln!("+ receipt >= 1994: {:?}", r5.rows);

    // Just the date filters
    let r6 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'").unwrap();
    eprintln!("receipt 1994 only: {:?}", r6.rows);
}

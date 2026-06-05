//! Check what type l_shipdate actually is in storage

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
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
fn test_shipdate_actual_value() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))").unwrap();
    let n = load_lineitem(&storage);
    eprintln!("loaded {} lineitem rows", n);

    // Get l_shipdate value
    let r = engine.execute("SELECT l_shipdate FROM lineitem LIMIT 5").unwrap();
    eprintln!("first 5 shipdates: {:?}", r.rows);

    // Now compare
    let r2 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01'").unwrap();
    eprintln!("shipdate >= '1994-01-01' (expect ~45000, got 60000): {:?}", r2.rows);

    let r3 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate < '1995-01-01'").unwrap();
    eprintln!("shipdate < '1995-01-01' (expect ~7500, got 60000): {:?}", r3.rows);

    // Check storage directly
    let s = storage.read().unwrap();
    if let Some(rows) = s.scan("lineitem").ok() {
        eprintln!("storage directly - first row: {:?}", rows.first());
        eprintln!("storage directly - l_shipdate value of row 0: {:?}", rows.first().and_then(|r| r.get(10)));
    }
}

//! Q14/Q16 isolation

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
fn diag_q14_q16() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))").unwrap();
    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))").unwrap();
    engine.execute("CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)").unwrap();
    load(&storage, "lineitem", 16);
    load(&storage, "part", 9);
    load(&storage, "partsupp", 5);
    load(&storage, "supplier", 7);

    // Q14: 1995-09 date range - all 1994 data so 0
    let r1 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'").unwrap();
    eprintln!("Q14 step1 shipdate 1995-09: {:?}", r1.rows);
    // This will be 0 because canonical SF=0.01 only has 1994 dates
    // So Q14 legitimately returns 0 rows for SUM

    // Q16: p_size IN
    let r2 = engine
        .execute("SELECT COUNT(*) FROM part WHERE p_size IN (49, 14, 23, 45, 19, 3, 36, 9)")
        .unwrap();
    eprintln!("Q16 step1 part p_size IN: {:?}", r2.rows);

    // Q16: p_brand <>
    let r3 = engine
        .execute("SELECT COUNT(*) FROM part WHERE p_brand <> 'Brand#45'")
        .unwrap();
    eprintln!("Q16 step2 brand not Brand#45: {:?}", r3.rows);

    // Q16: p_type NOT LIKE
    let r4 = engine
        .execute("SELECT COUNT(*) FROM part WHERE p_type NOT LIKE 'MEDIUM POLISHED%'")
        .unwrap();
    eprintln!("Q16 step3 type NOT LIKE: {:?}", r4.rows);

    // Q16: full
    let r5 = engine.execute("SELECT COUNT(*) FROM partsupp, part WHERE p_partkey = ps_partkey AND p_brand <> 'Brand#45' AND p_type NOT LIKE 'MEDIUM POLISHED%' AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9) AND ps_suppkey NOT IN (SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%bad%deals%')").unwrap();
    eprintln!("Q16 full (NOT IN subquery): {:?}", r5.rows);
}

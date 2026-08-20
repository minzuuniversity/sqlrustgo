//! Q12 isolation - CASE WHEN in SUM aggregate with 2-way join

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::sync::Arc;

const FIXTURE: &str = match option_env!("TPCH_DATA_DIR") {
    Some(p) => p,
    None => "tests/data/tpch-sf01",
};

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
            let mut s = storage.write();
            let _ = s.insert(tbl, batch.clone());
            count += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        let mut s = storage.write();
        let _ = s.insert(tbl, batch.clone());
        count += batch.len();
    }
    count
}

#[test]
fn diag_q12_case_when() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber))").unwrap();
    let ord_n = load(&storage, "orders", 9);
    let li_n = load(&storage, "lineitem", 16);
    eprintln!("loaded orders={} lineitem={}", ord_n, li_n);

    // Step 1: count lineitem passing the date/shipmode filter
    let r1 = engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'").unwrap();
    eprintln!(
        "Step 1: filtered lineitem count (expect ~6200): {:?}",
        r1.rows
    );

    // Step 2: count lineitem with join
    let r2 = engine.execute("SELECT COUNT(*) FROM orders, lineitem WHERE l_orderkey = o_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01'").unwrap();
    eprintln!("Step 2: with join count (expect ~6200): {:?}", r2.rows);

    // Step 3: simple CASE WHEN without join
    let r3 = engine
        .execute("SELECT SUM(CASE WHEN o_orderpriority = '1-URGENT' THEN 1 ELSE 0 END) FROM orders")
        .unwrap();
    eprintln!("Step 3: SUM(CASE) on orders (expect ~800): {:?}", r3.rows);

    // Step 4: full Q12
    let r4 = engine.execute(queries::Q12).unwrap_or_else(|e| {
        eprintln!("Q12 ERR: {}", e);
        sqlrustgo::ExecutorResult::new(vec![], 0)
    });
    eprintln!("Step 4: full Q12: {:?}", r4.rows);
}

mod queries {
    pub const Q12: &str = "SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count, SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count FROM orders, lineitem WHERE l_orderkey = o_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode";
}

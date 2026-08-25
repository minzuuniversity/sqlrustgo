//! SF=1 lineitem bulk_load time benchmark — identify storage-side cost.
//! If SF=1 bulk_load takes >1000s, that explains the perceived "hang"
//! (it's actually slow fixture loading, not slow query).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp/tpch-sf1";

fn resolve_data_dir() -> String {
    std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

#[test]
#[ignore]
fn sf1_bulk_load_bench() {
    eprintln!("=== SF=1 bulk_load benchmark ===");
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    let data_dir = resolve_data_dir();

    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)").unwrap();

    eprintln!("part bulk_load START");
    let t0 = Instant::now();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("part", &format!("{}/part.tbl", data_dir))
            .unwrap();
    }
    eprintln!("part bulk_load DONE in {:?}", t0.elapsed());

    let lineitem_path = format!("{}/lineitem.tbl", data_dir);
    eprintln!(
        "lineitem bulk_load START (file size: {} bytes)",
        std::fs::metadata(&lineitem_path)
            .map(|m| m.len())
            .unwrap_or(0)
    );
    let t0 = Instant::now();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("lineitem", &lineitem_path).unwrap();
    }
    eprintln!("lineitem bulk_load DONE in {:?}", t0.elapsed());

    // Sanity check: count lineitem
    let count: usize = storage.read().scan("lineitem").unwrap().len();
    eprintln!("lineitem row count: {}", count);
}

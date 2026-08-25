//! Q17 SF=1 diagnostic test — identifies hang location
//!
//! Run: cargo test --release --test q17_sf1_diag --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage,
};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp/tpch-sf1";

fn resolve_data_dir() -> String {
    std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    let data_dir = resolve_data_dir();

    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)").unwrap();

    eprintln!("=== Loading fixtures ===");
    let t0 = Instant::now();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("part", &format!("{}/part.tbl", data_dir))
            .unwrap();
        eprintln!("part loaded: {:?}", t0.elapsed());
        st.bulk_load_tbl_file("lineitem", &format!("{}/lineitem.tbl", data_dir))
            .unwrap();
        eprintln!("lineitem loaded: {:?}", t0.elapsed());
    }
    engine
}

#[test]
#[ignore]
fn q17_sf1_diag() {
    reset_v312_58_sprint3_diag();
    let mut engine = setup();

    const Q17_SQL: &str = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly \
         FROM lineitem, part \
         WHERE p_partkey = l_partkey \
           AND p_brand = 'Brand#23' \
           AND p_container = 'LG CASE' \
           AND l_quantity < (SELECT 0.2 * AVG(l_quantity) \
                             FROM lineitem \
                             WHERE l_partkey = p_partkey)";

    eprintln!("=== Executing Q17 ===");
    let t_start = Instant::now();
    let r = engine.execute(Q17_SQL).unwrap();
    let elapsed = t_start.elapsed();

    let diag = dump_v312_58_sprint3_diag();

    eprintln!("=== Q17 RESULT ===");
    eprintln!("elapsed: {:?}", elapsed);
    eprintln!("row_count: {}", r.rows.len());
    if !r.rows.is_empty() {
        eprintln!("value: {:?}", r.rows[0][0]);
    }
    eprintln!("{}", diag);
    eprintln!("==================");

    assert!(elapsed.as_secs() < 1800);
    assert_eq!(r.rows.len(), 1);
}

//! TPC-H SF=1 Real Data Test - Simple queries only
//! Run with: cargo test --test tpch_sf1_test -- --nocapture --ignored
//!
//! NOTE: SF=1 requires ~5GB memory. Only run on machines with 16GB+ RAM.

use parking_lot::RwLock;
use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};
use std::path::Path;
use std::sync::Arc;

const TPCK_DATA_DIR: &str = "data/tpch-sf1";

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn setup_schema(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)").unwrap();
}

fn setup_sqlrustgo_engine_sf1_lineitem() -> ExecutionEngine<MemoryStorage> {
    let mut engine = create_engine();
    setup_schema(&mut engine);

    let filepath = format!("{}/lineitem.tbl", TPCK_DATA_DIR);
    if Path::new(&filepath).exists() {
        let mut storage = engine.storage_ref().write();
        match storage.bulk_load_tbl_file("lineitem", &filepath) {
            Ok(count) => println!("Loaded lineitem: {} rows", count),
            Err(e) => println!("Failed to load: {:?}", e),
        }
    } else {
        println!("SF=1 data not found at {}. Run: cd /tmp/tpch-dbgen && ./dbgen -s 1 -f && cp *.tbl ~/workspace/yinglichina/sqlrustgo/data/tpch-sf1/", TPCK_DATA_DIR);
    }

    engine
}

#[test]
#[ignore = "tpch_sf1_test: SF=1 requires ~5GB memory, may OOM on 16GB systems; run with --ignored on high-memory machines"]
fn test_sqlrustgo_sf1_count() {
    let mut engine = setup_sqlrustgo_engine_sf1_lineitem();

    // COUNT(*) query
    println!("\nCOUNT(*) query (SF=1, 6M rows):");
    let start = std::time::Instant::now();
    let result = engine.execute("SELECT COUNT(*) FROM lineitem");
    let elapsed = start.elapsed();

    match result {
        Ok(rows) => {
            println!("COUNT(*): {:?} rows in {:?}", rows.rows.len(), elapsed);
            for row in &rows.rows {
                println!("  {:?}", row);
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

#[test]
#[ignore = "tpch_sf1_test: SF=1 requires ~5GB memory, may OOM on 16GB systems; run with --ignored on high-memory machines"]
fn test_sqlrustgo_sf1_sum_filtered() {
    let mut engine = setup_sqlrustgo_engine_sf1_lineitem();

    // SUM with filter
    println!("\nSUM(l_quantity) with filter (SF=1):");
    let start = std::time::Instant::now();
    let result = engine.execute("SELECT SUM(l_quantity) FROM lineitem WHERE l_quantity < 10");
    let elapsed = start.elapsed();

    match result {
        Ok(rows) => {
            println!("SUM: {:?} in {:?}", rows.rows, elapsed);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}

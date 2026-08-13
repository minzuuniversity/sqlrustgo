//! TPC-H Index Usage Test
//! Run with: cargo test --test tpch_index_test -- --nocapture

use parking_lot::RwLock;
use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};
use std::path::Path;
use std::sync::Arc;

const TPCK_DATA_DIR: &str = "data/tpch-sf03";

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn setup_schema(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)").unwrap();
}

#[test]
fn test_index_usage() {
    let mut engine = create_engine();
    setup_schema(&mut engine);

    // Load only first 1000 rows for quick test
    let filepath = format!("{}/lineitem.tbl", TPCK_DATA_DIR);
    if Path::new(&filepath).exists() {
        let load_result = {
            let mut storage = engine.storage_ref().write();
            storage.bulk_load_tbl_file("lineitem", &filepath)
        };
        match load_result {
            Ok(count) => println!("Loaded {} rows", count),
            Err(e) => {
                println!("bulk_load failed: {:?}", e);
            }
        }
    }

    // Indexes are auto-created by bulk_load

    // Test: Simple query with WHERE on indexed column
    println!("\nTest 1: WHERE l_quantity = 10");
    let start = std::time::Instant::now();
    match engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_quantity = 10") {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("  Result: {:?} in {:?}", result.rows, elapsed);
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Test: Range query
    println!("\nTest 2: WHERE l_quantity < 10");
    let start = std::time::Instant::now();
    match engine.execute("SELECT COUNT(*) FROM lineitem WHERE l_quantity < 10") {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("  Result: {:?} in {:?}", result.rows, elapsed);
        }
        Err(e) => println!("  Error: {:?}", e),
    }

    // Test: Full scan (no WHERE)
    println!("\nTest 3: COUNT(*) without WHERE (full scan)");
    let start = std::time::Instant::now();
    match engine.execute("SELECT COUNT(*) FROM lineitem") {
        Ok(result) => {
            let elapsed = start.elapsed();
            println!("  Result: {:?} in {:?}", result.rows, elapsed);
        }
        Err(e) => println!("  Error: {:?}", e),
    }
}

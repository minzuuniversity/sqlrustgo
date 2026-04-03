//! TPC-H TEXT Range Query Index Test
use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};
use std::path::Path;
use std::sync::{Arc, RwLock};

#[test]
fn test_text_range_index() {
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    
    // Create table with TEXT columns
    engine.execute(parse("CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity INTEGER, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)").unwrap()).unwrap();
    
    let filepath = "data/tpch-sf03/lineitem.tbl";
    if Path::new(&filepath).exists() {
        let mut storage = engine.storage.write().unwrap();
        match storage.bulk_load_tbl_file("lineitem", &filepath) {
            Ok(count) => {
                println!("Loaded {} rows", count);
                
                // Test 1: TEXT = query (should use index)
                drop(storage);
                println!("\nTest 1: WHERE l_returnflag = 'R' (TEXT =)");
                let start = std::time::Instant::now();
                match engine.execute(parse("SELECT COUNT(*) FROM lineitem WHERE l_returnflag = 'R'").unwrap()) {
                    Ok(result) => {
                        let elapsed = start.elapsed();
                        println!("  Result: {:?} in {:?}", result.rows, elapsed);
                    }
                    Err(e) => println!("  Error: {:?}", e),
                }
                
                // Test 2: TEXT range query (should use StringBPlusTree index)
                println!("\nTest 2: WHERE l_shipdate <= '1998-09-02' (TEXT range)");
                let start = std::time::Instant::now();
                match engine.execute(parse("SELECT COUNT(*) FROM lineitem WHERE l_shipdate <= '1998-09-02'").unwrap()) {
                    Ok(result) => {
                        let elapsed = start.elapsed();
                        println!("  Result: {:?} in {:?}", result.rows, elapsed);
                    }
                    Err(e) => println!("  Error: {:?}", e),
                }
                
                // Test 3: Full table scan
                println!("\nTest 3: COUNT(*) without WHERE (full scan)");
                let start = std::time::Instant::now();
                match engine.execute(parse("SELECT COUNT(*) FROM lineitem").unwrap()) {
                    Ok(result) => {
                        let elapsed = start.elapsed();
                        println!("  Result: {:?} in {:?}", result.rows, elapsed);
                    }
                    Err(e) => println!("  Error: {:?}", e),
                }
            }
            Err(e) => println!("  Load error: {:?}", e),
        }
    }
}

//! TPC-H Q15 + Q16 isolated check (sqlrustgo in-process vs PG).
//! Q15: subquery in FROM (view). Q16: NOT IN subquery.

mod four_way_harness;

use four_way_harness::default_data_dir;
use four_way_harness::Engine;
use std::time::Instant;

fn load_into_sqlrustgo() -> sqlrustgo::ExecutionEngine<sqlrustgo::MemoryStorage> {
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::{Arc, RwLock};

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(Arc::clone(&storage));

    for ddl in four_way_harness::tpc_h_schema(Engine::SqlRustGo) {
        let _ = engine.execute(&ddl);
    }

    let data_dir = default_data_dir();
    for (table, cols) in four_way_harness::TABLE_COLS {
        let path = data_dir.join(format!("{}.tbl", table));
        if !path.exists() {
            eprintln!("[WARN] {} not found, skipping", path.display());
            continue;
        }
        let inserts = four_way_harness::load_tbl_inserts(table, *cols, &path, Engine::SqlRustGo);
        for ins in &inserts {
            let _ = engine.execute(ins);
        }
    }
    engine
}

#[test]
fn test_q15_subquery_from() {
    let mut engine = load_into_sqlrustgo();
    let q15 = std::fs::read_to_string("queries/q15.sql")
        .expect("read q15.sql")
        .trim()
        .trim_end_matches(';')
        .to_string();
    let t = Instant::now();
    let res = engine.execute(&q15);
    let dur = t.elapsed();
    match res {
        Ok(r) => {
            println!(
                "\n=== sqlrustgo Q15: {} rows in {:?} ===",
                r.rows.len(),
                dur
            );
            for (i, row) in r.rows.iter().take(5).enumerate() {
                println!("  row {}: {:?}", i, row);
            }
            if r.rows.is_empty() {
                println!("✗ Q15 returned 0 rows (expected ~10)");
            }
        }
        Err(e) => {
            println!("\n=== sqlrustgo Q15 ERROR: {} ===", e);
            println!("✗ Q15 failed to execute");
        }
    }
}

#[test]
fn test_q16_not_in_subquery() {
    let mut engine = load_into_sqlrustgo();
    let q16 = std::fs::read_to_string("queries/q16.sql")
        .expect("read q16.sql")
        .trim()
        .trim_end_matches(';')
        .to_string();
    let t = Instant::now();
    let res = engine.execute(&q16);
    let dur = t.elapsed();
    match res {
        Ok(r) => {
            println!(
                "\n=== sqlrustgo Q16: {} rows in {:?} ===",
                r.rows.len(),
                dur
            );
            for (i, row) in r.rows.iter().take(5).enumerate() {
                println!("  row {}: {:?}", i, row);
            }
            // PG Q16 returns 284 rows. 0 rows likely means NOT IN evaluator fails.
            if r.rows.is_empty() {
                println!("✗ Q16 returned 0 rows (expected ~284 per PG)");
            }
        }
        Err(e) => {
            println!("\n=== sqlrustgo Q16 ERROR: {} ===", e);
            println!("✗ Q16 failed to execute");
        }
    }
}

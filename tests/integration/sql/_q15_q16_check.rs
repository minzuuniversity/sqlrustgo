//! TPC-H Q15 + Q16 isolated check (sqlrustgo in-process vs PG).
//! Q15: subquery in FROM (view). Q16: NOT IN subquery.

mod four_way_harness;

// The shared TPC-H fixture loader (issue #5379). Loading through the
// storage layer in batches instead of executing one INSERT statement per
// row is ~2 orders of magnitude faster; see the comment at the load site.
#[path = "../../common/tpch_fixture.rs"]
mod tpch_fixture;

use four_way_harness::default_data_dir;
use four_way_harness::Engine;
use std::time::Instant;

fn load_into_sqlrustgo() -> sqlrustgo::ExecutionEngine<sqlrustgo::MemoryStorage> {
    use parking_lot::RwLock;
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::Arc;

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
        // Load through the storage layer in 10k-row batches instead of
        // executing one INSERT statement per row.
        //
        // The statement-per-row path issues ~86,000 INSERTs per load and
        // costs ~1 hour per load in a debug build. Before issue #5379 the
        // fixture was a dangling symlink, so this target skipped the load
        // entirely and finished in milliseconds — the cost was invisible.
        // Now that the fixture is generated at run time and reachable, the
        // same target needs ~2 hours of CI time, which does not fit the
        // `test` job's budget and would starve every other target.
        //
        // The two loaders are equivalent: the shared loader derives the same
        // typed values (NULL / Integer / Float / Text) from the same TBL
        // text, verified cell-by-cell over all 8 tables and 86,482 rows.
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let rows = tpch_fixture::parse_tbl(&content);
        let _ = tpch_fixture::load_rows_into_storage(&storage, &rows, table, *cols);
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

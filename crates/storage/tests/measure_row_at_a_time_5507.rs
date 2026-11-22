//! #5507 — measurement harness: a row-at-a-time load must not be quadratic.
//!
//! This is a **measurement**, not a gate. It is `#[ignore]`d on purpose: a
//! wall-clock assertion inside the normal suite is exactly the pattern that
//! made `wal_legacy::tests::test_wal_perf_1000_insert`
//! (`crates/storage/src/wal_legacy.rs:1473`) fail intermittently under CPU
//! contention. The numbers this prints are recorded in the PR discussion and
//! re-read after the change, rather than re-asserted on every CI run.
//!
//! Run with:
//!
//! ```text
//! cargo test -p sqlrustgo-storage --all-features \
//!   --test measure_row_at_a_time_5507 -- --ignored --nocapture
//! ```
//!
//! ## What it measures
//!
//! The shape from #5412: one transaction per inserted row. Every commit used
//! to publish the post-commit view by cloning the whole table map, which
//! shared every table's `Arc` between the live map and the committed view —
//! so the **next** write had to detach (deep-copy) that whole table. N rows
//! therefore cost O(N) copies of an O(N)-row table: O(N²) in total.
//!
//! After #5507 the commit publishes the transaction's own delta, so the live
//! and the committed view never share storage and a write copies nothing.
//!
//! Read the table as: does the per-row cost grow with the number of rows
//! already inserted? A quadratic implementation roughly quadruples the time
//! when the row count doubles; a linear one roughly doubles it.

use sqlrustgo_storage::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo};
use sqlrustgo_types::Value;
use std::time::Instant;

fn build() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    let info = TableInfo {
        name: "t1".to_string(),
        columns: vec![ColumnDefinition::new("id", "INTEGER")],
        ..Default::default()
    };
    s.create_table(&info).unwrap();
    s
}

fn row_at_a_time(s: &mut MemoryStorage, rows: usize) -> f64 {
    let start = Instant::now();
    for i in 0..rows {
        s.begin_transaction().unwrap();
        s.insert("t1", vec![vec![Value::Integer(i as i64)]])
            .unwrap();
        s.commit_transaction().unwrap();
    }
    start.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "measurement, not a gate — run with --ignored --nocapture"]
fn measure_row_at_a_time_commit_cost_5507() {
    println!("rows      total ms      ms/row");
    println!("------------------------------------------------");
    let mut previous: Option<(usize, f64)> = None;
    for rows in [500usize, 1000, 2000, 4000] {
        let mut s = build();
        let ms = row_at_a_time(&mut s, rows);
        let per_row = ms / rows as f64;
        println!(
            "{rows:<9} {ms:>9.3}    {per_row:>7.4}   (rows in table: {})",
            s.scan("t1").unwrap().len()
        );
        if let Some(prev) = previous {
            let growth = ms / prev.1;
            println!(
                "           growth vs previous size: {growth:.2}x  \
                 (quadratic ≈ 4x, linear ≈ 2x)"
            );
        }
        previous = Some((rows, ms));
    }
}

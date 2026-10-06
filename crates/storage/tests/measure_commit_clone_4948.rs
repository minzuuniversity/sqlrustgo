//! Issue #4948 — measurement harness for the COMMIT deep-copy.
//!
//! This is a **measurement**, not a gate. It is `#[ignore]`d on purpose:
//! a wall-clock assertion inside the normal suite is exactly the pattern
//! that made `wal_legacy::tests::test_wal_perf_1000_insert`
//! (`crates/storage/src/wal_legacy.rs:1473`) fail intermittently under CPU
//! contention. The numbers this prints are recorded in
//! `docs/releases/v4.1.0/evidence/` and re-read after the change, rather
//! than re-asserted on every CI run.
//!
//! Run with:
//!
//! ```text
//! cargo test -p sqlrustgo-storage --all-features \
//!   --test measure_commit_clone_4948 -- --ignored --nocapture
//! ```
//!
//! ## What it measures
//!
//! `MemoryStorage::commit_transaction_with_log` ends with
//!
//! ```ignore
//! self.committed_tables = self.tables.clone();
//! ```
//!
//! O(total rows in the database) **per COMMIT**, regardless of how little
//! the transaction actually touched. The harness loads N rows across T
//! tables, then opens and closes an **empty** transaction. An empty
//! transaction does no work, so whatever time is spent is the snapshot
//! cost and nothing else.
//!
//! Read the table as: does `commit ms` track `total rows`? If it does, the
//! clone is O(database size) per COMMIT, which is what #4948 asks to fix.

use sqlrustgo_storage::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo};
use sqlrustgo_types::Value;
use std::time::Instant;

fn build(tables: usize, rows_per_table: usize, cols: usize) -> MemoryStorage {
    let mut s = MemoryStorage::new();
    for t in 0..tables {
        let name = format!("t{}", t);
        let columns: Vec<ColumnDefinition> = (0..cols)
            .map(|c| ColumnDefinition {
                name: format!("c{}", c),
                data_type: "INT".to_string(),
                nullable: true,
                ..Default::default()
            })
            .collect();
        s.create_table(&TableInfo {
            name: name.clone(),
            columns,
            ..Default::default()
        })
        .unwrap();

        // Batched so setup does not dominate the run.
        let batch: Vec<Vec<Value>> = (0..rows_per_table)
            .map(|r| {
                (0..cols)
                    .map(|c| Value::Integer(((r * cols + c) % 1000) as i64))
                    .collect()
            })
            .collect();
        s.insert(&name, batch).unwrap();
    }
    s
}

#[test]
#[ignore = "wall-clock measurement harness — run manually, results go in the evidence doc"]
fn measure_commit_clone_cost_4948() {
    println!(
        "\n{:<8} {:<12} {:<12} {:<12}",
        "tables", "rows/table", "total rows", "commit ms"
    );
    println!("{}", "-".repeat(48));

    let shapes = [
        (1usize, 1_000usize, 4usize),
        (10, 1_000, 4),
        (10, 10_000, 4),
        (50, 10_000, 4),
    ];

    for (tables, rows, cols) in shapes {
        let mut s = build(tables, rows, cols);

        // Warm up once so the reported number is not dominated by
        // first-touch page faults on the freshly built maps.
        s.begin_transaction().unwrap();
        s.commit_transaction_with_log();

        let reps = 5;
        let mut best = f64::MAX;
        for _ in 0..reps {
            s.begin_transaction().unwrap();
            let start = Instant::now();
            s.commit_transaction_with_log();
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed < best {
                best = elapsed;
            }
        }
        println!(
            "{:<8} {:<12} {:<12} {:<12.3}",
            tables,
            rows,
            tables * rows,
            best * 1000.0
        );
    }
    println!();
}

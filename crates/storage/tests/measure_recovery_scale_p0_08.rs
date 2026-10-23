//! P0-08 measurement harness — WAL recovery replays one row at a time.
//!
//! # What this measures
//!
//! `apply_wal_entry` applies one WAL entry at a time. For an `Insert`
//! entry it does two things, and both are per-entry:
//!
//! 1. **dedup** — `storage.scan_in_db(db, table)` and then
//!    `existing.iter().any(|r| r == &record)` — a **full scan of the table**
//!    for every replayed row;
//! 2. **write** — `recovery_force_insert_in_db` → `FileStorage::insert_direct`
//!    → `save_table_window` → `save_table_full` (a whole-table JSON rewrite)
//!    or the delta-append path.
//!
//! Both are O(rows-so-far), so replaying N rows costs O(N²) work. The
//! defect report measured 58 288 INSERT entries against a 68 288-row table
//! and a first full-table query of **24 min 22 s** after a restart.
//!
//! # Why this is a measurement, not a test
//!
//! A wall-clock assertion is not a regression guard: it is a coin flip on a
//! loaded machine. What *is* a guard is the **shape** of the cost — whether
//! replaying N rows costs O(N) or O(N²). This harness prints the per-row
//! cost at two table sizes; if the per-row cost grows with the table, the
//! quadratic behaviour is back.
//!
//! Run it manually:
//!
//! ```text
//! SQLRUSTGO_RECOVERY_ROWS=2000 cargo test -p sqlrustgo-storage \
//!     --test measure_recovery_scale_p0_08 -- --ignored --nocapture
//! ```
//!
//! The numbers it prints are recorded in
//! `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`.

use std::path::PathBuf;
use std::time::Instant;

use sqlrustgo_storage::engine::{ColumnDefinition, TableData, TableInfo};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::recovery_engine::{RecoveryEngine, RecoveryEngineImpl};
use sqlrustgo_storage::wal::{MemoryWalManager, WalEntry, WalEntryType, WalManager};
use sqlrustgo_storage::wal_record_codec::record_to_bytes;
use sqlrustgo_storage::{Record, StorageEngine, Value};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_p008_{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_table_info(name: &str, cols: &[&str]) -> TableInfo {
    TableInfo {
        name: name.into(),
        columns: cols
            .iter()
            .map(|&c| ColumnDefinition {
                name: c.into(),
                data_type: "INTEGER".into(),
                nullable: false,
                primary_key: c == "id",
                char_max_length: None,
                collation: None,
                default_value: None,
                auto_increment: false,
            })
            .collect(),
        ..Default::default()
    }
}

fn row(i: u64) -> Record {
    vec![Value::Integer(i as i64), Value::Integer((i % 97) as i64)]
}

/// How many rows to replay. Overridable so the same harness can produce a
/// quick signal locally and a full-scale number for the evidence document.
fn rows_from_env() -> u64 {
    std::env::var("SQLRUSTGO_RECOVERY_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000)
}

/// The per-row cost of the dedup scan, measured on its own.
///
/// `apply_wal_entry` calls `scan_in_db` once per replayed `Insert`. This
/// measures what that call costs on a table of `n` rows, with no writes in
/// between, so the growth curve is attributable to the scan alone.
#[test]
#[ignore = "wall-clock measurement harness — run manually, results go in the evidence doc"]
fn measure_recovery_dedup_scan_cost_p0_08() {
    let n = rows_from_env();
    let dir = temp_dir("dedup");
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    let info = make_table_info("t", &["id", "v"]);
    storage
        .insert_table("t".into(), TableData { info, rows: vec![] })
        .unwrap();

    // Seed the table without paying the scan cost.
    for i in 0..n {
        storage.force_insert_in_db("default", "t", row(i)).unwrap();
    }

    let start = Instant::now();
    let mut seen = 0usize;
    for i in 0..n {
        let existing = storage.scan_in_db("default", "t").unwrap();
        if existing.iter().any(|r| r == &row(i)) {
            seen += 1;
        }
    }
    let elapsed = start.elapsed();
    let per_row_us = elapsed.as_secs_f64() * 1e6 / n as f64;
    eprintln!(
        "[P0-08 dedup] rows={} matched={} total={:?} per_row={:.2}us \
         projected_58288_rows={:.1}s",
        n,
        seen,
        elapsed,
        per_row_us,
        per_row_us * 58_288.0 / 1e6
    );
    assert_eq!(seen as u64, n, "every seeded row must be found by the scan");

    let _ = std::fs::remove_dir_all(&dir);
}

/// The per-row cost of the whole replay shape: dedup scan + write, exactly
/// as `apply_wal_entry` does it for an `Insert` entry.
#[test]
#[ignore = "wall-clock measurement harness — run manually, results go in the evidence doc"]
fn measure_recovery_replay_cost_p0_08() {
    let n = rows_from_env();
    let dir = temp_dir("replay");
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    let info = make_table_info("t", &["id", "v"]);
    storage
        .insert_table("t".into(), TableData { info, rows: vec![] })
        .unwrap();

    let start = Instant::now();
    let mut applied = 0usize;
    for i in 0..n {
        let record = row(i);
        // The dedup step `apply_wal_entry` performs before writing.
        if let Ok(existing) = storage.scan_in_db("default", "t") {
            if existing.iter().any(|r| r == &record) {
                continue;
            }
        }
        // The write step.
        storage.force_insert_in_db("default", "t", record).unwrap();
        applied += 1;
    }
    let elapsed = start.elapsed();
    let per_row_us = elapsed.as_secs_f64() * 1e6 / n as f64;
    eprintln!(
        "[P0-08 replay] rows={} applied={} total={:?} per_row={:.2}us \
         projected_58288_rows={:.1}s",
        n,
        applied,
        elapsed,
        per_row_us,
        per_row_us * 58_288.0 / 1e6
    );
    assert_eq!(applied as u64, n);

    let _ = std::fs::remove_dir_all(&dir);
}

/// The per-row cost of the write path alone, with no scan in the loop.
///
/// Isolates `insert_direct` → `save_table_window`: if the whole-table
/// rewrite is taken on every row, this is O(N²) in file bytes written; if
/// the delta-append path is taken, it is O(N).
#[test]
#[ignore = "wall-clock measurement harness — run manually, results go in the evidence doc"]
fn measure_recovery_write_cost_p0_08() {
    let n = rows_from_env();
    let dir = temp_dir("write");
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    let info = make_table_info("t", &["id", "v"]);
    storage
        .insert_table("t".into(), TableData { info, rows: vec![] })
        .unwrap();

    let start = Instant::now();
    for i in 0..n {
        storage
            .force_insert_in_db("default", "t", row(i))
            .unwrap();
    }
    let elapsed = start.elapsed();
    let per_row_us = elapsed.as_secs_f64() * 1e6 / n as f64;
    eprintln!(
        "[P0-08 write] rows={} total={:?} per_row={:.2}us projected_58288_rows={:.1}s",
        n,
        elapsed,
        per_row_us,
        per_row_us * 58_288.0 / 1e6
    );

    // How much did the table actually grow on disk? A whole-table rewrite
    // per row leaves a `.delta` file that is never used and a base snapshot
    // rewritten N times; the delta path leaves a growing `.delta` file.
    let mut files: Vec<(String, u64)> = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(meta) = e.metadata() {
                    files.push((p.display().to_string(), meta.len()));
                }
            }
        }
    }
    files.sort();
    for (p, len) in &files {
        eprintln!("[P0-08 files] {} = {} bytes", p, len);
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The real thing: a restart replays the WAL through
/// `RecoveryEngine::recover`.
///
/// This is the path P0-08 is about, and the one the row index changes.
/// The three measurements above attribute the cost; this one is the
/// end-to-end number the evidence document quotes.
#[test]
#[ignore = "wall-clock measurement harness — run manually, results go in the evidence doc"]
fn measure_recovery_wal_replay_p0_08() {
    let n = rows_from_env();
    let dir = temp_dir("walreplay");
    let mut storage = FileStorage::new(dir.clone()).unwrap();
    let info = make_table_info("t", &["id", "v"]);
    storage
        .insert_table("t".into(), TableData { info, rows: vec![] })
        .unwrap();

    let mut wal = MemoryWalManager::new();
    for i in 0..n {
        wal.append(WalEntry {
            tx_id: 0,
            entry_type: WalEntryType::Insert,
            table_id: 0,
            key: None,
            data: Some(record_to_bytes(&row(i))),
            lsn: i + 1,
            timestamp: 0,
            table_name: Some("t".into()),
            database_name: Some("default".into()),
        })
        .unwrap();
    }

    let mut engine = RecoveryEngineImpl;
    let start = Instant::now();
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    let elapsed = start.elapsed();
    let per_row_us = elapsed.as_secs_f64() * 1e6 / n as f64;

    let rows = storage.scan_in_db("default", "t").unwrap();
    eprintln!(
        "[P0-08 wal-replay] entries={} replayed={} rows_in_table={} skipped={} \
         total={:?} per_row={:.2}us projected_58288_rows={:.1}s",
        report.entries_total,
        report.rows_inserted,
        rows.len(),
        report.skipped_entries,
        elapsed,
        per_row_us,
        per_row_us * 58_288.0 / 1e6
    );
    assert_eq!(
        rows.len() as u64, n,
        "every replayed row must be in the table"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

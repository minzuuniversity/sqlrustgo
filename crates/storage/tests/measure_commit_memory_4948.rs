//! Issue #4948 acceptance criterion 2 — memory cost *of the commit itself*.
//!
//! Criterion 1 (COMMIT no longer deep-copies) is proven by wall clock in
//! `measure_commit_clone_4948.rs`. This file measures how much resident
//! memory the commit adds.
//!
//! ## Why peak RSS is the wrong metric — and what this measures instead
//!
//! The obvious approach (`/usr/bin/time -l` → `maximum resident set size`)
//! produces **no difference at all** between the pre- and post-#4948 code.
//! Measured, 10 tables x 50 000 rows:
//!
//! ```text
//! pre-Arc  : 484.2 / 483.0 MB
//! post-Arc : 483.0 / 483.0 / 482.9 / 483.0 / 484.1 MB
//! ```
//!
//! The reason: `MemoryStorage::insert` on the **autocommit** path already
//! writes both `tables` and `committed_tables`
//!
//! ```ignore
//! if let Some(log) = self.tx_log.as_mut() { ... } else {
//!     committed_tables.entry(..).or_default().extend(padded.iter().cloned());
//! }
//! tables.entry(..).or_default().extend(padded);
//! ```
//!
//! so the high-water mark is already reached while the fixture is being
//! built — at roughly 2x the dataset — and the copy the commit performs on
//! top of that never raises the peak. Peak RSS therefore cannot see the
//! thing this issue is about.
//!
//! What the issue actually asks ("当前常驻约 2×") is how much the commit
//! adds to the resident set. `getrusage(RUSAGE_SELF).ru_maxrss` is a
//! monotonic high-water mark, so sampling it **immediately before and
//! immediately after** the commit isolates exactly that: any newly
//! allocated buffer beyond the pre-commit peak shows up as the delta.
//!
//! No `libc` dependency — the C struct is declared inline. `ru_maxrss` is in
//! bytes on macOS (kilobytes on Linux).

use sqlrustgo_storage::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo};
use sqlrustgo_types::Value;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct Rusage {
    ru_utime: Timeval,
    ru_stime: Timeval,
    ru_maxrss: i64,
    ru_ixrss: i64,
    ru_idrss: i64,
    ru_isrss: i64,
    ru_minflt: i64,
    ru_majflt: i64,
    ru_nswap: i64,
    ru_inblock: i64,
    ru_oublock: i64,
    ru_msgsnd: i64,
    ru_msgrcv: i64,
    ru_nsignals: i64,
    ru_nvcsw: i64,
    ru_nivcsw: i64,
}

extern "C" {
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

const RUSAGE_SELF: i32 = 0;

/// High-water resident set of this process, in bytes.
fn max_rss() -> i64 {
    let mut ru = Rusage::default();
    // SAFETY: `ru` is a correctly sized, properly aligned `rusage` and the
    // return value is only consulted to detect failure.
    let rc = unsafe { getrusage(RUSAGE_SELF, &mut ru as *mut Rusage) };
    assert_eq!(rc, 0, "getrusage failed");
    ru.ru_maxrss
}

fn rows_per_table() -> usize {
    std::env::var("SQLRUSTGO_4948_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000)
}

fn build(tables: usize, rows: usize, cols: usize) -> MemoryStorage {
    let mut s = MemoryStorage::new();
    for t in 0..tables {
        let name = format!("t{}", t);
        let columns: Vec<ColumnDefinition> = (0..cols)
            .map(|c| ColumnDefinition {
                name: format!("c{}", c),
                data_type: "TEXT".to_string(),
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

        // TEXT payloads so a row costs a realistic number of bytes rather
        // than a couple of inlined integers.
        let batch: Vec<Vec<Value>> = (0..rows)
            .map(|r| {
                (0..cols)
                    .map(|c| Value::Text(format!("{}-{}-{}", name, r, "x".repeat(24))))
                    .collect()
            })
            .collect();
        s.insert(&name, batch).unwrap();
    }
    s
}

#[test]
#[ignore = "memory measurement — run manually; prints the commit's RSS delta"]
fn measure_commit_memory_4948() {
    const TABLES: usize = 10;
    const COLS: usize = 4;
    let rows = rows_per_table();

    let mut s = build(TABLES, rows, COLS);
    let live: usize = (0..TABLES)
        .map(|t| s.scan(&format!("t{}", t)).unwrap().len())
        .sum();
    assert_eq!(live, TABLES * rows, "fixture built as expected");

    // Everything above is identical in both versions. The commit is the only
    // thing measured.
    let before = max_rss();
    s.begin_transaction().unwrap();
    s.commit_transaction_with_log();
    let after = max_rss();

    let mb = |b: i64| b as f64 / (1024.0 * 1024.0);
    println!(
        "\n4948 commit memory: {TABLES} tables x {rows} rows x {COLS} cols = {} rows",
        TABLES * rows
    );
    println!("  peak RSS before commit : {:>9.1} MB", mb(before));
    println!("  peak RSS after  commit : {:>9.1} MB", mb(after));
    println!(
        "  DELTA attributable to the commit : {:>9.1} MB",
        mb(after - before)
    );

    assert!(s.scan("t0").unwrap().len() > 0, "storage still usable");
}

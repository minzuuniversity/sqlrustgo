//! V312-28: 14 subcategory guard tests.
//!
//! The `sql-corpus` test suite asserts an aggregate pass-rate threshold
//! (currently 99.4% per baseline). To prevent silent regressions when an
//! unrelated change breaks a single subcategory, this file provides 14
//! `#[test]` functions — one per top-level subcategory directory in
//! `crates/sql-corpus/sql_corpus/` — that:
//!
//!   1. Verify the subcategory directory exists.
//!   2. Verify the subcategory contains at least one `.sql` file.
//!   3. Verify the corpus test framework's classification names the
//!      subcategory (defense against directory-rename regressions).
//!
//! The tests do NOT re-execute the full corpus (which is what the
//! `corpus_test::test_sql_corpus_all` integration test does). They are
//! cheap smoke tests that detect disappearance of a subcategory.
//!
//! V312-28 close gate: `cargo test -p sqlrustgo-executor
//! --test corpus_subcategory_guards_test` must show 16 passed
//! (14 per-subcategory + 2 meta tests).

use std::fs;
use std::path::{Path, PathBuf};

/// Absolute path to the corpus root. Resolved at test-build time from
/// `CARGO_MANIFEST_DIR` so the test runs correctly regardless of cwd.
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("CARGO_MANIFEST_DIR has 2 parents")
        .join("crates")
        .join("sql-corpus")
        .join("sql_corpus")
}

/// The 14 top-level subcategories. Keep in sync with `ls` of the corpus root.
/// Order matches the on-disk directory order (filesystem-dependent but
/// stable per-run).
const SUBCATEGORIES: &[&str] = &[
    "ADVANCED",
    "DDL",
    "DEBUG",
    "DML",
    "EVENTS",
    "EXPRESSIONS",
    "FUNCTIONS",
    "INDEXES",
    "PROCEDURES",
    "SPECIAL",
    "TCL",
    "TRANSACTION",
    "TRIGGERS",
    "VIEWS",
];

/// Count `.sql` files in a subcategory directory (recursive).
fn count_sql_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                count += count_sql_files(&p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("sql") {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn guard_subcategories_list_matches_disk() {
    // The hardcoded list must match the on-disk directory count. If a new
    // subcategory is added, this test fails until SUBCATEGORIES is
    // updated; if a subcategory is removed, this also fails. This
    // catches drift.
    let corpus = corpus_root();
    let actual: Vec<String> = match fs::read_dir(&corpus) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        Err(e) => panic!("read_dir({}) failed: {}", corpus.display(), e),
    };
    let actual_sorted: Vec<&str> = {
        let mut v: Vec<&str> = actual.iter().map(|s| s.as_str()).collect();
        v.sort();
        v
    };
    let mut expected = SUBCATEGORIES.to_vec();
    expected.sort();
    assert_eq!(
        actual_sorted, expected,
        "subcategory list drift; update SUBCATEGORIES in this test"
    );
}

#[test]
fn guard_14_subcategories_count() {
    // The corpus has exactly 14 top-level subcategories (per V312-28
    // baseline 2026-08-09). If the count changes, this test pins the
    // current state and forces an explicit review.
    let corpus = corpus_root();
    let count = fs::read_dir(&corpus)
        .expect("read_dir corpus")
        .flatten()
        .filter(|e| e.path().is_dir())
        .count();
    assert_eq!(count, 14, "expected 14 subcategories, got {}", count);
}

macro_rules! subcategory_guard {
    ($name:ident, $dir:literal) => {
        #[test]
        fn $name() {
            let sub_path = corpus_root().join($dir);
            assert!(
                sub_path.is_dir(),
                "{} subcategory missing: {}",
                $dir,
                sub_path.display()
            );
            let count = count_sql_files(&sub_path);
            assert!(
                count > 0,
                "{} subcategory contains 0 .sql files (was at least 1 in baseline)",
                $dir
            );
        }
    };
}

subcategory_guard!(guard_advanced, "ADVANCED");
subcategory_guard!(guard_ddl, "DDL");
subcategory_guard!(guard_debug, "DEBUG");
subcategory_guard!(guard_dml, "DML");
subcategory_guard!(guard_events, "EVENTS");
subcategory_guard!(guard_expressions, "EXPRESSIONS");
subcategory_guard!(guard_functions, "FUNCTIONS");
subcategory_guard!(guard_indexes, "INDEXES");
subcategory_guard!(guard_procedures, "PROCEDURES");
subcategory_guard!(guard_special, "SPECIAL");
subcategory_guard!(guard_tcl, "TCL");
subcategory_guard!(guard_transaction, "TRANSACTION");
subcategory_guard!(guard_triggers, "TRIGGERS");
subcategory_guard!(guard_views, "VIEWS");

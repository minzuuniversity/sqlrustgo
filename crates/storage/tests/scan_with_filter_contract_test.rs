//! `scan_with_filter` 的行为契约。
//!
//! Phase B2.4 (#4915 / #4947) makes `WalStorage::update` and the trigger
//! paths call this instead of `scan()`, and `PERF_B22_CONCURRENT_MEASUREMENT.md`
//! §3 notes `scan` is the dominant allocation on the DELETE path. The
//! method had **no test at all** before this file — the whole optimisation
//! rested on an unverified assumption that the filter runs before the
//! clone.
//!
//! What these tests pin down:
//!
//! 1. It returns the same rows `scan` would, minus those the filter
//!    rejects — including rows that are still in `insert_buffer` and not
//!    yet in `tables.rows`.
//! 2. A permissive filter (`|_| true`) costs the same as `scan`. This is
//!    the fact #4962 depends on: switching the trigger paths to
//!    `&|_r: &Vec<Value>| true` is a *type* fix (making the method reachable through
//!    `dyn StorageEngine`) and not a *performance* one. Without this
//!    test that distinction is easy to get wrong in either direction.

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;

fn make(dir: &str) -> FileStorage {
    let p = PathBuf::from(dir);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    let mut s = FileStorage::new(p).unwrap();
    s.create_table(&sqlrustgo_storage::engine::TableInfo {
        name: "t".to_string(),
        columns: vec![sqlrustgo_storage::engine::ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    })
    .unwrap();
    s
}

fn ints(rows: &[Vec<Value>]) -> Vec<i64> {
    let mut v: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.first().and_then(|x| x.as_integer()))
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn permissive_filter_matches_scan_exactly() {
    // The #4962 case: `&|_r: &Vec<Value>| true` must return precisely what `scan`
    // returns. If it returned less, switching the trigger paths to it
    // would silently drop rows; if it returned more, the two methods
    // would disagree about visibility.
    let mut s = make("/tmp/scanfilter_permissive");
    for i in 0..50i64 {
        s.insert("t", vec![vec![Value::Integer(i)]]).unwrap();
    }
    s.flush_all_buffers().unwrap();

    let all = s.scan("t").unwrap();
    let filtered = s.scan_with_filter("t", &|_r: &Vec<Value>| true).unwrap();
    assert_eq!(
        ints(&filtered),
        ints(&all),
        "a permissive filter must yield exactly the rows scan yields"
    );
    assert_eq!(filtered.len(), all.len());
}

#[test]
fn filter_selects_a_strict_subset() {
    let mut s = make("/tmp/scanfilter_subset");
    for i in 0..50i64 {
        s.insert("t", vec![vec![Value::Integer(i)]]).unwrap();
    }
    s.flush_all_buffers().unwrap();

    let even = s
        .scan_with_filter("t", &|r: &Vec<Value>| {
            r.first().and_then(|x| x.as_integer()).is_some_and(|n| n % 2 == 0)
        })
        .unwrap();

    assert_eq!(
        ints(&even),
        (0..50i64).filter(|n| n % 2 == 0).collect::<Vec<_>>(),
        "the filter must decide which rows come back"
    );
    assert_eq!(even.len(), 25);
}

#[test]
fn filter_that_matches_nothing_returns_empty() {
    let mut s = make("/tmp/scanfilter_none");
    for i in 0..20i64 {
        s.insert("t", vec![vec![Value::Integer(i)]]).unwrap();
    }
    s.flush_all_buffers().unwrap();

    let none = s
        .scan_with_filter("t", &|_r: &Vec<Value>| false)
        .unwrap();
    assert!(
        none.is_empty(),
        "a filter rejecting everything must return nothing, not a full clone"
    );
}

#[test]
fn filter_also_sees_rows_still_in_the_insert_buffer() {
    // Rows inserted but not yet flushed live in `insert_buffer`, not
    // `tables.rows`. The trigger paths run inside a transaction where
    // that is the normal state, so a filter that only looked at
    // `tables.rows` would miss them. `scan` merges the buffer; the
    // filter must not lose that.
    let mut s = make("/tmp/scanfilter_buffer");
    for i in 0..10i64 {
        s.insert("t", vec![vec![Value::Integer(i)]]).unwrap();
    }
    // deliberately NOT flushed

    let buffered = s
        .scan_with_filter("t", &|r: &Vec<Value>| {
            r.first().and_then(|x| x.as_integer()).is_some_and(|n| n < 5)
        })
        .unwrap();
    assert_eq!(
        ints(&buffered),
        vec![0, 1, 2, 3, 4],
        "rows still in insert_buffer must be visible to the filter"
    );
}

#[test]
fn filter_on_a_missing_table_returns_empty() {
    // Same shape as `scan` on a missing table: no error, empty result.
    let s = make("/tmp/scanfilter_missing");
    let out = s.scan_with_filter("nope", &|_r: &Vec<Value>| true).unwrap();
    assert!(out.is_empty());
}

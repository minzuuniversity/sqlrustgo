//! Regression tests for Issue #4490 — commonly-missing SQL scalar functions
//! no longer silently return Null.
//!
//! Each test exercises one of NOW / CURDATE / CURTIME / YEAR / MONTH / DAY /
//! DATEDIFF / ROUND / RAND / LENGTH and asserts a positive (non-Null) result.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn first_value(result: &sqlrustgo::ExecutorResult) -> &Value {
    result
        .rows
        .first()
        .and_then(|row| row.first())
        .expect("at least one row")
}

#[test]
fn now_returns_timestamp_text() {
    let mut e = engine();
    let r = e.execute("SELECT NOW()").unwrap();
    match first_value(&r) {
        Value::Text(s) => {
            assert!(
                s.len() >= 19,
                "NOW() should produce YYYY-MM-DD HH:MM:SS, got {s:?}"
            );
            // Cheap regex-like check: positions 4, 7, 10, 13, 16 must be `-`/`:`.
            let bytes = s.as_bytes();
            assert_eq!(bytes[4], b'-');
            assert_eq!(bytes[7], b'-');
            assert_eq!(bytes[10], b' ');
            assert_eq!(bytes[13], b':');
            assert_eq!(bytes[16], b':');
        }
        other => panic!("NOW() should be Text, got {other:?}"),
    }
}

#[test]
fn curdate_returns_date_text() {
    let mut e = engine();
    let r = e.execute("SELECT CURDATE()").unwrap();
    match first_value(&r) {
        Value::Text(s) => {
            assert_eq!(s.len(), 10, "CURDATE() should produce YYYY-MM-DD, got {s:?}");
            let bytes = s.as_bytes();
            assert_eq!(bytes[4], b'-');
            assert_eq!(bytes[7], b'-');
        }
        other => panic!("CURDATE() should be Text, got {other:?}"),
    }
}

#[test]
fn curtime_returns_time_text() {
    let mut e = engine();
    let r = e.execute("SELECT CURTIME()").unwrap();
    match first_value(&r) {
        Value::Text(s) => {
            assert!(
                s.len() >= 5 && s.as_bytes()[2] == b':',
                "CURTIME() should produce HH:MM:SS, got {s:?}"
            );
        }
        other => panic!("CURTIME() should be Text, got {other:?}"),
    }
}

#[test]
fn round_two_decimals() {
    let mut e = engine();
    let r = e.execute("SELECT ROUND(3.14159, 2)").unwrap();
    match first_value(&r) {
        Value::Float(f) => {
            assert!((f - 3.14).abs() < 1e-9, "ROUND(3.14159, 2) should be 3.14, got {f}")
        }
        other => panic!("ROUND should be Float, got {other:?}"),
    }
}

#[test]
fn round_default_decimals_is_zero() {
    let mut e = engine();
    let r = e.execute("SELECT ROUND(3.7)").unwrap();
    assert!(matches!(first_value(&r), Value::Float(f) if (*f - 4.0).abs() < 1e-9));
}

#[test]
fn rand_returns_float_in_unit_interval() {
    let mut e = engine();
    let r = e.execute("SELECT RAND()").unwrap();
    match first_value(&r) {
        Value::Float(f) => {
            assert!(*f >= 0.0 && *f < 1.0, "RAND() should be in [0, 1), got {f}");
        }
        other => panic!("RAND() should be Float, got {other:?}"),
    }
}

#[test]
fn rand_returns_different_values() {
    let mut e = engine();
    let r1 = e.execute("SELECT RAND()").unwrap();
    let r2 = e.execute("SELECT RAND()").unwrap();
    let v1 = first_value(&r1).clone();
    let v2 = first_value(&r2).clone();
    assert_ne!(
        v1, v2,
        "two RAND() calls in the same engine should not collide on a single value"
    );
}

#[test]
fn length_returns_char_count() {
    let mut e = engine();
    let r = e.execute("SELECT LENGTH('alice')").unwrap();
    assert!(matches!(first_value(&r), Value::Integer(n) if *n == 5));
}

#[test]
fn year_extracts_year_from_iso_date() {
    let mut e = engine();
    let r = e.execute("SELECT YEAR('2019-05-15')").unwrap();
    assert!(matches!(first_value(&r), Value::Integer(n) if *n == 2019));
}

#[test]
fn month_extracts_month_from_iso_date() {
    let mut e = engine();
    let r = e.execute("SELECT MONTH('2019-05-15')").unwrap();
    assert!(matches!(first_value(&r), Value::Integer(n) if *n == 5));
}

#[test]
fn day_extracts_day_from_iso_date() {
    let mut e = engine();
    let r = e.execute("SELECT DAY('2019-05-15')").unwrap();
    assert!(matches!(first_value(&r), Value::Integer(n) if *n == 15));
}

#[test]
fn year_extracts_year_from_datetime() {
    let mut e = engine();
    let r = e.execute("SELECT YEAR('2019-05-15 12:34:56')").unwrap();
    assert!(matches!(first_value(&r), Value::Integer(n) if *n == 2019));
}

#[test]
fn datediff_full_year() {
    let mut e = engine();
    let r = e
        .execute("SELECT DATEDIFF('2019-01-01', '2018-01-01')")
        .unwrap();
    assert!(
        matches!(first_value(&r), Value::Integer(n) if (364..=366).contains(n)),
        "DATEDIFF(year boundary) should be ~365, got {r:?}"
    );
}

#[test]
fn datediff_zero_same_day() {
    let mut e = engine();
    let r = e
        .execute("SELECT DATEDIFF('2019-05-15', '2019-05-15')")
        .unwrap();
    assert!(matches!(first_value(&r), Value::Integer(n) if *n == 0));
}

#[test]
fn datediff_negative_order() {
    let mut e = engine();
    let r = e
        .execute("SELECT DATEDIFF('2018-01-01', '2019-01-01')")
        .unwrap();
    let n = match first_value(&r) {
        Value::Integer(n) => *n,
        _ => panic!("not integer"),
    };
    assert!(n < 0, "DATEDIFF(reversed) should be negative, got {n}");
}

#[test]
fn unknown_function_still_returns_null() {
    let mut e = engine();
    let r = e.execute("SELECT FOO_BAR_BAZ()").unwrap();
    assert!(matches!(first_value(&r), Value::Null));
}

#[test]
fn round_uses_table_column() {
    let mut e = engine();
    e.execute("CREATE TABLE t(x FLOAT)").unwrap();
    e.execute("INSERT INTO t VALUES (1.234), (5.678)").unwrap();
    let r = e.execute("SELECT ROUND(x, 1) FROM t ORDER BY x").unwrap();
    assert_eq!(r.rows.len(), 2);
    assert!(matches!(&r.rows[0][0], Value::Float(f) if (*f - 1.2).abs() < 1e-9));
    assert!(matches!(&r.rows[1][0], Value::Float(f) if (*f - 5.7).abs() < 1e-9));
}
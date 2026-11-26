//! Issue #4716 — DATE_TRUNC must match the spec
//! `openspec/specs/executor-date-trunc-function/spec.md`:
//!
//! - `DATE_TRUNC('unit', date)` truncates to the start of the unit and
//!   **returns a string**.
//! - An unsupported unit must raise an error containing
//!   "DATE_TRUNC unit must be one of" instead of silently returning NULL.
//!
//! Before the fix the executor returned `YYYY-MM-DD` for DAY (dropping the
//! time part), an **epoch integer** for HOUR/MINUTE/SECOND, and NULL for an
//! unknown unit — three different shapes, none of them the spec's.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn with_dt(dt: &str) -> ExecutionEngine<MemoryStorage> {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(dt TEXT)").unwrap();
    x.execute(&format!("INSERT INTO t VALUES ('{dt}')"))
        .unwrap();
    x
}

fn trunc(x: &mut ExecutionEngine<MemoryStorage>, unit: &str) -> String {
    let r = x
        .execute(&format!("SELECT DATE_TRUNC('{unit}', dt) FROM t"))
        .unwrap_or_else(|e| panic!("DATE_TRUNC('{unit}', dt) must execute: {e}"));
    assert_eq!(r.rows.len(), 1);
    format!("{:?}", r.rows[0][0])
}

// ---------------------------------------------------------------------------
// The spec's scenarios
// ---------------------------------------------------------------------------

#[test]
fn spec_scenario_month_truncation() {
    let mut x = with_dt("2026-09-15");
    assert_eq!(trunc(&mut x, "month"), "Text(\"2026-09-01\")");
}

#[test]
fn spec_scenario_year_truncation() {
    let mut x = with_dt("2026-09-15");
    assert_eq!(trunc(&mut x, "year"), "Text(\"2026-01-01\")");
}

#[test]
fn spec_scenario_quarter_truncation() {
    // 2026-09 is Q3 → 2026-07-01
    let mut x = with_dt("2026-09-15");
    assert_eq!(trunc(&mut x, "quarter"), "Text(\"2026-07-01\")");
}

#[test]
fn spec_scenario_day_truncation() {
    let mut x = with_dt("2026-09-15 12:34:56");
    assert_eq!(trunc(&mut x, "day"), "Text(\"2026-09-15 00:00:00\")");
}

#[test]
fn spec_scenario_hour_truncation() {
    let mut x = with_dt("2026-09-15 12:34:56");
    assert_eq!(trunc(&mut x, "hour"), "Text(\"2026-09-15 12:00:00\")");
}

#[test]
fn spec_scenario_unknown_unit_raises() {
    let mut x = with_dt("2026-09-15");
    let err = x
        .execute("SELECT DATE_TRUNC('decade', dt) FROM t")
        .expect_err("an unsupported unit must be rejected");
    assert!(
        err.to_string().contains("DATE_TRUNC unit must be one of"),
        "the error must name the supported units, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// The remaining supported units
// ---------------------------------------------------------------------------

#[test]
fn week_truncation_returns_the_monday_of_the_iso_week() {
    // 2026-01-15 is a Thursday → the Monday of that week is 2026-01-12.
    let mut x = with_dt("2026-01-15 12:34:56");
    assert_eq!(trunc(&mut x, "week"), "Text(\"2026-01-12\")");
}

#[test]
fn minute_truncation_zeroes_the_seconds() {
    let mut x = with_dt("2026-09-15 12:34:56");
    assert_eq!(trunc(&mut x, "minute"), "Text(\"2026-09-15 12:34:00\")");
}

#[test]
fn second_truncation_keeps_the_second() {
    let mut x = with_dt("2026-09-15 12:34:56");
    assert_eq!(trunc(&mut x, "second"), "Text(\"2026-09-15 12:34:56\")");
}

#[test]
fn every_supported_unit_returns_a_string() {
    for unit in [
        "year", "quarter", "month", "week", "day", "hour", "minute", "second",
    ] {
        let mut x = with_dt("2026-09-15 12:34:56");
        let out = trunc(&mut x, unit);
        assert!(
            out.starts_with("Text("),
            "DATE_TRUNC must return a string for unit {unit:?}, got {out}"
        );
    }
}

// ---------------------------------------------------------------------------
// The unit may only be known at runtime
// ---------------------------------------------------------------------------

#[test]
fn runtime_unknown_unit_raises() {
    // The parser can only validate a literal unit; a unit read from a
    // column must be rejected by the executor.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(u TEXT, dt TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('decade', '2026-09-15')")
        .unwrap();
    let err = x
        .execute("SELECT DATE_TRUNC(u, dt) FROM t")
        .expect_err("an unsupported runtime unit must be rejected");
    assert!(
        err.to_string().contains("DATE_TRUNC unit must be one of"),
        "the error must name the supported units, got: {err}"
    );
}

#[test]
fn runtime_supported_unit_is_truncated() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(u TEXT, dt TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('month', '2026-09-15')")
        .unwrap();
    let r = x.execute("SELECT DATE_TRUNC(u, dt) FROM t").unwrap();
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2026-09-01\")");
}

#[test]
fn null_unit_yields_null() {
    // The function is strict: a NULL unit is not an unsupported unit.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(dt TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2026-09-15')").unwrap();
    let r = x.execute("SELECT DATE_TRUNC(NULL, dt) FROM t").unwrap();
    assert!(
        matches!(r.rows[0][0], sqlrustgo::Value::Null),
        "a NULL unit must yield NULL, got: {:?}",
        r.rows[0][0]
    );
}

// ---------------------------------------------------------------------------
// Epoch input keeps working
// ---------------------------------------------------------------------------

#[test]
fn epoch_input_is_truncated_like_a_text_timestamp() {
    // 2026-09-15 12:34:56 UTC = 1789475696
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(dt BIGINT)").unwrap();
    x.execute("INSERT INTO t VALUES (1789475696)").unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('hour', dt) FROM t")
        .expect("DATE_TRUNC must accept an epoch timestamp");
    assert_eq!(
        format!("{:?}", r.rows[0][0]),
        "Text(\"2026-09-15 12:00:00\")"
    );
}

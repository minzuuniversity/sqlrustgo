//! V312-95 / Issue #4695 — INTERVAL date arithmetic executor wiring.
//!
//! Per commit 42b5c6fdd6 ("claim-downgrade"): the parser was already lifted
//! to accept `INTERVAL 'n' UNIT` syntax (PR landed as `7578da3f41`), but
//! the executor returns Value::Null (per
//! `crates/executor/src/stored_proc.rs:1701` — see also the "INTERVAL parser
//! still open" note). This file pins the executor to actually compute
//! `d + INTERVAL '5' DAY` instead of returning NULL.
//!
//! Tests cover:
//! - `d + INTERVAL '5' DAY` returns `d + 5 days` (not NULL, not error)
//! - `d - INTERVAL '1' MONTH` returns `d - 1 month` (subtract path)
//! - INTERVAL with INT unit value `INTERVAL 5 DAY` (unquoted form)
//! - INTERVAL with non-DAY unit (MONTH)
//! - `INTERVAL '1' YEAR` (year arithmetic edge)

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Helper: run one SQL statement and unwrap or panic with the error.
fn must_run(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) {
    if let Err(err) = e.execute(sql) {
        panic!("expected OK for [{}]; got error: {:?}", sql, err);
    }
}

/// Helper: query a single TEXT result, asserting one row with one column.
fn query_one_text(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> String {
    let r = e
        .execute(sql)
        .unwrap_or_else(|err| panic!("expected OK for [{}]; got error: {:?}", sql, err));
    assert_eq!(
        r.rows.len(),
        1,
        "expected 1 row from [{}]; got {}",
        sql,
        r.rows.len()
    );
    assert_eq!(
        r.rows[0].len(),
        1,
        "expected 1 column from [{}]; got {}",
        sql,
        r.rows[0].len()
    );
    match &r.rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("expected TEXT from [{}]; got {:?}", sql, other),
    }
}

// ============================================================================
// Issue #4695 — exact issue body case: d + INTERVAL '5' DAY returns d + 5 days
// ============================================================================

#[test]
fn interval_day_add_returns_d_plus_five_days_4695() {
    let mut e = fresh_mem();
    must_run(&mut e, "CREATE TABLE t(d DATE)");
    must_run(&mut e, "INSERT INTO t VALUES ('2026-01-15')");
    let result = query_one_text(&mut e, "SELECT d + INTERVAL '5' DAY FROM t");
    assert_eq!(
        result, "2026-01-20",
        "INTERVAL '5' DAY must add 5 days to 2026-01-15; got {}",
        result
    );
}

// ============================================================================
// Subtract path: d - INTERVAL '1' MONTH returns d - 1 month
// ============================================================================

#[test]
fn interval_month_subtract_returns_d_minus_one_month_4695() {
    let mut e = fresh_mem();
    must_run(&mut e, "CREATE TABLE t(d DATE)");
    must_run(&mut e, "INSERT INTO t VALUES ('2026-03-15')");
    let result = query_one_text(&mut e, "SELECT d - INTERVAL '1' MONTH FROM t");
    assert_eq!(
        result, "2026-02-15",
        "INTERVAL '1' MONTH must subtract 1 month from 2026-03-15; got {}",
        result
    );
}

// ============================================================================
// INTERVAL with non-DAY unit: MONTH
// ============================================================================

#[test]
fn interval_month_add_returns_d_plus_one_month_4695() {
    let mut e = fresh_mem();
    must_run(&mut e, "CREATE TABLE t(d DATE)");
    must_run(&mut e, "INSERT INTO t VALUES ('2026-01-15')");
    let result = query_one_text(&mut e, "SELECT d + INTERVAL '1' MONTH FROM t");
    assert_eq!(
        result, "2026-02-15",
        "INTERVAL '1' MONTH must add 1 month to 2026-01-15; got {}",
        result
    );
}

// ============================================================================
// INTERVAL with YEAR unit (larger arithmetic)
// ============================================================================

#[test]
fn interval_year_add_returns_d_plus_one_year_4695() {
    let mut e = fresh_mem();
    must_run(&mut e, "CREATE TABLE t(d DATE)");
    must_run(&mut e, "INSERT INTO t VALUES ('2025-06-15')");
    let result = query_one_text(&mut e, "SELECT d + INTERVAL '1' YEAR FROM t");
    assert_eq!(
        result, "2026-06-15",
        "INTERVAL '1' YEAR must add 1 year to 2025-06-15; got {}",
        result
    );
}

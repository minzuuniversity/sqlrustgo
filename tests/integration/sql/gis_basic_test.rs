//! V311-11 F-03: GIS spatial type and function integration tests
//!
//! PR #3540 (`feat(F-03 GIS Phase 2): Value::Point, ST_WITHIN, GIS serialization`)
//! and PR #7210 (`feat(F-03 GIS Phase 2): Add ST_WITHIN function to executor`)
//! merged the GIS surface into v3.11.0. This test exercises the end-to-end
//! SQL path through `ExecutionEngine::execute` for the canonical GIS query:
//! `SELECT ST_WITHIN(point, polygon)`.
//!
//! Coverage:
//!   - `Value::Point(f64, f64)` is a first-class value type
//!   - `ST_WITHIN` is registered in `eval_fn` dispatch
//!   - `POINT(x, y)` text form is parsed to `Value::Point` via `Point::parse`
//!   - `POLYGON((x y, ...))` text form is parsed to a `Polygon` via `Polygon::parse`
//!   - The ray-casting algorithm correctly classifies point-in-polygon
//!   - Negative cases (point outside, malformed input) return Value::Null
//!
//! The `Value::Point` variant is not yet constructable from SQL literal
//! syntax (no `POINT(x, y)` parser support), so this test passes POINT
//! and POLYGON values as text literals that the executor's ST_WITHIN
//! handler parses internally.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Helper: a unit square (0,0) - (5,0) - (5,5) - (0,5) in MySQL format.
const UNIT_SQUARE: &str = "POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))";

#[test]
fn st_within_point_inside_square() {
    let mut e = fresh();
    let r = e
        .execute(&format!(
            "SELECT ST_WITHIN('POINT(2, 3)', '{}')",
            UNIT_SQUARE
        ))
        .expect("ST_WITHIN should evaluate");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(true));
}

#[test]
fn st_within_point_outside_square() {
    let mut e = fresh();
    let r = e
        .execute(&format!(
            "SELECT ST_WITHIN('POINT(10, 10)', '{}')",
            UNIT_SQUARE
        ))
        .expect("ST_WITHIN should evaluate");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(false));
}

#[test]
fn st_within_point_on_boundary_returns_true() {
    // The ray-casting algorithm is boundary-inclusive for vertices
    // that are also listed in the polygon (the closing vertex
    // (0,0) in this case).
    let mut e = fresh();
    let r = e
        .execute(&format!(
            "SELECT ST_WITHIN('POINT(0, 0)', '{}')",
            UNIT_SQUARE
        ))
        .expect("ST_WITHIN should evaluate");
    assert_eq!(r.rows.len(), 1);
    // Boundary semantics: a vertex of the polygon is "inside".
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(true));
}

#[test]
fn st_within_point_just_outside_square() {
    let mut e = fresh();
    let r = e
        .execute(&format!(
            "SELECT ST_WITHIN('POINT(5.001, 2)', '{}')",
            UNIT_SQUARE
        ))
        .expect("ST_WITHIN should evaluate");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(false));
}

#[test]
fn st_within_concave_polygon() {
    // L-shaped polygon: (0,0)-(5,0)-(5,2)-(2,2)-(2,5)-(0,5)-(0,0).
    // Point (1, 1) is inside the L (in the bottom-left quadrant).
    // Point (3, 3) is outside the L (in the upper-right "cut out" area).
    let l_shape = "POLYGON((0 0, 5 0, 5 2, 2 2, 2 5, 0 5, 0 0))";
    let mut e = fresh();
    let r1 = e
        .execute(&format!("SELECT ST_WITHIN('POINT(1, 1)', '{}')", l_shape))
        .expect("ST_WITHIN should evaluate");
    assert_eq!(r1.rows[0][0], sqlrustgo::Value::Boolean(true));
    let r2 = e
        .execute(&format!("SELECT ST_WITHIN('POINT(3, 3)', '{}')", l_shape))
        .expect("ST_WITHIN should evaluate");
    assert_eq!(r2.rows[0][0], sqlrustgo::Value::Boolean(false));
}

#[test]
fn st_within_malformed_point_returns_null() {
    let mut e = fresh();
    let r = e
        .execute(&format!(
            "SELECT ST_WITHIN('NOT_A_POINT', '{}')",
            UNIT_SQUARE
        ))
        .expect("malformed point should not fail the query");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Null);
}

#[test]
fn st_within_malformed_polygon_returns_null() {
    let mut e = fresh();
    let r = e
        .execute("SELECT ST_WITHIN('POINT(1, 1)', 'NOT_A_POLYGON')")
        .expect("malformed polygon should not fail the query");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Null);
}

#[test]
fn st_within_wrong_arity_returns_null() {
    // ST_WITHIN takes exactly 2 args; 1 arg should yield Null.
    let mut e = fresh();
    let r = e
        .execute("SELECT ST_WITHIN('POINT(1, 1)')")
        .expect("wrong arity should not fail");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Null);
}

//! #5209: the index-routing decision, asserted directly.
//!
//! # Why this file exists separately from `equality_index_routing_5209.rs`
//!
//! Those tests assert that queries return the right rows — and they pass
//! **whether or not the routing branch is present**, because a full scan
//! returns the same rows an index seek does. A green suite there cannot
//! distinguish "the index was used" from "the index was never consulted",
//! so removing the routing would leave it vacuously green.
//!
//! That is not hypothetical: it is exactly what happened the first time
//! this routing was written and then reverted for a correctness reason.
//!
//! These tests therefore assert the *decision* — which column and value
//! the router extracts from a WHERE clause — rather than the query result.
//! Deleting the routing makes them fail.
//!
//! The safety properties are asserted alongside: an `OR` chain must never
//! be decomposed (seeking one branch would drop the other's rows), and a
//! non-equality predicate must not be extracted (there is no single key).

use sqlrustgo::engine_select_pk::try_extract_eq_conjunct;
use sqlrustgo_parser::Expression;
use sqlrustgo_types::Value;

fn eq(col: &str, lit: &str) -> Expression {
    Expression::BinaryOp(
        Box::new(Expression::Identifier(col.to_string())),
        "=".to_string(),
        Box::new(Expression::Literal(lit.to_string())),
    )
}

fn binop(l: Expression, op: &str, r: Expression) -> Expression {
    Expression::BinaryOp(Box::new(l), op.to_string(), Box::new(r))
}

fn some(e: Expression) -> Option<Expression> {
    Some(e)
}

fn and(a: Expression, b: Expression) -> Expression {
    binop(a, "AND", b)
}

fn or(a: Expression, b: Expression) -> Expression {
    binop(a, "OR", b)
}

#[test]
fn eq_conjunct_found_in_and_chain() {
    let w = some(and(eq("id", "1"), eq("k", "2")));
    assert_eq!(
        try_extract_eq_conjunct(&w),
        Some(("id".to_string(), Value::Integer(1))),
        "#5209: an AND chain must expose its equality so the caller can \
         seek on it instead of scanning"
    );
}

#[test]
fn eq_conjunct_found_in_later_and_branch() {
    let w = some(and(eq("k", "2"), eq("id", "1")));
    assert_eq!(
        try_extract_eq_conjunct(&w),
        Some(("k".to_string(), Value::Integer(2))),
        "#5209: the leftmost equality is usable; the caller re-checks the \
         remaining conjuncts, which is why seeking on one is sound"
    );
}

#[test]
fn literal_on_the_left_is_recognised() {
    let w = some(Expression::BinaryOp(
        Box::new(Expression::Literal("5".to_string())),
        "=".to_string(),
        Box::new(Expression::Identifier("k".to_string())),
    ));
    assert_eq!(
        try_extract_eq_conjunct(&w),
        Some(("k".to_string(), Value::Integer(5)))
    );
}

#[test]
fn or_chain_is_never_decomposed() {
    // Seeking on one branch of an OR would drop the other's rows —
    // a correctness bug, not a slow path.
    let w = some(or(eq("id", "1"), eq("id", "400")));
    assert_eq!(
        try_extract_eq_conjunct(&w),
        None,
        "#5209: an OR must fall through to the full scan"
    );
    // An OR *inside* an AND is different, and routing there is safe: the
    // seek yields a candidate superset (rows with id = 1) and the rest of
    // the WHERE is evaluated against those candidates, so the OR branch is
    // not lost. `equality_index_routing_5209.rs` pins that empirically —
    // `id = 1 AND k = 999` seeks on `id = 1`, finds one candidate, and must
    // still return 0 rows because the second conjunct rejects it.
    let w = some(and(eq("id", "1"), or(eq("id", "1"), eq("id", "400"))));
    assert_eq!(
        try_extract_eq_conjunct(&w),
        Some(("id".to_string(), Value::Integer(1))),
        "#5209: an equality in the AND's left branch is usable; the OR is \
         evaluated downstream against the candidates the seek returns"
    );
}

#[test]
fn non_equality_predicates_are_not_extracted() {
    let gt = Expression::BinaryOp(
        Box::new(Expression::Identifier("k".to_string())),
        ">".to_string(),
        Box::new(Expression::Literal("5".to_string())),
    );
    assert_eq!(try_extract_eq_conjunct(&some(gt)), None);
    assert_eq!(try_extract_eq_conjunct(&None), None);
}

/// Pins the call site itself.
///
/// The tests above exercise `try_extract_eq_conjunct` directly, so they
/// keep passing if the function is never called. The correctness suite in
/// `equality_index_routing_5209.rs` cannot cover this either: a full scan
/// returns the same rows an index seek does, so it is green either way.
/// Between them, nothing would notice the routing being deleted — which
/// is precisely how this defect survived: the index was built, correct,
/// and never consulted.
///
/// A source-level assertion is the only deterministic option available
/// here. It follows the technique already used in this crate for the LOAD
/// DATA counter (`call_site_routes_skipped_into_the_ok_packet`).
#[test]
fn engine_select_actually_calls_the_routing() {
    let src = include_str!("../../../src/engine_select.rs");
    assert_eq!(
        src.matches("try_extract_eq_conjunct(&select.where_clause)")
            .count(),
        1,
        "#5209: engine_select must route an equality conjunct to \\
         scan_with_index_in_db; the decision function exists but if this \\
         call site is gone every query silently falls back to a full scan"
    );
    assert_eq!(
        src.matches("scan_with_index_in_db(&current_db, lookup_table, &idx_column, &idx_value)")
            .count(),
        1,
        "#5209: the routed lookup must actually call the storage index"
    );
}

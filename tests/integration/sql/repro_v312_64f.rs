//! V312-64f / Issue #4699 — Recursive CTE integration tests.
//!
//! Tests cover:
//! - Hierarchy traversal (issue body case)
//! - Count 1..N (terminating recursion)
//! - UNION (non-ALL) dedup
//! - Empty anchor
//! - Non-UNION body rejection
//! - Multiple CTEs mixed (recursive + non-recursive)
//! - Cleanup after recursive CTE
//! - MAX_RECURSION_ROWS exceeded
//! - Aggregation inside step
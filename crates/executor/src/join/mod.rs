//! Hash-join building blocks.
//!
//! Sprint 8 (v3.9.0 / v3.9.1): TPC-H Q3 / Q8 / Q21 are O(N²) under the
//! current nested-loop join code path in `src/engine_select.rs`. The
//! proposals `openspec/changes/2026-06-08-v390-sprint8-q3-exists/`,
//! `…-q8-exists/`, and `…-q21-exists/` all defer the multi-way hash-join
//! implementation to Sprint 8. This module provides the building
//! blocks for that work:
//!
//! - [`hash_join_inner_outer`] — 2-way hash join. Hashes the smaller
//!   side on a single key column, then probes the larger side once.
//!   Complexity: O(R + S) time and O(min(R, S)) space, vs the
//!   O(R × S) time of nested loop.
//!
//! - [`multi_way_hash_chain`] — chained 2-way hash joins for a
//!   left-deep join tree. Useful for Q3 (customer × orders × lineitem)
//!   and similar 3+ table joins. Each level hashes the right side and
//!   probes with the accumulated rows.
//!
//! These helpers are intentionally **library functions**, not yet
//! wired into `ExecutionEngine::execute_joins`. The wiring is a
//! separate follow-up (Sprint 8 task) — see
//! `openspec/changes/2026-06-08-v390-sprint8-q3-exists/tasks.md` §6.
//!
//! Used by `crates/executor/src/lib.rs` re-exports.

pub mod hash_anti_join;
pub mod hash_join;
pub mod hash_semi_join;

//! Test runner for the inline `#[cfg(test)] mod tests` block in
//! `tests/common/tpch_wire_harness.rs`.
//!
//! The inline tests in the common helper are not visible to a
//! standalone test binary (the helper is a shared module, not a
//! library target). This file re-includes `common` so the inline
//! tests get compiled and registered with `cargo test`.
//!
//! Run with:
//!   cargo test --test tpch_wire_harness_compare_cells_test -- --nocapture

mod common;

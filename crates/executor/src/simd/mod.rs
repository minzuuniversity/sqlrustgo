//! SIMD-accelerated vectorized execution.
//!
//! This module provides SIMD-accelerated operations for filtering and aggregation.
//! Functions use `#[inline]` and are designed to be auto-vectorized by the Rust
//! compiler when compiled with `-C opt-level=3` or `-C target-cpu=native`.
//!
//! Supported types: Int32, Int64, Float64
//! Supported operations: eq, ne, lt, gt, lte, gte (filters) and sum, avg, count, min, max (aggregations)

pub mod agg;
pub mod filter;

pub use agg::{
    avg_f64, avg_f64_column, avg_i32, avg_i64, count_f64, count_f64_column, count_i32,
    count_i64, max_f64, min_f64, sum_f64, sum_f64_column, sum_f64_ignore_nan, sum_i32,
    sum_i32_column, sum_i32_ignore_nulls, sum_i64,
};
pub use filter::{
    filter_column_f64_eq, filter_column_i32_eq, filter_column_i64_eq, filter_f64_eq,
    filter_f64_gte, filter_f64_gt, filter_f64_lte, filter_f64_lt, filter_f64_ne, filter_i32_eq,
    filter_i32_gte, filter_i32_gt, filter_i32_lte, filter_i32_lt, filter_i32_ne, filter_i64_eq,
    filter_i64_gte, filter_i64_gt, filter_i64_lte, filter_i64_lt, filter_i64_ne, FilterResult,
};

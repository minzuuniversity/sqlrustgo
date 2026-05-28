# SIMD Vectorized Expression Engine Specification

## Issue #1461

## Overview

This specification defines the SIMD-accelerated vectorized expression engine for SQLRustGo v3.6.0. The engine uses Rust's stable `std::simd` module (available since Rust 1.93.1) to accelerate filter operations and aggregations on columnar data.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SIMD Vectorized Engine                    │
├─────────────────────────────────────────────────────────────┤
│  simd/filter.rs          │  simd/agg.rs                      │
│  ──────────────────────  │  ──────────────────────────────  │
│  • filter_i32_eq/gt/lt   │  • sum_i32 / sum_f64             │
│  • filter_i32_ne/gte/lte │  • avg_f64                       │
│  • filter_i64_eq/gt      │  • count                         │
│  • filter_f64_eq/gt/lt   │  • Horizontal SIMD reduction     │
│  • filter_f64_ne/gte/lte │                                 │
└─────────────────────────────────────────────────────────────┘
```

## Filter Operations

### Supported Types and Operations

| Type   | Operations                     |
|--------|--------------------------------|
| i32    | eq, ne, lt, gt, lte, gte       |
| i64    | eq, gt                         |
| f64    | eq, ne, lt, gt, lte, gte       |

### SIMD Chunk Sizes

| Type   | Lane Size | 256-bit Lanes |
|--------|-----------|---------------|
| i32    | 4 bytes   | 8 elements    |
| i64    | 8 bytes   | 4 elements    |
| f64    | 8 bytes   | 4 elements    |

### Float64 Comparison Semantics

Float64 comparisons follow IEEE 754 semantics:
- NaN comparisons return false for all comparisons (eq, ne, lt, gt, lte, gte)
- -0.0 and +0.0 are considered equal
- Inf comparisons follow standard mathematical rules

## Aggregation Operations

### Supported Aggregations

| Function   | Input Type | Output Type | SIMD Strategy        |
|------------|------------|-------------|----------------------|
| sum_i32    | i32        | i64         | Horizontal pairwise   |
| sum_f64    | f64        | f64         | Horizontal pairwise  |
| avg_f64    | f64        | f64         | sum + count          |
| count      | any        | usize       | POPCNT on validity   |

### SIMD Reduction Strategy

For horizontal reductions (sum, avg), the algorithm:
1. Processes data in SIMD chunks
2. Reduces SIMD vectors using pairwise addition
3. Accumulates scalar remainder
4. Combines for final result

## API

### Filter Functions

```rust
pub fn filter_f64_eq(data: &[f64], target: f64) -> FilterResult
pub fn filter_f64_ne(data: &[f64], target: f64) -> FilterResult
pub fn filter_f64_lt(data: &[f64], target: f64) -> FilterResult
pub fn filter_f64_gt(data: &[f64], target: f64) -> FilterResult
pub fn filter_f64_lte(data: &[f64], target: f64) -> FilterResult
pub fn filter_f64_gte(data: &[f64], target: f64) -> FilterResult

pub fn filter_column_f64_eq(col: &ColumnVector, target: f64) -> FilterResult
```

### Aggregation Functions

```rust
pub fn sum_i32(data: &[i32]) -> i64
pub fn sum_f64(data: &[f64]) -> f64
pub fn avg_f64(data: &[f64]) -> f64
pub fn count(data: &[f64], nulls: &NullBitmap) -> usize
```

## Performance Targets

| Operation      | Speedup vs Scalar |
|----------------|-------------------|
| filter_f64_eq  | 3-4x              |
| sum_f64        | 2-3x              |
| avg_f64        | 2-3x              |
| count          | 4-8x              |

## Testing Requirements

All SIMD functions must have:
- Basic correctness tests
- Edge case tests (empty input, single element)
- Partial chunk tests (data not aligned to SIMD width)
- Multi-chunk tests (data spanning multiple SIMD operations)
- NaN handling tests for float operations
- Property-based tests for aggregation correctness

## Dependencies

- Rust 1.93.1+ (stable std::simd)
- No external SIMD libraries (use std::simd only)

## Milestones

- [x] Filter i32 (eq, gt, lt, ne, gte, lte)
- [x] Filter i64 (eq, gt)
- [ ] Filter f64 (eq, ne, lt, gt, lte, gte) - **THIS ISSUE**
- [ ] Aggregation (sum_i32, sum_f64, avg_f64, count) - **THIS ISSUE**

## Files to Create/Modify

```
crates/executor/src/simd/
├── mod.rs          (modify - add agg exports)
├── filter.rs      (modify - add f64 filters)
└── agg.rs         (create - aggregation)
```

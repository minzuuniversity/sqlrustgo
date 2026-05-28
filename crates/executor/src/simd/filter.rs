//! SIMD-accelerated vectorized filter using pure Rust with autovectorization.
//!
//! The Rust compiler auto-vectorizes these functions when compiled with
//! `-C target-cpu=native` or `-C opt-level=3`.
//!
//! For guaranteed SIMD, use the nightly `std::simd` or `packed_simd` crate.

use crate::chunk::ColumnVector;

/// Filter result containing indices that pass the filter.
#[derive(Debug)]
pub struct FilterResult {
    /// Indices that passed the filter.
    pub indices: Vec<usize>,
    /// Total elements scanned.
    pub total: usize,
}

impl FilterResult {
    /// Creates a new empty filter result.
    pub fn new() -> Self {
        Self {
            indices: Vec::new(),
            total: 0,
        }
    }

    /// Returns true if no elements passed the filter.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Returns the count of matching elements.
    pub fn count(&self) -> usize {
        self.indices.len()
    }
}

impl Default for FilterResult {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Int32 Filters ────────────────────────────────────────────────────────────

/// Filters an Int32 column where value == target.
#[inline]
pub fn filter_i32_eq(data: &[i32], target: i32) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val == target {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int32 column where value > threshold.
#[inline]
pub fn filter_i32_gt(data: &[i32], threshold: i32) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val > threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int32 column where value >= threshold.
#[inline]
pub fn filter_i32_gte(data: &[i32], threshold: i32) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val >= threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int32 column where value < threshold.
#[inline]
pub fn filter_i32_lt(data: &[i32], threshold: i32) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val < threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int32 column where value <= threshold.
#[inline]
pub fn filter_i32_lte(data: &[i32], threshold: i32) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val <= threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int32 column where value != target.
#[inline]
pub fn filter_i32_ne(data: &[i32], target: i32) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val != target {
            result.indices.push(i);
        }
    }
    result
}

// ─── Int64 Filters ───────────────────────────────────────────────────────────

/// Filters an Int64 column where value == target.
#[inline]
pub fn filter_i64_eq(data: &[i64], target: i64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val == target {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int64 column where value > threshold.
#[inline]
pub fn filter_i64_gt(data: &[i64], threshold: i64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val > threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int64 column where value >= threshold.
#[inline]
pub fn filter_i64_gte(data: &[i64], threshold: i64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val >= threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int64 column where value < threshold.
#[inline]
pub fn filter_i64_lt(data: &[i64], threshold: i64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val < threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int64 column where value <= threshold.
#[inline]
pub fn filter_i64_lte(data: &[i64], threshold: i64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val <= threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters an Int64 column where value != target.
#[inline]
pub fn filter_i64_ne(data: &[i64], target: i64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val != target {
            result.indices.push(i);
        }
    }
    result
}

// ─── Float64 Filters ────────────────────────────────────────────────────────

/// Filters a Float64 column where value == target.
/// NaN values return false (do not match).
#[inline]
pub fn filter_f64_eq(data: &[f64], target: f64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val == target {
            result.indices.push(i);
        }
    }
    result
}

/// Filters a Float64 column where value != target.
/// NaN values return true (do not match target).
#[inline]
pub fn filter_f64_ne(data: &[f64], target: f64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val != target {
            result.indices.push(i);
        }
    }
    result
}

/// Filters a Float64 column where value > threshold.
/// NaN values return false.
#[inline]
pub fn filter_f64_gt(data: &[f64], threshold: f64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val > threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters a Float64 column where value >= threshold.
/// NaN values return false.
#[inline]
pub fn filter_f64_gte(data: &[f64], threshold: f64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val >= threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters a Float64 column where value < threshold.
/// NaN values return false.
#[inline]
pub fn filter_f64_lt(data: &[f64], threshold: f64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val < threshold {
            result.indices.push(i);
        }
    }
    result
}

/// Filters a Float64 column where value <= threshold.
/// NaN values return false.
#[inline]
pub fn filter_f64_lte(data: &[f64], threshold: f64) -> FilterResult {
    let mut result = FilterResult::new();
    for (i, &val) in data.iter().enumerate() {
        result.total += 1;
        if val <= threshold {
            result.indices.push(i);
        }
    }
    result
}

// ─── Column Helpers ──────────────────────────────────────────────────────────

/// Filters a ColumnVector using an Int32 equality filter.
pub fn filter_column_i32_eq(col: &ColumnVector, target: i32) -> FilterResult {
    if let Some(data) = col.as_int32_slice() {
        filter_i32_eq(data, target)
    } else {
        FilterResult::new()
    }
}

/// Filters a ColumnVector using an Int64 equality filter.
pub fn filter_column_i64_eq(col: &ColumnVector, target: i64) -> FilterResult {
    if let Some(data) = col.as_int64_slice() {
        filter_i64_eq(data, target)
    } else {
        FilterResult::new()
    }
}

/// Filters a ColumnVector using a Float64 equality filter.
pub fn filter_column_f64_eq(col: &ColumnVector, target: f64) -> FilterResult {
    if let Some(data) = col.as_float64_slice() {
        filter_f64_eq(data, target)
    } else {
        FilterResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_i32_eq_basic() {
        let data = vec![1, 2, 3, 2, 5, 2, 7, 8];
        let result = filter_i32_eq(&data, 2);
        assert_eq!(result.indices, vec![1, 3, 5]);
        assert_eq!(result.total, 8);
    }

    #[test]
    fn test_filter_i32_gt_basic() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let result = filter_i32_gt(&data, 4);
        assert_eq!(result.indices, vec![4, 5, 6, 7]);
        assert_eq!(result.total, 8);
    }

    #[test]
    fn test_filter_i32_lt_basic() {
        let data = vec![1, 2, 3, 4, 5];
        let result = filter_i32_lt(&data, 3);
        assert_eq!(result.indices, vec![0, 1]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i32_ne() {
        let data = vec![1, 2, 3, 2, 1];
        let result = filter_i32_ne(&data, 2);
        assert_eq!(result.indices, vec![0, 2, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i32_gte() {
        let data = vec![1, 2, 3, 4, 5];
        let result = filter_i32_gte(&data, 3);
        assert_eq!(result.indices, vec![2, 3, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i32_lte() {
        let data = vec![1, 2, 3, 4, 5];
        let result = filter_i32_lte(&data, 3);
        assert_eq!(result.indices, vec![0, 1, 2]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_eq() {
        let data = vec![10i64, 20, 30, 20, 50];
        let result = filter_i64_eq(&data, 20);
        assert_eq!(result.indices, vec![1, 3]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_gt() {
        let data = vec![10i64, 20, 30, 40, 50];
        let result = filter_i64_gt(&data, 25);
        assert_eq!(result.indices, vec![2, 3, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_gte() {
        let data = vec![10i64, 20, 30, 40, 50];
        let result = filter_i64_gte(&data, 30);
        assert_eq!(result.indices, vec![2, 3, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_lt() {
        let data = vec![10i64, 20, 30, 40, 50];
        let result = filter_i64_lt(&data, 30);
        assert_eq!(result.indices, vec![0, 1]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_lte() {
        let data = vec![10i64, 20, 30, 40, 50];
        let result = filter_i64_lte(&data, 30);
        assert_eq!(result.indices, vec![0, 1, 2]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_ne() {
        let data = vec![10i64, 20, 30, 20, 50];
        let result = filter_i64_ne(&data, 20);
        assert_eq!(result.indices, vec![0, 2, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_eq() {
        let data = vec![1.0, 2.0, 3.0, 2.0, 5.0];
        let result = filter_f64_eq(&data, 2.0);
        assert_eq!(result.indices, vec![1, 3]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_gt() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = filter_f64_gt(&data, 2.5);
        assert_eq!(result.indices, vec![2, 3, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_ne() {
        let data = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let result = filter_f64_ne(&data, 2.0);
        assert_eq!(result.indices, vec![0, 2, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_gte() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = filter_f64_gte(&data, 3.0);
        assert_eq!(result.indices, vec![2, 3, 4]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_lt() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = filter_f64_lt(&data, 3.0);
        assert_eq!(result.indices, vec![0, 1]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_lte() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = filter_f64_lte(&data, 3.0);
        assert_eq!(result.indices, vec![0, 1, 2]);
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_result_empty() {
        let result = FilterResult::new();
        assert!(result.is_empty());
        assert_eq!(result.count(), 0);
    }

    #[test]
    fn test_filter_i32_large() {
        let data: Vec<i32> = (0..1000).collect();
        let result = filter_i32_eq(&data, 500);
        assert_eq!(result.indices, vec![500]);
        assert_eq!(result.total, 1000);
    }

    #[test]
    fn test_filter_i32_none() {
        let data = vec![1, 2, 3, 4, 5];
        let result = filter_i32_eq(&data, 99);
        assert!(result.indices.is_empty());
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_f64_nan() {
        let data = vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0];
        let result = filter_f64_eq(&data, f64::NAN);
        // NaN != NaN in IEEE 754, so no match
        assert!(result.indices.is_empty());
        assert_eq!(result.total, 5);
    }

    #[test]
    fn test_filter_i64_large() {
        let data: Vec<i64> = (0..1000).map(|x| x * 10).collect();
        let result = filter_i64_eq(&data, 5000);
        assert_eq!(result.indices, vec![500]);
        assert_eq!(result.total, 1000);
    }

    #[test]
    fn test_filter_f64_large() {
        let data: Vec<f64> = (0..1000).map(|x| x as f64).collect();
        let result = filter_f64_gte(&data, 500.0);
        assert_eq!(result.indices.len(), 500);
        assert_eq!(result.total, 1000);
    }
}

//! SIMD-accelerated aggregation using pure Rust with autovectorization.
//!
//! The Rust compiler auto-vectorizes these functions when compiled with
//! `-C target-cpu=native` or `-C opt-level=3`.

use crate::chunk::ColumnVector;

// ─── Int32 Aggregation ────────────────────────────────────────────────────────

/// Computes the sum of i32 values.
/// Returns i64 to avoid overflow.
#[inline]
pub fn sum_i32(data: &[i32]) -> i64 {
    data.iter().map(|&x| x as i64).sum()
}

/// Computes the sum of i32 values, ignoring nulls.
#[inline]
pub fn sum_i32_ignore_nulls(data: &[i32]) -> i64 {
    data.iter().filter(|&&x| x != 0).map(|&x| x as i64).sum()
}

/// Counts the number of non-null i32 values.
#[inline]
pub fn count_i32(data: &[i32]) -> usize {
    data.len()
}

/// Computes the average of i32 values.
#[inline]
pub fn avg_i32(data: &[i32]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64
}

// ─── Int64 Aggregation ────────────────────────────────────────────────────────

/// Computes the sum of i64 values.
#[inline]
pub fn sum_i64(data: &[i64]) -> i64 {
    data.iter().sum()
}

/// Counts the number of non-null i64 values.
#[inline]
pub fn count_i64(data: &[i64]) -> usize {
    data.len()
}

/// Computes the average of i64 values.
#[inline]
pub fn avg_i64(data: &[i64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.iter().map(|&x| x as f64).sum::<f64>() / data.len() as f64
}

// ─── Float64 Aggregation ──────────────────────────────────────────────────────

/// Computes the sum of f64 values.
/// NaN values are handled: if any value is NaN, returns NaN.
#[inline]
pub fn sum_f64(data: &[f64]) -> f64 {
    data.iter().sum()
}

/// Computes the sum of f64 values, ignoring NaN values.
#[inline]
pub fn sum_f64_ignore_nan(data: &[f64]) -> f64 {
    data.iter().filter(|&&x| !x.is_nan()).sum()
}

/// Counts the number of non-NaN f64 values.
#[inline]
pub fn count_f64(data: &[f64]) -> usize {
    data.iter().filter(|&&x| !x.is_nan()).count()
}

/// Computes the average of f64 values.
/// Returns NaN if all values are NaN or data is empty.
#[inline]
pub fn avg_f64(data: &[f64]) -> f64 {
    let valid: f64 = data.iter().filter(|&&x| !x.is_nan()).sum();
    let count = data.iter().filter(|&&x| !x.is_nan()).count() as f64;
    if count == 0.0 {
        f64::NAN
    } else {
        valid / count
    }
}

/// Computes the min of f64 values.
/// Returns NaN if all values are NaN or data is empty.
#[inline]
pub fn min_f64(data: &[f64]) -> f64 {
    data.iter()
        .filter(|&&x| !x.is_nan())
        .fold(f64::INFINITY, |a, &b| a.min(b))
}

/// Computes the max of f64 values.
/// Returns NaN if all values are NaN or data is empty.
#[inline]
pub fn max_f64(data: &[f64]) -> f64 {
    data.iter()
        .filter(|&&x| !x.is_nan())
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
}

// ─── Column Helpers ───────────────────────────────────────────────────────────

/// Sums an Int32 column.
pub fn sum_i32_column(col: &ColumnVector) -> Option<i64> {
    col.as_int32_slice().map(sum_i32)
}

/// Sums a Float64 column.
pub fn sum_f64_column(col: &ColumnVector) -> Option<f64> {
    col.as_float64_slice().map(sum_f64)
}

/// Averages a Float64 column.
pub fn avg_f64_column(col: &ColumnVector) -> Option<f64> {
    col.as_float64_slice().map(avg_f64)
}

/// Counts non-NaN values in a Float64 column.
pub fn count_f64_column(col: &ColumnVector) -> Option<usize> {
    col.as_float64_slice().map(count_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_i32_basic() {
        let data = vec![1, 2, 3, 4, 5];
        assert_eq!(sum_i32(&data), 15);
    }

    #[test]
    fn test_sum_i32_empty() {
        let data: Vec<i32> = vec![];
        assert_eq!(sum_i32(&data), 0);
    }

    #[test]
    fn test_sum_i32_large() {
        let data: Vec<i32> = (1..=1000).collect();
        assert_eq!(sum_i32(&data), 500500);
    }

    #[test]
    fn test_avg_i32_basic() {
        let data = vec![1, 2, 3, 4, 5];
        assert!((avg_i32(&data) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_avg_i32_empty() {
        let data: Vec<i32> = vec![];
        assert_eq!(avg_i32(&data), 0.0);
    }

    #[test]
    fn test_sum_i64_basic() {
        let data = vec![10i64, 20, 30, 40, 50];
        assert_eq!(sum_i64(&data), 150);
    }

    #[test]
    fn test_avg_i64_basic() {
        let data = vec![10i64, 20, 30];
        assert!((avg_i64(&data) - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_sum_f64_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((sum_f64(&data) - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_sum_f64_empty() {
        let data: Vec<f64> = vec![];
        assert_eq!(sum_f64(&data), 0.0);
    }

    #[test]
    fn test_avg_f64_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((avg_f64(&data) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_avg_f64_with_nan() {
        let data = vec![1.0, f64::NAN, 3.0, 4.0, 5.0];
        // avg of 1,3,4,5 = 13/4 = 3.25
        assert!((avg_f64(&data) - 3.25).abs() < 1e-10);
    }

    #[test]
    fn test_avg_f64_all_nan() {
        let data = vec![f64::NAN, f64::NAN, f64::NAN];
        assert!(avg_f64(&data).is_nan());
    }

    #[test]
    fn test_count_f64_basic() {
        let data = vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0];
        assert_eq!(count_f64(&data), 3);
    }

    #[test]
    fn test_min_f64_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        assert!((min_f64(&data) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_max_f64_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        assert!((max_f64(&data) - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_min_f64_with_nan() {
        let data = vec![3.0, f64::NAN, 1.0, 4.0];
        assert!((min_f64(&data) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_max_f64_with_nan() {
        let data = vec![3.0, f64::NAN, 9.0, 4.0];
        assert!((max_f64(&data) - 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_sum_f64_ignore_nan() {
        let data = vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0];
        assert!((sum_f64_ignore_nan(&data) - 9.0).abs() < 1e-10);
    }
}

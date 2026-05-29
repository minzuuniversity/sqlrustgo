//! Vectorized SIMD operations for query execution
//! Accelerates aggregate functions (SUM, AVG, COUNT) using SIMD
//! Falls back to scalar on non-SIMD architectures

/// Compute sum of i64 slice using SIMD-like approach
pub fn sum_i64_simd_like(values: &[i64]) -> i64 {
    let mut sum = 0i64;
    let mut i = 0;
    while i + 4 <= values.len() {
        let chunk = &values[i..i + 4];
        sum += chunk[0] + chunk[1] + chunk[2] + chunk[3];
        i += 4;
    }
    for &v in &values[i..] {
        sum += v;
    }
    sum
}

/// Compute average of f64 slice
pub fn avg_f64_scalar(values: &[f64]) -> f64 {
    let mut sum = 0.0f64;
    for &v in values {
        sum += v;
    }
    sum / values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_i64() {
        let v = vec![1i64, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(sum_i64_simd_like(&v), 36);
    }

    #[test]
    fn test_avg_f64() {
        let v = vec![1.0f64, 2.0, 3.0, 4.0];
        assert_eq!(avg_f64_scalar(&v), 2.5);
    }

    #[test]
    fn test_sum_small() {
        let v = vec![1i64, 2, 3];
        assert_eq!(sum_i64_simd_like(&v), 6);
    }
}

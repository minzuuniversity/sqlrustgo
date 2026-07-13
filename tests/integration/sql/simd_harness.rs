//! SIMD Harness (P3-5 #3184)
//!
//! Shared utilities for SIMD integration testing. Provides:
//! - `SimdConfig` (lanes, target_arch)
//! - `TargetArch` (X86_64, Aarch64)
//! - `detect_simd` — runtime feature detection simulation
//! - `simd_eq_i32` / `simd_sum_i32` / `simd_dot_product_f32`
//!   (mock implementations, scalar fallback for unit tests)
//!
//! This file is **not** a test target itself (no `#[test]`); shared
//! by `simd_test.rs`.

#![allow(dead_code)] // helpers consumed by test targets

/// Target architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Unknown,
}

/// SIMD configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimdConfig {
    pub simd_lanes: usize, // 4 (SSE2), 8 (AVX2), 4 (NEON)
    pub target_arch: TargetArch,
}

impl SimdConfig {
    pub fn new(target_arch: TargetArch, simd_lanes: usize) -> Self {
        Self {
            simd_lanes,
            target_arch,
        }
    }

    pub fn x86_sse2() -> Self {
        Self::new(TargetArch::X86_64, 4)
    }

    pub fn x86_avx2() -> Self {
        Self::new(TargetArch::X86_64, 8)
    }

    pub fn aarch64_neon() -> Self {
        Self::new(TargetArch::Aarch64, 4)
    }

    pub fn unknown() -> Self {
        Self::new(TargetArch::Unknown, 1)
    }
}

/// Detect SIMD lanes for the current target (mock — returns a
/// reasonable default).
pub fn detect_simd() -> SimdConfig {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            SimdConfig::x86_avx2()
        } else {
            SimdConfig::x86_sse2()
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        SimdConfig::aarch64_neon()
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        SimdConfig::unknown()
    }
}

/// SIMD-style element-wise equality (mock — scalar fallback).
pub fn simd_eq_i32(a: &[i32], b: &[i32]) -> Vec<bool> {
    let n = a.len().min(b.len());
    a.iter()
        .take(n)
        .zip(b.iter().take(n))
        .map(|(x, y)| x == y)
        .collect()
}

/// SIMD-style element-wise less-than.
pub fn simd_lt_i32(a: &[i32], b: &[i32]) -> Vec<bool> {
    let n = a.len().min(b.len());
    a.iter()
        .take(n)
        .zip(b.iter().take(n))
        .map(|(x, y)| x < y)
        .collect()
}

/// SIMD-style element-wise greater-than.
pub fn simd_gt_i32(a: &[i32], b: &[i32]) -> Vec<bool> {
    let n = a.len().min(b.len());
    a.iter()
        .take(n)
        .zip(b.iter().take(n))
        .map(|(x, y)| x > y)
        .collect()
}

/// SIMD-style element-wise not-equal.
pub fn simd_ne_i32(a: &[i32], b: &[i32]) -> Vec<bool> {
    let n = a.len().min(b.len());
    a.iter()
        .take(n)
        .zip(b.iter().take(n))
        .map(|(x, y)| x != y)
        .collect()
}

/// SIMD-style sum (mock — scalar fallback).
pub fn simd_sum_i32(a: &[i32]) -> i64 {
    a.iter().map(|x| *x as i64).sum()
}

/// SIMD-style average.
pub fn simd_avg_i32(a: &[i32]) -> f64 {
    if a.is_empty() {
        0.0
    } else {
        simd_sum_i32(a) as f64 / a.len() as f64
    }
}

/// SIMD-style min.
pub fn simd_min_i32(a: &[i32]) -> Option<i32> {
    a.iter().min().copied()
}

/// SIMD-style max.
pub fn simd_max_i32(a: &[i32]) -> Option<i32> {
    a.iter().max().copied()
}

/// SIMD-style dot product.
pub fn simd_dot_product_f32(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    a.iter()
        .take(n)
        .zip(b.iter().take(n))
        .map(|(x, y)| x * y)
        .sum()
}

/// SIMD-style batch distance — returns the dot product of query
/// against each vector.
pub fn simd_batch_distance(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| simd_dot_product_f32(query, v))
        .collect()
}

/// String find (char) — scalar fallback.
pub fn string_find_char(s: &str, c: char) -> Option<usize> {
    s.find(c)
}

/// String find (substring) — scalar fallback.
pub fn string_find_substring(s: &str, sub: &str) -> Option<usize> {
    s.find(sub)
}

/// ASCII lowercase conversion (scalar fallback).
pub fn ascii_lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn simd_config_lanes() {
        assert_eq!(SimdConfig::x86_sse2().simd_lanes, 4);
        assert_eq!(SimdConfig::x86_avx2().simd_lanes, 8);
        assert_eq!(SimdConfig::aarch64_neon().simd_lanes, 4);
    }

    #[test]
    fn simd_eq_basic() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 0, 3, 0];
        let r = simd_eq_i32(&a, &b);
        assert_eq!(r, vec![true, false, true, false]);
    }

    #[test]
    fn simd_sum_basic() {
        let a = vec![1, 2, 3, 4, 5];
        assert_eq!(simd_sum_i32(&a), 15);
    }

    #[test]
    fn simd_dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert!((simd_dot_product_f32(&a, &b) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn simd_batch_distance_basic() {
        let query = vec![1.0, 2.0];
        let vectors = vec![vec![3.0, 4.0], vec![5.0, 6.0]];
        let r = simd_batch_distance(&query, &vectors);
        assert_eq!(r.len(), 2);
        assert!((r[0] - 11.0).abs() < 1e-6); // 1*3 + 2*4 = 11
        assert!((r[1] - 17.0).abs() < 1e-6); // 1*5 + 2*6 = 17
    }
}

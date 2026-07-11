//! SIMD-accelerated batch filter evaluation
//!
//! v3.10.0 Issue #3703 Phase 5: SIMD optimization for single-core speedup
//!
//! Provides batch predicate evaluation for columnar data. Uses portable
//! SIMD-like operations (4-element chunks) that work on all architectures
//! without requiring nightly Rust.
//!
//! Feature gate: `simd` (experimental)

use sqlrustgo_types::Value;

/// BitMask result from batch predicate evaluation
///
/// Each bit represents whether a row passed the predicate.
/// Limited to 64 elements per batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitMask {
    bits: u64,
}

impl BitMask {
    /// Create a new BitMask with the given u64
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    /// Create an all-true mask of the given length (up to 64)
    pub fn all_true(len: usize) -> Self {
        let len = len.min(64);
        Self {
            bits: if len == 64 {
                u64::MAX
            } else {
                (1u64 << len) - 1
            },
        }
    }

    /// Create an all-false mask
    pub const fn all_false() -> Self {
        Self { bits: 0 }
    }

    /// Check if the bit at position i is set
    pub fn is_set(&self, i: usize) -> bool {
        if i >= 64 {
            false
        } else {
            (self.bits >> i) & 1 == 1
        }
    }

    /// Get the raw bits
    pub fn bits(&self) -> u64 {
        self.bits
    }

    /// Count the number of set bits
    pub fn count(&self) -> u32 {
        self.bits.count_ones()
    }

    /// Bitwise AND of two masks
    pub fn and(self, other: Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    /// Bitwise OR of two masks
    pub fn or(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

/// Batch predicate trait for SIMD evaluation
pub trait BatchPredicate {
    /// Evaluate a batch of up to 64 values
    /// Returns a BitMask with 1 = passed, 0 = failed
    fn eval_batch_i64(&self, values: &[i64]) -> BitMask;
}

/// Predicate: value < threshold
pub struct LessThanPredicate {
    pub threshold: i64,
}

impl BatchPredicate for LessThanPredicate {
    fn eval_batch_i64(&self, values: &[i64]) -> BitMask {
        // Portable SIMD-like: process 4 elements at a time
        let mut mask = BitMask::all_false();
        let len = values.len().min(64);

        let mut i = 0;
        // Process 4 elements at a time
        while i + 4 <= len {
            let pass_0 = (values[i] < self.threshold) as u64;
            let pass_1 = (values[i + 1] < self.threshold) as u64;
            let pass_2 = (values[i + 2] < self.threshold) as u64;
            let pass_3 = (values[i + 3] < self.threshold) as u64;

            let chunk = pass_0 | (pass_1 << 1) | (pass_2 << 2) | (pass_3 << 3);
            mask.bits |= chunk << i;
            i += 4;
        }

        // Tail
        while i < len {
            if values[i] < self.threshold {
                mask.bits |= 1 << i;
            }
            i += 1;
        }

        mask
    }
}

/// Predicate: value > threshold
pub struct GreaterThanPredicate {
    pub threshold: i64,
}

impl BatchPredicate for GreaterThanPredicate {
    fn eval_batch_i64(&self, values: &[i64]) -> BitMask {
        let mut mask = BitMask::all_false();
        let len = values.len().min(64);

        let mut i = 0;
        while i + 4 <= len {
            let pass_0 = (values[i] > self.threshold) as u64;
            let pass_1 = (values[i + 1] > self.threshold) as u64;
            let pass_2 = (values[i + 2] > self.threshold) as u64;
            let pass_3 = (values[i + 3] > self.threshold) as u64;

            let chunk = pass_0 | (pass_1 << 1) | (pass_2 << 2) | (pass_3 << 3);
            mask.bits |= chunk << i;
            i += 4;
        }

        while i < len {
            if values[i] > self.threshold {
                mask.bits |= 1 << i;
            }
            i += 1;
        }

        mask
    }
}

/// Predicate: value == literal
pub struct EqualsPredicate {
    pub value: i64,
}

impl BatchPredicate for EqualsPredicate {
    fn eval_batch_i64(&self, values: &[i64]) -> BitMask {
        let mut mask = BitMask::all_false();
        let len = values.len().min(64);

        let mut i = 0;
        while i + 4 <= len {
            let pass_0 = (values[i] == self.value) as u64;
            let pass_1 = (values[i + 1] == self.value) as u64;
            let pass_2 = (values[i + 2] == self.value) as u64;
            let pass_3 = (values[i + 3] == self.value) as u64;

            let chunk = pass_0 | (pass_1 << 1) | (pass_2 << 2) | (pass_3 << 3);
            mask.bits |= chunk << i;
            i += 4;
        }

        while i < len {
            if values[i] == self.value {
                mask.bits |= 1 << i;
            }
            i += 1;
        }

        mask
    }
}

/// Extract a column of i64 values from a columnar record batch
///
/// Returns None if the column doesn't exist or values are not i64.
pub fn extract_i64_column(records: &[Vec<Value>], col_idx: usize) -> Option<Vec<i64>> {
    let mut result = Vec::with_capacity(records.len());
    for record in records {
        if col_idx >= record.len() {
            return None;
        }
        if let Value::Integer(i) = &record[col_idx] {
            result.push(*i);
        } else {
            return None;
        }
    }
    Some(result)
}

/// Apply a BitMask to a record batch, returning only the passing records
pub fn apply_mask(records: &[Vec<Value>], mask: BitMask) -> Vec<Vec<Value>> {
    let mut result = Vec::new();
    for (i, record) in records.iter().enumerate() {
        if i < 64 && mask.is_set(i) {
            result.push(record.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmask_all_true() {
        let mask = BitMask::all_true(8);
        assert_eq!(mask.count(), 8);
        for i in 0..8 {
            assert!(mask.is_set(i));
        }
    }

    #[test]
    fn test_bitmask_all_false() {
        let mask = BitMask::all_false();
        assert_eq!(mask.count(), 0);
    }

    #[test]
    fn test_less_than_predicate() {
        let pred = LessThanPredicate { threshold: 5 };
        let values: Vec<i64> = (0..10).collect();
        let mask = pred.eval_batch_i64(&values);

        // 0..5 should pass, 5..10 should fail
        for i in 0..5 {
            assert!(mask.is_set(i), "Index {} should pass", i);
        }
        for i in 5..10 {
            assert!(!mask.is_set(i), "Index {} should fail", i);
        }
    }

    #[test]
    fn test_greater_than_predicate() {
        let pred = GreaterThanPredicate { threshold: 5 };
        let values: Vec<i64> = (0..10).collect();
        let mask = pred.eval_batch_i64(&values);

        // 0..5 should fail, 5..10 should pass (5 > 5 is false, 6..10 are true)
        for i in 0..=5 {
            assert!(!mask.is_set(i), "Index {} should fail", i);
        }
        for i in 6..10 {
            assert!(mask.is_set(i), "Index {} should pass", i);
        }
    }

    #[test]
    fn test_equals_predicate() {
        let pred = EqualsPredicate { value: 5 };
        let values: Vec<i64> = (0..10).collect();
        let mask = pred.eval_batch_i64(&values);

        for i in 0..10 {
            if i == 5 {
                assert!(mask.is_set(i), "Index 5 should pass");
            } else {
                assert!(!mask.is_set(i), "Index {} should fail", i);
            }
        }
    }

    #[test]
    fn test_simd_chunk_processing() {
        let pred = LessThanPredicate { threshold: 100 };
        // Test with size that exercises chunk processing (multiple of 4)
        let values: Vec<i64> = (0..16).collect();
        let mask = pred.eval_batch_i64(&values);

        // All should pass
        assert_eq!(mask.count(), 16);
    }

    #[test]
    fn test_extract_i64_column() {
        let records = vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
            vec![Value::Integer(3), Value::Integer(30)],
        ];
        let col = extract_i64_column(&records, 0);
        assert_eq!(col, Some(vec![1, 2, 3]));

        let col = extract_i64_column(&records, 1);
        assert_eq!(col, Some(vec![10, 20, 30]));
    }

    #[test]
    fn test_extract_i64_column_invalid() {
        let records = vec![vec![Value::Text("hello".into())]];
        let col = extract_i64_column(&records, 0);
        assert_eq!(col, None);
    }

    #[test]
    fn test_apply_mask() {
        let records = vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
            vec![Value::Integer(3)],
            vec![Value::Integer(4)],
        ];
        let mut mask = BitMask::all_false();
        mask.bits |= 0b0101; // Pass indices 0 and 2

        let result = apply_mask(&records, mask);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], records[0]);
        assert_eq!(result[1], records[2]);
    }

    #[test]
    fn test_mask_and_operation() {
        let m1 = BitMask::from_bits(0b1100);
        let m2 = BitMask::from_bits(0b1010);
        let m3 = m1.and(m2);
        assert_eq!(m3.bits(), 0b1000);
    }

    #[test]
    fn test_mask_or_operation() {
        let m1 = BitMask::from_bits(0b1100);
        let m2 = BitMask::from_bits(0b1010);
        let m3 = m1.or(m2);
        assert_eq!(m3.bits(), 0b1110);
    }
}

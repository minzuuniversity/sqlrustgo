//! ColumnVector: a single column of typed data with null support.
use super::types::{PrimitiveType, PrimitiveValue};
use serde::{Deserialize, Serialize};

/// A bitmap for tracking null values in a column.
/// Stored as a packed bitset for cache efficiency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NullBitmap {
    /// Packed null bits. Bit i = 1 means null.
    bits: Vec<u64>,
    /// Number of rows in this column.
    len: usize,
}

impl NullBitmap {
    /// Creates a new NullBitmap with all values set to null.
    pub fn new_none(len: usize) -> Self {
        let num_words = (len + 63) / 64;
        Self {
            bits: vec![u64::MAX; num_words],
            len,
        }
    }

    /// Creates a new NullBitmap with all values set to non-null.
    pub fn new_all_valid(len: usize) -> Self {
        let num_words = (len + 63) / 64;
        Self {
            bits: vec![0u64; num_words],
            len,
        }
    }

    /// Returns the number of rows.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if there are no rows.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns true if row `i` is null.
    #[inline]
    pub fn is_null(&self, i: usize) -> bool {
        debug_assert!(i < self.len);
        let word = self.bits[i / 64];
        let bit = (word >> (i % 64)) & 1;
        bit == 1
    }

    /// Sets row `i` as null (bit = 1).
    #[inline]
    pub fn set_null(&mut self, i: usize) {
        debug_assert!(i < self.len);
        self.bits[i / 64] |= 1u64 << (i % 64);
    }

    /// Sets row `i` as non-null (bit = 0).
    #[inline]
    pub fn set_valid(&mut self, i: usize) {
        debug_assert!(i < self.len);
        self.bits[i / 64] &= !(1u64 << (i % 64));
    }

    /// Returns the count of null values.
    pub fn null_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.len {
            if self.is_null(i) {
                count += 1;
            }
        }
        count
    }
}

/// Runtime column data storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnData {
    Int32(Vec<i32>),
    Int64(Vec<i64>),
    Float64(Vec<f64>),
    String(Vec<String>),
    Blob(Vec<Vec<u8>>),
    Bool(Vec<u8>), // 0=false, 1=true, 2=NULL
    UInt32(Vec<u32>),
    Date(Vec<i32>),
    Timestamp(Vec<i64>),
}

impl ColumnData {
    /// Returns the type of this column.
    pub fn type_id(&self) -> PrimitiveType {
        match self {
            ColumnData::Int32(_) => PrimitiveType::Int32,
            ColumnData::Int64(_) => PrimitiveType::Int64,
            ColumnData::Float64(_) => PrimitiveType::Float64,
            ColumnData::String(_) => PrimitiveType::String,
            ColumnData::Blob(_) => PrimitiveType::Blob,
            ColumnData::Bool(_) => PrimitiveType::Bool,
            ColumnData::UInt32(_) => PrimitiveType::UInt32,
            ColumnData::Date(_) => PrimitiveType::Date,
            ColumnData::Timestamp(_) => PrimitiveType::Timestamp,
        }
    }

    /// Returns the number of rows.
    pub fn len(&self) -> usize {
        match self {
            ColumnData::Int32(v) => v.len(),
            ColumnData::Int64(v) => v.len(),
            ColumnData::Float64(v) => v.len(),
            ColumnData::String(v) => v.len(),
            ColumnData::Blob(v) => v.len(),
            ColumnData::Bool(v) => v.len(),
            ColumnData::UInt32(v) => v.len(),
            ColumnData::Date(v) => v.len(),
            ColumnData::Timestamp(v) => v.len(),
        }
    }

    /// Returns true if the column is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single column vector with type and null information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnVector {
    /// The column data.
    data: ColumnData,
    /// Null bitmap.
    nulls: NullBitmap,
}

impl ColumnVector {
    /// Creates a new ColumnVector from data with all values non-null.
    pub fn new(data: ColumnData) -> Self {
        let len = data.len();
        Self {
            data,
            nulls: NullBitmap::new_all_valid(len),
        }
    }

    /// Creates a new ColumnVector from data with specified nulls.
    pub fn new_with_nulls(data: ColumnData, nulls: NullBitmap) -> Self {
        debug_assert_eq!(data.len(), nulls.len());
        Self { data, nulls }
    }

    /// Returns the number of rows.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the column has no rows.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the data type.
    #[inline]
    pub fn type_id(&self) -> PrimitiveType {
        self.data.type_id()
    }

    /// Returns true if row `i` is null.
    #[inline]
    pub fn is_null(&self, i: usize) -> bool {
        self.nulls.is_null(i)
    }

    /// Gets the value at row `i` as a PrimitiveValue.
    pub fn get(&self, i: usize) -> PrimitiveValue {
        if self.is_null(i) {
            return self.type_id().null_value();
        }
        match &self.data {
            ColumnData::Int32(v) => PrimitiveValue::Int32(v[i]),
            ColumnData::Int64(v) => PrimitiveValue::Int64(v[i]),
            ColumnData::Float64(v) => PrimitiveValue::Float64(v[i]),
            ColumnData::String(v) => PrimitiveValue::String(v[i].clone()),
            ColumnData::Blob(v) => PrimitiveValue::Blob(v[i].clone()),
            ColumnData::Bool(v) => PrimitiveValue::Bool(v[i] == 1),
            ColumnData::UInt32(v) => PrimitiveValue::UInt32(v[i]),
            ColumnData::Date(v) => PrimitiveValue::Date(v[i]),
            ColumnData::Timestamp(v) => PrimitiveValue::Timestamp(v[i]),
        }
    }

    // ─── SIMD-friendly accessors ───────────────────────────────────────────

    /// Returns the Int32 slice for SIMD operations.
    #[inline]
    pub fn as_int32_slice(&self) -> Option<&[i32]> {
        match &self.data {
            ColumnData::Int32(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns the Int64 slice for SIMD operations.
    #[inline]
    pub fn as_int64_slice(&self) -> Option<&[i64]> {
        match &self.data {
            ColumnData::Int64(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns the Float64 slice for SIMD operations.
    #[inline]
    pub fn as_float64_slice(&self) -> Option<&[f64]> {
        match &self.data {
            ColumnData::Float64(v) => Some(v.as_slice()),
            _ => None,
        }
    }

    /// Returns a reference to the null bitmap.
    #[inline]
    pub fn nulls(&self) -> &NullBitmap {
        &self.nulls
    }

    /// Returns the underlying ColumnData.
    #[inline]
    pub fn data(&self) -> &ColumnData {
        &self.data
    }
}

// ─── Builders ────────────────────────────────────────────────────────────────

impl ColumnVector {
    /// Creates an Int32 column.
    pub fn from_int32(values: Vec<i32>) -> Self {
        Self::new(ColumnData::Int32(values))
    }

    /// Creates an Int64 column.
    pub fn from_int64(values: Vec<i64>) -> Self {
        Self::new(ColumnData::Int64(values))
    }

    /// Creates a Float64 column.
    pub fn from_float64(values: Vec<f64>) -> Self {
        Self::new(ColumnData::Float64(values))
    }

    /// Creates a String column.
    pub fn from_string(values: Vec<String>) -> Self {
        Self::new(ColumnData::String(values))
    }

    /// Creates a Bool column (0=false, 1=true, 2=NULL).
    pub fn from_bool(values: Vec<u8>) -> Self {
        Self::new(ColumnData::Bool(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_bitmap() {
        let mut nb = NullBitmap::new_all_valid(100);
        assert!(!nb.is_null(0));
        nb.set_null(5);
        assert!(nb.is_null(5));
        assert!(!nb.is_null(4));
        nb.set_valid(5);
        assert!(!nb.is_null(5));
    }

    #[test]
    fn test_column_vector_int32() {
        let col = ColumnVector::from_int32(vec![1, 2, 3, 4, 5]);
        assert_eq!(col.len(), 5);
        assert_eq!(col.get(0), PrimitiveValue::Int32(1));
        assert!(!col.is_null(0));
    }

    #[test]
    fn test_column_vector_string() {
        let col = ColumnVector::from_string(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(col.len(), 3);
        assert_eq!(col.get(1), PrimitiveValue::String("b".into()));
    }

    #[test]
    fn test_column_vector_with_nulls() {
        let mut nb = NullBitmap::new_all_valid(3);
        nb.set_null(1);
        let col = ColumnVector::new_with_nulls(ColumnData::Int32(vec![1, 2, 3]), nb);
        assert!(!col.is_null(0));
        assert!(col.is_null(1));
        assert!(!col.is_null(2));
        assert_eq!(col.get(1), PrimitiveValue::Int32(0)); // null returns null_value
    }

    #[test]
    fn test_simd_accessors() {
        let col = ColumnVector::from_int32(vec![1, 2, 3]);
        assert_eq!(col.as_int32_slice(), Some(&[1, 2, 3][..]));
        assert!(col.as_int64_slice().is_none());

        let col64 = ColumnVector::from_int64(vec![1, 2, 3]);
        assert!(col64.as_int32_slice().is_none());
        assert_eq!(col64.as_int64_slice(), Some(&[1, 2, 3][..]));
    }
}

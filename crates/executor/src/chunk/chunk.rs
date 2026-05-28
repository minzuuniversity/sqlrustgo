//! Chunk: a batch of rows stored in columnar format.
//!
//! A Chunk is the fundamental data unit for pipeline execution.
//! It contains multiple ColumnVectors of equal length, representing
//! a batch of rows.

use super::column_vector::ColumnVector;
use serde::{Deserialize, Serialize};

/// A chunk is a collection of column vectors of equal length.
/// It represents a batch of rows for pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The columns in this chunk.
    columns: Vec<ColumnVector>,
    /// Number of rows in this chunk.
    rows: usize,
}

impl Chunk {
    /// Creates a new Chunk from a vector of column vectors.
    ///
    /// # Panics
    ///
    /// Panics if any column has a different length than the first column.
    pub fn new(columns: Vec<ColumnVector>) -> Self {
        if columns.is_empty() {
            return Self { columns, rows: 0 };
        }
        let rows = columns[0].len();
        for col in &columns {
            assert_eq!(
                col.len(),
                rows,
                "all columns must have the same length: expected {}, got {}",
                rows,
                col.len()
            );
        }
        Self { columns, rows }
    }

    /// Returns the number of rows.
    #[inline]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the number of columns.
    #[inline]
    pub fn columns(&self) -> usize {
        self.columns.len()
    }

    /// Returns true if the chunk is empty (no rows).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// Returns the column at index `col_idx`.
    ///
    /// # Panics
    ///
    /// Panics if `col_idx >= self.columns()`.
    #[inline]
    pub fn column(&self, col_idx: usize) -> &ColumnVector {
        &self.columns[col_idx]
    }

    /// Returns an iterator over columns.
    pub fn columns_iter(&self) -> impl Iterator<Item = &ColumnVector> {
        self.columns.iter()
    }

    /// Returns all columns as a slice.
    pub fn columns_slice(&self) -> &[ColumnVector] {
        &self.columns
    }
}

// ─── Chunk builders ──────────────────────────────────────────────────────────

impl Chunk {
    /// Builds a Chunk from a vector of (column_name, ColumnVector).
    /// This is a convenience wrapper.
    pub fn from_columns(columns: Vec<ColumnVector>) -> Self {
        Self::new(columns)
    }
}

#[cfg(test)]
mod tests {
    use crate::chunk::{Chunk, ColumnVector};

    #[test]
    fn test_chunk_basic() {
        let col1 = ColumnVector::from_int32(vec![1, 2, 3]);
        let col2 = ColumnVector::from_int64(vec![10, 20, 30]);
        let chunk = Chunk::new(vec![col1, col2]);
        assert_eq!(chunk.rows(), 3);
        assert_eq!(chunk.columns(), 2);
    }

    #[test]
    fn test_chunk_empty() {
        let chunk: Chunk = Chunk::new(vec![]);
        assert!(chunk.is_empty());
        assert_eq!(chunk.rows(), 0);
    }

    #[test]
    #[should_panic]
    fn test_chunk_mismatched_lengths() {
        let col1 = ColumnVector::from_int32(vec![1, 2, 3]);
        let col2 = ColumnVector::from_int64(vec![10, 20]);
        Chunk::new(vec![col1, col2]);
    }

    #[test]
    fn test_chunk_column_access() {
        let col1 = ColumnVector::from_int32(vec![1, 2, 3]);
        let col2 = ColumnVector::from_string(vec!["a".into(), "b".into(), "c".into()]);
        let chunk = Chunk::new(vec![col1, col2]);

        assert_eq!(chunk.column(0).len(), 3);
        assert_eq!(chunk.column(1).len(), 3);
    }
}

//! RecordBatch - columnar data format for SIMD-friendly filtering
//!
//! v3.10.0 Issue #3703 Phase 5: Columnar RecordBatch for filter stage
//!
//! Provides columnar storage for use with SIMD batch evaluation.
//! Rows are stored in columns rather than rows, enabling efficient
//! vectorized operations on individual columns.

use sqlrustgo_types::Value;

/// RecordBatch - columnar data format
///
/// Unlike the row-oriented `Vec<Vec<Value>>`, RecordBatch stores
/// each column as a separate vector, enabling SIMD operations on
/// individual columns.
#[derive(Debug, Clone)]
pub struct RecordBatch {
    /// Column names
    columns: Vec<String>,
    /// Column data - one Vec per column
    data: Vec<Vec<Value>>,
    /// Number of rows
    num_rows: usize,
}

impl RecordBatch {
    /// Create an empty RecordBatch
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            data: Vec::new(),
            num_rows: 0,
        }
    }

    /// Create a new RecordBatch from columnar data
    pub fn new(columns: Vec<String>, data: Vec<Vec<Value>>) -> Self {
        let num_rows = data.first().map(|c| c.len()).unwrap_or(0);
        Self {
            columns,
            data,
            num_rows,
        }
    }

    /// Create a RecordBatch from row-oriented data
    ///
    /// Converts `Vec<Vec<Value>>` (rows) into columnar format.
    /// Returns None if rows are inconsistent.
    pub fn from_rows(column_names: Vec<String>, rows: Vec<Vec<Value>>) -> Option<Self> {
        if rows.is_empty() {
            return Some(Self::new(column_names, Vec::new()));
        }
        let num_cols = rows[0].len();
        if column_names.len() != num_cols {
            return None;
        }
        // Verify all rows have same number of columns
        for row in &rows {
            if row.len() != num_cols {
                return None;
            }
        }

        let num_rows = rows.len();
        let mut data: Vec<Vec<Value>> = (0..num_cols)
            .map(|_| Vec::with_capacity(num_rows))
            .collect();

        for row in rows {
            for (col_idx, val) in row.into_iter().enumerate() {
                data[col_idx].push(val);
            }
        }

        Some(Self {
            columns: column_names,
            data,
            num_rows,
        })
    }

    /// Get the number of rows
    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    /// Get the number of columns
    pub fn num_columns(&self) -> usize {
        self.columns.len()
    }

    /// Get the column names
    pub fn column_names(&self) -> &[String] {
        &self.columns
    }

    /// Get a specific column's data
    pub fn column(&self, idx: usize) -> Option<&Vec<Value>> {
        self.data.get(idx)
    }

    /// Get a specific column by name
    pub fn column_by_name(&self, name: &str) -> Option<&Vec<Value>> {
        self.columns
            .iter()
            .position(|n| n == name)
            .and_then(|idx| self.data.get(idx))
    }

    /// Convert back to row-oriented format
    pub fn to_rows(&self) -> Vec<Vec<Value>> {
        let mut rows = Vec::with_capacity(self.num_rows);
        for row_idx in 0..self.num_rows {
            let mut row = Vec::with_capacity(self.num_columns());
            for col in &self.data {
                row.push(col[row_idx].clone());
            }
            rows.push(row);
        }
        rows
    }

    /// Apply a BitMask to filter rows
    pub fn filter(&self, mask: &crate::simd_eval::BitMask) -> Self {
        let mut new_data: Vec<Vec<Value>> = self
            .data
            .iter()
            .map(|_col| Vec::with_capacity(self.num_rows / 2))
            .collect();

        for row_idx in 0..self.num_rows.min(64) {
            if mask.is_set(row_idx) {
                for (col_idx, col) in self.data.iter().enumerate() {
                    new_data[col_idx].push(col[row_idx].clone());
                }
            }
        }

        Self {
            columns: self.columns.clone(),
            data: new_data,
            num_rows: mask.count() as usize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simd_eval::BitMask;
    use sqlrustgo_types::Value;

    #[test]
    fn test_empty_batch() {
        let batch = RecordBatch::empty();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 0);
    }

    #[test]
    fn test_from_rows() {
        let rows = vec![
            vec![Value::Integer(1), Value::Text("a".into())],
            vec![Value::Integer(2), Value::Text("b".into())],
            vec![Value::Integer(3), Value::Text("c".into())],
        ];
        let columns = vec!["id".to_string(), "name".to_string()];

        let batch = RecordBatch::from_rows(columns, rows).unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert_eq!(batch.num_columns(), 2);

        // Verify columnar layout
        let id_col = batch.column(0).unwrap();
        assert_eq!(id_col.len(), 3);
        assert_eq!(id_col[0], Value::Integer(1));
        assert_eq!(id_col[2], Value::Integer(3));
    }

    #[test]
    fn test_to_rows() {
        let rows = vec![
            vec![Value::Integer(1), Value::Text("a".into())],
            vec![Value::Integer(2), Value::Text("b".into())],
        ];
        let columns = vec!["id".to_string(), "name".to_string()];

        let batch = RecordBatch::from_rows(columns, rows.clone()).unwrap();
        let roundtrip = batch.to_rows();
        assert_eq!(roundtrip, rows);
    }

    #[test]
    fn test_column_by_name() {
        let rows = vec![
            vec![Value::Integer(1), Value::Text("a".into())],
            vec![Value::Integer(2), Value::Text("b".into())],
        ];
        let columns = vec!["id".to_string(), "name".to_string()];
        let batch = RecordBatch::from_rows(columns, rows).unwrap();

        let id_col = batch.column_by_name("id").unwrap();
        assert_eq!(id_col.len(), 2);
        assert_eq!(id_col[0], Value::Integer(1));

        let missing = batch.column_by_name("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_inconsistent_rows() {
        let rows = vec![
            vec![Value::Integer(1), Value::Text("a".into())],
            vec![Value::Integer(2)], // Missing column
        ];
        let columns = vec!["id".to_string(), "name".to_string()];
        let batch = RecordBatch::from_rows(columns, rows);
        assert!(batch.is_none());
    }

    #[test]
    fn test_filter_batch() {
        let rows = vec![
            vec![Value::Integer(1), Value::Text("a".into())],
            vec![Value::Integer(2), Value::Text("b".into())],
            vec![Value::Integer(3), Value::Text("c".into())],
            vec![Value::Integer(4), Value::Text("d".into())],
        ];
        let columns = vec!["id".to_string(), "name".to_string()];
        let batch = RecordBatch::from_rows(columns, rows).unwrap();

        // Pass indices 1 and 3
        let mask = BitMask::from_bits(0b1010);
        let filtered = batch.filter(&mask);

        assert_eq!(filtered.num_rows(), 2);
        let id_col = filtered.column(0).unwrap();
        assert_eq!(id_col[0], Value::Integer(2));
        assert_eq!(id_col[1], Value::Integer(4));
    }
}

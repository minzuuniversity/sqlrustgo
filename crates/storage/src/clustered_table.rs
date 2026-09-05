//! ClusteredTable: InnoDB-style B+ Tree table storage
//!
//! V311-01 (F-23 Clustered Index main-path integration)
//!
//! A `ClusteredTable` stores rows ordered by primary key in a B+ Tree
//! (in-memory `BTreeMap<Value, Record>` for V311-01 v1; disk-backed pages
//! deferred to v3.12+).
//!
//! Each row is keyed by the primary key column value (single-column PK only
//! for V311-01 v1). Range scans produce sorted output automatically.
//!
//! API surface is the `StorageEngine` trait minus rarely-used methods
//! (which delegate to an internal `MemoryStorage` for non-PK path access).
//!
//! Backing structure:
//! - `pk_index: BTreeMap<Value, Record>` ← primary key → row
//! - `secondary_index: HashMap<String, BTreeMap<Value, Vec<Value>>>` ← col → index
//! - `table_info: TableInfo` ← schema

use crate::engine::{Record, TableInfo};
use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::collections::{BTreeMap, HashMap};

/// InnoDB-style clustered table storage.
pub struct ClusteredTable {
    /// Primary key column index (0-based into Record).
    pk_col_idx: usize,
    /// The actual B+ Tree: pk_value → row.
    pk_index: BTreeMap<Value, Record>,
    /// Secondary indexes: column_name → index_value → list_of_pk_values.
    #[allow(dead_code)]
    secondary_index: HashMap<String, BTreeMap<Value, Vec<Value>>>,
    /// Schema metadata.
    table_info: TableInfo,
    /// Page split counter (for benchmarking).
    splits: u64,
}

impl ClusteredTable {
    /// Create a new `ClusteredTable`.
    /// `pk_col_idx` is the 0-based index of the PRIMARY KEY column.
    pub fn new(table_info: TableInfo, pk_col_idx: usize) -> Self {
        Self {
            pk_col_idx,
            pk_index: BTreeMap::new(),
            secondary_index: HashMap::new(),
            table_info,
            splits: 0,
        }
    }

    /// Look up a row by primary key value (O(log N)).
    pub fn lookup_pk(&self, pk_value: &Value) -> Option<&Record> {
        self.pk_index.get(pk_value)
    }

    /// Range scan over primary key: O(log N + k) where k is result count.
    pub fn range_scan_pk(&self, lo: &Value, hi: &Value) -> Vec<Record> {
        self.pk_index
            .range::<Value, _>((Bound::Included(lo.clone()), Bound::Included(hi.clone())))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Insert a single row. Returns Err if PK already exists.
    pub fn insert(&mut self, row: Record) -> SqlResult<()> {
        let pk = row.get(self.pk_col_idx).cloned().ok_or_else(|| {
            SqlError::ExecutionError(format!(
                "Primary key column index {} out of range",
                self.pk_col_idx
            ))
        })?;
        if self.pk_index.contains_key(&pk) {
            return Err(SqlError::ExecutionError(format!(
                "Duplicate primary key value: {:?}",
                pk
            )));
        }
        self.pk_index.insert(pk, row);
        Ok(())
    }

    /// Delete a row by primary key. Returns true if found+deleted.
    pub fn delete_pk(&mut self, pk_value: &Value) -> bool {
        self.pk_index.remove(pk_value).is_some()
    }

    /// Full table scan (rows sorted by PK). O(N).
    pub fn full_scan(&self) -> Vec<Record> {
        self.pk_index.values().cloned().collect()
    }

    /// Update a row by primary key (preserving B+ Tree ordering).
    /// The replacement row MUST have the same PK value at `pk_col_idx`.
    pub fn update_pk(&mut self, pk_value: &Value, new_row: Record) -> SqlResult<bool> {
        if !self.pk_index.contains_key(pk_value) {
            return Ok(false);
        }
        // Verify the new row has the same PK (defense in depth).
        let new_pk = new_row.get(self.pk_col_idx).cloned().ok_or_else(|| {
            SqlError::ExecutionError(format!(
                "Primary key column index {} out of range",
                self.pk_col_idx
            ))
        })?;
        if &new_pk != pk_value {
            return Err(SqlError::ExecutionError(format!(
                "Cannot change primary key value during UPDATE (was {:?}, got {:?})",
                pk_value, new_pk
            )));
        }
        self.pk_index.insert(pk_value.clone(), new_row);
        Ok(true)
    }

    /// Count rows in the table (O(1)).
    pub fn len(&self) -> usize {
        self.pk_index.len()
    }

    /// Returns true if the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.pk_index.is_empty()
    }

    /// Get the table info.
    pub fn table_info(&self) -> &TableInfo {
        &self.table_info
    }

    /// Get the split count (for benchmarking).
    pub fn split_count(&self) -> u64 {
        self.splits
    }
}

use std::ops::Bound;

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    fn make_info() -> TableInfo {
        TableInfo {
            name: "orders".to_string(),
            columns: vec![
                crate::engine::ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: true,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                },
                crate::engine::ColumnDefinition {
                    name: "name".to_string(),
                    data_type: "TEXT".to_string(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                },
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),

            original_sql: String::new(),

            partition_info: None,
            compression: None,
        }
    }

    #[test]
    fn test_insert_and_lookup_pk() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        let row = vec![Value::Integer(1), Value::Text("alice".to_string())];
        ct.insert(row.clone()).unwrap();
        assert_eq!(ct.lookup_pk(&Value::Integer(1)), Some(&row));
    }

    #[test]
    fn test_range_scan_returns_sorted() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        for i in [5, 3, 1, 4, 2] {
            ct.insert(vec![Value::Integer(i), Value::Text(format!("row{}", i))])
                .unwrap();
        }
        let result = ct.range_scan_pk(&Value::Integer(1), &Value::Integer(10));
        let ids: Vec<i64> = result
            .iter()
            .map(|r| if let Value::Integer(n) = r[0] { n } else { -1 })
            .collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_pk_uniqueness_violation() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        ct.insert(vec![Value::Integer(1), Value::Text("a".to_string())])
            .unwrap();
        let result = ct.insert(vec![Value::Integer(1), Value::Text("b".to_string())]);
        assert!(result.is_err(), "expected PK violation");
    }

    #[test]
    fn test_update_preserves_order() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        ct.insert(vec![Value::Integer(1), Value::Text("a".to_string())])
            .unwrap();
        ct.update_pk(
            &Value::Integer(1),
            vec![Value::Integer(1), Value::Text("A".to_string())],
        )
        .unwrap();
        let rows = ct.full_scan();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][1], Value::Text("A".to_string()));
    }

    #[test]
    fn test_delete() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        ct.insert(vec![Value::Integer(1), Value::Text("a".to_string())])
            .unwrap();
        assert!(ct.delete_pk(&Value::Integer(1)));
        assert_eq!(ct.len(), 0);
    }

    #[test]
    fn test_clustered_table_len_and_is_empty() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        assert!(ct.is_empty());
        assert_eq!(ct.len(), 0);
    }

    #[test]
    fn test_clustered_table_full_scan() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        ct.insert(vec![Value::Integer(1), Value::Integer(10)])
            .unwrap();
        ct.insert(vec![Value::Integer(2), Value::Integer(20)])
            .unwrap();
        let all = ct.full_scan();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_clustered_table_update_not_found() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        let result = ct.update_pk(
            &Value::Integer(999),
            vec![Value::Integer(999), Value::Integer(99)],
        );
        assert!(result.is_ok());
        assert!(!result.unwrap()); // returns false when pk not found
    }

    #[test]
    fn test_clustered_table_range_scan() {
        let mut ct = ClusteredTable::new(make_info(), 0);
        ct.insert(vec![Value::Integer(10), Value::Integer(100)])
            .unwrap();
        ct.insert(vec![Value::Integer(20), Value::Integer(200)])
            .unwrap();
        ct.insert(vec![Value::Integer(30), Value::Integer(300)])
            .unwrap();
        let results = ct.range_scan_pk(&Value::Integer(15), &Value::Integer(25));
        assert_eq!(results.len(), 1);
    }
}

//! DDL execution methods: CREATE TABLE, CREATE SEQUENCE, ALTER SEQUENCE.
//!
//! Extracted from `src/execution_engine.rs` for V312-F-4 / ISSUE #4027
//! (C-ARCH-05 line count limit: 1500). The methods form a second
//! `impl<S: StorageEngine + 'static> ExecutionEngine<S>` block — Rust
//! allows methods of the same type to be split across multiple impl
//! blocks across multiple files. All call sites are unchanged.

use crate::execution_engine::ExecutionEngine;
use crate::{SqlError, SqlResult, Value};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    AlterSequenceStatement, CompressionAlgorithm, CreateSequenceStatement, CreateTableStatement,
    StorageEngineSpec,
};
use sqlrustgo_storage::clustered_table::ClusteredTable;
use sqlrustgo_storage::{engine::CheckConstraint, ColumnDefinition, StorageEngine, TableInfo};
use std::collections::HashMap;
use std::sync::Arc;

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// CREATE TABLE — including the V312-18 CREATE TABLE AS SELECT
    /// variant and the V311-01 F-23 ENGINE=InnoDB CLUSTERED path.
    pub(crate) fn execute_create_table(
        &self,
        create: &CreateTableStatement,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();

        // V312-18: CREATE TABLE AS SELECT
        if let Some(ref select_stmt) = create.select {
            // Execute the SELECT first to get the column names and data
            drop(storage); // release write lock to allow reads for SELECT
            let select_result = self.execute_select(select_stmt)?;
            storage = self.storage.write();

            // Determine column names: use explicit column names if provided, else
            // from the SELECT projection (alias or name).
            //
            // V313-14 / Issue #4042: previously the inferred path returned an
            // empty Vec and the table got placeholder names like
            // "column1", "column2", ... so `CREATE TABLE t AS SELECT 42 AS n`
            // produced a table with one column named "column1"; subsequent
            // `SELECT n FROM t` then failed to bind the column reference
            // and the legacy eval_identifier fallback emitted Text("n")
            // instead of Integer(42). With this fix the inferred names
            // mirror the SELECT-column aliases (or, when no alias is
            // given, the column expression name).
            let select_column_names: Vec<String> = if !create.columns.is_empty() {
                // Use explicit column names from CREATE TABLE (col1, col2, ...)
                create.columns.iter().map(|c| c.name.clone()).collect()
            } else {
                select_stmt
                    .columns
                    .iter()
                    .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                    .collect()
            };

            // V312-18: OR REPLACE - drop existing table first
            if create.or_replace && storage.has_table(&create.name) {
                storage.drop_table(&create.name)?;
            }

            // Check IF NOT EXISTS
            if storage.has_table(&create.name) {
                if create.if_not_exists {
                    return Ok(ExecutorResult::empty());
                } else {
                    return Err(SqlError::ExecutionError(format!(
                        "Table '{}' already exists",
                        create.name
                    )));
                }
            }

            // Infer column types from SELECT result rows
            let num_select_cols = if let Some(first_row) = select_result.rows.first() {
                first_row.len()
            } else {
                0
            };

            // Validate column name count vs select column count.
            //
            // V313-14 / Issue #4042: the original guard only rejected
            // `create.columns.len() < num_select_cols` (extra SELECT
            // columns silently dropped) but accepted the inverse case
            // `create.columns.len() > num_select_cols` (missing SELECT
            // columns silently filled with NULLs). The create_as.test
            // line 112-115 fixture expects the inverse case to error
            // with a binder message; this change enforces equality
            // for explicit-column CTAS.
            if !create.columns.is_empty() && create.columns.len() != num_select_cols {
                return Err(SqlError::ExecutionError(format!(
                    "Binder error: column count mismatch — CREATE TABLE declares {} \
                     column name(s) but SELECT produces {} column(s).",
                    create.columns.len(),
                    num_select_cols
                )));
            }

            // Build column definitions from SELECT result
            // Use explicit column definitions if provided, else infer from data
            let columns: Vec<ColumnDefinition> = if !create.columns.is_empty() {
                // Use explicit column definitions
                create
                    .columns
                    .iter()
                    .map(|c| ColumnDefinition {
                        name: c.name.clone(),
                        data_type: c.data_type.clone(),
                        nullable: c.nullable,
                        primary_key: c.primary_key,
                        char_max_length: c.char_max_length,
                        collation: c.collation.clone(),
                    })
                    .collect()
            } else {
                // Infer from SELECT result - all columns nullable since they come
                // from SELECT. V313-14 / Issue #4042: name comes from the
                // SELECT-column alias (or raw name) so subsequent
                // `SELECT alias FROM new_table` can bind correctly.
                (0..num_select_cols)
                    .map(|i| {
                        let inferred_type = select_result
                            .rows
                            .first()
                            .and_then(|row| row.get(i))
                            .map(|v| match v {
                                Value::Integer(_) => "INTEGER".to_string(),
                                Value::Float(_) => "FLOAT".to_string(),
                                Value::Text(_) => "TEXT".to_string(),
                                Value::Null => "TEXT".to_string(),
                                Value::Blob(_) => "BLOB".to_string(),
                                Value::Boolean(_) => "BOOLEAN".to_string(),
                                Value::Point(_, _) => "POINT".to_string(),
                                &Value::Json(_) => "JSON".to_string(),
                            })
                            .unwrap_or_else(|| "TEXT".to_string());
                        let inferred_name = select_column_names
                            .get(i)
                            .cloned()
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| format!("column{}", i + 1));
                        ColumnDefinition {
                            name: inferred_name,
                            data_type: inferred_type,
                            nullable: true,
                            primary_key: false,
                            char_max_length: None,
                            collation: None,
                        }
                    })
                    .collect()
            };

            // Handle WITH NO DATA - skip data insertion if false
            let with_data = create.with_data.unwrap_or(true);
            if !with_data {
                // Create empty table
                let info = TableInfo {
                    name: create.name.clone(),
                    columns: columns.clone(),
                    foreign_keys: vec![],
                    unique_constraints: vec![],
                    check_constraints: vec![],
                    partition_info: None,
                    compression: None,
                    collations: std::collections::HashMap::new(),
                };
                storage.create_table(&info)?;
                return Ok(ExecutorResult::new(vec![], 0));
            }

            // Insert data rows
            let info = TableInfo {
                name: create.name.clone(),
                columns: columns.clone(),
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
                compression: None,
                collations: std::collections::HashMap::new(),
            };
            storage.create_table(&info)?;

            // Insert rows from SELECT result
            let insert_count = if !select_result.rows.is_empty() {
                storage.insert(&create.name, select_result.rows.clone())?;
                select_result.rows.len()
            } else {
                0
            };

            return Ok(ExecutorResult::new(vec![], insert_count));
        }

        // Standard CREATE TABLE (without AS SELECT)
        let columns: Vec<ColumnDefinition> = create
            .columns
            .iter()
            .map(|c| ColumnDefinition {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                nullable: c.nullable,
                primary_key: c.primary_key,
                char_max_length: c.char_max_length,
                collation: c.collation.clone(),
            })
            .collect();
        // V312-26 / #4077: collect per-column collation (each column
        // carries its own `collation` field in HEAD's CreateTable AST)
        // into a name→collation map for the executor to consult during
        // set-op.
        let collations: std::collections::HashMap<String, String> = create
            .columns
            .iter()
            .filter_map(|c| c.collation.as_ref().map(|n| (c.name.clone(), n.to_lowercase())))
            .collect();
        let compression = create.compress.as_ref().map(|spec| match spec.algorithm {
            CompressionAlgorithm::Lz4 => "LZ4".to_string(),
            CompressionAlgorithm::Zstd => "ZSTD".to_string(),
            CompressionAlgorithm::Zlib => "ZLIB".to_string(),
        });

        // V312-18: OR REPLACE for regular CREATE TABLE
        if create.or_replace && storage.has_table(&create.name) {
            storage.drop_table(&create.name)?;
        }

        // Check IF NOT EXISTS for regular CREATE TABLE
        if storage.has_table(&create.name) {
            if create.if_not_exists {
                return Ok(ExecutorResult::empty());
            } else {
                return Err(SqlError::ExecutionError(format!(
                    "Table '{}' already exists",
                    create.name
                )));
            }
        }

        let check_constraints: Vec<CheckConstraint> = create
            .constraints
            .iter()
            .filter_map(|c| match c {
                sqlrustgo_parser::TableConstraint::Check { expression, name } => {
                    Some(CheckConstraint {
                        name: name.clone(),
                        expression: expression.clone(),
                    })
                }
                _ => None,
            })
            .collect();

        let info = TableInfo {
            name: create.name.clone(),
            columns: columns.clone(),
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: check_constraints.clone(),
            partition_info: None,
            compression,
            collations: collations.clone(),
        };

        // V311-01 F-23: route to ClusteredTable when storage_engine = Clustered.
        // - ENGINE=InnoDB CLUSTERED → B+ Tree storage with PK ordering.
        // - absent / ENGINE=InnoDB (no CLUSTERED) → existing Heap path.
        if matches!(create.storage_engine, Some(StorageEngineSpec::Clustered)) {
            // Find PK column index (must exist for ClusteredTable).
            let pk_col_idx = columns.iter().position(|c| c.primary_key).ok_or_else(|| {
                SqlError::ExecutionError(
                    "Clustered table requires PRIMARY KEY on a single column".to_string(),
                )
            })?;
            // Create ClusteredTable
            let ct = ClusteredTable::new(info.clone(), pk_col_idx);
            // Also register with Heap so other code paths (catalog, schema checks) find it
            storage.create_table(&TableInfo {
                name: info.name.clone(),
                columns,
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints,
                partition_info: None,
                compression: None,
            collations: HashMap::new(),
            })?;
            self.clustered_tables
                .write()
                .insert(create.name.clone(), Arc::new(parking_lot::RwLock::new(ct)));
            return Ok(ExecutorResult::empty());
        }
        storage.create_table(&info)?;
        Ok(ExecutorResult::empty())
    }

    /// CREATE SEQUENCE — register a new sequence with start/increment/min/max/cache options.
    pub(crate) fn execute_create_sequence(
        &self,
        seq_stmt: &CreateSequenceStatement,
    ) -> SqlResult<ExecutorResult> {
        use sqlrustgo_storage::engine::SequenceInfo;

        let mut storage = self.storage.write();

        // Check if sequence already exists
        if storage.has_sequence(&seq_stmt.name) {
            if seq_stmt.if_not_exists {
                return Ok(ExecutorResult::empty());
            }
            return Err(SqlError::ExecutionError(format!(
                "Sequence '{}' already exists",
                seq_stmt.name
            )));
        }

        // Parse sequence options from String to i64
        let start_with = seq_stmt
            .start_with
            .as_ref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1);
        let increment_by = seq_stmt
            .increment_by
            .as_ref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1);
        let minvalue = seq_stmt
            .minvalue
            .as_ref()
            .and_then(|s| {
                if s == "NO MINVALUE" {
                    Some(i64::MIN)
                } else {
                    s.parse::<i64>().ok()
                }
            })
            .unwrap_or(1);
        let maxvalue = seq_stmt
            .maxvalue
            .as_ref()
            .and_then(|s| {
                if s == "NO MAXVALUE" {
                    Some(i64::MAX)
                } else {
                    s.parse::<i64>().ok()
                }
            })
            .unwrap_or(i64::MAX);
        let cache = seq_stmt
            .cache
            .as_ref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(1);
        let cycle = seq_stmt.cycle.unwrap_or(false);

        // Construct SequenceInfo and persist
        let seq_info = SequenceInfo {
            name: seq_stmt.name.clone(),
            start_with,
            increment_by,
            minvalue,
            maxvalue,
            current_value: start_with - increment_by,
            cache,
            cycle,
        };
        storage.create_sequence(seq_info)?;
        Ok(ExecutorResult::empty())
    }

    /// ALTER SEQUENCE — RESTART [WITH value] reset.
    pub(crate) fn execute_alter_sequence(
        &self,
        seq_stmt: &AlterSequenceStatement,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();

        // Get existing sequence or error
        let mut seq_info = match storage.get_sequence(&seq_stmt.name) {
            Some(info) => info,
            None => {
                return Err(SqlError::ExecutionError(format!(
                    "Sequence '{}' not found",
                    seq_stmt.name
                )));
            }
        };

        // Handle RESTART [WITH value]
        if seq_stmt.restart_with.is_some() {
            let restart_val = seq_stmt
                .restart_with
                .as_ref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(seq_info.start_with);
            seq_info.current_value = restart_val - seq_info.increment_by;
        } else {
            // RESTART without WITH resets to start_value
            seq_info.current_value = seq_info.start_with - seq_info.increment_by;
        }

        // Update the sequence
        storage.create_sequence(seq_info)?;
        Ok(ExecutorResult::empty())
    }
}

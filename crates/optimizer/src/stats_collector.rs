use crate::stats::{ColumnStats, StatsResult, TableStats};
use crate::StatsCollector;
use sqlrustgo_storage::StorageEngine;
use sqlrustgo_types::Value;
use std::collections::HashSet;

pub fn collect_table_stats<S: StorageEngine + ?Sized>(
    storage: &S,
    table: &str,
) -> StatsResult<TableStats> {
    let records = storage
        .scan(table)
        .map_err(|e| crate::stats::StatsError::UpdateFailed(e.to_string()))?;
    let row_count = records.len() as u64;

    let mut column_stats = std::collections::HashMap::new();

    let table_info = storage
        .get_table_info(table)
        .map_err(|e| crate::stats::StatsError::UpdateFailed(e.to_string()))?;
    let num_columns = table_info.columns.len();

    for col_idx in 0..num_columns {
        let col_meta = &table_info.columns[col_idx];
        let col_name = &col_meta.name;

        let mut distinct = HashSet::new();
        let mut null_count = 0u64;
        let mut min_val: Option<Value> = None;
        let mut max_val: Option<Value> = None;
        let mut sum: f64 = 0.0;
        let mut numeric_count: u64 = 0;

        for record in &records {
            if let Some(val) = record.get(col_idx) {
                match val {
                    Value::Null => null_count += 1,
                    _ => {
                        distinct.insert(val.clone());
                        match val {
                            Value::Integer(i) => {
                                sum += *i as f64;
                                numeric_count += 1;
                                let i_val = *i;
                                match &min_val {
                                    Some(Value::Integer(min_i)) if i_val < *min_i => {
                                        min_val = Some(val.clone())
                                    }
                                    None => min_val = Some(val.clone()),
                                    _ => {}
                                }
                                match &max_val {
                                    Some(Value::Integer(max_i)) if i_val > *max_i => {
                                        max_val = Some(val.clone())
                                    }
                                    None => max_val = Some(val.clone()),
                                    _ => {}
                                }
                            }
                            Value::Float(f) => {
                                sum += *f;
                                numeric_count += 1;
                                let f_val = *f;
                                match &min_val {
                                    Some(Value::Float(min_f)) if f_val < *min_f => {
                                        min_val = Some(val.clone())
                                    }
                                    None => min_val = Some(val.clone()),
                                    _ => {}
                                }
                                match &max_val {
                                    Some(Value::Float(max_f)) if f_val > *max_f => {
                                        max_val = Some(val.clone())
                                    }
                                    None => max_val = Some(val.clone()),
                                    _ => {}
                                }
                            }
                            Value::Text(_) | Value::Blob(_) => {
                                if min_val.is_none() {
                                    min_val = Some(val.clone());
                                }
                                if max_val.is_none() {
                                    max_val = Some(val.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let avg_value = if numeric_count > 0 {
            Some(sum / numeric_count as f64)
        } else {
            None
        };

        let col_stats = ColumnStats::new(col_name.clone())
            .with_distinct_count(distinct.len() as u64)
            .with_null_count(null_count)
            .with_range(min_val, max_val)
            .with_average(avg_value.unwrap_or(0.0));

        column_stats.insert(col_name.clone(), col_stats);
    }

    let table_stats = TableStats::new(table)
        .with_row_count(row_count)
        .with_last_updated(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .with_column_stats(column_stats);

    Ok(table_stats)
}

pub fn estimate_selectivity(
    table_stats: &TableStats,
    predicate: &sqlrustgo_storage::predicate::Predicate,
) -> f64 {
    use sqlrustgo_storage::predicate::Predicate;

    match predicate {
        Predicate::Eq(_, _) => {
            let total = table_stats.row_count as f64;
            if total == 0.0 {
                return 1.0;
            }
            let avg_distinct: f64 = table_stats
                .column_stats
                .values()
                .map(|s| s.distinct_count as f64)
                .sum::<f64>()
                / table_stats.column_stats.len().max(1) as f64;
            1.0 / avg_distinct.max(1.0)
        }
        Predicate::Lt(_, _) | Predicate::Lte(_, _) | Predicate::Gt(_, _) | Predicate::Gte(_, _) => {
            0.25
        }
        Predicate::And(_, _) => 0.1,
        Predicate::Or(_, _) => 0.5,
        _ => 1.0,
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageStatsCollector;

impl StorageStatsCollector {
    pub fn new() -> Self {
        Self
    }
}

impl StatsCollector for StorageStatsCollector {
    fn collect_table_stats(
        &self,
        storage: &dyn StorageEngine,
        table: &str,
    ) -> StatsResult<TableStats> {
        collect_table_stats(storage, table)
    }

    fn collect_row_count(&self, storage: &dyn StorageEngine, table: &str) -> StatsResult<u64> {
        let records = storage
            .scan(table)
            .map_err(|e| crate::stats::StatsError::UpdateFailed(e.to_string()))?;
        Ok(records.len() as u64)
    }

    fn collect_column_stats(
        &self,
        storage: &dyn StorageEngine,
        table: &str,
        column: &str,
        column_index: usize,
    ) -> StatsResult<ColumnStats> {
        let records = storage
            .scan(table)
            .map_err(|e| crate::stats::StatsError::UpdateFailed(e.to_string()))?;

        let mut distinct = HashSet::new();
        let mut null_count = 0u64;
        let mut min_val: Option<Value> = None;
        let mut max_val: Option<Value> = None;
        let mut sum: f64 = 0.0;
        let mut numeric_count: u64 = 0;

        for record in &records {
            if let Some(val) = record.get(column_index) {
                match val {
                    Value::Null => null_count += 1,
                    _ => {
                        distinct.insert(val.clone());
                        sum += match val {
                            Value::Integer(i) => *i as f64,
                            Value::Float(f) => *f,
                            _ => 0.0,
                        };
                        if matches!(val, Value::Integer(_) | Value::Float(_)) {
                            numeric_count += 1;
                        }
                        if min_val.is_none() {
                            min_val = Some(val.clone());
                        }
                        if max_val.is_none() {
                            max_val = Some(val.clone());
                        }
                    }
                }
            }
        }

        let avg_value = if numeric_count > 0 {
            Some(sum / numeric_count as f64)
        } else {
            None
        };

        Ok(ColumnStats::new(column)
            .with_distinct_count(distinct.len() as u64)
            .with_null_count(null_count)
            .with_range(min_val, max_val)
            .with_average(avg_value.unwrap_or(0.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::engine::{
        ColumnDefinition, Record, SqlResult, StorageEngine, TableInfo,
    };

    /// Helper: create a MemoryStorage pre-populated with test data
    fn make_storage_with_data(
        table: &str,
        records: Vec<Record>,
        columns: Vec<ColumnDefinition>,
    ) -> sqlrustgo_storage::MemoryStorage {
        let mut storage = sqlrustgo_storage::MemoryStorage::new();
        let info = TableInfo {
            name: table.to_string(),
            columns,
            table_foreign_keys: None,
            collations: std::collections::HashMap::new(),
        };
        storage.create_table(&info).unwrap();
        storage.insert(table, records).unwrap();
        storage
    }

    #[test]
    fn test_collect_table_stats() {
        let records = vec![
            vec![Value::Integer(1), Value::Text("Alice".to_string())],
            vec![Value::Integer(2), Value::Text("Bob".to_string())],
            vec![Value::Integer(3), Value::Text("Carol".to_string())],
        ];
        let columns = vec![
            ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                is_unique: true,
                is_primary_key: false,
                auto_increment: false,
                references: None,
                compression: None,
                collation: None,

                default_value: None,            },
            ColumnDefinition {
                name: "name".to_string(),
                data_type: "TEXT".to_string(),
                nullable: false,
                is_unique: false,
                is_primary_key: false,
                auto_increment: false,
                references: None,
                compression: None,
                collation: None,

                default_value: None,            },
        ];
        let storage = make_storage_with_data("users", records, columns);

        let result = collect_table_stats(&storage, "users");
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.row_count, 3);
        assert!(stats.column_stats.contains_key("id"));
        assert!(stats.column_stats.contains_key("name"));
    }

    #[test]
    fn test_estimate_selectivity() {
        let mut col_stats = HashMap::new();
        col_stats.insert(
            "id".to_string(),
            ColumnStats::new("id").with_distinct_count(100),
        );

        let table_stats = TableStats::new("test")
            .with_row_count(1000)
            .with_column_stats(col_stats);

        let selectivity = estimate_selectivity(
            &table_stats,
            &sqlrustgo_storage::predicate::Predicate::Eq(
                Box::new(sqlrustgo_storage::predicate::Expr::Column("id".to_string())),
                Box::new(sqlrustgo_storage::predicate::Expr::Value(Value::Integer(1))),
            ),
        );

        assert!(selectivity > 0.0 && selectivity < 1.0);
    }
}

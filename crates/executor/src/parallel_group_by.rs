//! Parallel GROUP BY executor
//!
//! v3.10.0 Issue #3703 Phase 3 follow-up for #3736.
//!
//! Hash partitions rows by group key, computes partial aggregates
//! per partition in parallel, then merges partial HashMaps on the
//! main thread.
//!
//! Spec: openspec/changes/issue-3703-intra-query-parallel-executor/
//!       specs/intra-query-parallel-group-by/spec.md
//!
//! Correctness invariants:
//! - SUM, COUNT, MIN, MAX are associative (bit-exact merge for integers)
//! - AVG is computed from merged (sum, count) — preserves float to within
//!   1e-9 across partitions

use sqlrustgo_parser::{AggregateCall, AggregateFunction, Expression};
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tracing::instrument;

#[derive(Debug, Clone)]
pub enum PartialSlot {
    Count(i64),
    CountNonNull(i64),
    Sum(SumState),
    Avg(AvgState),
    MinMax(MinMaxState),
}

impl Default for PartialSlot {
    fn default() -> Self {
        PartialSlot::Count(0)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SumState {
    pub int_sum: i64,
    pub float_sum: f64,
    pub any_float: bool,
    pub has_value: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AvgState {
    pub sum: SumState,
    pub count: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MinMaxState {
    pub min: Option<Value>,
    pub max: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct PartialAggregate {
    pub slots: Vec<PartialSlot>,
}

impl PartialAggregate {
    pub fn new(n_slots: usize) -> Self {
        Self {
            slots: (0..n_slots).map(|_| PartialSlot::default()).collect(),
        }
    }

    /// Update from a single row's pre-evaluated aggregate values
    pub fn update(&mut self, aggregates: &[AggregateCall], values: &[Value]) {
        debug_assert_eq!(self.slots.len(), aggregates.len());
        for (i, agg) in aggregates.iter().enumerate() {
            match agg.func {
                AggregateFunction::Count => self.update_count(i, agg, &values[i]),
                AggregateFunction::Sum => self.update_sum(i, &values[i]),
                AggregateFunction::Avg => self.update_avg(i, &values[i]),
                AggregateFunction::Min => self.update_min(i, &values[i]),
                AggregateFunction::Max => self.update_max(i, &values[i]),
            }
        }
    }

    fn update_count(&mut self, i: usize, agg: &AggregateCall, v: &Value) {
        if agg.args.is_empty() {
            // COUNT(*)
            if let PartialSlot::Count(c) = &mut self.slots[i] {
                *c += 1;
            }
        } else if !matches!(v, Value::Null) {
            if let PartialSlot::Count(c) = &mut self.slots[i] {
                *c += 1;
            } else {
                self.slots[i] = PartialSlot::CountNonNull(1);
            }
        }
    }

    fn update_sum(&mut self, i: usize, v: &Value) {
        if matches!(v, Value::Null) {
            return;
        }
        // Promote Count/CountNonNull -> Sum
        match self.slots[i] {
            PartialSlot::Count(_) | PartialSlot::CountNonNull(_) => {
                self.slots[i] = PartialSlot::Sum(SumState::default());
            }
            _ => {}
        }
        if let PartialSlot::Sum(s) = &mut self.slots[i] {
            s.has_value = true;
            match v {
                Value::Integer(n) => {
                    if s.any_float {
                        s.float_sum += *n as f64;
                    } else {
                        s.int_sum = s.int_sum.wrapping_add(*n);
                    }
                }
                Value::Float(f) => {
                    if !s.any_float {
                        s.float_sum = s.int_sum as f64;
                        s.any_float = true;
                    }
                    s.float_sum += f;
                }
                _ => {}
            }
        }
    }

    fn update_avg(&mut self, i: usize, v: &Value) {
        if !matches!(self.slots[i], PartialSlot::Avg(_)) {
            self.slots[i] = PartialSlot::Avg(AvgState::default());
        }
        if let PartialSlot::Avg(a) = &mut self.slots[i] {
            if matches!(v, Value::Null) {
                return;
            }
            a.count += 1;
            a.sum.has_value = true;
            match v {
                Value::Integer(n) => {
                    if a.sum.any_float {
                        a.sum.float_sum += *n as f64;
                    } else {
                        a.sum.int_sum = a.sum.int_sum.wrapping_add(*n);
                    }
                }
                Value::Float(f) => {
                    if !a.sum.any_float {
                        a.sum.float_sum = a.sum.int_sum as f64;
                        a.sum.any_float = true;
                    }
                    a.sum.float_sum += f;
                }
                _ => {}
            }
        }
    }

    fn update_min(&mut self, i: usize, v: &Value) {
        if matches!(v, Value::Null) {
            return;
        }
        let take = match &self.slots[i] {
            PartialSlot::MinMax(m) => match &m.min {
                None => true,
                Some(existing) => v < existing,
            },
            _ => true,
        };
        if !matches!(self.slots[i], PartialSlot::MinMax(_)) {
            self.slots[i] = PartialSlot::MinMax(MinMaxState::default());
        }
        if take {
            if let PartialSlot::MinMax(m) = &mut self.slots[i] {
                m.min = Some(v.clone());
            }
        }
    }

    fn update_max(&mut self, i: usize, v: &Value) {
        if matches!(v, Value::Null) {
            return;
        }
        let take = match &self.slots[i] {
            PartialSlot::MinMax(m) => match &m.max {
                None => true,
                Some(existing) => v > existing,
            },
            _ => true,
        };
        if !matches!(self.slots[i], PartialSlot::MinMax(_)) {
            self.slots[i] = PartialSlot::MinMax(MinMaxState::default());
        }
        if take {
            if let PartialSlot::MinMax(m) = &mut self.slots[i] {
                m.max = Some(v.clone());
            }
        }
    }

    /// Merge another partial aggregate into this one
    pub fn merge(&mut self, other: &PartialAggregate) {
        debug_assert_eq!(self.slots.len(), other.slots.len());
        for (i, other_slot) in other.slots.iter().enumerate() {
            // Handle mismatched slot types via promotion
            let result_clone = other_slot.clone();
            match (&mut self.slots[i], other_slot) {
                (PartialSlot::Count(_) | PartialSlot::CountNonNull(_), PartialSlot::Sum(_)) => {
                    // Fresh Count slot being merged with Sum slot
                    // The merging group has no Sum contribution yet, but
                    // we need to keep the Sum data. Promote self.
                    self.slots[i] = result_clone;
                }
                (PartialSlot::Count(_) | PartialSlot::CountNonNull(_), PartialSlot::Avg(_)) => {
                    self.slots[i] = result_clone;
                }
                (PartialSlot::Count(_) | PartialSlot::CountNonNull(_), PartialSlot::MinMax(_)) => {
                    self.slots[i] = result_clone;
                }
                (PartialSlot::Sum(a), PartialSlot::Sum(b)) => Self::merge_sum(a, b),
                (PartialSlot::Avg(a), PartialSlot::Avg(b)) => Self::merge_avg(a, b),
                (PartialSlot::MinMax(a), PartialSlot::MinMax(b)) => {
                    if let (Some(x), Some(y)) = (&a.min, &b.min) {
                        if y < x {
                            a.min = Some(y.clone());
                        }
                    } else if a.min.is_none() {
                        a.min = b.min.clone();
                    }
                    if let (Some(x), Some(y)) = (&a.max, &b.max) {
                        if y > x {
                            a.max = Some(y.clone());
                        }
                    } else if a.max.is_none() {
                        a.max = b.max.clone();
                    }
                }
                (PartialSlot::Count(a), PartialSlot::Count(b)) => *a += *b,
                (PartialSlot::CountNonNull(a), PartialSlot::CountNonNull(b)) => *a += *b,
                (PartialSlot::Count(a), PartialSlot::CountNonNull(b)) => *a += *b,
                (PartialSlot::CountNonNull(a), PartialSlot::Count(b)) => *a += *b,
                _ => {}
            }
        }
    }

    fn merge_sum(a: &mut SumState, b: &SumState) {
        if !b.has_value {
            return;
        }
        a.has_value = true;
        if b.any_float {
            if !a.any_float {
                a.float_sum = a.int_sum as f64;
                a.any_float = true;
            }
            a.float_sum += b.float_sum;
        } else {
            a.int_sum = a.int_sum.wrapping_add(b.int_sum);
        }
    }

    fn merge_avg(a: &mut AvgState, b: &AvgState) {
        a.count += b.count;
        Self::merge_sum(&mut a.sum, &b.sum);
    }

    /// Convert to a final result row
    pub fn to_values(&self, aggregates: &[AggregateCall]) -> Vec<Value> {
        let mut out = Vec::with_capacity(aggregates.len());
        for (i, agg) in aggregates.iter().enumerate() {
            let value = match (&self.slots[i], agg.func.clone()) {
                (PartialSlot::Count(c), _) => Value::Integer(*c),
                (PartialSlot::CountNonNull(c), _) => Value::Integer(*c),
                (PartialSlot::Sum(s), _) => {
                    if !s.has_value {
                        Value::Null
                    } else if s.any_float {
                        Value::Float(s.float_sum)
                    } else {
                        Value::Integer(s.int_sum)
                    }
                }
                (PartialSlot::Avg(a), AggregateFunction::Avg) => {
                    if a.count == 0 {
                        Value::Null
                    } else if a.sum.any_float {
                        Value::Float(a.sum.float_sum / a.count as f64)
                    } else {
                        Value::Float(a.sum.int_sum as f64 / a.count as f64)
                    }
                }
                (PartialSlot::MinMax(m), AggregateFunction::Min) => {
                    m.min.clone().unwrap_or(Value::Null)
                }
                (PartialSlot::MinMax(m), AggregateFunction::Max) => {
                    m.max.clone().unwrap_or(Value::Null)
                }
                _ => Value::Null,
            };
            out.push(value);
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupKey(pub String);

/// Evaluate a simple expression against a row.
/// Supports Identifier (column reference) and Literal (constant).
/// Other expressions return Value::Null.
pub fn evaluate_simple_expr(expr: &Expression, row: &[Value], table_info: &TableInfo) -> Value {
    match expr {
        Expression::Identifier(name) => {
            let target = name.to_lowercase();
            for (i, col) in table_info.columns.iter().enumerate() {
                if col.name.to_lowercase() == target {
                    return row.get(i).cloned().unwrap_or(Value::Null);
                }
            }
            Value::Null
        }
        Expression::Literal(s) => match s.parse::<i64>() {
            Ok(n) => Value::Integer(n),
            Err(_) => match s.parse::<f64>() {
                Ok(f) => Value::Float(f),
                Err(_) => Value::Text(s.clone()),
            },
        },
        _ => Value::Null,
    }
}

/// Compute group keys for a batch of rows
pub fn compute_group_keys(
    rows: &[Vec<Value>],
    group_exprs: &[Expression],
    table_info: &TableInfo,
) -> Vec<GroupKey> {
    rows.iter()
        .map(|row| {
            let s = group_exprs
                .iter()
                .map(|expr| {
                    let v = evaluate_simple_expr(expr, row, table_info);
                    value_to_key_string(&v)
                })
                .collect::<Vec<_>>()
                .join("\x00");
            GroupKey(s)
        })
        .collect()
}

fn value_to_key_string(v: &Value) -> String {
    match v {
        Value::Null => "NULL".to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format!("{:?}", f),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => format!("{:?}", b),
        Value::Point(x, y) => format!("POINT({:?}, {:?})", x, y),
        Value::Boolean(b) => b.to_string(),
        Value::Json(v) => format!("{:?}", v),
    }
}

/// Parallel GROUP BY executor
pub struct ParallelGroupBy {
    pub degree: usize,
}

impl ParallelGroupBy {
    pub fn new(degree: usize) -> Self {
        Self {
            degree: degree.max(1),
        }
    }

    /// Hash-partition rows into N buckets keyed by GroupKey hash
    pub fn partition(
        &self,
        rows: Vec<Vec<Value>>,
        group_keys: Vec<GroupKey>,
    ) -> Vec<Vec<(GroupKey, Vec<Value>)>> {
        let n = self.degree;
        let mut partitions: Vec<Vec<(GroupKey, Vec<Value>)>> = (0..n).map(|_| Vec::new()).collect();
        for (row, key) in rows.into_iter().zip(group_keys) {
            let mut hasher = DefaultHasher::new();
            key.0.hash(&mut hasher);
            let bucket = (hasher.finish() as usize) % n;
            partitions[bucket].push((key, row));
        }
        partitions
    }

    /// Compute partial aggregates for one partition
    pub fn partial_aggregate_partition(
        &self,
        partition: Vec<(GroupKey, Vec<Value>)>,
        aggregate_calls: &[AggregateCall],
        table_info: &TableInfo,
    ) -> HashMap<GroupKey, PartialAggregate> {
        let mut by_key: HashMap<GroupKey, PartialAggregate> = HashMap::new();
        for (key, row) in partition {
            let values: Vec<Value> = aggregate_calls
                .iter()
                .map(|agg| {
                    if agg.args.is_empty() {
                        Value::Null
                    } else if let Some(arg) = agg.args.first() {
                        evaluate_simple_expr(arg, &row, table_info)
                    } else {
                        Value::Null
                    }
                })
                .collect();
            let entry = by_key
                .entry(key)
                .or_insert_with(|| PartialAggregate::new(aggregate_calls.len()));
            entry.update(aggregate_calls, &values);
        }
        by_key
    }

    /// Merge multiple partial aggregate HashMaps into one
    pub fn merge_partitions(
        &self,
        partitions: Vec<HashMap<GroupKey, PartialAggregate>>,
    ) -> HashMap<GroupKey, PartialAggregate> {
        let mut merged: HashMap<GroupKey, PartialAggregate> = HashMap::new();
        for partition in partitions {
            for (key, partial) in partition {
                let entry = merged
                    .entry(key)
                    .or_insert_with(|| PartialAggregate::new(partial.slots.len()));
                entry.merge(&partial);
            }
        }
        merged
    }

    /// Top-level: parallel GROUP BY execution
    #[instrument(skip_all, fields(rows = rows.len(), degree = self.degree))]
    pub fn execute(
        &self,
        rows: Vec<Vec<Value>>,
        group_keys: Vec<GroupKey>,
        aggregate_calls: &[AggregateCall],
        table_info: &TableInfo,
    ) -> Vec<(GroupKey, Vec<Value>)> {
        let partitions = self.partition(rows, group_keys);

        let partials: Vec<HashMap<GroupKey, PartialAggregate>> = if self.degree > 1 {
            #[cfg(feature = "parallel-executor")]
            {
                use rayon::prelude::*;
                partitions
                    .into_par_iter()
                    .map(|p| self.partial_aggregate_partition(p, aggregate_calls, table_info))
                    .collect()
            }
            #[cfg(not(feature = "parallel-executor"))]
            {
                partitions
                    .into_iter()
                    .map(|p| self.partial_aggregate_partition(p, aggregate_calls, table_info))
                    .collect()
            }
        } else {
            partitions
                .into_iter()
                .map(|p| self.partial_aggregate_partition(p, aggregate_calls, table_info))
                .collect()
        };

        let merged = self.merge_partitions(partials);

        merged
            .into_iter()
            .map(|(k, p)| (k, p.to_values(aggregate_calls)))
            .collect()
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use sqlrustgo_storage::ColumnDefinition;

    fn empty_table_info() -> TableInfo {
        TableInfo {
            name: "t".to_string(),
            columns: vec![],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        }
    }

    fn table_with_id_col() -> TableInfo {
        TableInfo {
            name: "t".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INT".to_string(),
                ..Default::default()
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
        }
    }

    #[test]
    fn test_parallel_group_by_new_clamps_degree() {
        assert_eq!(ParallelGroupBy::new(0).degree, 1);
        assert_eq!(ParallelGroupBy::new(8).degree, 8);
    }

    #[test]
    fn test_evaluate_simple_expr_identifier() {
        let table = table_with_id_col();
        let row = vec![Value::Integer(42)];
        assert_eq!(
            evaluate_simple_expr(&Expression::Identifier("id".into()), &row, &table),
            Value::Integer(42)
        );
        assert_eq!(
            evaluate_simple_expr(&Expression::Identifier("ID".into()), &row, &table),
            Value::Integer(42)
        );
        assert_eq!(
            evaluate_simple_expr(&Expression::Identifier("missing".into()), &row, &table),
            Value::Null
        );
    }

    #[test]
    fn test_evaluate_simple_expr_literal() {
        let table = empty_table_info();
        let row: Vec<Value> = vec![];
        assert_eq!(
            evaluate_simple_expr(&Expression::Literal("42".into()), &row, &table),
            Value::Integer(42)
        );
        assert_eq!(
            evaluate_simple_expr(&Expression::Literal("3.14".into()), &row, &table),
            Value::Float(3.14)
        );
        assert_eq!(
            evaluate_simple_expr(&Expression::Literal("hello".into()), &row, &table),
            Value::Text("hello".into())
        );
    }

    #[test]
    fn test_compute_group_keys_basic() {
        let table = table_with_id_col();
        let rows = vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
            vec![Value::Integer(1)],
        ];
        let keys = compute_group_keys(&rows, &[Expression::Identifier("id".into())], &table);
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].0, "1");
        assert_eq!(keys[1].0, "2");
        assert_eq!(keys[2].0, "1");
    }

    #[test]
    fn test_value_to_key_string_point() {
        assert_eq!(value_to_key_string(&Value::Point(1.0, 2.0)), "POINT(1.0, 2.0)");
    }
}

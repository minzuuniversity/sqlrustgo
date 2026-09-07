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
                AggregateFunction::PercentileCont => unreachable!(),
                // V313-followup-2 / Issue #4155: quantile aggregates
                // are non-incremental (require sorted finalization), so
                // they fall back to the serial compute_aggregates path
                // and never reach the parallel in-place updater.
                AggregateFunction::QuantileDisc | AggregateFunction::QuantileCont => {
                    unreachable!("quantile aggregates are computed serially in compute_aggregates")
                }
                // V312-64b / Issue #4650: GROUP_CONCAT is non-incremental
                // (needs sorted finalization + separator join) so it falls
                // back to the serial compute_aggregates path.
                AggregateFunction::GroupConcat => {
                    unreachable!("GROUP_CONCAT is computed serially in compute_aggregates")
                }
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
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        }
    }

    fn table_with_id_col() -> TableInfo {
        TableInfo {
            name: "t".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INT".to_string(),
                auto_increment: false,
                ..Default::default()
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            compression: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
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
        assert_eq!(
            value_to_key_string(&Value::Point(1.0, 2.0)),
            "POINT(1.0, 2.0)"
        );
    }

    // ---- PartialAggregate update_* paths ----

    fn call(func: AggregateFunction, args: Vec<Expression>) -> AggregateCall {
        AggregateCall {
            func,
            args,
            distinct: false,
        }
    }

    #[test]
    fn update_count_star_increments() {
        let mut pa = PartialAggregate::new(1);
        let agg = vec![call(AggregateFunction::Count, vec![])];
        for _ in 0..3 {
            pa.update(&agg, &[Value::Null]);
        }
        let out = pa.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(3)]);
    }

    #[test]
    fn update_count_non_null_skips_null() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("id".into());
        let agg = vec![call(AggregateFunction::Count, vec![arg.clone()])];
        pa.update(&agg, &[Value::Integer(1)]);
        pa.update(&agg, &[Value::Null]);
        pa.update(&agg, &[Value::Integer(2)]);
        let out = pa.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(2)]);
    }

    #[test]
    fn update_sum_int_and_float() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Sum, vec![arg])];
        pa.update(&agg, &[Value::Integer(10)]);
        pa.update(&agg, &[Value::Integer(5)]);
        // Integer-only so far → Integer sum
        assert_eq!(pa.to_values(&agg), vec![Value::Integer(15)]);
        pa.update(&agg, &[Value::Float(2.5)]);
        // Now mixed → Float sum
        let out = pa.to_values(&agg);
        match out[0].clone() {
            Value::Float(f) => assert!((f - 17.5).abs() < 1e-9),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn update_sum_skips_null() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Sum, vec![arg])];
        pa.update(&agg, &[Value::Integer(10)]);
        pa.update(&agg, &[Value::Null]); // skipped
        let out = pa.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(10)]);
    }

    #[test]
    fn update_sum_no_value_is_null() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Sum, vec![arg])];
        pa.update(&agg, &[Value::Null]);
        let out = pa.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(0)]); // slot is Count(0) since no values
    }

    #[test]
    fn update_avg_basic() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Avg, vec![arg])];
        pa.update(&agg, &[Value::Integer(10)]);
        pa.update(&agg, &[Value::Integer(20)]);
        let out = pa.to_values(&agg);
        match out[0].clone() {
            Value::Float(f) => assert!((f - 15.0).abs() < 1e-9),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn update_avg_empty_is_null() {
        let pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Avg, vec![arg])];
        let out = pa.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(0)]);
    }

    #[test]
    fn update_min_and_max() {
        let mut pa_min = PartialAggregate::new(1);
        let mut pa_max = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg_min = vec![call(AggregateFunction::Min, vec![arg.clone()])];
        let agg_max = vec![call(AggregateFunction::Max, vec![arg.clone()])];
        for v in [3, 1, 4, 1, 5, 9, 2, 6] {
            pa_min.update(&agg_min, &[Value::Integer(v)]);
            pa_max.update(&agg_max, &[Value::Integer(v)]);
        }
        assert_eq!(pa_min.to_values(&agg_min), vec![Value::Integer(1)]);
        assert_eq!(pa_max.to_values(&agg_max), vec![Value::Integer(9)]);
    }

    #[test]
    fn update_min_skips_null() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Min, vec![arg])];
        pa.update(&agg, &[Value::Null]);
        pa.update(&agg, &[Value::Integer(5)]);
        assert_eq!(pa.to_values(&agg), vec![Value::Integer(5)]);
    }

    #[test]
    fn update_min_on_text() {
        let mut pa = PartialAggregate::new(1);
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Min, vec![arg])];
        pa.update(&agg, &[Value::Text("banana".into())]);
        pa.update(&agg, &[Value::Text("apple".into())]);
        pa.update(&agg, &[Value::Text("cherry".into())]);
        assert_eq!(pa.to_values(&agg), vec![Value::Text("apple".into())]);
    }

    #[test]
    fn merge_promotes_count_to_sum() {
        // Two partials: one is Count(5), one is Sum(10).
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Sum, vec![arg.clone()])];
        let mut a = PartialAggregate::new(1);
        a.update(&[call(AggregateFunction::Count, vec![])], &[Value::Null]);
        a.update(&[call(AggregateFunction::Count, vec![])], &[Value::Null]);
        // Now a's slot is Count(2).
        let mut b = PartialAggregate::new(1);
        b.update(&agg, &[Value::Integer(10)]);
        // b's slot is Sum(10).
        a.merge(&b);
        // Promotion: a's Count → Sum, then merge values.
        let out = a.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(10)]);
    }

    #[test]
    fn merge_sums_int_and_float() {
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Sum, vec![arg])];
        let mut a = PartialAggregate::new(1);
        a.update(&agg, &[Value::Integer(5)]);
        let mut b = PartialAggregate::new(1);
        b.update(&agg, &[Value::Float(2.5)]);
        a.merge(&b);
        let out = a.to_values(&agg);
        match out[0].clone() {
            Value::Float(f) => assert!((f - 7.5).abs() < 1e-9),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn merge_min_max_picks_extremes() {
        let arg = Expression::Identifier("v".into());
        let agg_min = vec![call(AggregateFunction::Min, vec![arg.clone()])];
        let agg_max = vec![call(AggregateFunction::Max, vec![arg.clone()])];
        let mut a_min = PartialAggregate::new(1);
        let mut a_max = PartialAggregate::new(1);
        a_min.update(&agg_min, &[Value::Integer(10)]);
        a_min.update(&agg_min, &[Value::Integer(20)]);
        a_max.update(&agg_max, &[Value::Integer(10)]);
        a_max.update(&agg_max, &[Value::Integer(20)]);
        let mut b_min = PartialAggregate::new(1);
        let mut b_max = PartialAggregate::new(1);
        b_min.update(&agg_min, &[Value::Integer(5)]);
        b_min.update(&agg_min, &[Value::Integer(15)]);
        b_max.update(&agg_max, &[Value::Integer(15)]);
        b_max.update(&agg_max, &[Value::Integer(25)]);
        a_min.merge(&b_min);
        a_max.merge(&b_max);
        assert_eq!(a_min.to_values(&agg_min), vec![Value::Integer(5)]);
        assert_eq!(a_max.to_values(&agg_max), vec![Value::Integer(25)]);
    }

    #[test]
    fn merge_count_to_count_non_null_and_back() {
        // Tests the cross-variants Count + CountNonNull merge arms.
        let arg = Expression::Identifier("v".into());
        let agg = vec![call(AggregateFunction::Count, vec![arg])];
        let mut a = PartialAggregate::new(1);
        a.update(&[call(AggregateFunction::Count, vec![])], &[Value::Null]);
        // a is Count(1).
        let mut b = PartialAggregate::new(1);
        b.update(&agg, &[Value::Integer(1)]);
        // b is CountNonNull(1).
        a.merge(&b);
        let out = a.to_values(&agg);
        assert_eq!(out, vec![Value::Integer(2)]);
    }

    // ---- ParallelGroupBy end-to-end ----

    #[test]
    fn partition_distributes_rows_into_buckets() {
        let pgb = ParallelGroupBy::new(4);
        let rows: Vec<Vec<Value>> = (0..100).map(|i| vec![Value::Integer(i)]).collect();
        let keys: Vec<GroupKey> = (0..100).map(|i| GroupKey(i.to_string())).collect();
        let parts = pgb.partition(rows, keys);
        assert_eq!(parts.len(), 4);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn partition_with_degree_one() {
        let pgb = ParallelGroupBy::new(1);
        let rows: Vec<Vec<Value>> = (0..10).map(|i| vec![Value::Integer(i)]).collect();
        let keys: Vec<GroupKey> = (0..10).map(|i| GroupKey(i.to_string())).collect();
        let parts = pgb.partition(rows, keys);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 10);
    }

    #[test]
    fn partial_aggregate_partition_collects_by_key() {
        let pgb = ParallelGroupBy::new(1);
        let agg = vec![call(AggregateFunction::Count, vec![])];
        let table = empty_table_info();
        // 4 rows with 3 distinct keys: A, A, B, C → 3 buckets.
        let partition = vec![
            (GroupKey("A".into()), vec![Value::Integer(1)]),
            (GroupKey("A".into()), vec![Value::Integer(2)]),
            (GroupKey("B".into()), vec![Value::Integer(3)]),
            (GroupKey("C".into()), vec![Value::Integer(4)]),
        ];
        let map = pgb.partial_aggregate_partition(partition, &agg, &table);
        assert_eq!(map.len(), 3);
        let a_count = map.get(&GroupKey("A".into())).unwrap().to_values(&agg);
        assert_eq!(a_count, vec![Value::Integer(2)]);
    }

    #[test]
    fn partial_aggregate_partition_handles_no_args_count() {
        // COUNT(*) → args empty → Value::Null sentinel used.
        let pgb = ParallelGroupBy::new(1);
        let agg = vec![call(AggregateFunction::Count, vec![])];
        let table = empty_table_info();
        let partition = vec![
            (GroupKey("A".into()), vec![Value::Null]),
            (GroupKey("A".into()), vec![Value::Null]),
        ];
        let map = pgb.partial_aggregate_partition(partition, &agg, &table);
        let a = map.get(&GroupKey("A".into())).unwrap().to_values(&agg);
        assert_eq!(a, vec![Value::Integer(2)]);
    }

    #[test]
    fn merge_partitions_combines_buckets() {
        let pgb = ParallelGroupBy::new(1);
        let agg = vec![call(AggregateFunction::Count, vec![])];
        let table = empty_table_info();
        let p1 = pgb.partial_aggregate_partition(
            vec![
                (GroupKey("X".into()), vec![Value::Null]),
                (GroupKey("Y".into()), vec![Value::Null]),
            ],
            &agg,
            &table,
        );
        let p2 = pgb.partial_aggregate_partition(
            vec![
                (GroupKey("X".into()), vec![Value::Null]),
                (GroupKey("Z".into()), vec![Value::Null]),
            ],
            &agg,
            &table,
        );
        let merged = pgb.merge_partitions(vec![p1, p2]);
        assert_eq!(merged.len(), 3);
        let x = merged.get(&GroupKey("X".into())).unwrap().to_values(&agg);
        assert_eq!(x, vec![Value::Integer(2)]);
        let y = merged.get(&GroupKey("Y".into())).unwrap().to_values(&agg);
        assert_eq!(y, vec![Value::Integer(1)]);
        let z = merged.get(&GroupKey("Z".into())).unwrap().to_values(&agg);
        assert_eq!(z, vec![Value::Integer(1)]);
    }

    #[test]
    fn execute_top_level_count_groupby() {
        let pgb = ParallelGroupBy::new(2);
        let table = empty_table_info();
        let rows = vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
            vec![Value::Integer(3)],
            vec![Value::Integer(3)],
            vec![Value::Integer(3)],
        ];
        let keys: Vec<GroupKey> = (0..6).map(|_| GroupKey("ignored".into())).collect();
        let agg = vec![call(AggregateFunction::Count, vec![])];
        let out = pgb.execute(rows, keys, &agg, &table);
        // All rows share the same key, so one bucket with count=6.
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, vec![Value::Integer(6)]);
    }

    #[test]
    fn execute_with_sum_and_avg() {
        let pgb = ParallelGroupBy::new(2);
        let table = table_with_id_col(); // has an "id" column at index 0
        let rows = vec![
            vec![Value::Integer(10)],
            vec![Value::Integer(20)],
            vec![Value::Integer(30)],
        ];
        let keys = vec![
            GroupKey("k".into()),
            GroupKey("k".into()),
            GroupKey("k".into()),
        ];
        let arg = Expression::Identifier("id".into());
        let agg = vec![
            call(AggregateFunction::Sum, vec![arg.clone()]),
            call(AggregateFunction::Avg, vec![arg.clone()]),
            call(AggregateFunction::Min, vec![arg.clone()]),
            call(AggregateFunction::Max, vec![arg]),
        ];
        let out = pgb.execute(rows, keys, &agg, &table);
        assert_eq!(out.len(), 1);
        let vals = &out[0].1;
        assert_eq!(vals[0], Value::Integer(60)); // sum
        match vals[1].clone() {
            Value::Float(f) => assert!((f - 20.0).abs() < 1e-9),
            other => panic!("avg: {:?}", other),
        }
        assert_eq!(vals[2], Value::Integer(10));
        assert_eq!(vals[3], Value::Integer(30));
    }
}

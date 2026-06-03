//! Engine SELECT execution — extracted from execution_engine.rs (PR-900)
//!
//! Handles SELECT statement dispatch, projection, join planning, and result assembly.

use crate::engine_utils::*;
use crate::expr_utils::*;
use crate::{ExecutionEngine, ExecutorResult, SqlError, SqlResult, Value};
use sqlrustgo_parser::{
    AggregateCall, AggregateFunction, Expression, JoinClause as ParserJoinClause, JoinType,
    SelectStatement,
};
use sqlrustgo_storage::{StorageEngine, TableInfo};

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    pub(crate) fn execute_select(&self, select: &SelectStatement) -> SqlResult<ExecutorResult> {
        // Sprint 1b fix (Q7/Q8/Q9): handle FROM (subquery) AS alias by
        // first executing the subquery to materialize its result into a
        // synthetic in-memory table, then running the outer SELECT against
        // it. We collect the materialized rows + schema before acquiring
        // the storage read lock to avoid reentrant lock issues.
        let materialized: Option<(Vec<Vec<Value>>, TableInfo)> =
            if let Some(subq) = &select.from_subquery {
                // Drop the read lock (if held) and execute subquery; subquery
                // itself takes a read lock internally. Since the outer has not
                // yet acquired a lock, this is a fresh acquisition.
                let sub_result = self.execute_select(subq)?;
                // Build a synthetic TableInfo from the subquery's column list.
                let mut table_info = TableInfo {
                    name: select.table.clone(),
                    columns: Vec::new(),
                    foreign_keys: Vec::new(),
                    unique_constraints: Vec::new(),
                    check_constraints: Vec::new(),
                    partition_info: None,
                };
                for col in &subq.columns {
                    let col_name = col.alias.clone().unwrap_or_else(|| col.name.clone());
                    // Type inference: peek at the first non-null value
                    let inferred_type_str: String = sub_result
                        .rows
                        .iter()
                        .find(|r| r.iter().any(|v| !matches!(v, Value::Null)))
                        .and_then(|first_row| {
                            let col_idx = subq
                                .columns
                                .iter()
                                .position(|c| c.alias.as_ref().unwrap_or(&c.name) == &col_name)?;
                            first_row.get(col_idx).map(|v| match v {
                                Value::Integer(_) => "INTEGER",
                                Value::Float(_) => "FLOAT",
                                Value::Text(_) => "TEXT",
                                Value::Boolean(_) => "BOOLEAN",
                                Value::Blob(_) => "BLOB",
                                Value::Null => "NULL",
                            })
                        })
                        .unwrap_or("TEXT")
                        .to_string();
                    table_info
                        .columns
                        .push(sqlrustgo_storage::ColumnDefinition {
                            name: col_name,
                            data_type: inferred_type_str,
                            nullable: true,
                            primary_key: false,
                        });
                }
                Some((sub_result.rows, table_info))
            } else {
                None
            };

        let storage = self.storage.read().unwrap();

        if select.table.is_empty() {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        // Step 1: FROM/JOIN - get initial rows and schema
        let (mut rows, table_info) = if !select.join_clause.is_empty() {
            self.execute_joins(select)?
        } else if let Some((rows, info)) = materialized {
            (rows, info)
        } else {
            let rows = storage.scan(&select.table)?;
            let table_info = storage.get_table_info(&select.table)?;
            (rows, table_info)
        };

        // Step 2: WHERE
        if let Some(ref where_expr) = select.where_clause {
            rows.retain(|row| eval_predicate(where_expr, row, &table_info));
        }

        // Step 3: GROUP BY + AGGREGATE
        if !select.aggregates.is_empty() {
            let group_exprs = &select.group_by;
            if group_exprs.is_empty() {
                let mut agg_values =
                    self.compute_aggregates(&select.aggregates, &rows, &table_info)?;

                if let Some(ref having_expr) = select.having {
                    let having_schema = build_aggregate_schema(&[], &select.aggregates)?;
                    if !eval_predicate(having_expr, &agg_values, &having_schema) {
                        return Ok(ExecutorResult::new(vec![], 0));
                    }
                }

                return Ok(ExecutorResult::new(vec![agg_values], 1));
            } else {
                let mut groups: std::collections::HashMap<String, Vec<Vec<Value>>> =
                    std::collections::HashMap::new();
                for row in &rows {
                    let key = group_exprs
                        .iter()
                        .map(|expr| evaluate_expr_to_string(expr, row, &table_info))
                        .collect::<Vec<_>>()
                        .join("\x00");
                    groups.entry(key).or_default().push(row.clone());
                }

                let mut agg_result_rows: Vec<Vec<Value>> = Vec::new();

                for (key, group_rows) in groups.iter() {
                    let key_values: Vec<Value> = key
                        .split('\x00')
                        .map(|s| {
                            if s == "NULL" {
                                Value::Null
                            } else if let Ok(n) = s.parse::<i64>() {
                                Value::Integer(n)
                            } else {
                                Value::Text(s.to_string())
                            }
                        })
                        .collect();
                    let agg_values =
                        self.compute_aggregates(&select.aggregates, group_rows, &table_info)?;
                    let mut combined = key_values;
                    combined.extend(agg_values);
                    agg_result_rows.push(combined);
                }

                if let Some(ref having_expr) = select.having {
                    let having_schema = build_aggregate_schema(group_exprs, &select.aggregates)?;
                    agg_result_rows.retain(|row| eval_predicate(having_expr, row, &having_schema));
                }

                let row_count = agg_result_rows.len();
                return Ok(ExecutorResult::new(agg_result_rows, row_count));
            }
        }

        // Step 4: LIMIT / OFFSET
        let limited_rows = if let Some(limit) = select.limit {
            let offset = select.offset.unwrap_or(0);
            if offset as usize >= rows.len() {
                vec![]
            } else {
                rows.into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect()
            }
        } else {
            rows
        };

        // Step 5: SELECT projection — apply each `select.columns` expression
        // to the accumulated row and emit a row of projected values. This
        // is what makes `SELECT EXTRACT(YEAR FROM col) AS o_year` actually
        // return `o_year` instead of the full table schema.
        //
        // Sprint 2: SELECT * (no columns or a `*` entry) skips projection
        // and returns the accumulated rows as-is — that's the existing
        // behavior, just made explicit here.
        let is_star = select.columns.is_empty() || select.columns.iter().any(|c| c.name == "*");
        let projected_rows: Vec<Vec<Value>> = if is_star {
            limited_rows
        } else {
            limited_rows
                .into_iter()
                .map(|row| {
                    select
                        .columns
                        .iter()
                        .map(|col| match &col.expression {
                            Some(expr) => {
                                evaluate_expression(expr, &row, &table_info).unwrap_or(Value::Null)
                            }
                            None => row.first().cloned().unwrap_or(Value::Null),
                        })
                        .collect()
                })
                .collect()
        };

        let row_count = projected_rows.len();
        Ok(ExecutorResult::new(projected_rows, row_count))
    }

    fn compute_aggregates(
        &self,
        aggregates: &[AggregateCall],
        rows: &[Vec<Value>],
        table_info: &TableInfo,
    ) -> SqlResult<Vec<Value>> {
        let mut results = Vec::with_capacity(aggregates.len());
        for agg in aggregates {
            let values: Vec<Value> = if let Some(arg) = agg.args.first() {
                rows.iter()
                    .map(|row| evaluate_expression(arg, row, table_info).unwrap_or(Value::Null))
                    .collect()
            } else {
                vec![Value::Integer(rows.len() as i64)]
            };

            let result = match agg.func {
                AggregateFunction::Count => {
                    if agg.args.is_empty() {
                        // COUNT(*) - count all rows
                        Value::Integer(rows.len() as i64)
                    } else {
                        // COUNT(col) - count non-NULL values
                        let non_null_count =
                            values.iter().filter(|v| !matches!(v, Value::Null)).count();
                        Value::Integer(non_null_count as i64)
                    }
                }
                AggregateFunction::Sum => {
                    // TPC-H Sprint 1 fix (Q8/Q9): accept Float in Sum.
                    // l_extendedprice * (1 - l_discount) returns Float.
                    // Q6 fix: empty result set returns 0 (not Null) for COUNT/SUM semantic.
                    let mut int_sum: i64 = 0;
                    let mut float_sum: f64 = 0.0;
                    let mut any_float = false;
                    for v in &values {
                        match v {
                            Value::Integer(n) => {
                                if any_float {
                                    float_sum += *n as f64;
                                } else {
                                    int_sum += n;
                                }
                            }
                            Value::Float(f) => {
                                if !any_float {
                                    float_sum = int_sum as f64;
                                    any_float = true;
                                }
                                float_sum += f;
                            }
                            _ => {}
                        }
                    }
                    if values.is_empty() {
                        Value::Integer(0)
                    } else if any_float {
                        Value::Float(float_sum)
                    } else {
                        Value::Integer(int_sum)
                    }
                }
                AggregateFunction::Avg => {
                    // TPC-H Sprint 1 fix (Q1): AVG over Float.
                    let mut int_sum: i64 = 0;
                    let mut float_sum: f64 = 0.0;
                    let mut any_float = false;
                    let mut count: i64 = 0;
                    for v in &values {
                        match v {
                            Value::Integer(n) => {
                                if any_float {
                                    float_sum += *n as f64;
                                } else {
                                    int_sum += n;
                                }
                                count += 1;
                            }
                            Value::Float(f) => {
                                if !any_float {
                                    float_sum = int_sum as f64;
                                    any_float = true;
                                }
                                float_sum += f;
                                count += 1;
                            }
                            _ => {}
                        }
                    }
                    if count > 0 {
                        if any_float {
                            Value::Float(float_sum / count as f64)
                        } else {
                            Value::Integer(int_sum / count)
                        }
                    } else {
                        Value::Null
                    }
                }
                AggregateFunction::Min => {
                    let min = values
                        .iter()
                        .filter_map(|v| {
                            if let Value::Integer(n) = v {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .min();
                    min.map(Value::Integer).unwrap_or(Value::Null)
                }
                AggregateFunction::Max => {
                    let max = values
                        .iter()
                        .filter_map(|v| {
                            if let Value::Integer(n) = v {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .max();
                    max.map(Value::Integer).unwrap_or(Value::Null)
                }
            };
            results.push(result);
        }
        Ok(results)
    }

    /// Execute a chain of JOINs: start from the base table, then apply each
    /// JoinClause in order (left-associative: t1 JOIN t2 JOIN t3 → ((t1 JOIN t2) JOIN t3)).
    /// This function only generates joined rows, does NOT apply WHERE/AGG/HAVING.
    fn execute_joins(&self, select: &SelectStatement) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
        let storage = self.storage.read().unwrap();

        // Seed with the base table from FROM clause. If the FROM has an
        // alias (`FROM t a`), prefix the columns with the alias so the
        // alias is queryable in subsequent JOIN ON conditions.
        let mut rows = storage.scan(&select.table)?;
        let raw_info = storage.get_table_info(&select.table)?;
        let base_prefix = select.from_alias.as_ref().unwrap_or(&select.table);
        let mut table_info = if select.from_alias.is_some() {
            // Wrap the base columns in alias-prefixed names.
            let mut new_info = raw_info.clone();
            new_info.name = base_prefix.clone();
            for col in &mut new_info.columns {
                col.name = format!("{}.{}", base_prefix, col.name);
            }
            new_info
        } else {
            raw_info
        };

        for join_clause in &select.join_clause {
            let (new_rows, new_info) =
                self.execute_single_join(&rows, &table_info, join_clause, &storage)?;
            rows = new_rows;
            table_info = new_info;
        }

        Ok((rows, table_info))
    }

    /// Execute a single JOIN against an existing (left) row set + schema.
    /// `join_clause` is consumed separately so callers can iterate a Vec<JoinClause>.
    fn execute_single_join(
        &self,
        left_rows: &[Vec<Value>],
        left_table_info: &TableInfo,
        join_clause: &ParserJoinClause,
        storage: &S,
    ) -> SqlResult<(Vec<Vec<Value>>, TableInfo)> {
        use sqlrustgo_parser::JoinType as ParserJoinType;
        use std::collections::HashMap;

        let right_table_name = join_clause.table.clone();
        let right_alias = join_clause.alias.as_ref().unwrap_or(&right_table_name);

        // Scan the right table fresh each call (left side is already materialized).
        // Wrap right_table_info with the right alias prefix so ON conditions
        // like `n2.n_nationkey` can route to it.
        let right_raw_rows = storage.scan(&right_table_name)?;
        let right_raw_info = storage.get_table_info(&right_table_name)?;
        let right_rows = right_raw_rows;
        let mut right_table_info = right_raw_info.clone();
        if join_clause.alias.is_some() {
            right_table_info.name = right_alias.clone();
            for col in &mut right_table_info.columns {
                col.name = format!("{}.{}", right_alias, col.name);
            }
        }

        // The accumulated left_table_info.name encodes previous joins
        // (e.g. "a_join_n1_join_customer") and is the qualifier scope
        // for ON conditions referencing left side columns.
        let left_alias = left_table_info.name.clone();

        // Extract join key column indices from ON clause
        // For "b.num = c.bid" or "t1.id = t2.id", the canonical form
        // resolves one column from left and one from right.
        // Pass the right *alias* (when set) so qualifiers like `n2.col`
        // route to the right side.
        let join_key = self.find_join_key_index(
            &join_clause.on_clause,
            &left_table_info,
            &left_alias,
            &right_table_info,
            right_alias,
        )?;
        let pairs: Vec<(usize, usize)> = match join_key {
            JoinKey::Pair(li, ri) => vec![(li, ri)],
            JoinKey::Pairs(v) => v,
            JoinKey::Left(_) | JoinKey::Right(_) => {
                return Err(SqlError::ExecutionError(
                    "Join ON must be a binary equality between left and right columns".to_string(),
                ));
            }
        };

        // Determine join type
        let join_type = match join_clause.join_type {
            ParserJoinType::Inner => JoinType::Inner,
            ParserJoinType::Left => JoinType::Left,
            ParserJoinType::Right => JoinType::Right,
            ParserJoinType::Full => JoinType::Full,
            ParserJoinType::Cross => JoinType::Cross,
        };

        let left_col_count = left_table_info.columns.len();
        let right_col_count = right_table_info.columns.len();

        // Helper: render a composite join key as a single string for hashing.
        // The key is the tuple of values for the (left|right) column indices
        // in `pairs`. Any NULL in any component means no match (SQL UNKNOWN
        // for `=`).
        let key_of = |row: &[Value], indices: &[(usize, bool)]| -> Option<String> {
            // indices: (col_idx, is_left)
            let mut parts: Vec<String> = Vec::with_capacity(indices.len());
            for (idx, _is_left) in indices {
                match row.get(*idx) {
                    Some(Value::Null) => return None,
                    Some(v) => parts.push(format!("{:?}", v)),
                    None => return None,
                }
            }
            Some(parts.join("|"))
        };
        let left_key_indices: Vec<(usize, bool)> =
            pairs.iter().map(|(li, _)| (*li, true)).collect();
        let right_key_indices: Vec<(usize, bool)> =
            pairs.iter().map(|(_, ri)| (*ri, false)).collect();

        let mut matched_results = match join_type {
            JoinType::Inner | JoinType::Left | JoinType::Right | JoinType::Full => {
                // Hash-based matching
                // SQL semantics: NULL = NULL is UNKNOWN (not a match), so skip NULL keys
                let mut right_hash: HashMap<String, Vec<Vec<Value>>> = HashMap::new();
                for right_row in &right_rows {
                    let key = match key_of(right_row, &right_key_indices) {
                        Some(k) => k,
                        None => continue,
                    };
                    right_hash.entry(key).or_default().push(right_row.clone());
                }

                let mut matched: Vec<Vec<Value>> = Vec::new();
                let mut left_matched: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();
                let mut right_matched: std::collections::HashSet<usize> =
                    std::collections::HashSet::new();

                // Match left rows to right
                for (li, left_row) in left_rows.iter().enumerate() {
                    let key = match key_of(left_row, &left_key_indices) {
                        Some(k) => k,
                        None => {
                            // NULL in any join key column — no match.
                            continue;
                        }
                    };
                    if let Some(right_match_rows) = right_hash.get(&key) {
                        left_matched.insert(li);
                        for right_row in right_match_rows {
                            // Find the original right row index
                            if let Some(ri) = right_rows.iter().position(|r| r == right_row) {
                                right_matched.insert(ri);
                            }
                            let mut combined = left_row.clone();
                            combined.extend(right_row.clone());
                            matched.push(combined);
                        }
                    }
                }

                // For LEFT/RIGHT/FULL, add unmatched rows
                if matches!(join_type, JoinType::Left | JoinType::Full) {
                    for (li, left_row) in left_rows.iter().enumerate() {
                        if !left_matched.contains(&li) {
                            let mut combined = left_row.clone();
                            combined.extend(vec![Value::Null; right_col_count]);
                            matched.push(combined);
                        }
                    }
                }

                if matches!(join_type, JoinType::Right | JoinType::Full) {
                    for (ri, right_row) in right_rows.iter().enumerate() {
                        if !right_matched.contains(&ri) {
                            let mut combined = vec![Value::Null; left_col_count];
                            combined.extend(right_row.clone());
                            matched.push(combined);
                        }
                    }
                }

                matched
            }
            JoinType::Cross => {
                let mut results = Vec::new();
                for left_row in left_rows {
                    for right_row in &right_rows {
                        let mut combined = left_row.clone();
                        combined.extend(right_row.clone());
                        results.push(combined);
                    }
                }
                results
            }
        };

        let combined_schema = build_combined_schema(
            &left_table_info,
            &left_alias,
            &right_table_info,
            right_alias,
        )?;
        Ok((matched_results, combined_schema))
    }

    /// Resolve a join-key column index, looking across both sides of the join.
    /// For multi-join chains, the left side is the accumulated `a_join_b...` and
    /// would otherwise reject qualifiers that point at a freshly-joined right
    /// table (or at a table embedded in the accumulated left). Returning a
    /// `JoinKey { side, index }` lets the caller pick the index in the correct
    /// row vector.
    ///
    /// The accumulated left's column *names* are `left_alias.col`, but the
    /// `left_alias` is `a_join_b` (or similar). Users typically write
    /// `b.num = c.bid` where neither qualifier matches the accumulated alias,
    /// so we also fall back to a column-name search across both sides.
    fn find_join_key_index(
        &self,
        expr: &Expression,
        left_info: &TableInfo,
        left_name: &str,
        right_info: &TableInfo,
        right_name: &str,
    ) -> SqlResult<JoinKey> {
        match expr {
            Expression::Identifier(name) => {
                if let Some((qualifier, col_name)) = name.split_once('.') {
                    // Qualified name: must match either side
                    if qualifier == left_name {
                        let idx = lookup_column(left_info, col_name).ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Column '{}.{}' not found in {}",
                                qualifier, col_name, left_name
                            ))
                        })?;
                        Ok(JoinKey::Left(idx))
                    } else if qualifier == right_name {
                        let idx = lookup_column(right_info, col_name).ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Column '{}.{}' not found in {}",
                                qualifier, col_name, right_name
                            ))
                        })?;
                        Ok(JoinKey::Right(idx))
                    } else {
                        // Qualifier doesn't match left or right (e.g. an
                        // intermediate table already absorbed into left via
                        // build_combined_schema, where columns are named
                        // "a_join_b.col" but the user writes "b.col").
                        // Try column-name lookup on both sides as a best-effort.
                        if let Some(idx) = lookup_column(left_info, col_name) {
                            return Ok(JoinKey::Left(idx));
                        }
                        if let Some(idx) = lookup_column(right_info, col_name) {
                            return Ok(JoinKey::Right(idx));
                        }
                        Err(SqlError::ExecutionError(format!(
                            "Column '{}' not found in either '{}' or '{}'",
                            name, left_name, right_name
                        )))
                    }
                } else {
                    // Unqualified: search both sides
                    if let Some(idx) = lookup_column(left_info, name) {
                        return Ok(JoinKey::Left(idx));
                    }
                    if let Some(idx) = lookup_column(right_info, name) {
                        return Ok(JoinKey::Right(idx));
                    }
                    Err(SqlError::ExecutionError(format!(
                        "Column '{}' not found in either '{}' or '{}'",
                        name, left_name, right_name
                    )))
                }
            }
            Expression::BinaryOp(left_expr, op, right_expr) if op.to_uppercase() == "AND" => {
                // TPC-H Q9: `ON a.id = b.a_id AND a.sub_id = b.a_sub`.
                // Each AND branch must itself be a binary `=` between a
                // left and a right column. Combine the resulting pairs.
                let lk = self
                    .find_join_key_index(left_expr, left_info, left_name, right_info, right_name)?;
                let rk = self.find_join_key_index(
                    right_expr, left_info, left_name, right_info, right_name,
                )?;
                match (lk, rk) {
                    (JoinKey::Pair(li1, ri1), JoinKey::Pair(li2, ri2)) => {
                        Ok(JoinKey::Pairs(vec![(li1, ri1), (li2, ri2)]))
                    }
                    (JoinKey::Pairs(mut v), JoinKey::Pair(li, ri)) => {
                        v.push((li, ri));
                        Ok(JoinKey::Pairs(v))
                    }
                    (JoinKey::Pair(li, ri), JoinKey::Pairs(mut v)) => {
                        let mut all = vec![(li, ri)];
                        all.append(&mut v);
                        Ok(JoinKey::Pairs(all))
                    }
                    (JoinKey::Pairs(mut v1), JoinKey::Pairs(v2)) => {
                        v1.extend(v2);
                        Ok(JoinKey::Pairs(v1))
                    }
                    _ => Err(SqlError::ExecutionError(
                        "Multi-column AND ON requires each branch to be a binary `=` between left and right columns".to_string(),
                    )),
                }
            }
            Expression::BinaryOp(left_expr, _op, right_expr) => {
                // Standard SQL: left side of `=` references left table,
                // right side references right table. Resolve each independently.
                let lk = self
                    .find_join_key_index(left_expr, left_info, left_name, right_info, right_name)?;
                let rk = self.find_join_key_index(
                    right_expr, left_info, left_name, right_info, right_name,
                )?;
                // We only support the canonical case: one key from each side.
                match (lk, rk) {
                    (JoinKey::Left(li), JoinKey::Right(ri)) => Ok(JoinKey::Pair(li, ri)),
                    (JoinKey::Right(ri), JoinKey::Left(li)) => Ok(JoinKey::Pair(li, ri)),
                    _ => Err(SqlError::ExecutionError(
                        "Join condition must reference one column from each side".to_string(),
                    )),
                }
            }
            _ => Err(SqlError::ExecutionError(
                "Unsupported join condition expression".to_string(),
            )),
        }
    }
}

/// Which side of a single join a resolved column index belongs to, or a
/// canonical pair (left, right) for binary `=` ON conditions.
/// `Pairs` carries multiple `(left_idx, right_idx)` pairs for
/// multi-column ON (`ON a.id = b.a_id AND a.sub_id = b.a_sub`).
#[derive(Debug, Clone)]
enum JoinKey {
    Left(usize),
    Right(usize),
    Pair(usize, usize),
    Pairs(Vec<(usize, usize)>),
}

/// Look up a column in a (possibly accumulated) schema.
///
/// `build_combined_schema` rewrites column names as `alias.col`; for chained
/// joins, the column name may pick up multiple prefixes (e.g. an original
/// `b.num` becomes `a_join_b.b.num` after a second join). This helper accepts
/// any of: a bare column name (`num`), a single-prefix form (`b.num`), or
/// the full accumulated form (`a_join_b.b.num`) — they all map to the same
/// index.
fn lookup_column(info: &TableInfo, col_name: &str) -> Option<usize> {
    // Strip any qualifier from the caller's reference (we only care about
    // the final segment; the qualifier is matched separately).
    let bare = col_name.rsplit('.').next().unwrap_or(col_name);

    info.columns.iter().position(|c| {
        if c.name == col_name {
            return true;
        }
        // The accumulated column name may have one or more `.`-prefix
        // segments. Find the final segment and compare.
        if let Some((_, suffix)) = c.name.rsplit_once('.') {
            if suffix == bare {
                return true;
            }
        }
        if c.name == bare {
            return true;
        }
        false
    })
}

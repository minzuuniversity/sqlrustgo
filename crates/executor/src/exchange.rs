//! Exchange Operator — multi-partition data redistribution
//!
//! Part of INT-2 (issue #3185, P3-6, v3.10 scope).
//! Complements `parallel_executor.rs` (Partition) and
//! `parallel_vector_executor.rs` (Vector scan) by adding the missing
//! "merge partitions back into a single result" stage.
//!
//! Three modes (per distributed-DB literature):
//!   - Gather        (1→1)   partial result merge (COUNT/SUM partials)
//!   - Broadcast     (1→N)   replicate small side (hash join build)
//!   - Repartition   (N→N)   re-hash on key (hash join both sides)
//!
//! ## Status (v3.9.0)
//! Framework stage: SKELETON. Default OFF in v3.9.0 (opt-in via
//! `ENABLE_EXCHANGE_OPT=1` env). v3.10+ will default ON.
//!
//! ## Usage (illustrative, NOT wired yet)
//! ```ignore
//! use sqlrustgo_executor::exchange::{ExchangeSpec, ExchangeMode, run_exchange};
//!
//! // After parallel scan, get partial results from each partition
//! let partials: Vec<Vec<Record>> = partitions.iter()
//!     .map(|p| parallel_scan_agg(p))
//!     .collect();
//!
//! // Exchange stage
//! let spec = ExchangeSpec::gather();
//! let final_result = run_exchange(&spec, partials)?;
//! ```
//!
//! ## Companion
//! - `parallel_executor.rs` — produces `partials`
//! - `parallel_vector_executor.rs` — produces `Vec<Vec<Record>>` from SIMD scan
//! - `merge.rs` — does row-level MERGE statement (different concept, not used here)
//!
//! Maintainer: Hermes Agent
//! Last touched: 2026-06-14 (kickoff for v3.10)

use sqlrustgo_storage::Record;
use sqlrustgo_types::{SqlError, SqlResult};

/// Exchange mode — which redistribution pattern to use.
///
/// See `docs/openspec/3185-exchange-operator.md` §1.2 for the 3 modes
/// and §2.2 for how Repartition delegates to `PartitionStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExchangeMode {
    /// 1→1: sum/merge partial results into one output.
    /// Trigger: `COUNT(*)`, `SUM(x)`, `MIN(x)`, `MAX(x)` over partitioned data.
    Gather,
    /// 1→N: replicate a small input to all N partitions.
    /// Trigger: hash-join with small build side.
    Broadcast,
    /// N→N: re-hash on a key so matching keys end up in same partition.
    /// Trigger: hash-join with both sides large.
    /// **Inner strategy**: delegated to `crates/distributed/src/partition.rs`
    /// `PartitionStrategy` (Hash/Range/Key/List). v3.10 default: Hash.
    /// v3.10.1: expose `partition_strategy` field for CBO choice.
    Repartition,
}

impl ExchangeMode {
    /// All known modes — for gate check.
    pub const ALL: &'static [ExchangeMode] = &[Self::Gather, Self::Broadcast, Self::Repartition];

    /// Whether this mode preserves row order within partition (sanity test).
    /// - Gather: yes (concatenate partition results in order)
    /// - Broadcast: yes (per-partition replication is order-preserving)
    /// - Repartition: no (hashing scrambles order by design)
    pub fn preserves_partition_order(&self) -> bool {
        match self {
            Self::Gather | Self::Broadcast => true,
            Self::Repartition => false,
        }
    }
}

/// Exchange specification — concrete config for a single exchange step.
///
/// v3.9.0: skeleton with no AggFunction (just identity concat).
/// v3.10 T1: add `agg_kind: Option<AggMergeKind>` for real dispatch.
#[derive(Debug, Clone)]
pub struct ExchangeSpec {
    pub mode: ExchangeMode,
    /// Number of partitions involved (1 for Gather, ≥1 for Broadcast, ≥1 for Repartition).
    pub num_partitions: usize,
    /// Optional key columns (required for Repartition, ignored for Gather/Broadcast).
    /// Stored as column index for v3.10 (typed ColumnRef in v3.10.1).
    pub key_indices: Vec<usize>,
    /// Safety guard: refuse to broadcast inputs larger than this.
    /// Default 16MB (matches 3185 spec §三 broadcast OOM 缓解).
    pub broadcast_max_bytes: usize,
    /// v3.10 T1: how to merge partial aggregate results (Gather mode only).
    /// None = identity concat (current T0 behavior). Some(...) = real merge.
    pub agg_merge: Option<AggMergeKind>,
}

/// How to merge partial aggregate results from N partitions into 1.
///
/// Maps to `crates/executor/src/vectorization.rs::AggFunction` 5 variants.
/// Exchange-side enum (not directly coupled) so v3.10 exchange.rs stays
/// independent of vectorization types until wired in v3.10 main path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggMergeKind {
    /// Sum all partial counts → total count.
    Count,
    /// Sum all partial sums → total sum.
    Sum,
    /// Sum + count → avg (only valid for numeric columns).
    Avg,
    /// MIN(partial_min) → global min.
    Min,
    /// MAX(partial_max) → global max.
    Max,
}

impl ExchangeSpec {
    /// Default Gather spec (most common — used by COUNT/SUM/AVG partial merge).
    pub fn gather() -> Self {
        Self {
            mode: ExchangeMode::Gather,
            num_partitions: 1,
            key_indices: vec![],
            broadcast_max_bytes: 16 * 1024 * 1024,
        }
    }

    /// Broadcast spec with size cap.
    pub fn broadcast(max_bytes: usize) -> Self {
        Self {
            mode: ExchangeMode::Broadcast,
            num_partitions: 1, // input is one side; output is num_partitions copies
            key_indices: vec![],
            broadcast_max_bytes: max_bytes,
        }
    }

    /// Repartition spec (hash join both sides, on key).
    pub fn repartition(num_partitions: usize, key_indices: Vec<usize>) -> Self {
        Self {
            mode: ExchangeMode::Repartition,
            num_partitions,
            key_indices,
            broadcast_max_bytes: 16 * 1024 * 1024,
        }
    }

    /// Validate spec consistency. Returns Err on impossible config.
    pub fn validate(&self) -> SqlResult<()> {
        match self.mode {
            ExchangeMode::Gather => {
                if self.num_partitions != 1 {
                    return Err(SqlError::InvalidArgument(format!(
                        "Gather requires num_partitions=1, got {}",
                        self.num_partitions
                    )));
                }
            }
            ExchangeMode::Broadcast => {
                if self.broadcast_max_bytes == 0 {
                    return Err(SqlError::InvalidArgument(
                        "Broadcast requires broadcast_max_bytes > 0".to_string(),
                    ));
                }
            }
            ExchangeMode::Repartition => {
                if self.num_partitions == 0 {
                    return Err(SqlError::InvalidArgument(
                        "Repartition requires num_partitions >= 1".to_string(),
                    ));
                }
                if self.key_indices.is_empty() {
                    return Err(SqlError::InvalidArgument(
                        "Repartition requires key_indices non-empty".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Cost estimate for an exchange step.
///
/// Used by CBO (#3182) to decide whether to enable parallel path.
/// v3.10 stage: rough model. v3.10.1: refine with backpressure + spill cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExchangeCost {
    /// Estimated rows in (sum across inputs).
    pub input_rows: usize,
    /// Estimated rows out.
    pub output_rows: usize,
    /// Estimated bytes shuffled (Gather: 0, Broadcast: input * num_partitions,
    /// Repartition: input_rows * avg_row_size).
    pub shuffle_bytes: u64,
}

impl ExchangeCost {
    /// Rough cost model — v3.10 stage. Returns 0 cost for safety (opt-in).
    /// TODO (v3.10.1): wire into CBO via `unified_cost.rs`.
    pub fn estimate(spec: &ExchangeSpec, input_rows: usize, avg_row_bytes: usize) -> Self {
        let shuffle_bytes = match spec.mode {
            ExchangeMode::Gather => 0, // single-point merge, no shuffle
            ExchangeMode::Broadcast => {
                (input_rows as u64) * (avg_row_bytes as u64) * (spec.num_partitions.max(1) as u64)
            }
            ExchangeMode::Repartition => {
                (input_rows as u64) * (avg_row_bytes as u64)
            }
        };
        Self {
            input_rows,
            output_rows: input_rows, // exchange doesn't filter
            shuffle_bytes,
        }
    }
}

/// Trait for exchange operators.
///
/// v3.10: 3 concrete impls (GatherExchange, BroadcastExchange, RepartitionExchange).
/// v3.10.1: SIMD gather (per #3184).
pub trait ExchangeOperator: Send + Sync {
    /// Execute exchange, given N inputs and the spec.
    /// Returns the redistributed output (1 partition for Gather,
    /// N partitions for Broadcast, N partitions for Repartition).
    fn execute(&self, inputs: Vec<Vec<Record>>) -> SqlResult<Vec<Vec<Record>>>;

    /// Cost estimate (used by CBO).
    fn estimate_cost(&self, input_rows: usize) -> ExchangeCost;

    /// Which mode this operator implements.
    fn mode(&self) -> ExchangeMode;
}

// =============================================================================
// Concrete impls (stubs for v3.9.0; full impls in v3.10)
// =============================================================================

/// Gather (1→1) — sum/merge partial results.
///
/// v3.9.0: identity (return as-is) — framework stage, not full impl.
/// v3.10 T1: real merge by AggMergeKind (Count/Sum/Avg/Min/Max).
///
/// `kind: None` → identity concat (T0 behavior, current).
/// `kind: Some(AggMergeKind::Count)` → sum partial counts (T1 new).
pub struct GatherExchange {
    pub kind: Option<AggMergeKind>,
}

impl ExchangeOperator for GatherExchange {
    fn execute(&self, inputs: Vec<Vec<Record>>) -> SqlResult<Vec<Vec<Record>>> {
        // v3.9.0 stub path: identity concat (preserves T0 behavior).
        // v3.10 T1 path: dispatch by AggMergeKind.
        match self.kind {
            None => {
                // T0 behavior — keep for v3.9.0 compat
                let mut out = Vec::with_capacity(inputs.iter().map(|p| p.len()).sum());
                for partition in inputs {
                    out.extend(partition);
                }
                Ok(vec![out])
            }
            Some(kind) => gather_merge(inputs, kind),
        }
    }

    fn estimate_cost(&self, input_rows: usize) -> ExchangeCost {
        ExchangeCost {
            input_rows,
            output_rows: input_rows,
            shuffle_bytes: 0,
        }
    }

    fn mode(&self) -> ExchangeMode {
        ExchangeMode::Gather
    }
}

/// v3.10 T1: real partial-result merge.
///
/// Expects each input partition to contain exactly 1 row (the partial
/// aggregate result from that partition). The first column holds the
/// aggregate value; for Avg, second column is the partial count.
fn gather_merge(inputs: Vec<Vec<Record>>, kind: AggMergeKind) -> SqlResult<Vec<Vec<Record>>> {
    use sqlrustgo_types::Value;

    // v3.10 T1: this is the production path. v3.9.0 callers pass kind=None
    // and never reach here, so this code is "exercisable" but not "wired".
    if inputs.is_empty() {
        return Ok(vec![vec![]]);
    }
    let mut all_rows: Vec<Record> = Vec::with_capacity(inputs.iter().map(|p| p.len()).sum());
    for p in inputs {
        all_rows.extend(p);
    }

    // For each "value" column, fold the partials by AggMergeKind.
    // Output: single row, same arity as input.
    if all_rows.is_empty() {
        return Ok(vec![vec![]]);
    }
    let arity = all_rows[0].len();
    let mut out_row: Vec<Value> = Vec::with_capacity(arity);
    for col_idx in 0..arity {
        out_row.push(merge_column(&all_rows, col_idx, kind)?);
    }
    Ok(vec![out_row])
}

/// Merge a single column across partitions according to AggMergeKind.
fn merge_column(
    rows: &[Record],
    col_idx: usize,
    kind: AggMergeKind,
) -> SqlResult<Value> {
    use sqlrustgo_types::Value;

    // Collect non-null partials.
    let partials: Vec<&Value> = rows
        .iter()
        .filter_map(|r| r.get(col_idx))
        .filter(|v| !matches!(v, Value::Null))
        .collect();

    if partials.is_empty() {
        return Ok(Value::Null);
    }

    match kind {
        AggMergeKind::Count => {
            // Count: sum partial counts (each partial is itself a count).
            let mut total: i64 = 0;
            for v in &partials {
                if let Value::Integer(n) = v {
                    total = total.checked_add(*n).ok_or_else(|| {
                        SqlError::RuntimeError("Count overflow".to_string())
                    })?;
                } else {
                    return Err(SqlError::RuntimeError(format!(
                        "Count expects Integer partial, got {:?}",
                        v
                    )));
                }
            }
            Ok(Value::Integer(total))
        }
        AggMergeKind::Sum => {
            // Sum: sum partial sums. Handle Integer + Float.
            let mut int_total: Option<i64> = Some(0);
            let mut float_total: f64 = 0.0;
            let mut has_float = false;
            for v in &partials {
                match v {
                    Value::Integer(n) => {
                        if let Some(t) = int_total {
                            int_total = t.checked_add(*n);
                        }
                    }
                    Value::Float(f) => {
                        has_float = true;
                        float_total += f;
                    }
                    _ => {
                        return Err(SqlError::RuntimeError(format!(
                            "Sum expects numeric partial, got {:?}",
                            v
                        )));
                    }
                }
            }
            if has_float {
                // Promote: combine int partials as floats.
                if let Some(t) = int_total {
                    float_total += t as f64;
                }
                Ok(Value::Float(float_total))
            } else {
                Ok(Value::Integer(int_total.unwrap_or(0)))
            }
        }
        AggMergeKind::Avg => {
            // Avg: needs (sum_partial, count_partial) pair.
            // Convention: even columns = sum partials, odd = count partials.
            // For single-column Avg, caller must have aggregated count elsewhere.
            // This impl handles the 2-column form: col_idx=sum, col_idx+1=count.
            let sum_col = col_idx;
            let count_col = col_idx + 1;
            if count_col >= rows[0].len() {
                return Err(SqlError::RuntimeError(
                    "Avg needs (sum, count) column pair".to_string(),
                ));
            }
            let mut sum: f64 = 0.0;
            let mut count: i64 = 0;
            for row in rows {
                if let (Some(s), Some(c)) = (row.get(sum_col), row.get(count_col)) {
                    if !matches!(s, Value::Null) && !matches!(c, Value::Null) {
                        sum += match s {
                            Value::Integer(n) => *n as f64,
                            Value::Float(f) => *f,
                            _ => 0.0,
                        };
                        if let Value::Integer(n) = c {
                            count += n;
                        }
                    }
                }
            }
            if count == 0 {
                Ok(Value::Null)
            } else {
                Ok(Value::Float(sum / count as f64))
            }
        }
        AggMergeKind::Min => {
            // Min: MIN of all partial MINs.
            let mut best: Option<Value> = None;
            for v in &partials {
                best = Some(match &best {
                    None => (*v).clone(),
                    Some(cur) => {
                        if compare_values(v, cur) == std::cmp::Ordering::Less {
                            (*v).clone()
                        } else {
                            cur.clone()
                        }
                    }
                });
            }
            Ok(best.unwrap_or(Value::Null))
        }
        AggMergeKind::Max => {
            // Max: MAX of all partial MAXs.
            let mut best: Option<Value> = None;
            for v in &partials {
                best = Some(match &best {
                    None => (*v).clone(),
                    Some(cur) => {
                        if compare_values(v, cur) == std::cmp::Ordering::Greater {
                            (*v).clone()
                        } else {
                            cur.clone()
                        }
                    }
                });
            }
            Ok(best.unwrap_or(Value::Null))
        }
    }
}

/// Compare two SQL values for MIN/MAX. Returns Ordering.
///
/// Supports: Integer, Float, Text (lexicographic), Date (string compare).
fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    use sqlrustgo_types::Value;
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Integer(x), Value::Float(y)) => {
            (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::Float(x), Value::Integer(y)) => {
            x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal)
        }
        // Issue #4846: PAD SPACE semantics for CHAR(n) comparison.
        // SQLite/MySQL/PostgreSQL all trim trailing whitespace on Text
        // equality; CHAR(n) values are stored blank-padded, so without
        // this fix `WHERE id='U1'` on a CHAR(10) column would not match
        // the stored 'U1        ' value.
        (Value::Text(x), Value::Text(y)) => x.trim_end().cmp(y.trim_end()),
        (Value::Date(x), Value::Date(y)) => x.cmp(y),
        (Value::Timestamp(x), Value::Timestamp(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal, // Null, mixed types: equal (degraded)
    }
}

/// Broadcast (1→N) — replicate small side.
///
/// v3.9.0: stub. v3.10: enforce broadcast_max_bytes, replicate.
pub struct BroadcastExchange {
    pub max_bytes: usize,
}

impl ExchangeOperator for BroadcastExchange {
    fn execute(&self, mut inputs: Vec<Vec<Record>>) -> SqlResult<Vec<Vec<Record>>> {
        // Broadcast: one input, replicate to N output partitions.
        // For v3.9.0 stub, return input as single partition (1 input expected).
        if inputs.len() != 1 {
            return Err(SqlError::InvalidArgument(format!(
                "Broadcast expects 1 input, got {}",
                inputs.len()
            )));
        }
        let input = inputs.remove(0);
        // v3.10 will: estimate size, refuse if > max_bytes, replicate to N outputs
        Ok(vec![input])
    }

    fn estimate_cost(&self, input_rows: usize) -> ExchangeCost {
        // Rough: assume 1KB avg row
        ExchangeCost::estimate(
            &ExchangeSpec::broadcast(self.max_bytes),
            input_rows,
            1024,
        )
    }

    fn mode(&self) -> ExchangeMode {
        ExchangeMode::Broadcast
    }
}

/// Repartition (N→N) — hash on key.
///
/// v3.9.0: stub. v3.10: hash-partition each input row by key_indices.
pub struct RepartitionExchange {
    pub num_partitions: usize,
    pub key_indices: Vec<usize>,
}

impl ExchangeOperator for RepartitionExchange {
    fn execute(&self, inputs: Vec<Vec<Record>>) -> SqlResult<Vec<Vec<Record>>> {
        // v3.9.0 stub: just re-distribute round-robin (placeholder).
        // v3.10 will: hash(key) % num_partitions per row, return N partitions.
        let mut outputs: Vec<Vec<Record>> = (0..self.num_partitions)
            .map(|_| Vec::new())
            .collect();
        let mut counter: usize = 0;
        for partition in inputs {
            for row in partition {
                outputs[counter % self.num_partitions].push(row);
                counter += 1;
            }
        }
        Ok(outputs)
    }

    fn estimate_cost(&self, input_rows: usize) -> ExchangeCost {
        ExchangeCost::estimate(
            &ExchangeSpec::repartition(self.num_partitions, self.key_indices.clone()),
            input_rows,
            1024,
        )
    }

    fn mode(&self) -> ExchangeMode {
        ExchangeMode::Repartition
    }
}

/// Factory: create exchange operator from spec.
pub fn from_spec(spec: &ExchangeSpec) -> SqlResult<Box<dyn ExchangeOperator>> {
    spec.validate()?;
    Ok(match spec.mode {
        ExchangeMode::Gather => Box::new(GatherExchange {
            kind: spec.agg_merge,
        }),
        ExchangeMode::Broadcast => Box::new(BroadcastExchange {
            max_bytes: spec.broadcast_max_bytes,
        }),
        ExchangeMode::Repartition => Box::new(RepartitionExchange {
            num_partitions: spec.num_partitions.max(1),
            key_indices: spec.key_indices.clone(),
        }),
    })
}

/// Convenience: run a one-shot exchange with the spec's defaults.
///
/// This is the v3.9.0 entry point — used by callers that already have
/// `Vec<Vec<Record>>` from parallel scan and want the merged result.
pub fn run_exchange(
    spec: &ExchangeSpec,
    inputs: Vec<Vec<Record>>,
) -> SqlResult<Vec<Vec<Record>>> {
    let op = from_spec(spec)?;
    op.execute(inputs)
}

// =============================================================================
// Tests (5 self-tests per 3185 spec §2.3 — minimum viable)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    fn make_rows(n: usize) -> Vec<Record> {
        (0..n)
            .map(|i| vec![Value::Integer(i as i64), Value::Text(format!("row{}", i))])
            .collect()
    }

    #[test]
    fn test_exchange_modes_coverage() {
        // 3185 spec §2.3 类别 1: 3 mode 都有
        assert_eq!(ExchangeMode::ALL.len(), 3);
        assert!(ExchangeMode::ALL.contains(&ExchangeMode::Gather));
        assert!(ExchangeMode::ALL.contains(&ExchangeMode::Broadcast));
        assert!(ExchangeMode::ALL.contains(&ExchangeMode::Repartition));
    }

    #[test]
    fn test_spec_validate_gather_requires_one_partition() {
        let ok = ExchangeSpec::gather();
        assert!(ok.validate().is_ok());

        let bad = ExchangeSpec {
            mode: ExchangeMode::Gather,
            num_partitions: 2,
            key_indices: vec![],
            broadcast_max_bytes: 1024,
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn test_spec_validate_repartition_requires_key() {
        let bad = ExchangeSpec {
            mode: ExchangeMode::Repartition,
            num_partitions: 4,
            key_indices: vec![],
            broadcast_max_bytes: 1024,
        };
        assert!(bad.validate().is_err());

        let ok = ExchangeSpec::repartition(4, vec![0]);
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn test_gather_concatenates_inputs() {
        let inputs = vec![make_rows(3), make_rows(2), make_rows(1)];
        let out = run_exchange(&ExchangeSpec::gather(), inputs).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 6);
    }

    #[test]
    fn test_repartition_distributes_inputs() {
        // Stub: round-robin. Real impl in v3.10: hash on key.
        let inputs = vec![make_rows(10)];
        let out = run_exchange(
            &ExchangeSpec::repartition(3, vec![0]),
            inputs,
        )
        .unwrap();
        assert_eq!(out.len(), 3);
        let total: usize = out.iter().map(|p| p.len()).sum();
        assert_eq!(total, 10);
    }

    // =================================================================
    // T1 tests — real AggMergeKind dispatch (v3.10 T1 spec)
    // =================================================================

    /// T1.1: Count partials sum correctly (3 partitions × 100 = 300).
    #[test]
    fn test_t1_gather_count_merges_partials() {
        use sqlrustgo_types::Value;
        // 3 partitions, each with 1 row containing partial count
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(100)]],
            vec![vec![Value::Integer(100)]],
            vec![vec![Value::Integer(100)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Count);
        let out = run_exchange(&spec, inputs).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 1);
        assert_eq!(out[0][0][0], Value::Integer(300));
    }

    /// T1.2: Sum partials sum correctly (Integer).
    #[test]
    fn test_t1_gather_sum_merges_integers() {
        use sqlrustgo_types::Value;
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(50)]],
            vec![vec![Value::Integer(75)]],
            vec![vec![Value::Integer(125)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Sum);
        let out = run_exchange(&spec, inputs).unwrap();
        assert_eq!(out[0][0][0], Value::Integer(250));
    }

    /// T1.3: Sum partials with Float promotion.
    #[test]
    fn test_t1_gather_sum_promotes_to_float() {
        use sqlrustgo_types::Value;
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(50)]],
            vec![vec![Value::Float(1.5)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Sum);
        let out = run_exchange(&spec, inputs).unwrap();
        // 50 + 1.5 = 51.5 (Float promotion)
        assert_eq!(out[0][0][0], Value::Float(51.5));
    }

    /// T1.4: Avg uses 2-column (sum, count) pair.
    #[test]
    fn test_t1_gather_avg_uses_pair() {
        use sqlrustgo_types::Value;
        // 3 partitions each contribute (sum_partial, count_partial)
        // Total sum = 100+200+300 = 600, total count = 5+10+15 = 30
        // avg = 600 / 30 = 20.0
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(100), Value::Integer(5)]],
            vec![vec![Value::Integer(200), Value::Integer(10)]],
            vec![vec![Value::Integer(300), Value::Integer(15)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Avg);
        let out = run_exchange(&spec, inputs).unwrap();
        assert_eq!(out[0][0][0], Value::Float(20.0));
    }

    /// T1.5: Min picks smallest across partitions.
    #[test]
    fn test_t1_gather_min_picks_smallest() {
        use sqlrustgo_types::Value;
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(50)]],
            vec![vec![Value::Integer(5)]],
            vec![vec![Value::Integer(100)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Min);
        let out = run_exchange(&spec, inputs).unwrap();
        assert_eq!(out[0][0][0], Value::Integer(5));
    }

    /// T1.6: Max picks largest across partitions.
    #[test]
    fn test_t1_gather_max_picks_largest() {
        use sqlrustgo_types::Value;
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(50)]],
            vec![vec![Value::Integer(5)]],
            vec![vec![Value::Integer(1000)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Max);
        let out = run_exchange(&spec, inputs).unwrap();
        assert_eq!(out[0][0][0], Value::Integer(1000));
    }

    /// T1.7: NULL partials are skipped (not counted as 0).
    #[test]
    fn test_t1_gather_skips_null_partials() {
        use sqlrustgo_types::Value;
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(10)]],
            vec![vec![Value::Null]], // skipped
            vec![vec![Value::Integer(20)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Sum);
        let out = run_exchange(&spec, inputs).unwrap();
        // 10 + 20 = 30 (NULL skipped, not 0)
        assert_eq!(out[0][0][0], Value::Integer(30));
    }

    /// T1.8: Backward compat — kind=None still does identity concat (T0 behavior).
    #[test]
    fn test_t1_gather_none_preserves_t0_behavior() {
        let inputs = vec![make_rows(3), make_rows(2), make_rows(1)];
        let out = run_exchange(&ExchangeSpec::gather(), inputs).unwrap();
        // Should still concatenate (T0 behavior unchanged)
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 6);
    }

    /// T1.9: Count overflow is detected.
    #[test]
    fn test_t1_gather_count_overflow_detected() {
        use sqlrustgo_types::Value;
        let inputs: Vec<Vec<Record>> = vec![
            vec![vec![Value::Integer(i64::MAX)]],
            vec![vec![Value::Integer(1)]],
        ];
        let mut spec = ExchangeSpec::gather();
        spec.agg_merge = Some(AggMergeKind::Count);
        let result = run_exchange(&spec, inputs);
        assert!(result.is_err());
    }
}

//! 2-way and multi-way hash join helpers.
//!
//! Both helpers are **stateless**: they take ownership of input rows
//! and return joined rows. The caller is responsible for projecting
//! non-key columns out of the joined rows (this module emits raw
//! `R ++ S` concatenations, in that order).
//!
//! Equality semantics: keys are compared via `Value::PartialEq`.
//! `Value::Null` keys are dropped on both sides (SQL three-valued
//! logic; `NULL = NULL` is not true and would produce no match).
//!
//! Sprint 8 deferred (see
//! `openspec/changes/2026-06-08-v390-sprint8-q3-exists/`): the
//! helpers here are NOT yet wired into
//! `src/engine_select.rs::execute_joins`. They live as library
//! functions so they can be unit-tested in isolation today and
//! integrated next sprint.

use std::collections::HashMap;
use std::hash::Hash;

use sqlrustgo_types::Value;

/// Build a 2-way hash join. The smaller of `left_rows` and
/// `right_rows` (by length) is hashed on its key column and the
/// larger side is probed once.
///
/// `left_key_idx` is the column index of the join key in
/// `left_rows`. `right_key_idx` is the column index in `right_rows`.
/// The output rows are `(left_row, right_row)` concatenations, in
/// that order; rows with `Null` keys are skipped on both sides.
///
/// Complexity:
/// - Time: `O(|left| + |right|)` (build + probe once each).
/// - Space: `O(min(|left|, |right|))` for the hash table.
///
/// Returns the joined rows in arbitrary order. For TPC-H-style
/// aggregation queries, the caller should sort or hash-aggregate
/// afterward; this helper preserves no ordering.
pub fn hash_join_inner_outer(
    left_rows: &[Vec<Value>],
    left_key_idx: usize,
    right_rows: &[Vec<Value>],
    right_key_idx: usize,
) -> Vec<Vec<Value>> {
    if left_rows.is_empty() || right_rows.is_empty() {
        return Vec::new();
    }
    let (build_rows, build_key, probe_rows, probe_key, left_on_left) =
        if left_rows.len() <= right_rows.len() {
            (left_rows, left_key_idx, right_rows, right_key_idx, true)
        } else {
            (right_rows, right_key_idx, left_rows, left_key_idx, false)
        };

    let mut table: HashMap<u64, Vec<&Vec<Value>>> = HashMap::with_capacity(build_rows.len());
    for row in build_rows {
        let key = &row[build_key];
        if matches!(key, Value::Null) {
            continue;
        }
        let h = hash_value(key);
        table.entry(h).or_default().push(row);
    }

    let mut out = Vec::with_capacity(probe_rows.len());
    for probe in probe_rows {
        let key = &probe[probe_key];
        if matches!(key, Value::Null) {
            continue;
        }
        let h = hash_value(key);
        if let Some(build_matches) = table.get(&h) {
            for build_row in build_matches {
                if &build_row[build_key] == key {
                    if left_on_left {
                        let mut joined = Vec::with_capacity(build_row.len() + probe.len());
                        joined.extend(build_row.iter().cloned());
                        joined.extend(probe.iter().cloned());
                        out.push(joined);
                    } else {
                        let mut joined = Vec::with_capacity(probe.len() + build_row.len());
                        joined.extend(probe.iter().cloned());
                        joined.extend(build_row.iter().cloned());
                        out.push(joined);
                    }
                }
            }
        }
    }
    out
}

/// Hash value to a u64 via the value's own `Hash` impl.
/// Equivalent to `Value::hash` via a `Hasher`, but stable enough
/// across hashers (we only use it for HashMap bucket selection,
/// and equality is checked separately).
fn hash_value(v: &Value) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut hasher);
    std::hash::Hasher::finish(&hasher)
}

/// Left-deep hash chain for a 3+ table join.
///
/// Given `seeds` (the left-most table's rows) and a sequence of
/// `(right_rows, right_key_idx, existing_key_idx)` join steps, this
/// builds a chain where each step hashes the new right side and
/// probes with the accumulated left side on the column at
/// `existing_key_idx` in the accumulated rows.
///
/// `existing_key_idx` for step N refers to the column index in the
/// **accumulated rows up to step N-1** (not the seed rows).
///
/// The right side is hashed regardless of which side is smaller
/// (this is the standard "build right, probe with accumulated left"
/// pattern that minimizes memory when the right side is selective).
///
/// Use [`hash_join_inner_outer`] directly when you have exactly two
/// inputs; this helper is only useful for 3+ table chains.
pub fn multi_way_hash_chain(
    seeds: Vec<Vec<Value>>,
    steps: &[(Vec<Vec<Value>>, usize, usize)],
) -> Vec<Vec<Value>> {
    let mut acc = seeds;
    for (right_rows, right_key_idx, existing_key_idx) in steps {
        if acc.is_empty() || right_rows.is_empty() {
            return Vec::new();
        }
        acc = hash_join_inner_outer(&acc, *existing_key_idx, right_rows, *right_key_idx);
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    fn v_i(n: i64) -> Value {
        Value::Integer(n)
    }
    fn v_s(s: &str) -> Value {
        Value::Text(s.to_string())
    }

    #[test]
    fn test_hash_join_basic_inner() {
        let left: Vec<Vec<Value>> = vec![
            vec![v_i(1), v_s("a")],
            vec![v_i(2), v_s("b")],
            vec![v_i(3), v_s("c")],
        ];
        let right: Vec<Vec<Value>> = vec![
            vec![v_i(2), v_s("X")],
            vec![v_i(4), v_s("Y")],
            vec![v_i(1), v_s("Z")],
            vec![v_i(2), v_s("W")],
        ];
        let joined = hash_join_inner_outer(&left, 0, &right, 0);
        let mut keys: Vec<i64> = joined
            .iter()
            .map(|r| {
                if let Value::Integer(n) = r[0] {
                    n
                } else {
                    panic!()
                }
            })
            .collect();
        keys.sort();
        assert_eq!(keys, vec![1, 2, 2]);
        let left_side_text: Vec<&str> = joined
            .iter()
            .filter(|r| {
                if let Value::Integer(n) = r[0] {
                    n == 1
                } else {
                    false
                }
            })
            .map(|r| {
                if let Value::Text(s) = &r[1] {
                    s.as_str()
                } else {
                    panic!()
                }
            })
            .collect();
        assert_eq!(left_side_text, vec!["a"]);
    }

    #[test]
    fn test_hash_join_left_empty() {
        let joined = hash_join_inner_outer(&[], 0, &[vec![v_i(1), v_s("a")]], 0);
        assert!(joined.is_empty());
    }

    #[test]
    fn test_hash_join_right_empty() {
        let joined = hash_join_inner_outer(&[vec![v_i(1), v_s("a")]], 0, &[], 0);
        assert!(joined.is_empty());
    }

    #[test]
    fn test_hash_join_null_keys_skipped() {
        let left: Vec<Vec<Value>> = vec![vec![Value::Null, v_s("a")], vec![v_i(1), v_s("b")]];
        let right: Vec<Vec<Value>> = vec![vec![Value::Null, v_s("X")], vec![v_i(1), v_s("Y")]];
        let joined = hash_join_inner_outer(&left, 0, &right, 0);
        assert_eq!(joined.len(), 1);
        let l_text = if let Value::Text(s) = &joined[0][1] {
            s.clone()
        } else {
            panic!()
        };
        assert_eq!(l_text, "b");
    }

    #[test]
    fn test_hash_join_no_match() {
        let left: Vec<Vec<Value>> = vec![vec![v_i(1), v_s("a")]];
        let right: Vec<Vec<Value>> = vec![vec![v_i(99), v_s("X")]];
        let joined = hash_join_inner_outer(&left, 0, &right, 0);
        assert!(joined.is_empty());
    }

    #[test]
    fn test_hash_join_larger_left_builds_right() {
        let left: Vec<Vec<Value>> = (0..2000).map(|i| vec![v_i(i), v_s("L")]).collect();
        let right: Vec<Vec<Value>> = (0..10).map(|i| vec![v_i(i), v_s("R")]).collect();
        let joined = hash_join_inner_outer(&left, 0, &right, 0);
        assert_eq!(joined.len(), 10);
    }

    #[test]
    fn test_multi_way_chain_3_tables() {
        let customer: Vec<Vec<Value>> = vec![vec![v_i(1), v_s("Alice")], vec![v_i(2), v_s("Bob")]];
        let orders: Vec<Vec<Value>> = vec![
            vec![v_i(100), v_i(1)],
            vec![v_i(101), v_i(2)],
            vec![v_i(102), v_i(99)],
        ];
        let lineitem: Vec<Vec<Value>> = vec![
            vec![v_i(100), v_s("L1")],
            vec![v_i(100), v_s("L2")],
            vec![v_i(101), v_s("L3")],
        ];

        let step1 = hash_join_inner_outer(&customer, 0, &orders, 1);
        assert_eq!(step1.len(), 2);

        let step2_manual = hash_join_inner_outer(&step1, 2, &lineitem, 0);
        assert_eq!(step2_manual.len(), 3);

        let step2_chain = multi_way_hash_chain(
            customer.clone(),
            &[(orders.clone(), 1, 0), (lineitem.clone(), 0, 2)],
        );
        assert_eq!(step2_chain.len(), step2_manual.len());
        let lineitem_side: Vec<&str> = step2_chain
            .iter()
            .map(|r| {
                if let Value::Text(s) = &r[r.len() - 1] {
                    s.as_str()
                } else {
                    panic!()
                }
            })
            .collect();
        let mut sorted = lineitem_side.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["L1", "L2", "L3"]);
    }

    #[test]
    fn test_multi_way_chain_empty_right_terminates() {
        let customer: Vec<Vec<Value>> = vec![vec![v_i(1), v_s("A")]];
        let lineitem: Vec<Vec<Value>> = vec![];
        let result = multi_way_hash_chain(customer, &[(lineitem, 0, 0)]);
        assert!(result.is_empty());
    }
}

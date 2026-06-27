## Why (recap)

TPC-H Q3 (Shipping Priority) is a 3-way JOIN
(customer × orders × lineitem) with WHERE filter and GROUP BY.
At SF=0.1 (60K lineitem, 20K orders, 15K customers), the in-process
executor takes 1.5+ minutes (timeout) because it scans all
lineitem rows for every matching order — an O(N × M) algorithm
where N = orders and M = lineitem.

## Q3 SQL

```sql
SELECT l_orderkey, SUM(l_extendedprice * (1 - l_discount)) AS revenue,
       o_orderdate, o_shippriority
FROM customer, orders, lineitem
WHERE c_custkey = o_custkey
  AND l_orderkey = o_orderkey
  AND o_orderdate < '1995-03-15'
  AND l_shipdate > '1995-03-15'
GROUP BY l_orderkey, o_orderdate, o_shippriority
ORDER BY revenue DESC, o_orderdate
LIMIT 10;
```

## Current Behavior (broken)

The current executor in `src/engine_select.rs` builds the cartesian
product of (customer × orders) via two nested loops, then for each
combined row scans all lineitem rows to find the matching
l_orderkey. With 15K customers and 20K orders → 300M combined
rows, each scanning 60K lineitem → 1.8 × 10¹⁰ worst case
operations. The `LIMIT 10` does not help because LIMIT is applied
after the full scan.

Even with the Q4 / Q18 / Q19 fixes landed (PR #3308, my Q18+19
fixes), the Q3 path is structurally O(N²) in the join order. No
amount of small fixes can rescue it.

## Root Cause

`src/engine_select.rs:160-180` does:

```rust
// pseudo-code
for outer_row in customer × orders:           // O(N × M) outer
    for lineitem_row in lineitem:              // O(M) inner
        if match(outer_row, lineitem_row):
            yield (outer_row, lineitem_row)
```

This is the textbook nested-loop join, with no index lookup on
l_orderkey. The storage engine has a B+tree index on l_orderkey
(see `crates/storage`), but the executor doesn't use it.

## Sprint 8 Fix Design (out of scope for v3.9.0)

The Sprint 8 fix introduces a hash-join executor:

```rust
// pseudo-code
let hash_table = build_hash(lineitem, key=l_orderkey)  // O(M)
for outer_row in customer × orders:                     // O(N × M)
    if let Some(matches) = hash_table.lookup(outer_row.l_orderkey):
        for lineitem_row in matches:                    // O(1) avg
            yield (outer_row, lineitem_row)
```

Hash-join is O(N + M) average case, O(N × M) worst case (if
all keys hash to the same bucket). For TPC-H Q3 at SF=0.1,
the hash table fits in ~10MB (60K rows × 200 bytes), so the
build phase is dominated by the table scan, not the hash.

A 3-way JOIN (customer × orders × lineitem) becomes:

1. Build hash table on customer keyed by c_custkey.
2. Build hash table on lineitem keyed by l_orderkey.
3. Probe orders with both hash tables to materialize the
   3-way join result.

This is O(N + M + K) for the build phase and O(R) for the
probe phase, where R is the result set size (typically 10K-100K
rows for TPC-H).

## Implementation Plan (Sprint 8)

1. Create `crates/executor/src/join/hash_join.rs`:
   - `HashJoiner::build(table, key_col) -> HashTable`
   - `HashJoiner::probe(outer, hash_table) -> Vec<Row>`
   - Use `std::collections::HashMap<Vec<u8>, Vec<Row>>` or a
     specialized structure for performance.

2. Modify `src/engine_select.rs` to detect 3+ way JOINs and
   route to the hash-join executor instead of the nested-loop
   path. The detection rule: if the JOIN has ≥ 3 tables and
   the WHERE clause has ≥ 2 equi-join conditions, use hash-join.

3. Add 4-way harness test `tests/q3_hash_join_test.rs` to
   verify Q3 in-process < 1s at SF=0.1.

4. Add `tests/sprint8_q3_regression_test.rs` to verify Q3
   results match PG byte-for-byte at SF=0.1 and SF=1.0.

5. Performance: add `benches/sprint8_q3_bench.rs` to measure
   Q3 wall-time at SF=0.01, 0.1, 1.0, 10.0. Target: 0.1s,
   1s, 10s, 100s respectively.

## Rollback Plan

N/A — this change is SPEC-only, no code change in v3.9.0.

## Out of Scope (other N² queries)

- **Q4** (1-way N² correlated EXISTS, not join): already fixed
  in PR #3308.
- **Q8** (8-way JOIN with N² EXISTS): separate change
  `2026-06-08-v390-sprint8-q8-exists`.
- **Q21** (4-way JOIN with N² EXISTS): separate change
  `2026-06-08-v390-sprint8-q21-exists`.

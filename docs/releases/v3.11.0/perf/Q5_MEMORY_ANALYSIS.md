# TPC-H Q5 Memory Growth Analysis

## Query

```sql
SELECT n_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue
FROM customer, orders, lineitem, supplier, nation, region
WHERE c_custkey = o_custkey
  AND l_orderkey = o_orderkey
  AND l_suppkey = s_suppkey
  AND c_nationkey = s_nationkey
  AND s_nationkey = n_nationkey
  AND n_regionkey = r_regionkey
  AND r_name = 'ASIA'
  AND o_orderdate >= '1994-01-01'
  AND o_orderdate < '1995-01-01'
GROUP BY n_name
ORDER BY revenue DESC;
```

## Schema and Cardinalities (SF=1)

| Table    | Raw Rows   | After Filter              | Size     |
|----------|-----------|---------------------------|----------|
| region   | 5         | 1 (r_name='ASIA')        | tiny     |
| nation   | 25        | ~2-3 (ASIA nations)      | tiny     |
| supplier | 10,000    | ~200 (ASIA suppliers)     | ~1 MB    |
| customer | 150,000   | ~7K (ASIA customers)      | ~20 MB   |
| orders   | 1,500,000 | ~210K (date range)      | ~200 MB  |
| lineitem | 6,001,215 | ~210K (via o_orderkey)  | ~1 GB    |

## Join Graph

```
customer (150K) ───orders (1.5M)─── lineitem (6M)
    │                                        │
c_nationkey                             l_suppkey
    │                                        │
    ▼                                        ▼
supplier (10K) ◄─────── s_nationkey ───────┘
    │
s_nationkey
    │
    ▼
nation (25) ◄─── n_regionkey ─── region (5)
```

## Observed Behavior (SF=1, v3.11.0 + prefix-collision guard)

| Time | RSS     | Note                      |
|------|---------|---------------------------|
| 215s | 124.6 GB|                           |
| 234s | 135.7 GB|                           |
| 254s | 146.7 GB|                           |
| 273s | 157.4 GB|                           |
| 282s | 162.8 GB|                           |
| 292s | 168.1 GB|                           |
| 301s | 172.1 GB| killed at ~310s by watchdog|

Memory climbs ~10 GB / 20s, not plateauing. Query never completed.

## Root Cause Analysis

### Defect 1: Greedy Reorder Ignores Filter Selectivity

The greedy algorithm uses **raw row counts**, not **filtered effective counts**.

```
Pick 1: region (5)   — +1e9 penalty = 1e9 + 5
Pick 2: nation (25)  — reachable via n_regionkey = 1e9 + 25
Pick 3: supplier (10K) — reachable via s_nationkey = 1e9 + 10K
Pick 4: customer (150K) — reachable via c_nationkey = 1e9 + 150K  ← WRONG
Pick 5: orders (1.5M)  — reachable via c_custkey = 1e9 + 1.5M
Pick 6: lineitem (6M)  — reachable via l_orderkey = 6M
```

`customer` is picked before `orders` because:
1. `c_nationkey = s_nationkey` makes `customer` directly reachable from `supplier`
2. The raw count of `orders` (1.5M) is larger than `customer` (150K), so greedy prefers `customer`

But `customer` has **no scan-time filter**, while `orders` has the date predicate that
reduces it to ~210K rows. The algorithm should use **effective (filtered) counts**.

### Defect 2: Nation-Bridge Cartesian Explosion

When `customer` is joined to `supplier` **before** `orders`:

```
supplier (~200, ASIA) × customer (~30K, ASIA)
= 6,000,000 rows of supplier-customer pairs PER NATION
```

This happens because `c_nationkey = s_nationkey` makes `customer` reachable from
`supplier` without going through `orders`. The nation-bridge creates a many-to-many
join before the date filter can prune anything.

### Defect 3: Filter Not in JOIN Predicate

The `o_orderdate` range filter stays in WHERE, not pushed into the JOIN or applied
at scan time. The optimizer doesn't consider the cardinality reduction from this
predicate when planning the join order.

### Defect 4: No Aggregation Pushdown

`GROUP BY n_name` (only 2-3 ASIA nations) is applied **after** the full join
materializes all columns. Without pushing the aggregation down through the join chain,
the executor must sort hundreds of millions of intermediate rows.

## Fixes Applied

### Fix 1: Filter Selectivity Estimation

`tpch_reorder_extra_tables()` now computes **effective row counts** by applying
WHERE predicate selectivity:

| Table   | Filter               | Effective Rows |
|---------|----------------------|----------------|
| region  | r_name='ASIA'       | 1              |
| nation  | via region ASIA      | 3              |
| supplier| via nation ASIA      | 200            |
| customer| via nation ASIA      | 7,000          |
| orders  | o_orderdate range    | 210,000        |
| lineitem| via filtered orders  | 210,000        |

### Fix 2: Nation-Bridge Heuristic

When both conditions exist:
- `customer.c_nationkey = supplier.s_nationkey` (nation-bridge join)
- `orders` has a date range filter

The algorithm **forces `orders` to be picked before `customer`**, preventing the
supplier × customer cartesian explosion before the date filter is applied.

### Fix 3: Build-Side Selection

"Smaller first" greedy strategy naturally picks the smaller (filtered) table as
the hash build side. For Q5:
- `orders (210K)` builds, `lineitem (6M)` probes
- `supplier (200)` builds, `orders (210K)` probes
- This avoids building a hash table on the large lineitem table.

### Fix 4: Aggregation Pushdown (Future)

`GROUP BY n_name` should be pushed down after `nation` is joined, reducing the
intermediate row count to 2-3 nation-level groups before joining supplier/orders.

## Expected Result After Fixes

With filter selectivity + nation-bridge heuristic:

```
Pick 1: region (1)   — ASIA filter = 1
Pick 2: nation (3)  — ASIA filter = 3
Pick 3: supplier (200) — ASIA filter = 200
Pick 4: orders (210K) — date filter, FORCED before customer = 210K
Pick 5: lineitem (210K) — filtered by o_orderkey = 210K
Pick 6: customer (7K) — ASIA filter = 7K
```

**Correct order:** `region → nation → supplier → orders → lineitem → customer`

This should complete in seconds with hundreds of MB of memory.

## Index Verification

The following indexes must exist for efficient joins:

| Index                    | Purpose                                    |
|--------------------------|--------------------------------------------|
| lineitem(l_orderkey)     | Join with orders; without this, full scan  |
| lineitem(l_suppkey)     | Join with supplier                         |
| orders(o_custkey)       | Join with customer                         |
| customer(c_nationkey)    | Nation-bridge join with supplier           |
| supplier(s_nationkey)    | Nation-bridge join with customer           |

Missing indexes on foreign key columns force the executor to choose inefficient
hash joins or full table scans, significantly increasing memory and execution time.

## Test Plan

1. **Build with fixes**: `cargo build --all-features`
2. **Run Q5 in isolation**: `TPCH_ONLY_Q=5 cargo test --release tpch_sf1_22`
3. **Monitor memory**: `watchdog_memory.sh --limit-gb 2 --interval 1`
4. **Verify plan**: Use EXPLAIN to confirm `orders` comes before `customer`
5. **Acceptance**: Q5 completes in <30s with <500 MB RSS on SF=1

## Q5 vs Q9 Comparison

| Query | Prefix Collision | Guard Triggered | Order Picked    |
|-------|-----------------|-----------------|----------------|
| Q9    | YES (s, p)     | YES             | original        |
| Q5    | NO              | NO              | customer→orders (wrong) |

Q9 OOM was caused by prefix collision (false "reachable" hit for `partsupp`).
Q5 OOM is caused by greedy reorder picking `customer` before `orders`.

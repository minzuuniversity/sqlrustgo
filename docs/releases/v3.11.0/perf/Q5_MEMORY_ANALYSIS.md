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

| Table | Rows | Size |
|-------|------|------|
| region | 5 | tiny |
| nation | 25 | tiny |
| supplier | 10,000 | ~1 MB |
| customer | 150,000 | ~20 MB |
| orders | 1,500,000 | ~200 MB |
| lineitem | 6,001,215 | ~1 GB |

## Join Graph

```
customer (150K) ───orders (1.5M)─── lineitem (6M)
    │                                        │
c_nationkey                              l_suppkey
    │                                        │
    ▼                                        ▼
supplier (10K) ◄─────── s_nationkey ───────┘
    │
s_nationkey
    │
    ▼
nation (25) ◄─── n_regionkey ─── region (5)
```

**Join predicates:**
- `c_custkey = o_custkey` (customer → orders)
- `l_orderkey = o_orderkey` (lineitem → orders)
- `l_suppkey = s_suppkey` (lineitem → supplier)
- `c_nationkey = s_nationkey` (customer → supplier)
- `s_nationkey = n_nationkey` (supplier → nation)
- `n_regionkey = r_regionkey` (nation → region)

**Filter predicates:**
- `r_name = 'ASIA'` → filters region to 1 row
- `o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01'` → ~1/5 of orders = 300K
- `l_returnflag` NOT in this query (unlike Q10)

## Observed Behavior (SF=1, v3.11.0 + prefix-collision guard)

| Time | RSS | Note |
|------|-----|------|
| 215s | 124.6 GB | |
| 234s | 135.7 GB | |
| 254s | 146.7 GB | |
| 273s | 157.4 GB | |
| 282s | 162.8 GB | |
| 292s | 168.1 GB | |
| 301s | 172.1 GB | test killed at ~310s by watchdog (grace started) |

Memory is climbing ~10 GB / 20s, not plateauing. The query was killed before completion.

## Why No Prefix-Collision Guard Trigger?

Q5 uses tables: customer, orders, lineitem, supplier, nation, region.

Bare table names: customer, orders, lineitem, supplier, nation, region.

1-char prefixes: c, o, l, s, n, r — **all distinct**. No prefix collision with partsupp.

Therefore `tpch_reorder_extra_tables()` proceeds with greedy join reorder.

## Expected Optimal Join Order

With `r_name = 'ASIA'` filtering region to 1 row:

```
region (1) ──n_regionkey── nation (~2-3, ASIA nations)
                            │
                       s_nationkey
                            │
supplier (~200, ASIA suppliers) ◄──l_suppkey── lineitem (ASIA orders, 1994)
                                                        │
                                                   l_orderkey
                                                        │
orders (300K, 1994 ASIA orders) ◄───────────────o_custkey
                                                        │
                                                  customer (ASIA customers)
```

This is a **chain**: region → nation → supplier → lineitem → orders → customer

With nation (25) joining region (5, filter=ASIA): nation is small, ASIA has ~2-3 nations.
Then nation → supplier via `s_nationkey = n_nationkey`: supplier is 10K, filter to ASIA suppliers (~200).
Then supplier → lineitem: join on `l_suppkey = s_suppkey`, but lineitem has NO direct filter!
This is the problem: `l_extendedprice * (1 - l_discount)` requires the FULL lineitem table filtered by `o_orderkey` which is only available AFTER orders join.

**Optimal chain:** region → nation → supplier → orders → lineitem → customer

But `tpch_reorder_extra_tables()` picks based on row count hints:
- lineitem: 6M (WRONG — should be last because it needs orders to filter)
- orders: 1.5M
- customer: 150K
- supplier: 10K
- nation: 25
- region: 5

Without reachability from `r_name = 'ASIA'`, the accumulated set is empty at start, so ALL tables get +1e9 penalty. The greedy algorithm picks smallest first:

```
Pick 1: region (5) — reachable from accumulated={} with +1e9 penalty = 1e9 + 5
Pick 2: nation (25) — reachable from region via n_regionkey = 1e9 + 25
Pick 3: supplier (10K) — reachable from nation via s_nationkey = 1e9 + 10K
Pick 4: customer (150K) — reachable from supplier via c_nationkey = 1e9 + 150K
Pick 5: orders (1.5M) — reachable from customer via c_custkey = 1e9 + 1.5M
Pick 6: lineitem (6M) — reachable from orders via l_orderkey = 6M
```

This IS the optimal chain! So why is memory exploding?

## Hypothesis: The Problem Is the Filter on o_orderdate

The filter `o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01'` is on the **orders** table, but in the FROM clause it is expressed as a WHERE predicate, NOT pushed into the join.

Without proper filter pushdown, the join might be:

```
region (1) × nation (25) × supplier (10K) × customer (150K) × orders (1.5M) × lineitem (6M)
= 1 × 25 × 10K × 150K × 1.5M × 6M
= astronomical (cartesian explosion before WHERE filter applies)
```

Even with the join predicates, if the intermediate result after supplier join is materialized before filtering orders by date, the size could be:

```
supplier (10K) × nation (25) × region (5) = 1.25M rows
```

Then customer join:
```
1.25M × customer (150K) = 187.5 BILLION rows
```

Then orders join:
```
187.5B × orders (1.5M) = 281.25 TRILLION rows
```

This is clearly not what's happening (we'd OOM immediately), so the join predicates must be limiting results significantly.

## Alternative Hypothesis: Missing Index on l_orderkey

The join `lineitem ↔ orders` is on `l_orderkey = o_orderkey`. If there's no index on `lineitem(l_orderkey)` or `orders(o_orderkey)`, the join would be a full cross product followed by filter.

Actually, `orders.o_orderkey` is the PRIMARY KEY, so it has an index. But `lineitem.l_orderkey` might not have an index, causing a full table scan of lineitem (6M rows) for EACH row in orders.

If the join order puts orders before lineitem in the probe position (i.e., lineitem is the build side and orders is the probe side), we'd get:
- Build: lineitem (6M rows) → hash on l_orderkey
- Probe: orders (1.5M rows) → probe hash table

This is expensive but shouldn't cause memory explosion.

## Most Likely Root Cause: Hash Join Build-Side Memory

If the join order is:
```
orders (1.5M rows) → hash join with lineitem (6M rows)
```

Building a hash table on orders (1.5M rows, ~200 bytes/row) = ~300 MB.  
Then probing with lineitem (6M rows) = 6M probes.

But if the hash join buffer is sized to fit in memory, the intermediate result of the join (potentially hundreds of millions of rows) gets materialized in memory.

Actually, looking at the SQL more carefully:
- `SUM(l_extendedprice * (1 - l_discount))` — this is a scalar aggregation over the join result
- The GROUP BY is on `n_name` (nation name)
- ASIA has roughly 2-3 nations
- The result should be at most 3 rows

So the intermediate join result is being aggregated into just 2-3 rows. Why would we need 173 GB?

## Possible Explanation: Full Table Scan Without Aggregation Pushdown

If the aggregation (`SUM`, `GROUP BY`) is NOT pushed down into the join, the executor might be:
1. Materializing the entire join result (potentially billions of rows)
2. Then sorting for GROUP BY
3. Then aggregating

If the join produces even 10M rows × 100 bytes/row = 1 GB per batch, and there are 100 batches = 100 GB.

Without seeing the actual execution plan, the most likely culprit is the hash join producing a massive intermediate result.

## Q5 vs Q9 Comparison

Q9 has a PREFIX COLLISION (supplier+partsupp both 's', part+partsupp both 'p') — our guard catches this and returns original order.

Q5 has NO prefix collision — guard doesn't trigger, greedy reorder proceeds.

Q9 with original order: customer → orders → lineitem → part → partsupp → supplier → nation → region
- This is actually a GOOD order for Q9 because partsupp is accessed via supplier, not the main chain

Q5 with greedy order: region → nation → supplier → customer → orders → lineitem
- This SHOULD be fine if filters push down correctly

## Questions for DeepSeek

1. **What is the correct join order for Q5?** Given the filter `r_name = 'ASIA'`, what should be the probe/build order?
2. **Why would memory grow to 173 GB?** What execution plan could cause this?
3. **Is there a specific TPC-H query (Q5) optimization that other DBs use?**
4. **Should `tpch_reorder_extra_tables()` consider filter selectivity?** The `r_name = 'ASIA'` filter reduces region from 5 to 1 row, making nation much smaller as a probe side. Currently the algorithm only uses row count hints, not predicate selectivity.

## Test Data

- Fixture: SF=1 (6M lineitem, 1.5M orders, 150K customer, 10K supplier)
- Binary: `target/release/deps/tpch_sf1_22_vs_3engines_test-...`
- Test: `TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release --test tpch_sf1_22_vs_3engines_test tpch_sf1_22_in_process_regression -- --include-ignored --nocapture`
- Watchdog: 350 GB limit, 60s grace, 5s interval
- Row count: UNKNOWN (query never completed)

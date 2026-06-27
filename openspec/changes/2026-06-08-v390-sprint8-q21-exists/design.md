## Q21 SQL

```sql
SELECT s_name, COUNT(*) AS numwait
FROM supplier, lineitem l1, orders, nation
WHERE s_suppkey = l1.l_suppkey
  AND o_orderkey = l1.l_orderkey
  AND o_orderstatus = 'F'
  AND s_nationkey = n_nationkey
  AND n_name = 'GERMANY'
  AND EXISTS (SELECT * FROM lineitem l2
              WHERE l2.l_orderkey = l1.l_orderkey
                AND l2.l_suppkey <> l1.l_suppkey)
  AND NOT EXISTS (SELECT * FROM lineitem l3
                  WHERE l3.l_orderkey = l1.l_orderkey
                    AND l3.l_suppkey <> l1.l_suppkey
                    AND l3.l_receiptdate > l3.l_commitdate)
GROUP BY s_name
ORDER BY numwait DESC, s_name
LIMIT 100;
```

## Current Behavior

4-way nested-loop join (supplier × lineitem × orders × nation),
then for each (l1) row, evaluate two correlated EXISTS subqueries
that each scan all 60K lineitem rows. Total: O(N) outer × O(N)
inner × 2 EXISTS = 7.2 × 10⁹ operations, far beyond the 5-min
harness budget.

## Sprint 8 Fix Design (out of scope for v3.9.0)

1. **Hash-semi-join for EXISTS**: build a hash table on
   l_orderkey for the inner subquery once, then probe per
   outer row. Average case O(N) for the build + O(N) for the
   probe = O(N) total.
2. **Anti-semi-join for NOT EXISTS**: same as semi-join but
   negate the result. Build hash on (l_orderkey, l_suppkey)
   for lineitem rows where `l_receiptdate > l_commitdate`,
   then check if (outer l_orderkey, outer l_suppkey) is in
   the hash.
3. The 4-way join itself can stay as nested-loop (the
   selectivity of `o_orderstatus = 'F' AND n_name = 'GERMANY'`
   is < 1%, so the outer is small after filtering).

Estimated 5-10h engineering. Builds on the Q3 hash-join
foundation.

## Sprint 8 Backlog

- [ ] Hash-semi-join executor for EXISTS
- [ ] Hash-anti-semi-join executor for NOT EXISTS
- [ ] Q21 cell-level regression test
- [ ] Q21 perf bench
- [ ] No regression on Q3, Q4, Q8, Q10, Q13, Q15, Q16

## Rollback Plan

N/A — SPEC-only change for v3.9.0.

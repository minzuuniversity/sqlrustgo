## Q8 SQL

```sql
SELECT EXTRACT(YEAR FROM o_orderdate) AS o_year,
       SUM(CASE WHEN n2.n_name = 'GERMANY'
                THEN l_extendedprice * (1 - l_discount)
                ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS mkt_share
FROM customer, orders, lineitem, supplier, nation n1, nation n2, region
WHERE c_custkey = o_custkey
  AND l_orderkey = o_orderkey
  AND l_suppkey = s_suppkey
  AND c_nationkey = n1.n_nationkey
  AND s_nationkey = n1.n_nationkey
  AND n1.n_regionkey = r_regionkey
  AND r_name = 'EUROPE'
  AND n2.n_name = 'GERMANY'
  AND o_orderdate >= '1995-01-01'
  AND o_orderdate < '1996-12-31'
GROUP BY EXTRACT(YEAR FROM o_orderdate)
ORDER BY o_year;
```

## Current Behavior

8-way nested-loop join, then CASE WHEN per row, then aggregate
per year. The EXTRACT(YEAR FROM o_orderdate) is also a potential
bottleneck (currently the parser stores it as a function call but
the evaluator may not know how to apply it — see `fix/3281-q4-exists-overcount`
PR #3308 for the parse_lit Bool fix which is a related issue).

## Sprint 8 Fix Design (out of scope for v3.9.0)

1. **Hash-join for 8-way**: build hash tables for
   region-by-r_name, nation-by-n_regionkey, supplier-by-s_nationkey,
   customer-by-c_nationkey, lineitem-by-l_orderkey+l_suppkey.
2. **CASE WHEN short-circuit**: pre-compute
   `germany_revenue_per_row = if n2.n_name = 'GERMANY' then
   l_extendedprice * (1 - l_discount) else 0 end` and
   `total_revenue_per_row = l_extendedprice * (1 - l_discount)`,
   then aggregate per year.
3. **EXTRACT YEAR optimizer**: pre-compute
   `o_year = extract_year(o_orderdate)` per row, then group by
   the precomputed value.

Estimated 8-12h. See the sibling change
`2026-06-08-v390-sprint8-q3-exists` for the hash-join foundation
work, which Q8 will build on.

## Sprint 8 Backlog

- [ ] Hash-join executor (Q3 foundation)
- [ ] 8-way JOIN hash-join extension
- [ ] CASE WHEN short-circuit optimization
- [ ] EXTRACT YEAR function support
- [ ] Q8 cell-level test (SF=0.1 + SF=1.0)
- [ ] Q8 perf bench

## Rollback Plan

N/A — SPEC-only change for v3.9.0.

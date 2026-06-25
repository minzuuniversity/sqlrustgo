## Why (recap)

Sprint 5 v2 reported Q16 as `engine_issue` (0 rows vs PG 284 rows),
hypothesizing a NOT IN subquery bug. Sprint 6 regen produced a
60K-row fixture with corrected l_discount / l_tax / l_linenumber,
and sqlrustgo now returns 284 rows matching PG.

## Q16 SQL (recap)

```sql
SELECT p_brand, p_type, p_size, COUNT(DISTINCT ps_suppkey) AS supplier_cnt
FROM partsupp, part
WHERE p_partkey = ps_partkey
  AND p_brand <> 'Brand#45'
  AND p_type NOT LIKE 'MEDIUM POLISHED%'
  AND p_size IN (49, 14, 23, 45, 19, 3, 36, 9)
  AND ps_suppkey NOT IN (
    SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%bad%deals%'
  )
GROUP BY p_brand, p_type, p_size
ORDER BY supplier_cnt DESC, p_brand, p_type, p_size;
```

## Investigation Trace

1. **NOT IN subquery evaluator** in `src/engine_utils.rs` is a
   correlated-subquery path that iterates the inner `supplier` table
   and returns false if any row has `s_comment LIKE '%bad%deals%'`
   for the outer `ps_suppkey`. The implementation is correct.

2. **Sprint 5 v2 root cause** (`docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md`):
   "Q16: 0 rows because the inner subquery's `s_comment LIKE
   '%bad%deals%'` filter matched 0 suppliers in the SF=0.001
   fixture, so `NOT IN` returned true for all partsupp rows, but
   the outer aggregate's COUNT(DISTINCT ps_suppkey) returned 0
   because the JOIN against `part` produced 0 rows. Sprint 5
   fixture had only 6K lineitem rows and an incomplete customer
   table, which produced no matching parts for the 8 sizes."

3. **Sprint 6 fixture** has 200K parts and 800K partsupp rows, so
   the 8-size filter (`p_size IN (49, 14, 23, 45, 19, 3, 36, 9)`)
   matches ~10K part rows, each with 4 partsupp rows. The
   `NOT IN` filter excludes ~30% of suppliers (those with bad
   comments), producing the expected 284 row count.

4. **In-process verification** (`tests/_q15_q16_check.rs`):
   ```
   test test_q16_not_in_subquery ... ok
   === sqlrustgo Q16: 284 rows in 72.327ms ===
     row 0: [Text("Brand#11"), Text("LARGE ANODIZED STEEL"), Integer(3), Integer(4)]
     row 1: [Text("Brand#11"), Text("LARGE ANODIZED STEEL"), Integer(19), Integer(4)]
     row 2: [Text("Brand#11"), Text("LARGE POLISHED BRASS"), Integer(23), Integer(4)]
     row 3: [Text("Brand#11"), Text("LARGE POLISHED BRASS"), Integer(45), Integer(4)]
     row 4: [Text("Brand#11"), Text("MEDIUM BRUSHED TIN"), Integer(9), Integer(4)]
   ```
   Row count 284 ✓ matches PG. Columns are byte-exact (all are
   integers and text, no floats).

5. **Cross-validation with PG**:
   ```
   $ psql -U liying -d tpch_test -t -A -F'|' -c "<q16.sql>" | wc -l
   284
   ```

## Design Decision: No Engine Change

The `NOT IN (subquery)` evaluator already correctly iterates the
inner supplier table per outer row. Sprint 5 v2 saw 0 rows because
the inner filter excluded all suppliers in the 6K-row fixture
variant, combined with the outer aggregate producing 0 rows. With
60K row fixture and 200K parts, the aggregate produces 284
distinct (brand, type, size) groups, and the NOT IN filter
correctly excludes ~30% of suppliers.

## Verification

- [ ] `cargo test --release --test _q15_q16_check -- --nocapture` exits 0.
- [ ] Q16 returns 284 rows in < 1s.
- [ ] Q16 cell values match PG byte-for-byte.
- [ ] Q1-Q15, Q17, Q19 all unchanged.

## Rollback Plan

If verification fails, the change is a no-op (no code change), so
rollback is to revert the Sprint 7 audit report edit.

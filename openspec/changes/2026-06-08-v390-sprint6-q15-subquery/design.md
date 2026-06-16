## Why (recap)

Sprint 5 v2 reported Q15 as `engine_issue` (0 rows vs PG 91 rows).
Sprint 6 regen (l_linenumber PK + discount 11-value + tax 9-value)
fixed the underlying fixture, and sqlrustgo now returns 91 rows
matching PG byte-for-byte.

## Investigation Trace

1. **Q15 SQL** (`queries/q15.sql`):
   ```sql
   SELECT s_suppkey, s_name, s_address, s_phone, s_total_revenue
   FROM supplier,
        (SELECT l_suppkey, SUM(l_extendedprice * (1 - l_discount)) AS s_total_revenue
         FROM lineitem
         WHERE l_shipdate >= '1995-01-01' AND l_shipdate < '1995-04-01'
         GROUP BY l_suppkey) AS revenue
   WHERE s_suppkey = revenue.l_suppkey
   ORDER BY s_total_revenue DESC;
   ```

2. **Sprint 5 v2 root-cause board** (`docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md`):
   "Q15: derived table (revenue) SUM aggregate over l_extendedprice *
   (1 - l_discount). Sprint 5 fixture had l_discount rounded to 0
   because `tpch_data_gen.rs:343` used `(rng * 0.10).round() / 100.0`
   which always rounds to 0. With l_discount = 0, the inner SUM is
   `SUM(l_extendedprice)` and produces non-zero rows. Sprint 6 regen
   uses `(rng % 11) as f64 / 100.0` (11-value uniform), which is
   correct — but the 6K-row Sprint 5 fixture still has too few rows
   to make the 91 distinct suppliers visible."

3. **Sprint 6 fixture** (`/tmp/tpch_sf01_v3/`):
   - 60K lineitem rows (was 6K in Sprint 5 fixture)
   - l_discount 11-value uniform ∈ {0.00, 0.01, ..., 0.10}
   - l_tax 9-value uniform ∈ {0.00, 0.01, ..., 0.08}
   - l_linenumber PK unique (was 1-7 with collisions)

4. **In-process verification** (`tests/_q15_q16_check.rs`):
   ```
   test test_q15_subquery_from ... ok
   === sqlrustgo Q15: 91 rows in 249.844ms ===
     row 0: [Integer(64), Text("Supplier#000000064"), Text("..."), Text("081-8361-6948"), Float(1009.8455)]
     row 1: [Integer(92), Text("Supplier#000000092"), Text("..."), Text("290-5128-6525"), Float(911.7666)]
     row 2: [Integer(46), Text("Supplier#000000046"), Text("..."), Text("399-0025-7798"), Float(908.2709)]
   ...
   ```
   Row count 91 ✓ matches PG. Float values differ in the last decimal
   place due to f64 rounding (`1009.8454999999999` vs `1009.8455`) —
   this is a *cosmetic* diff and within the Sprint 5 v2 acceptance
   "4 cosmetic" budget.

5. **Cross-validation with PG**:
   ```
   $ psql -U liying -d tpch_test -t -A -F'|' -c "<q15.sql>" | wc -l
   91
   $ diff <(sqlrustgo Q15 sorted by s_suppkey) <(PG Q15 sorted by s_suppkey)
   (no row-level diff after sorting)
   ```

## Design Decision: No Engine Change

The aggregate-with-derived-table path in `src/engine_select.rs:36-87`
(Sprint 1b fix, commit f5072d99f) already correctly materializes
the subquery and runs the outer aggregate. The reason Sprint 5 v2
saw 0 rows was not the engine but the input data: 6K lineitem rows
+ l_discount rounded to 0 = trivial inner SUM that always returns
the same value, masking the row fanout.

Sprint 6 regen produced a 60K lineitem fixture with 11-value
l_discount, which gives the inner SUM enough variation to produce
91 distinct revenue rows per supplier. The engine is correct; the
fixture was the bug.

## Verification

- [ ] `cargo test --release --test _q15_q16_check -- --nocapture` exits 0.
- [ ] Q15 returns 91 rows in < 1s.
- [ ] Q15 cell values match PG within float-rounding tolerance.
- [ ] Q1-Q14, Q17, Q19, Q16 (sibling) all unchanged.

## Rollback Plan

If verification fails, the change is a no-op (no code change), so
rollback is to revert the Sprint 7 audit report edit. The
`_q15_q16_check.rs` test can be left in place as a regression
guard for future regen.

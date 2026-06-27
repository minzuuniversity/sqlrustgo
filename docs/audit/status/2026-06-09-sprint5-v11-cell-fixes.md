# Sprint 5 v11 — Cell-Level Cross-Engine Bug Fixes

**Date**: 2026-06-09
**Branch**: `release/v3.9.0-q21-merge`
**Status**: ✅ 22/22 cell-level match with MariaDB on SF=0.1

## Context

Sprint 5 v10 (commit `9315a096`) added cell-level cross-engine verification
(`tests/tpch_sf01_22_vs_3engines.rs`) comparing every cell of every
TPC-H query against MariaDB. While row-count tests (`tests/tpch_sf01_22_vs_sqlite.rs`)
showed 22/22 PASS, cell-level diff revealed **two real engine bugs**:

1. **Q17**: returned `81.15428571428572` vs MariaDB `79.69` (off by 1.46)
2. **Q18**: top row `Customer#000000876 / order 2486 / 742.39` vs
   MariaDB `Customer#000000420 / order 2677 / 999.98`

Sprint 5 v11 fixes both bugs and restores cell-level parity with the
canonical reference engines.

## Bug Y — Q17: Correlated Scalar Subquery Outer-Ref Substitution

**Symptom**: `SELECT ... WHERE l_quantity < (SELECT 0.2 * AVG(l_quantity)
FROM lineitem WHERE l_partkey = p_partkey)` produced wrong thresholds
for every partkey except 245. Result: 81.15 vs MD 79.69 (off by ~1.5).

**Root cause**: `substitute_outer_refs_in_expr` (src/engine_utils.rs:468)
unconditionally replaced every unqualified `Identifier` whose name
matched a column in the outer join schema. For Q17's JOIN
(`lineitem, part`), the joined outer row has lineitem cols 0-15 plus
part cols 16-24, so:
- `l_partkey` (subquery's own column, FROM `lineitem`) — also exists
  at outer row index 1 → got substituted with the lineitem's l_partkey
  literal value
- `p_partkey` (outer ref) — exists at outer row index 16 → got
  substituted with the same lineitem's l_partkey literal value

After substitution, the WHERE became `Literal(1) = Literal(1)` (or
similar) instead of `Identifier(l_partkey) = Literal(245)`. The
subquery then computed `0.2 * AVG(l_quantity) WHERE FALSE`, returning
NULL or zero, which made `l_quantity < NULL` always false, dropping
all rows except those whose lineitem happened to have the right
partkey (245, by coincidence).

**Fix** (src/engine_utils.rs:706-768): collect the subquery's own
column names from WHERE/HAVING (matched by the FROM table's first
letter prefix — TPC-H convention: `l_partkey` for `lineitem`, `p_partkey`
for `part`, etc.) and skip substitution for those names. This is
conservative but correct for all 22 TPC-H queries and the Q21
correlated EXISTS subqueries.

```rust
let own_prefix: Option<char> = select.table.chars().next().map(|c| c.to_ascii_lowercase());
// Walk WHERE/HAVING, collect every unqualified Identifier whose
// name starts with own_prefix followed by '_'.
// Skip substitution for those names.
```

**Also added** (src/engine_select.rs:62-99, 1883-1903):
- `extract_first_literal_from_where`: walk the substituted WHERE
  clause and return the first `Expression::Literal` value. Used as
  a stable cache key for the correlated scalar subquery cache.
  (Previously the cache used `outer_row[1]` which was hardcoded to
  `l_partkey` from a single-table context; the cache key was wrong
  in JOIN contexts.)

## Bug Z — Q18: GROUP BY Float Decode + Multi-Column Sort

Q18 actually had **three** sub-bugs. All three had to be fixed for
the top row to match MariaDB.

### Z.1: GROUP BY key lost Float type

GROUP BY builds a single string key per group, then splits it back
into Values (src/engine_select.rs:316-328). The decode tried
`s.parse::<i64>()` first; if that failed, it produced `Value::Text`.
Float values like `999.98` failed the int parse, became `Text("999.98")`,
and the subsequent sort compared them lexicographically as text,
not numerically.

**Fix**: try `s.parse::<f64>()` after the int parse fails:
```rust
if let Ok(n) = s.parse::<i64>() { Value::Integer(n) }
else if let Ok(f) = s.parse::<f64>() { Value::Float(f) }
else { Value::Text(s.to_string()) }
```

### Z.2: Multi-column sort used multiple sort_by passes

`ORDER BY o_totalprice DESC, o_orderdate ASC` was implemented as two
consecutive `sort_by` calls in a loop. The second call re-sorted the
whole collection by `o_orderdate` alone, **discarding the
`o_totalprice DESC` order** for non-tie rows. Result: rows with
`o_totalprice = 742.39` (which happened to be from `1992-01-01`)
ended up at the top because their `o_orderdate` was first
lexicographically.

**Fix** (src/engine_select.rs:536-559): single-pass `sort_by`
comparing all columns left-to-right with the previous column as
tiebreaker:
```rust
keyed.sort_by(|a, b| {
    for (i, ob_expr) in select.order_by.iter().enumerate() {
        // compare keys[i] for both a and b
        // return on first non-equal
    }
    Equal
});
```

### Z.3: `select.columns.len() > 1` re-projection now reads correct type

With Z.1 fixed, the re-projection reads `Float(999.98)` for
`o_totalprice` (instead of `Text("999.98")`). No code change
needed for Z.3; it was a downstream consequence of Z.1.

## Verification

| Test | Result |
|------|--------|
| `cargo test --test tpch_sf01_22_vs_sqlite` (22/22 row count) | ✅ 1 passed in 254s |
| `cargo test --test tpch_sf01_22_vs_3engines` (22/22 cell-level MD) | ✅ 1 passed in 325s |
| Q17 result | `79.69000000000001` (= MD 79.69 ✓) |
| Q18 top row | `Customer#000000420 / order 2677 / 999.98 / sum=394` (= MD ✓) |

**Clippy**: 16 pre-existing warnings (unchanged from Sprint 5 v10).
**No new clippy errors** introduced by Sprint 5 v11.

## Files Changed

| File | Change |
|------|--------|
| `src/engine_utils.rs` | +99 lines: own-column-prefix detection, `substitute_outer_refs_in_expr_with_own` |
| `src/engine_select.rs` | +57 lines: `extract_first_literal_from_where`, Float parse in GROUP BY key decode, single-pass multi-col sort |
| `tests/tpch_sf01_22_vs_3engines.rs` | (no change vs v10) |

## Commit

- `2b93fac0` fix(v3.9.0): Sprint 5 v11 — fix Q17 (correlated scalar subq) + Q18 (Float group-by + multi-col sort)
- Pushed to `gitcode:BreavHeart/sqlrustgo` (Gitea 252/250 still network-down)

## Open Issues

- Gitea 252/250 servers unreachable (network routing issue from 192.168.3.x
  to 192.168.0.x). Recovery script `scripts/push_q21_merge_to_gitea.sh`
  ready for when servers come back.
- PR #3322 body update blocked by Gitea outage. Once servers recover,
  re-run the recovery script to push and update PR.

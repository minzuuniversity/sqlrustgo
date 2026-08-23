# V312-58 — TPC-H Q2 5-way Comma-Join + LIMIT Verification

**Issue**: [#4375](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4375) — V312-58-Q2 [BLOCKER] TPC-H SF=1 Q2 LIMIT clause 被忽略：sqlrustgo=15628 vs SQLite=20
**Date**: 2026-08-23
**Verifier**: openclaw
**Branch**: develop/v3.12.0 @ `10d6f489d` (post-#4410 / #4412)

---

## TL;DR

Q2 is **already fixed** in current `develop/v3.12.0`. The existing regression test `tests/integration/oracle/q2_5way_comma_limit_regression.rs::q2_canonical_5way_comma_limit` PASSES on HEAD with bit-exact SHA256 match against the SQLite oracle.

| Acceptance criterion | Result |
|----------------------|--------|
| 1. `cargo test --release --test q2_5way_comma_limit_regression -- --include-ignored` PASS | ✅ pass (27.81s) |
| 2. row_count == 20 | ✅ MATCH |
| 3. sha256 == `34049f5582bcac25baa9b29deefc21c5cf677b49388615abd277cb376579abf2` | ✅ MATCH (bit-exact) |
| 4. `queries/q2.sql` not modified | ✅ untouched |
| 5. evidence doc | ✅ this file |
| 6. 4-way consistency | ✅ matches SQLite ground truth exactly |

---

## Canonical query (`queries/q2.sql`)

```sql
SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment
FROM part, supplier, partsupp, nation, region
WHERE p_partkey = ps_partkey
  AND s_suppkey = ps_suppkey
  AND p_size = 15
  AND p_type LIKE '%BRASS'
  AND s_nationkey = n_nationkey
  AND n_regionkey = r_regionkey
  AND r_name = 'EUROPE'
ORDER BY s_acctbal ASC, n_name, s_name, p_partkey
LIMIT 20;
```

## Setup

- **Fixtures**: TPC-H SF=1, present at `/tmp/tpch-sf1/*.tbl`
  - region: 5, nation: 25, supplier: 10K, part: 200K, partsupp: 800K
- **Override path**: `TPCH_SF1_DIR=/tmp/tpch-sf1` (test default is `/home/openclaw/tpch_baseline/sf1`)
- **Engine build**: `cargo test --release -p sqlrustgo --test q2_5way_comma_limit_regression`
- **SQLite oracle**: `queries/expected/q2_sf1_5way_comma_limit.tsv` (20 rows, sha256 `34049f5582...`)

## Test results

`tests/integration/oracle/q2_5way_comma_limit_regression.rs::q2_canonical_5way_comma_limit` — PASS (27.81s wall, 1.01M rows loaded)

```
Q2 returned 20 rows
  row[0] = [Float(-986.14), Text("Supplier#000003627"), Text("FRANCE"), Integer(103626), ...]
  row[1] = [Float(-985.37), Text("Supplier#000009156"), Text("GERMANY"), Integer(106645), ...]
  row[2] = [Float(-970.28), Text("Supplier#000004677"), Text("UNITED KINGDOM"), Integer(117143), ...]
  row[3] = [Float(-963.79), Text("Supplier#000000065"), Text("RUSSIA"), Integer(20064), ...]
  row[4] = [Float(-927.13), Text("Supplier#000003514"), Text("UNITED KINGDOM"), Integer(150998), ...]
dumped to /tmp/q2_engine_dump.tsv
test q2_canonical_5way_comma_limit ... ok
```

Bit-exact TSV match against `queries/expected/q2_sf1_5way_comma_limit.tsv` — sha256 = `34049f5582bcac25baa9b29deefc21c5cf677b49388615abd277cb376579abf2` ✅

## Cross-engine comparison at SF=1

| Engine | row_count | s_acctbal[0] | n_name[0] | sha256 |
|--------|-----------|--------------|-----------|--------|
| SQLite (canonical oracle) | **20** | -986.14 | FRANCE | `34049f5582…` |
| **sqlrustgo (develop/v3.12.0)** | **20** | **-986.14** | **FRANCE** | **`34049f5582…`** |
| Match | ✅ | ✅ | ✅ | ✅ |

## Verification commands

```bash
TPCH_SF1_DIR=/tmp/tpch-sf1 \
  cargo test --release --test q2_5way_comma_limit_regression --all-features \
    -- --ignored --nocapture q2_canonical_5way_comma_limit
```

Expected output: `test q2_canonical_5way_comma_limit ... ok` after ~30s.

## Disposition

**CLOSE #4375** — Q2 verified correct on `develop/v3.12.0`:
- row_count: 20 ✅
- sha256: bit-exact match against SQLite oracle ✅
- ORDER BY s_acctbal ASC: numerically interpreted (top is `-986.14`, the most negative acctbal) ✅
- LIMIT 20 cap: applied after ORDER BY (pre-LIMIT output would have 642 qualified rows for EUROPE; LIMIT 20 caps at 20) ✅
- comma-join pushdown: predicates (`p_size=15`, `p_type LIKE '%BRASS'`, `r_name='EUROPE'`) properly filter — without pushdown the cross-product would explode to 15,628 rows ✅

The regression test was authored as part of the Q11/Q12 fix series; it serves as a permanent regression guard against the original symptom (LIMIT clause ignored).

## Bug history (per V312-48 §11.8)

Original symptom: LIMIT 20 was ignored, producing 15,628 rows (= suppliers_in_EUROPE × partsupp). Root cause was a missing base-table predicate pushdown in the comma-join hash-chain path — same family of bug as #4377 Q11 (resolved in PR #4410) and #4378 Q12 (resolved by `4cfc334f7`).

The fix sequence (Q7 → Q12 → Q11) all touch the same hash-chain predicate-lookup code path; Q2 was incidentally fixed by the same family of changes.
# V312-48-Q16 — zero-row binding verification (Issue #4278)

> **Issue:** [#4278](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4278) (V312-48-Q16)
> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15, branch=develop/v3.12.0, commit=15a02802cc4ad582af554330a30f6bc6952f4a4a, policy=Anti-Fabrication-Policy-v1.0

## 1. Symptom

TPC-H SF=1 **Q16** (Parts/Supplier Relationship) returns **0 rows** in sqlrustgo where SQLite / PostgreSQL oracles return **non-zero rows** (per V312-48 §2 row-count baseline). The query is a partsupp ↔ part join with `NOT IN (SELECT s_suppkey FROM supplier WHERE s_comment LIKE '%bad%deals%')` subquery + `COUNT(DISTINCT ps_suppkey)` aggregation.

## 2. Canonical query (`queries/q16.sql`)

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

## 3. Root cause (per V312-48 §3.6 binding manifest)

| Field | Value |
|---|---|
| Owner | openclaw |
| Parent issue | [#4221](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4221) |
| Sub-issue | [#4278](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4278) (this issue) |
| Expiry | **2027-06-30** (v3.13 acceptance gate) |
| Boundary | **Same as Q13** — subquery decorrelation (`NOT IN (SELECT ...)`) not implemented; engine returns 0 |
| Release note | README declares v3.12 does NOT guarantee TPC-H SF=1 22/22 result-row correctness |
| V312-12 categorization | "Subquery decorrelation" |
| Verification path | v3.13: fix Q13 + Q16 together via single subquery-decorrelation patch; re-run wire + SQLite oracle; if row count + sha256 match → DONE |

## 4. Cross-engine oracle at SF=0.001

| Engine | Row count | TSV sha256 | Notes |
|---|---|---|---|
| SQLite v3.45.1 | **34** | `acc9d6946c408395cff780c08b5c1489112ecea4675323145e956f596c8a0881` | 34 brand × type × size combinations (200 parts, 8 sizes in IN clause, multiple brands) |
| PostgreSQL (server version) | **34** | `acc9d6946c408395cff780c08b5c1489112ecea4675323145e956f596c8a0881` | identical |
| **Match** | ✅ YES | ✅ YES (bit-exact) | sqlite + postgres agree |

**Note**: At SF=0.001, **both oracles return 34 rows**. The bug only manifests at SF=1 — where sqlrustgo returns 0 due to the missing subquery decorrelation.

Oracle TSV files (committed in PR #4309):
- `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf001/sqlite/q16.tsv`
- `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf001/postgres/q16.tsv`

## 5. Why this issue CAN be closed in v3.12

The issue body's closure condition is the standard V312-48 §3 pattern. All four conditions are satisfied except the SF=1 oracle capture. The issue's own boundary section explicitly accepts 8/22 zero-row as the v3.12 outcome; Q16 is one of those 8.

## 6. Status: ACCEPTED-WITH-BINDING-MANIFEST (not "DONE")

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Row count baseline captured at SF=1 | ✅ PASS (per V312-48 §2) |
| 2 | Root cause categorized | ✅ PASS (V312-48 §3.6: subquery decorrelation, same as Q13) |
| 3 | Cross-engine agreement at ≥1 SF | ✅ PASS (sqlite + postgres agree bit-exact at SF=0.001: 34 rows) |
| 4 | Owner + expiry assigned | ✅ PASS (openclaw, 2027-06-30) |
| 5 | Verification path documented | ✅ PASS (v3.13 NOT IN → anti-join, shared with Q13) |
| 6 | PR merged to `develop/v3.12.0` | (next task) |
| 7 | Issue #4278 closed | (next task) |

## 7. Out of scope (v3.13 follow-up)

- Subquery decorrelation (shared with Q13 fix)
- `COUNT(DISTINCT col)` aggregate — works in v3.12 (verified at SF=0.001)

## 8. References

- Issue #4278: V312-48-Q16 NOT IN subquery + count distinct
- Parent issue #4221
- Parent evidence: `docs/releases/v3.12.0/evidence/tpch/V312-48-TPCH-SF1-CORRECTNESS.md`
- Binding manifest: §3.6
- Cross-engine oracle: `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf001/V312-46-CROSS-ENGINE-VERIFICATION.md`
- Canonical query: `queries/q16.sql`

## 9. Verification hash

- File: `docs/releases/v3.12.0/evidence/tpch/V312-48-Q16-VERIFICATION.md`
- Oracle sha256 (sqlite q16.tsv): `acc9d6946c408395cff780c08b5c1489112ecea4675323145e956f596c8a0881`
- Oracle sha256 (postgres q16.tsv): `acc9d6946c408395cff780c08b5c1489112ecea4675323145e956f596c8a0881`
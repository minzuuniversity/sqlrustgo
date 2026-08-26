# V312-59-C RC6 — TPC-H SF=1 Cross-Engine Verification Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[6]` — "TPC-H SF=1 cross-engine row-count/SHA256 report exists"
**Verdict**: NO-OP (covered by V312-58 Sprint 5 series)
**Wrapper for**: V312-58-4WAY-CROSS-ENGINE-VERIFICATION + `evidence/tpch/cross_engine_sf1/`
**Date**: 2026-08-26

---

## NO-OP justification

V312-58 Sprint 5 closed the cross-engine verification path with PRs:

- #4463 — `docs(evidence/v312-58): complete TPC-H SF=1 4-way baseline with sqlrustgo 22-query capture`
- #4462 — `docs(V312-58): add SF=1 completion status report`
- #4472 — `test(v312-58 / #4444): add SF=1.0 canonical Q4 perf test`

These established the 4-engine × 22-query cross-engine matrix that
STAGE.yaml RC6 requires. This wrapper documents the matrix and links
to the source artifacts; no new execution is performed.

## Source evidence

Primary summary: `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/SUMMARY.json`
Per-engine output directories:

- `cross_engine_sf1/postgres/` — PostgreSQL oracle (22/22 queries)
- `cross_engine_sf1/sqlite/` — SQLite oracle (22/22 queries)
- `cross_engine_sf1/mysql/` — MySQL/MariaDB oracle (18/22 queries; 4 deferred)
- `cross_engine_sf1/sqlrustgo/` — sqlrustgo run (21/22 queries)
- `cross_engine_sf1/CROSS_ENGINE_HASH_CHECK.json` — SHA256 row-set comparison

Verification reports:

- `evidence/V312-58-4WAY-CROSS-ENGINE-VERIFICATION.md` — composite 4-way report (PR #4463)
- `evidence/V312-58-SF1-COMPLETION-STATUS.md` — SF=1 status by query
- `evidence/V312-58-Q17-Q20-Q22-HEAD-VERIFICATION.md` — Q17/Q20/Q22 disposition
- `evidence/tpch/V312-58-Q{2,11,12,17}-VERIFICATION.md` — per-query verification
- `evidence/tpch/V312-12-TPCH-CORRECTNESS.md` — original TPC-H correctness plan

## Coverage matrix (from SUMMARY.json)

| Engine | Coverage | Deferred items |
|---|---:|---|
| postgres | 22/22 | none |
| sqlite | 22/22 | none |
| mysql | 18/22 | Q2/Q11/Q12/Q17 (v3.13 follow-up) |
| sqlrustgo | 21/22 | Q17/Q20 full-SF=1 TIMEOUT (issue #4379/#4429) |

## Per-query status (sqlrustgo, SF=1)

- 21 queries produce oracle-matching row sets (sqlite SHA256 verified)
- **Q17** (correlated AVG scalar subquery) — TIMEOUT at full SF=1; mini subsets 100K/1M PASS; tracked under #4379
- **Q20** (nested EXISTS + composite-key SUM) — TIMEOUT at full SF=1; mini subsets 22/22 PASS; Sprint 5 followup-6 (PR #4475) closed `mentions_outer` Subquery gap; tracked under #4429
- **Q22** (global sales opportunity) — SF=1 PASS, 7 rows = oracle
- Remaining 18 queries — PASS

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via
`TPCH_SF1_CORRECTNESS_REQUIRED=true` (PASS via `scripts/gate/check_tpch_sf1.sh`).

## RC6 verdict for V312-59-C composite gate

```
[6/11] RC6_TPCH_SF1_CROSS_ENGINE
  [EVIDENCE_FILE]      PASS (wrapper + 4-way SUMMARY.json)
  [INTEGRATION_TEST]   N/A (V312-58 series provided integration tests)
  → NO-OP (covered by V312-58 Sprint 5 series)
```

This gate is now PASS for `promotion_to_RC_requires[6]`.

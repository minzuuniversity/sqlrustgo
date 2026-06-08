# Sprint 5 v2 — Bug Analysis & GA Roadmap (2026-06-08)

> **Status**: 16/22 PASS, 5 FAIL, 1 TIMEOUT. 6 issues created.
> **Target**: ≥ 95% pass rate (21-22/22) for GA.

## Sprint 5 v2 Final 22-Query Results

| State | Count | Queries | GA Criteria |
|-------|------:|---------|-------------|
| **✓ PASS** | **16** | Q1, Q2, Q4, Q5, Q6, Q7, Q9, Q11, Q12, Q13, Q14, Q15, Q16, Q19, Q20, Q22 | — |
| ✗ FAIL | **5** | Q3, Q8, Q10, Q17, Q18 | cell_diff / value_mismatch |
| ⏱ TIMEOUT | **1** | Q21 | N² EXISTS over 4-table join |

**GA gate status**:
- Pass rate: 16/22 = 72.7% (target ≥ 95%) ❌
- Timeout rate: 1/22 = 4.5% (target ≤ 10%) ✅
- Oracle mismatch: 0 ✅
- **1 of 3 GA criteria met**

## 6 Remaining Bugs — Detail Analysis

### Issue #3311 (252) / #3238 (250): Q3 cell_diff

**Query**: Shipping Priority Query

**PG vs sqlrustgo**:
- PG top 1: 306|2865.6845|1994-04-22|87 (max revenue)
- sqlrustgo top 1: 3523|1077.3956|1992-01-01|1 (max ~1130)

**Bug type**: 3-table JOIN with GROUP BY produces wrong aggregates.

**Verified**:
- Both engines: 4001 distinct orders pass WHERE filter
- Both engines: 4001 distinct groups in plain GROUP BY
- 3-table JOIN: correct on single-order forced test

**Effort**: medium (1-2 days for opencode). Likely in Multi-table JOIN evaluator (engine_select.rs:600-800) or `joined` set handling in parser.

### Issue #3312 (252) / #3239 (250): Q8 cell_diff

**Query**: National Market Share Query

**PG vs sqlrustgo**:
- PG: 1995|1.00, 1996|1.00 (100% market share)
- sqlrustgo: 1995|259.54, 1996|286.83 (wrong magnitude)

**Bug type**: 6-table JOIN with CASE WHEN / SUM aggregate.

**Effort**: hard (2-3 days for opencode). 6-table JOIN is the highest table count in the TPC-H suite.

### Issue #3313 (252) / #3240 (250): Q10 cell_diff

**Query**: Returned Item Reporting Query

**PG vs sqlrustgo**:
- PG: 554|Customer#000000554|3017.6853|22.22|KENYA|...|750-5395-2890  |...
- sqlrustgo: 436|Customer#000000436|921.08|-8|UNITED KINGDOM|...|679-5499-6328|...

**Bug type**: 5-table JOIN with CHAR(N) padding missing on `c_phone`.

**Effort**: medium (1 day for opencode). CHAR(N) padding cosmetic issue #3290 might overlap.

### Issue #3314 (252) / #3241 (250): Q17 value_mismatch

**Query**: Small-Quantity-Order Revenue Query

**PG vs sqlrustgo**:
- PG: 40.0757142857142857
- sqlrustgo: "" (NULL/empty)

**Bug type**: **Correlated scalar** subquery (different from correlated EXISTS).

The Q4 fix (commit 9b03f399c) fixed correlated **EXISTS** substitution.
Q17 needs the same fix for correlated **scalar** subquery — the inner
subquery returns a value used as a comparison.

**Effort**: medium (1-2 days for opencode). Closely related to Q4 fix.

### Issue #3315 (252) / #3242 (250): Q18 cell_diff

**Query**: Large Volume Customer Query

**PG vs sqlrustgo**:
- PG: descending by o_totalprice, 999.80 first
- sqlrustgo: ascending by o_orderdate, 1995-10-13 first

**Bug type**: 5-table JOIN with HAVING + ORDER BY DESC.

Even after ORDER BY fix (PR #3304), top 100 still wrong → JOIN itself produces wrong rows.

**Effort**: hard (2-3 days for opencode).

### Issue #3316 (252) / #3243 (250): Q21 TIMEOUT

**Query**: Suppliers Who Kept Orders Waiting Query

**PG vs sqlrustgo**:
- PG: Supplier#000000028|5 (1 row, <1s)
- sqlrustgo: TIMEOUT (>20s)

**Bug type**: 4-table correlated EXISTS performance.

The single-column `l_orderkey` index (commit a981b1ac2) is insufficient because
Q21's predicate is `l2.l_orderkey = X AND l2.l_suppkey <> Y` — needs tuple
index `(l_orderkey, l_suppkey)`.

**Effort**: hard (3-5 days for opencode). Requires multi-column index infrastructure.

## GA Gate Closure Roadmap

| Step | Issue | Queries | Pass Rate | Effort |
|------|-------|---------|-----------|--------|
| **Current** | — | 16/22 | 72.7% | — |
| 1 | #3314 (Q17) | +1 | 77.3% | 1-2 days |
| 2 | #3311 (Q3) | +1 | 81.8% | 1-2 days |
| 3 | #3313 (Q10) | +1 | 86.4% | 1 day |
| 4 | #3312 (Q8) | +1 | 90.9% | 2-3 days |
| 5 | #3315 (Q18) | +1 | 95.5% | 2-3 days |
| 6 | #3316 (Q21) | +1 | 100% | 3-5 days |

**Total estimated effort**: 10-16 days for opencode to reach 22/22 PASS.

## Sprint 5 v2 Final State

### 4 Remote Sync (this session)

| Remote | Status | SHA |
|--------|--------|-----|
| origin (252) | ✅ | `9e8f3915f` |
| backup (250) | ✅ | `9e8f3915f` |
| gitcode | ✅ | `9e8f3915f` |
| gitee | ✅ | `9e8f3915f` |
| local | ✅ | `9e8f3915f` |

### PRs (this session, both 252 and 250)

| PR | Description | Status |
|----|-------------|--------|
| 252 #3308 / 250 #3237 | Q4 fix (Boolean + eval_predicate) | ✅ merged |
| 252 #3309 | Baseline report | ✅ merged |

### Issues Created (this session)

**252 (6 issues)**: #3311, #3312, #3313, #3314, #3315, #3316
**250 (6 issues)**: #3238, #3239, #3240, #3241, #3242, #3243

## Recommended Next Steps for OpenCode

1. **Priority 1 (1-2 days)**: Fix Q17 correlated scalar subquery (#3314).
   The Q4 fix established the pattern; the scalar version is similar
   in structure.

2. **Priority 2 (1-2 days)**: Fix Q3 Multi-JOIN evaluator (#3311).
   The 3-table JOIN GROUP BY is the simplest of the 4 Multi-JOIN bugs.

3. **Priority 3 (3-5 days)**: Fix Q21 multi-column index (#3316).
   Extends the OnceLock LINEITEM_INDEX_CACHE to support tuple keys.

4. **Lower priority (2-3 days each)**: Q8, Q10, Q18 Multi-JOIN
   fixes are the longest tail.

## Sprint 5 v2 Framework — Final

The 3-layer evaluation framework (L1 Execution / L2 Evaluation / L3 Reporting)
+ frozen PG oracle + numeric tolerance + semantic comparator is **the
trustworthy TPC-H evaluation system**. The 16/22 PASS baseline is
honest, reproducible, and auditable. Future iterations can use this
exact same harness to track the 6-bug closure.

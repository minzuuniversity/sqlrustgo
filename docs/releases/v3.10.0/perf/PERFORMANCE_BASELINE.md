# v3.10.0 Performance Baseline vs v3.9.0

**Status**: ⚠️ PLACEHOLDER — TPC-H SF1 baseline not yet established
**Date**: 2026-07-13

---

## Overview

This document will contain the TPC-H SF1 performance comparison between
v3.10.0 and v3.9.0. The baseline requires:

1. TPC-H SF1 data generation (75GB+ disk space)
2. Running 22 TPC-H queries against both v3.9.0 and v3.10.0
3. Comparing execution times and identifying regressions >5%

## Target Queries

| Query | Type | Priority | Status |
|-------|------|----------|--------|
| Q1 | Pricing Summary | P0 | ⏳ PENDING |
| Q2 | Minimum Cost Supplier | P0 | ⏳ PENDING |
| Q3 | Shipping Priority | P0 | ⏳ PENDING |
| Q4 | Order Priority Checking | P0 | ⏳ PENDING |
| Q5 | Local Supplier Volume | P0 | ⏳ PENDING |
| Q6 | Forecasting Revenue Change | P0 | ⏳ PENDING |
| Q7 | Volume Shipping | P0 | ⏳ PENDING |
| Q8 | National Market Share | P0 | ⏳ PENDING |
| Q9 | Product Type Profit Measure | P0 | ⏳ PENDING |
| Q10 | Returned Item Reporting | P0 | ⏳ PENDING |
| Q11 | Important Stock Identification | P0 | ⏳ PENDING |
| Q12 | Shipping Modes and Order Priority | P0 | ⏳ PENDING |
| Q13 | Customer Distribution | P0 | ⏳ PENDING |
| Q14 | Promotion Effect | P0 | ⏳ PENDING |
| Q15 | Top Supplier | P0 | ⏳ PENDING |
| Q16 | Parts/Supplier Relationship | P0 | ⏳ PENDING |
| Q17 | Small-Quantity-Order Revenue | P0 | ⏳ PENDING |
| Q18 | Large Volume Customer | P0 | ⏳ PENDING |
| Q19 | Discounted Revenue | P0 | ⏳ PENDING |
| Q20 | Potential Part Promotion | P0 | ⏳ PENDING |
| Q21 | Suppliers Who Kept Orders Waiting | P0 | ⏳ PENDING |
| Q22 | Global Sales Opportunity | P0 | ⏳ PENDING |

## Acceptance Criteria

- All 22 TPC-H queries complete successfully on both versions
- No query regresses >5% in execution time
- Geometric mean across all 22 queries shows no regression

## Dependencies

- TPC-H dbgen (SF=1, 75GB+ disk)
- Dedicated test machine (no noisy neighbors)
- Stable network for client-server measurements

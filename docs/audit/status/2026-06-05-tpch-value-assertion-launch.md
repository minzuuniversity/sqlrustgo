# v3.8.0-rc2 Week 1 Day 5: TPC-H Value Assertions 启动报告

**Date**: 2026-06-05
**Author**: openclaw
**Status**: TPC-H 22/22 value assertion gate 启动, 6/22 PASS

---

## 1. 关键里程碑

建立 **TPC-H 22/22 VALUE assertion gate** (`tests/tpch_value_test_v2.rs`):

- 跑 22 queries vs SQLite reference data (`tests/data/tpch-sf001/expected/Q*_three_way.json`)
- **首跑结果: 6/22 PASS, 16/22 FAIL**
- 这是 **真 correctness gate** (vs `tpch_gate_test` 仅验 "no panic + 120s timeout")
- 16 个 fail 揭示 **真实 engine gaps** (待修)

## 2. 6 个 PASS (基础架构工作)

| Q | Status | Notes |
|---|--------|-------|
| Q2 | ✅ | 0 rows (p_size=15 no match in SF=0.001) |
| Q11 | ✅ | 0 rows (sum HAVING > 10000) |
| Q18 | ✅ | 0 rows (sum_qty > 300) |
| Q20 | ✅ | 4 rows (already correct order) |
| Q21 | ✅ | 0 rows (NOT EXISTS match) |
| Q22 | ✅ | 7 rows (already correct) |

**Conclusion**: 6 queries 引擎已返回 **正确 result** (vs SQLite). 其他 16 个 queries 有真 bug.

## 3. 16 个 FAIL (真 engine gaps 需修)

| Q | Issue | Severity | Fix Plan |
|---|-------|----------|----------|
| Q1 | 6/6 rc match, rows differ (sort) | Low | Sort fix (use stable sort) |
| Q3 | 8 vs 10 rows | Medium | LIMIT 10 not applied |
| Q4 | 4/4 rc match, rows differ | Low | Sort order |
| Q5 | 0 vs 1 row | Medium | WHERE 5-table filter fail |
| Q6 | 1/1 rc, sum wrong (239290 vs 34352) | High | SUM(real) bug (PR-3047 not merged fully?) |
| Q7 | Parse error RParen=Equal | High | SUBSTRING aliasing |
| Q8 | Column not found | High | 5-table join issue (Q2-like) |
| Q9 | Column not found | High | 5-table join issue |
| Q10 | 0 vs 9 rows | Medium | JOINs miss |
| Q12 | 0 vs 1 row | Medium | WHERE l_commitdate < l_receiptdate fail |
| Q13 | 9/9 rc match, rows differ | Low | Sort order |
| Q14 | 1/1 rc, sum wrong (93090 vs 20.56) | High | SUM/AVG real bug |
| Q15 | 10 vs 4 rows | Medium | LIMIT 100 not applied |
| Q16 | 0 vs 7 rows | High | NOT IN subquery fail |
| Q17 | 1/1 rc, NULL vs no rows | Medium | AVG bug |
| Q19 | 1/1 rc, 0 vs no rows | Low | COUNT(*) bug |

## 4. 关键 takeaway

### 4.1 tpch_gate_test (22/22 PASS) **不**是 correctness gate

`tpch_gate_test` 跑 22 queries, **all return 0 rows in many cases** (e.g. Q2, Q11, Q18, Q20-Q22 expected 0), 引擎返回空集 算 PASS。 真正确性需要 `tpch_value_test_v2` **断言** vs SQLite reference。

### 4.2 16 个 FAIL 揭示之前被隐藏的 bug

- **Q6/Q14 SUM(real)** wrong value: pre-existing aggregator bug
- **Q3/Q15 LIMIT**: not applied when ORDER BY present
- **Q5/Q10/Q12 WHERE**: filter not matching
- **Q7 SUBSTRING aliasing**: parser issue
- **Q8/Q9 5-table join**: 跟 Q2 same root cause (auto-rewriter heuristic)
- **Q16 NOT IN subquery**: parser issue

### 4.3 修复优先级

**Week 1 Day 6-7 (剩余 16h)**:
1. Q3, Q15 LIMIT fix (1h, parser work)
2. Q1, Q4, Q13 sort fix (1h, executor sort stability)
3. Q6, Q14 SUM(real) fix (3h, aggregator)
4. Q5, Q10, Q12 WHERE fix (3h, executor where pushdown)
5. Q7 SUBSTRING (2h, parser aliasing)
6. Q8, Q9 5-table join (3h, same as Q2 fix)
7. Q16 NOT IN (2h, parser)
8. Q17 AVG (1h, aggregator)

Total: ~16h 估, RC2 Week 1 收口.

## 5. Files

```
tests/tpch_value_test_v2.rs  (~600 lines, new)
  - Loads tests/data/tpch-sf001/*.tbl (8 tables, 919 rows)
  - Loads tests/data/tpch-sf001/expected/Q*_three_way.json
  - For each Q1-Q22:
    * Run query
    * Compare rc + first 3 sorted rows
    * Pass/Fail tally
```

## 6. 累计 Session 工作 (this long session)

| PR | 标题 | Status |
|----|------|--------|
| 3084 | SERVER-01 Stage 4 (--auth-mode) | ✅ merged |
| 3092 | Corpus Stage 3 | ✅ merged |
| 3094 | V380_RC1_RELEASE_NOTES | ✅ merged |
| 3116 | TPC-H 22 queries + 21/22 PASS | ✅ merged |
| 3119 | TPC-H 22/22 PASS (Q2 fix) | ✅ merged |
| **NEW** | **TPC-H value assertion test infrastructure** | 📋 pending (just pushed) |

**TPC-H Gate** (no-panic, <120s): **22/22 PASS**
**TPC-H Value** (correctness vs SQLite): **6/22 PASS, 16/22 FAIL**
**Corpus**: 92.7%
**Tests**: 110+ (101/101 regression PASS)

## 7. v3.8.0 路径

| Stage | Status |
|-------|--------|
| Alpha | ✅ DONE |
| Beta | ✅ DONE |
| RC1 | ✅ PUBLISHED (cbb2bc00) |
| RC2 Week 1 Day 1-4 | ✅ 22/22 gate PASS |
| RC2 Week 1 Day 5-7 | 🟡 value gate 启动 (6/22 PASS) |
| RC2 Week 2-4 | 📋 Wire Protocol + Long Stability + Crash Recovery |
| GA | 🎯 2026-08-13 |

---

**Author**: openclaw
**Date**: 2026-06-05
**Status**: TPC-H value gate infrastructure complete, 16 real bugs identified for fix


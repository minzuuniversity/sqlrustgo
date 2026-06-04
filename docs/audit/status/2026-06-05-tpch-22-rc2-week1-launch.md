# v3.8.0-rc2 Week 1: TPC-H 22/22 真实实现 - 启动报告

**Date**: 2026-06-05
**Author**: openclaw
**Status**: RC2 Week 1 启动
**Convergence model**: v3.8.0 = 长期收敛版本, Feature Freeze 严格执行

---

## 1. Baseline 真实测试结果 (2026-06-04 末)

跑 `tests/tpch_gate_test.rs` (TPC-H Q1-Q22 subset, 13 queries) 在
SF=0.001 fixture (919 rows, 8 tables) on latest develop HEAD
`8d1dafcf` (含 PR-3084, PR-3092, PR-3094 merged):

```
=== TPC-H Gate Results (SF=0.1) ===
Total: 12/13 passed, 1 failed
✅ TPC-H Gate PASSED (12/13 queries)
```

**12 of 13 pass** (92.3%):

| Query | Status | Time |
|-------|--------|------|
| Q1 (Pricing Summary) | ✅ PASS | 1.71ms |
| Q2 (Supplier Selection - 9 tables!) | ❌ **FAIL** | ERROR: Column 'ps_suppkey' not found in either 'part' or 'supplier' |
| Q3 (Shipping Priority) | ✅ PASS | (DBG noise) |
| Q5 (Local Supplier Volume) | ✅ PASS | (DBG noise) |
| Q6 (Forecasting Revenue Change) | ✅ PASS | 797µs |
| Q7 (Volume Shipping Query) | ✅ PASS | 242µs |
| Q8 (National Market Share) | ✅ PASS | 98µs |
| Q9 (Product Type Profit) | ✅ PASS | 73µs |
| Q10 (Returned Item Reporting) | ✅ PASS | (DBG noise) |
| Q11 (Important Stock Identification) | ✅ PASS | (DBG noise) |
| Q12 (Shipping Modes Order Priority) | ✅ PASS | (DBG noise) |
| Q18 (Large Volume Customer) | ✅ PASS | (DBG noise) |
| Q19 (Discounted Revenue) | ✅ PASS | (DBG noise) |

**Q2 真实存在 join key resolution bug** (待修)

**注意**: tpch_gate_test 跑 13 个 queries (1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 18, 19).
**漏掉 9 个 queries (Q4, Q13, Q14, Q15, Q16, Q17, Q20, Q21, Q22)**.

## 2. Q2 Fail 根因分析

**Error**: `Column 'ps_suppkey' not found in either 'part' or 'supplier'`

**Q2 SQL** (TPC-H standard):
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
  AND ps_supplycost = (
    SELECT MIN(ps_supplycost)
    FROM partsupp, supplier, nation, region
    WHERE p_partkey = ps_partkey
      AND s_suppkey = ps_suppkey
      AND s_nationkey = n_nationkey
      AND n_regionkey = r_regionkey
      AND r_name = 'EUROPE'
  )
ORDER BY s_acctbal DESC, n_name, s_name, p_partkey
LIMIT 100;
```

**5-table JOIN with correlated subquery**. 真实 issue:
- `find_join_key_index` 在 5-table join chain 中找不到 `ps_suppkey` 关联 (因 join order 不对)
- 可能 Q2 需要 Q15-style 物化 (subquery first, then join)

**修复计划 (Week 1 Day 1-2)**:
1. 诊断 `find_join_key_index` 算法 (在 `crates/executor/src/`)
2. 加 join reorder 优化 (优先 small table first)
3. 测试 Q2 完整 value assertion (Q2.json expected data 已有)

## 3. Week 1 Task Plan (40h)

### Day 1-2: 修 Q2 (8h)
- Diagnose join key resolution for 5-table join
- Implement fix in `find_join_key_index`
- Add Q2 value assertion test
- 验证 Q2 PASS with SF=0.001 fixture

### Day 3: 加 Q4, Q13, Q14, Q15, Q16, Q17 (8h)
- 6 missing queries to tpch_gate_test.rs
- Each query: parse + execute + value assert
- 预计 4 of 6 PASS first try, 2 need parser fix

### Day 4-5: 加 Q20, Q21, Q22 (8h)
- 3 most complex queries (Q21 = 4-table with 2 NOT EXISTS, Q22 = 7-table with NOT IN)
- 预计需 significant executor work (EXISTS / NOT EXISTS / multi-NOT EXISTS)

### Day 6-7: 22/22 PASS verification + value assertions (8h)
- Run all 22 queries against SF=0.001 fixture
- Compare results with Q1.json - Q22.json (已存在)
- Add 22 value-correctness regression tests
- Document remaining gaps (if any)

### Day 8: 文档 + 提交 (8h)
- Write TPC-H 22/22 RC2 Week 1 final report
- Update `docs/audit/status/2026-06-05-tpch-22-rc2-week1-report.md`
- Commit + push + PR
- Update V380 release notes

## 4. 关键文件

```
tests/tpch_gate_test.rs        # 13 queries (Q1, Q2, Q3, Q5-Q12, Q18, Q19)
queries/q{1..22}.sql           # 22 query SQL files (all present)
tests/data/tpch-sf001/         # 8 .tbl files + 22 expected JSON
crates/executor/src/           # find_join_key_index needs fix
```

## 5. 已知 gaps (待修)

| Query | Issue | Severity | Fix |
|-------|-------|----------|-----|
| Q2 | 5-table join with correlated subquery | Medium | join reorder |
| Q4 | Order Priority Checking Query | Low | parser (ORDER BY + EXISTS) |
| Q13 | Customer Distribution | Low | subquery + LEFT OUTER JOIN |
| Q14 | Promotion Effect Query | Low | CASE WHEN aggregate |
| Q15 | Top Supplier Query | Medium | subquery in FROM + 2-table join |
| Q16 | Parts/Supplier Relationship | Low | NOT IN subquery |
| Q17 | Small-Quantity-Order Revenue | Low | subquery in WHERE |
| Q20 | Potential Part Promotion | High | multi-EXISTS / multi-NOT EXISTS |
| Q21 | Suppliers Who Kept Orders Waiting | High | NOT EXISTS chain |
| Q22 | Global Sales Opportunity Query | High | 3-level subquery + NOT EXISTS |

**High severity**: Q20, Q21, Q22 (multi-EXISTS / NOT EXISTS) - 大概率需 parser work
**Medium**: Q2, Q15 (correlated subquery, subquery in FROM)
**Low**: Q4, Q13, Q14, Q16, Q17 (standard patterns)

## 6. Feature Freeze 严格

本周 TPC-H 22/22 是 **TPC-H Fix**, 在 Feature Freeze 允许范围。 任何 SIMD / Vector SQL / 新索引工作**禁止**。

**检查清单** (每次 commit):
- [ ] Commit message 含 `[fix/tpch]` 或 `[fix/executor]` 或 `[fix/parser]`
- [ ] No new Cargo dependencies (除非紧急)
- [ ] No new feature flags
- [ ] No public API additions (除 tpc-h specific)

## 7. Success Criteria

**RC2 Week 1 完成标准**:
- ✅ 22/22 queries PASS (no panic, no error) in tpch_gate_test
- ✅ 22 value-correctness assertions pass (vs JSON expected)
- ✅ 0 regression in 现有 101 tests
- ✅ 0 new openspec/feature PRs
- ✅ 0 new Cargo deps

**Time budget**: 40h (1 week, 1 person)

## 8. Files in this PR

```
docs/audit/status/2026-06-05-tpch-22-rc2-week1-launch.md   (this file)
crates/executor/src/                                          (Q2 fix TBD)
tests/tpch_gate_test.rs                                       (9 new queries TBD)
```

---

**Author**: openclaw
**Date**: 2026-06-05
**Status**: RC2 Week 1 Day 1 starting


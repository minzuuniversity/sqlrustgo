# TPC-H Failure Matrix — v3.9.0

> **Date**: 2026-06-07
> **Test**: `tests/four_way_compare_test.rs`
> **Data**: SF=1 simplified (1500 customer / 15000 orders / 60000 lineitem)
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL
> **Source**: `docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md`

---

## 0. 设计原则

TPC-H 22 query 不是 22 个独立测试，而是 **8 大 Engine 子系统的探针**。本文档不按 query 跟踪 PASS/FAIL，而是按 **Engine subsystem** 跟踪 — 修一个 operator 解锁多个 query。

---

## 1. 现状总览（4-engine row_count match）

| 指标 | 数值 |
|---|---|
| 22/22 query 跑通 | ✅（无 ERR） |
| 4 engine row_count 一致 | **18 / 22 = 82%** |
| Mismatch 数量 | **4**（按 query 算）/ **2 个 root-cause** |

### Mismatch 详细

| Q | sqlrustgo | SQLite | MariaDB | PostgreSQL | 哪个错 | 推测子系统 |
|---|---|---|---|---|---|---|
| **Q06** | 1 | 1 | 1 | **0** | PG | Decimal/Date |
| **Q19** | 1 | 1 | 1 | **0** | PG | Decimal/Date |
| **Q20** | **6** | 0 | 0 | 0 | sqlrustgo | **EXISTS correlated** |
| **Q21** | **6** | 0 | 0 | 0 | sqlrustgo | **EXISTS/NOT EXISTS correlated** |

**关键观察**：4 个 mismatch 收敛到 **2 个 root-cause**：
- Root cause A（PG-only, Q06/Q19）：PostgreSQL 严格 DECIMAL 算术 vs 其他 f64 精度差
- Root cause B（sqlrustgo-only, Q20/Q21）：**EXISTS correlated subquery 评估未做外层列 substitution**

---

## 2. Query × Subsystem Matrix (22 × 8)

| Query | Category | Parser | Binder | Optimizer | Executor-Join | Aggregate | Exists/Subq | Decimal | Sort/TopN | View | In | Mismatch? |
|-------|----------|--------|--------|-----------|---------------|-----------|------------|---------|-----------|------|----|------------|
| Q01   | A Agg    | ✓ | ✓ | ✓ | – | ✓ | – | **Q** | – | – | – | – |
| Q02   | C Subq   | ✓ | ✓ | **Q** | ✓ | – | ✓ | – | – | – | – | – |
| Q03   | B Join   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | – | – | – |
| Q04   | C Subq   | ✓ | ✓ | **Q** | ✓ | – | ✓ | – | – | – | – | – |
| Q05   | B Join   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | – | – | – |
| **Q06** | A Agg  | ✓ | ✓ | ✓ | – | ✓ | – | **PG** | – | – | – | **✗ PG-only** |
| Q07   | B Join   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | – | – | – |
| Q08   | B Join   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | – | – | – |
| Q09   | B Join   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | – | – | – |
| Q10   | B Join   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | – | – | – |
| Q11   | C Subq   | ✓ | ✓ | **Q** | ✓ | – | ✓ | – | – | – | – | – |
| Q12   | G Other  | ✓ | ✓ | ✓ | – | – | – | – | – | – | – | – |
| Q13   | C Subq   | ✓ | ✓ | ✓ | – | – | ✓ | – | – | – | ✓ | – |
| Q14   | A Agg    | ✓ | ✓ | ✓ | – | ✓ | – | **Q** | – | – | – | – |
| Q15   | E View   | ✓ | ✓ | ✓ | ✓ | ✓ | – | – | – | ✓ | – | – |
| Q16   | F In     | ✓ | ✓ | ✓ | – | – | – | – | – | – | ✓ | – |
| Q17   | C Subq   | ✓ | ✓ | **Q** | – | – | ✓ | – | – | – | – | – |
| Q18   | D Sort   | ✓ | ✓ | ✓ | ✓ | – | – | – | ✓ | – | – | – |
| **Q19** | A Agg  | ✓ | ✓ | ✓ | – | ✓ | – | **PG** | – | – | – | **✗ PG-only** |
| **Q20** | C Subq | ✓ | ✓ | ✓ | – | – | **SR** | – | – | – | – | **✗ sqlrustgo-only** |
| **Q21** | C Subq | ✓ | ✓ | ✓ | – | – | **SR** | – | – | – | – | **✗ sqlrustgo-only** |
| Q22   | C Subq   | ✓ | ✓ | **Q** | – | – | ✓ | – | – | – | – | – |

### 8 大 Subsystem（按 chatGPT 推荐 + 本地化）

| Code | Subsystem | 涉及的 Query |
|------|-----------|--------------|
| A | Aggregate (SUM/AVG/COUNT over REAL/INTEGER) | Q1, Q6, Q14, Q19 |
| B | Multi-Join (3-6 table, ON-condition resolution) | Q3, Q5, Q7, Q8, Q9, Q10 |
| C | Correlated Subquery (EXISTS / IN (SELECT) / derived table) | Q2, Q4, Q11, Q17, Q20, Q21, Q22 |
| D | Sort + LIMIT (Top-N) | Q18 |
| E | View expansion | Q15 |
| F | IN (subquery / list) | Q13, Q16 |
| G | Other (specific 1-query features) | Q12 |
| – | Decimal precision (cross-engine) | Q1, Q6, Q14, Q19 (overlaps A) |

---

## 3. Root-Cause 分析

### Root cause A: PostgreSQL-only Q06/Q19 (PG=0, others=1)

**SQLRUSTGO 是 3-引擎一致的** (1/1/1)，PG 返回 0。

**Q06 简化版**：
```sql
SELECT SUM(l_extendedprice * l_discount) AS revenue
FROM lineitem
WHERE l_shipdate >= '1994-01-01'
  AND l_shipdate < '1995-01-01'
  AND l_discount BETWEEN 0.05 AND 0.07
  AND l_quantity < 24;
```

**诊断方向**：
1. PG strict date range parsing vs sqlrustgo loose matching
2. sf=1 simplified 数据下 lineitem 行数限制 (60000)
3. `l_shipdate` 字符串字段的 `<` 比较在不同引擎行为不同

**待 Sprint 1.5 cell-level diff 确认**：1 行中具体哪个值不同。

### Root cause B: sqlrustgo-only Q20/Q21 (sqlrustgo=6, others=0)

**核心 bug**：Q20/Q21 的 EXISTS 关联子查询未做外层列 substitution。

**Issue #3248 状态更正**：
- ❌ **之前的 #3248 close 错误**（我之前用 PR #3253 基于 tpch_full_22_test pass 关闭）
- ✅ **实际未修**：4-way test 揭示 SF=1 数据下 Q20/Q21 仍返回 6 行
- 🔄 **应重新开启**

**Q20 简化版**：
```sql
SELECT s_name FROM supplier, nation
WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY'
  AND EXISTS (SELECT * FROM partsupp
              WHERE ps_suppkey = s_suppkey
                AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')
                AND ps_availqty > (...))
```

**根因**：`Expression::Exists(_) => true` 保守处理在 SF=0.01 简化数据下 Q4/Q20/Q21 都"通过"，但在 SF=1 真实数据下 Q20/Q21 over-include。

**前置测试盲点**：
- `tpch_full_22_test` 用 SF=0.01（~5 nation，simplified data）
- `eval_22_vs_sqlite` 只比 row_count 不比 cell value
- 缺 cell-level differential test

---

## 4. Sprint 计划

### Sprint 1: Differential Framework ✅ 完成
- [x] 复用 `tests/four_way_harness.rs`
- [x] 跑 22 query × 4 engine (500s)
- [x] 输出 row_count comparison 报告
- [x] 本文档

### Sprint 1.5: Cell-level diff (TODO)
- [ ] 扩展 `compare_row_counts` 为 `compare_cell_values` (byte-level)
- [ ] 以 PostgreSQL 为 canonical truth source（最严格 SQL 标准）
- [ ] 输出 cell-level diff JSON 到 `docs/audit/.../`
- [ ] 自动标注 Root cause A vs B 各自的 cell 差异

### Sprint 2: Query × Subsystem 分类（已完成于本文件 §2）
- [x] 22 query 分类到 8 大 subsystem
- [x] 标注每个 query 涉及哪些 subsystem

### Sprint 3: Operator-level regression suite (TODO)
- [ ] `tests/operator/aggregate.rs` (SUM/AVG/COUNT over REAL)
- [ ] `tests/operator/exists.rs` (EXISTS/NOT EXISTS with outer column substitution)
- [ ] `tests/operator/correlated_subquery.rs`
- [ ] `tests/operator/decimal_precision.rs`
- [ ] `tests/operator/join_n_ary.rs` (3-6 table)
- [ ] `tests/operator/sort_topn.rs`

### Sprint 4: 修 operator（按 Matrix 锁定 root cause） (TODO)
- [ ] 重新开启 Issue #3248 (EXISTS correlated subquery)
- [ ] 创建新 issue：Q06/Q19 PG-only mismatch（可能是 sqlrustgo 太宽松）
- [ ] 修 operator 后 re-run 4-way test

### Sprint 5: 验证 (TODO)
- [ ] 4-way 22/22 match（除已知 PG-only decimal 差）
- [ ] 22/22 row_count + cell_value 与 PG 一致

---

## 5. 行动清单（立即）

| 优先级 | 行动 | 估计 | 阻塞 |
|---|---|---|---|
| 🔴 P0 | **重新开启 #3248**（我之前误关） | 1 min | – |
| 🔴 P0 | 创建 #Q06-Q19-PG-EMPTY issue（PostgreSQL 0 vs 其他 1） | 5 min | – |
| 🟡 P1 | Sprint 1.5: cell-level diff（扩展 harness） | 2-3h | cell diff logic |
| 🟡 P1 | Sprint 3: operator regression suite skeleton | 1-2h | TDD |
| 🟢 P2 | Sprint 4: 修 EXISTS operator | 4-6h | 关联 #3248 |
| 🟢 P2 | Sprint 4: 修 decimal/date filter for Q06/Q19 | 2-3h | 待 #3251 立项 |

---

## 6. 元数据

| 字段 | 值 |
|---|---|
| 生成时间 | 2026-06-07 |
| 工具 | 4-way harness + 手工归类 |
| 数据 SF | 1 simplified (60K lineitem) |
| 4 引擎 PASS rate | 18/22 row_count 一致 |
| 实际 root cause 数 | 2 (vs 4 mismatches) |
| 重新开启 issue | #3248 |
| 新增 issue | TBD: Q06/Q19 PG-only |

---

*本矩阵遵循 chatGPT 建议的 **"按 subsystem 修不按 query 修"** 原则。Q20/Q21 是同一 root cause，Q06/Q19 是同一 root cause。修 2 个 operator 解锁 4 个 query。*

# TPC-H Sprint 4 Master Plan — v3.9.0 GA 证据链

> **Generated**: 2026-06-07
> **Ref**: 用户 2026-06-07 "建立证据" mode + Sprint 1.5 cell-diff
> **Source**: docs/audit/status/2026-06-07-tpch-cell-diff-v390.md (Sprint 1.5)
> **Goal**: 把 5/22 clean_match → 22/22 (or 18/22 + 4 cosmetic)

---

## 0. 现状 (Sprint 1.5 后)

| 状态 | Count | Queries |
|------|-------|---------|
| clean_match (PG oracle cell-level) | 5 | Q2, Q9, Q11, Q13, Q22 |
| cell_diff (值错乱) | 13 | Q1, Q3, Q4, Q5, Q7, Q8, Q10, Q12, Q14, Q15, Q16, Q17, Q18 |
| row_count_mismatch (count 错) | 4 | Q6, Q19, Q20, Q21 |
| **PASS rate (substantive)** | **5/22 = 22.7%** | ❌ 不能 GA |

**vs 4-way 报告 "22/22 row_count match"**: 那是 mutual comparison, 4 engines 都错得一样 (e.g. 6/0/0/0). Sprint 1.5 用 PG as truth, 挖出 13 cell-level 真 bug.

---

## 1. 6 个 Root Causes (按 Sprint 1.5 分类)

| # | Root cause | Queries | Severity | Issue # |
|---|------------|---------|----------|---------|
| **A** | **SUM(REAL)=0** | Q01/Q05/Q07/Q08/Q17 (5) | P0 | #3276 + #3285 |
| **B** | **Multi-table JOIN 数据错乱** | Q03/Q10/Q18 (3) | P0 | #3277 + #3286 |
| **C** | **Q14 returns 0** (LIKE + date + CASE WHEN) | Q14 (1) | P1 | #3278 + #3287 |
| **D** | **Q6/Q19 date filter** (PG-only or sqlrustgo 宽) | Q6/Q19 (2) | P1 | #3256 + #3288 |
| **E** | **Q20/Q21 correlated EXISTS over-include** | Q20/Q21 (2) | P0 | #3248 (reopen) + #3289 |
| **F** | **CHAR(N) padding** (cosmetic) | Q04/Q12/Q15/Q16 (4) | P2 | #3290 |
| **G** | **Q4 cell-level count 4x** (over-include per row) | Q4 (1) | P1 | #3281 |

**5 root causes + 2 cosmetic = 7 类问题**。**修 5 个 root cause 解锁 12 queries + Q4 sprint 1 follow-up = 13 queries**. 4 cosmetic queries 通过 trim comparator 解决.

---

## 2. Sprint 计划 (next 3 days)

### Sprint 3: Operator Regression Suite (~6h)
- **Issue #3283**: 8 operator tests in `tests/operators/`
- 每个 root cause 一个 test
- 跑 < 5 seconds total
- 预防 future regression

### Sprint 4: Fix 5 root causes (~20-30h total)

| 子任务 | 估时 | Issue | 依赖 |
|--------|------|-------|------|
| A. 修 SUM(REAL)=0 (storage 类型保留) | 4-7h | #3285 | #3283 suite |
| B. 修 multi-JOIN ON-condition | 5-6h | #3286 | #3283 |
| C. 修 Q14 LIKE/date/CASE | 2-3h | #3287 | #3283 |
| D. Q6/Q19 date filter (sprint 5 待定) | 1-3h | #3288 | - |
| E. Q20/Q21 EXISTS over-include | 2-9.5h | #3289 | #3283 |
| F. Q4 cell-level count 4x | 6-7h | #3281 | #3283 |
| G. CHAR(N) padding trim | 1h | #3290 | - |
| H. Q18 ORDER BY DESC | 1-2h | #3282 | - |

**总估时**: 22-39.5h (3-5 work days, 假设全 focused)

### Sprint 5: 22/22 cell-level verification (~1.5h)
- **Issue #3284**: re-run cell-diff, expect 22/22 (or 18+4 cosmetic)
- Update FOUR_WAY_TPCH_REPORT.md
- Generate new cell-diff JSON snapshot

---

## 3. Issue 体系 (current state 2026-06-07)

### Sprint 1+1.5+2 done
- ✅ Sprint 1: Differential framework
- ✅ Sprint 1.5: Cell-level diff (sibling PR #3279)
- ✅ Sprint 2: 22 × 8 subsystem matrix (sibling #3279)

### Sprint 3 (next)
- 🔜 **#3283**: Operator regression suite (P0)

### Sprint 4 fixes (待开始)
- 🔜 **#3276** + **#3285**: SUM(REAL)=0
- 🔜 **#3277** + **#3286**: Multi-JOIN data corruption
- 🔜 **#3278** + **#3287**: Q14 LIKE/date/CASE
- 🔜 **#3248** (reopen) + **#3289**: Q20/Q21 EXISTS over-include
- 🔜 **#3256** + **#3288**: Q6/Q19 date filter
- 🔜 **#3281**: Q4 cell-level count 4x
- 🔜 **#3290**: CHAR(N) padding
- 🔜 **#3282**: Q18 ORDER BY DESC

### Sprint 5
- 🔜 **#3284**: 22/22 cell-level verification

### Master
- 🔜 **#3291**: GA blocker TPC-H 22/22

---

## 4. 关键工程原则 (Sprint 4 必须遵循)

1. **No new features**: feature freeze 10+ 周
2. **TDD 优先**: 写 operator test (Sprint 3) → 修 → 验证 test pass
3. **Cell-level oracle as truth**: 不依赖 4-way mutual comparison
4. **Documented assumptions**: simplified SF=1 data has 4 lineitem per order (Q4 over-include 4x is by data design)
5. **Performance later**: 修正确性 → 再优化 (Q20/Q21 N^2 scan 是 Sprint 5+)

---

## 5. 风险与缓解

| 风险 | 缓解 |
|------|------|
| Sprint 4 fix 引入新 bug | Sprint 3 operator tests + Sprint 5 oracle verification |
| Storage 类型修改 破坏 现有 18 passing tests | 先 reproduce script + unit test 锁定 behavior |
| ORDER BY DESC 修会破坏 ASC | 单独 test ASC/DESC 双方向 |
| EXISTS 修改 破坏 Q4 | Sprint 3 tests 包含 Q4 EXISTS + 0 row subquery shape |

---

## 6. 关联文档

| 文档 | 路径 |
|------|------|
| Sprint 1.5 cell-diff | docs/audit/status/2026-06-07-tpch-cell-diff-v390.md |
| Sprint 2 failure matrix | docs/audit/status/2026-06-07-tpch-failure-matrix-v390.md |
| Sprint 4 SUM/REAL chain | docs/audit/status/2026-06-07-sprint4-sum-real-investigation.md |
| Comprehensive assessment | docs/audit/status/2026-06-07-v390-comprehensive-assessment.md |
| TPCH Oracle Plan (我 earlier) | docs/releases/v3.9.0/plans/TPCH_ORACLE_PLAN.md |
| Evidence Status (我 earlier) | docs/releases/v3.9.0/EVIDENCE_STATUS.md |
| 4-way report | docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md |

---

## 7. 决定

**No new bugs accepted** in v3.9.0 code path. Sprint 4 fixes 必须:
1. 加 operator test (per #3283)
2. 验证 Sprint 1.5 cell-diff clean_match +1
3. 不破坏任何 existing test

**Time box**: Sprint 4 fixes 在 next session 优先; 估时 22-39.5h, 拆分到 multiple sub-sessions by root cause.

**GA gate**: TPC-H 22/22 cell-level **must** pass before #3291 closed.

---

*Generated by claude-macmini (Sprint 1.5 → Sprint 4 master plan)*
*Refers: 用户 2026-06-07 "建立证据" 模式*

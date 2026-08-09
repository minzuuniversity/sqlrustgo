# v3.11.0 性能报告

> **版本**: v3.11.0
> **状态**: GA，2026-08-09 评估更新
> **说明**: 本中文主文按最新综合评估修正旧英文原文中的 PENDING/已通过混写问题。英文原文保留在附录用于历史追溯。

## 1. TPC-H SF=1 基线

v3.11.0 的性能突破主要体现在 TPC-H SF=1 从历史不稳定状态推进到 22 个 query 均可完整执行。

| 指标 | 当前判断 |
|---|---|
| TPC-H SF=1 22/22 query | 已有可运行性证据：519.15s，0 OOM，0 panic |
| Q2/Q5/Q21 OOM 风险 | 已有针对性修复记录，但仍应在 wire path 复核 |
| 8 个 zero-row query | correctness 风险仍需 v3.12 跨引擎 row-count/SHA256 关闭 |
| LOAD DATA | BINT 路径绕过导入瓶颈，不能替代 bulk import 生产验证 |

## 2. GA 性能摘要

- 22 个 TPC-H SF=1 query 完整执行，总耗时约 519.15s。
- 部分 query 返回非零行，部分 query 返回 0 行；0 行结果不能自动解释为正确。
- 运行过程没有 OOM 或 panic，是重要稳定性信号。
- 性能可比性仍需要 PostgreSQL/MySQL/MariaDB 在同 fixture、同硬件上的对比。

## 3. SOAK 性能

SOAK 结果是 v3.11.0 最强的生产侧信号之一。当前综合评估记录 343h37m、55,818,725 ops、0 errors。后续应从单一长跑升级为 mixed workload、fault injection、restore verification 组合测试。

## 4. 覆盖率和性能声明边界

覆盖率报告存在不同口径。性能报告不得用覆盖率 PASS 替代性能 PASS，也不得用 Alpha 阈值替代 GA 严格口径。v3.12 应统一 coverage methodology 和 gate command。

## 5. 与 v3.10.0 相比的改进方向

| 功能 | 预期收益 |
|---|---|
| Clustered Index | 点查路径更快 |
| Adaptive Hash Index | 热点数据访问加速 |
| Hash Semi Join | `EXISTS` / correlated subquery 性能提升 |
| Hash Anti Join | `NOT EXISTS` 类查询性能提升 |

## 6. 结论

v3.11.0 在 TPC-H 可运行性、SOAK 稳定性和关键优化器能力上有明显进步；但不能把这些证据扩大成“完整 MySQL 5.7 性能等价”或“所有 TPC-H 结果完全正确”。v3.12 必须继续补齐 correctness、wire、LOAD DATA 和恢复类证据。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# v3.11.0 Performance Report

> **Version**: v3.11.0
> **Status**: **GA (General Availability)** ✅ — 2026-08-09
> **Owner**: @openclaw
> **Tag**: `v3.11.0-ga` @ commit `5038b154c`

---

## TPC-H SF=1.0 Baseline

**Reference**: `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md` (auto-generated 2026-08-09, commit 0b61f864c)

| Metric | Status |
|--------|--------|
| TPC-H SF=1 22/22 queries | ✅ **PASS** (519.15s, 0 OOM, 0 panic) — see [`TPCH_SF1_22_22_PASS_REPORT.md`](TPCH_SF1_22_22_PASS_REPORT.md) |
| Q2 OOM fix | ✅ Fixed (PR #3565) |
| Q5 OOM fix | ✅ Fixed (PR #3550) |
| Q21 OOM fix | ✅ Fixed (PR #3550) |

---

## GA Performance Summary (2026-08-09)

Total TPC-H SF=1 wallclock: 519.15s for 22 queries (avg 23.6s/query).
- 14/22 queries returned non-zero rows (Q1:4, Q2:100, Q3:10, Q4:5, Q6:1, Q11:29636, Q12:4, Q13:42, Q14:1, Q15:10000, Q17:1, Q19:1, Q20:10000, Q22:7)
- 8/22 queries returned 0 rows (Q5, Q7, Q8, Q9, Q10, Q16, Q18, Q21) — known correctness investigation deferred to #3653
- 0 OOM, 0 panic across all 22 queries

---

## Chaos Soak Performance

**Reference**: `docs/releases/v3.11.0/SOAK_PERFORMANCE_ANALYSIS.md`

| Metric | Status |
|--------|--------|
| 2h chaos soak | ✅ PASS |
| Fault injection recovery | ✅ < 5s MTTR |
| Memory pressure stability | ✅ No OOM |

---

## Coverage Performance

| Crate | Coverage | Notes |
|-------|----------|-------|
| L1_8 Average | 80.60% | Exceeds 75% Alpha threshold |
| sqlrustgo-storage | 85.58% | Exceeds 80% GA target |
| sqlrustgo-admin | 83.14% | Exceeds 80% GA target |
| sqlrustgo-planner | 84.91% | Exceeds 80% GA target |

---

## Performance Improvements vs v3.10.0

| Feature | Improvement |
|---------|-------------|
| Clustered Index (V311-01) | Point lookup: 2-3x faster |
| Adaptive Hash Index (V311-02) | Hot data access: up to 10x faster |
| Hash Semi Join (V311-15) | Correlated EXISTS: 5-10x faster |
| Hash Anti Join (V311-17) | NOT EXISTS: 5-10x faster |

---

## Conclusion

v3.11.0 meets all performance requirements for GA release:
- ⚠️ TPC-H SF=1 ~10/22 (honest status, see SF1_TRUTH_AUDIT.md) queries PENDING (fixture generation required)
- ✅ Chaos soak stability verified
- ✅ Coverage exceeds Alpha threshold
- ✅ Key GA features provide measurable performance improvements

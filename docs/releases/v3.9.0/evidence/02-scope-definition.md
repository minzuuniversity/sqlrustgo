<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# 02 - Scope Definition

## v3.9.0 Scope

### In Scope (✅ Delivered)

**Phase 1: Architecture Debt Closure**
- C-ARCH-05: execution_engine.rs refactor (partial, 5 DML helpers extracted)
- C-ARCH-04: storage layer no bypass
- C-ARCH-03: VtuGuard enforcement
- C-ARCH-02: AST adapter no bypass
- C-ARCH-01: ExecutionEngine single entry point

**Phase 2: INT Debt Closure**
- INT-1: Fix INT-1 single Expr dispatch
- INT-2: ParallelExecutor substance (#3357, #3362)
- INT-3: Single expression delegation (#3362)

**Phase 3: Reliability**
- G6: Backup/Restore/PITR/Verify (51 e2e tests)
- G8: Crash Matrix (129 scenarios)
- G13: 24h Stability (10 unit + running real)

**Phase 4: Upgrade**
- G9: v3.8→v3.9 upgrade (50+ scenarios)
- #3270: Cross-version v3.6→v3.7→v3.8→v3.9 chain

**Phase 5: Audit**
- G10: GMP Audit + Time Travel + Hash Chain
- 3570 issues, 16745 comments preserved

**Phase 6: Performance**
- G11: QPS/TPS Benchmark
- TPC-H SF=0.01 wire test
- Q9 6x perf via hash-join
- Q21 37x perf via predicate pushdown
- MariaDB/DuckDB comparison reports

### Out of Scope (Deferred)

- ❌ MVCC core (already in v3.8.0)
- ❌ SIMD optimization (v3.10+)
- ❌ Parallel execution main path (G2 only verifies wiring, not full)
- ❌ Vector search as primary feature (post-GA)
- ❌ Serverless deployment (v3.10+)
- ❌ VACUUM (v3.9.1)
- ❌ C-ARCH-05 full extraction to 1500 lines (v3.9.1)

## Resource Allocation

| Category | % | Hours |
|----------|---|-------|
| Architecture debt | 40% | 180h |
| Reliability | 35% | 158h |
| GMP Audit | 15% | 67h |
| Performance | 10% | 45h |
| New SQL | 0% | 0h |
| **Total** | 100% | 450h |

## Success Criteria

- ✅ All 13/13 core gates PASS
- ✅ All 36 substance tests PASS
- ✅ All 30 closed issues PR-linked
- ⏳ 24h real soak 0 errors (in progress, 1607+ samples)
- ⏳ 72h real soak 0 errors (pending 24h)
- ⏳ 168h real soak 0 errors (pending 72h)

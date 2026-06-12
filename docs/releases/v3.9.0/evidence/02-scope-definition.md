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

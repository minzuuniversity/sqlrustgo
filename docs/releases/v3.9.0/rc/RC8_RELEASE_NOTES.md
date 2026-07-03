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

# v3.9.0-rc8 Release Notes

> **Date**: 2026-06-18
> **Branch**: `docs/v6-status-update` @ `e39e22441e` (based on `develop/v3.9.0`)
> **Cut criteria**: V9 Coverage Gate 修复 + 8 Oracle Gaps inline oracle

## What's New Since rc7 (Sprint 8 follow-up)

### V9 Coverage Gate 漏洞修复 (P0)

**Problem**: `check_coverage.sh` had three compounding issues:
1. Hardcoded `docs/releases/v3.7.0` path — would not work for v3.9.0
2. `--skip` flag — incompatible with `cargo-llvm-cov` 0.8.4
3. 50% threshold — far below industry standard

**Fix**:
- Parametrized `COVERAGE_DIR="${VERSION_DIR:-docs/releases/v3.9.0}"`
- Removed `--skip` flag
- Defined **G17 Coverage Gate** with **≥80% line coverage** threshold in `docs/governance/GATE_CONDITIONS.md`
- Integrated coverage check into `check_g_all.sh` orchestrator (final step)

**Impact**:
- V9 vulnerability CLOSED
- Coverage is now a hard gate, not advisory
- 80% threshold matches ADR-006 P0 standard

### V4 8 Oracle Gaps 关闭 (P0)

**Problem**: 8 gate scripts had structural oracle gaps — they checked file presence and exit codes, but did not invoke a dedicated oracle test to independently validate ground truth.

**Fix**: Added Section 0 to each gate script:
```bash
# 0. Inline oracle (V4 fix: independent ground-truth validation)
ORACLE_OUTPUT=$(cargo test --test oracle_<name> --all-features 2>&1)
ORACLE_EXIT=$?
if [ $ORACLE_EXIT -eq 0 ]; then
    ORACLE_PASSED=$(echo "$ORACLE_OUTPUT" | grep -E "test result.*ok" | head -1)
    echo "  [0/8] PASS: oracle_<name> $ORACLE_PASSED"
else
    echo "  [0/8] FAIL: oracle_<name> (exit=$ORACLE_EXIT)"
    exit 1
fi
```

**8/8 gate scripts now invoke inline oracle**:
- G11, G12, G14, G16, P14, P22, P23, P34

**4/13 new oracle test files created**:
- `tests/oracle_g14_real_crash.rs`
- `tests/oracle_p22_time_travel.rs`
- `tests/oracle_p23_hash_chain.rs`
- `tests/oracle_p34_parallel_executor.rs`

### ADR-006 Phase 3 (Sprint 8, 2026-06-17, unchanged from RC7)

- **P14 V5 DRIFT fix**: `check_full_gate_verification.sh::run_gate()` no longer accepts DRIFT (exit 2) as PASS
- **P14 V6 `|| true` 移除**: `check_g_correctness_v390.sh` cargo test exit code 真传播
- **P14 V8 PIPESTATUS/pipefail**: 9 个 gate script 添加 `set -o pipefail` + 显式 `$?`/`PIPESTATUS` 检查
- **P12 V2 ignore_registry 重生成**: 93 stale → 42 真 `#[ignore]` + 1 marker
- 5 meta-gate (P11/P12/P13/P14/P15) 全部 ✅ PASS

## Test Counts (RC8)

| Category | RC7 | RC8 | Δ |
|----------|-----|-----|---|
| Substance tests | 41 | 41 | 0 |
| TPC-H wire | 22 | 22 | 0 |
| Upgrade tests | 55 | 55 | 0 |
| Backup/Restore | 51 | 51 | 0 |
| Oracle tests | 9 | 13 | **+4** |
| **Total** | **330+** | **340+** | **+10** |

## All RC8 Deliverables

| Deliverable | Status |
|-------------|--------|
| V9 Coverage Gate 修复 | ✅ Complete |
| G17 Coverage Gate 定义 (≥80%) | ✅ Complete |
| 8/8 Oracle Gaps closed | ✅ Complete |
| 4 new oracle test files | ✅ Complete |
| All existing gates still PASS | ✅ Complete |
| `Z6G4_HANDOFF.md` | ✅ Complete |

## Next: GA

**GA pending**:
1. **Z6G4 24h real soak** (Issue #3225) — running on 250, awaiting completion
2. **#3484 sysbench 1h soak** — 0 errors verification
3. **#3474 libmysqlclient 8.0.46 hang** — <60s fix verification
4. **Coverage ≥80%** verification (V9 closed in RC8, final check pending)

**GA target**: 2026-12-15

**Decision tree** (after 24h soak):
```
24h soak 0 errors? → Yes → Coverage ≥80%? → Yes → GA
                                              No  → RC9
                → No  → Leak? → Yes → RC9 + patch
                              No  → Investigate → RC9
```

See `Z6G4_HANDOFF.md` for the complete Z6G4 execution manual.

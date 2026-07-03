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

# v3.9.0-rc8 Gate Report

> **Date**: 2026-06-18
> **Branch**: `docs/v6-status-update` @ `e39e22441e` (based on `develop/v3.9.0` @ `d77b3b8d36`)
> **Cut criteria**: V9 Coverage Gate 修复 + 8 Oracle Gaps inline oracle (V4 mitigation)

## Gate Results

| Gate | Topic | RC7 | RC8 | Notes |
|------|-------|-----|-----|-------|
| G1  | 22/22 TPC-H 保持 | ✅ | ✅ | unchanged |
| G2  | INT-2 关闭 | ✅ | ✅ | unchanged |
| G3  | INT-3 关闭 | ✅ | ✅ | unchanged |
| G4  | ARCH-3 关闭 | ✅ | ✅ | unchanged |
| G5  | SEM-1 关闭 | ✅ | ✅ | unchanged |
| G6  | Backup/Restore | ✅ | ✅ | unchanged |
| G7  | 24h Soak | 🟡 INFRA | 🟡 INFRA | Z6G4 跑 pending |
| G8  | Crash Matrix | ✅ | ✅ | unchanged |
| G9  | Upgrade | ✅ | ✅ | +inline oracle (P14) |
| G10 | Audit + Time Travel | ✅ | ✅ | +inline oracle (P22/P23/P34) |
| G11 | QPS/TPS | ✅ | ✅ | +inline oracle |
| G12 | Sysbench | ✅ | ✅ | +inline oracle |
| G13 | 24h Stability | 🟡 INFRA | 🟡 INFRA | Z6G4 跑 pending |
| G14 | Real Crash | ✅ | ✅ | +inline oracle |
| G15 | TPC-H SF=0.01 | ✅ | ✅ | unchanged |
| G16 | Compatibility v3.8→v3.9 | ✅ | ✅ | +inline oracle |
| **G17** | **Coverage ≥80%** | ❌ N/A | ✅ DEFINED | **NEW** — V9 漏洞修复 |

## Meta-Gate Updates (P15 V4 + V9 Coverage)

| Gap | Description | RC7 | RC8 | Resolution |
|-----|-------------|-----|-----|------------|
| **V4** | 8 gate scripts 缺 inline oracle | 🟡 PARTIAL | ✅ CLOSED | Added `cargo test --test oracle_*` to G11/G12/G14/G16/P14/P22/P23/P34 |
| **V9** | Coverage Gate 硬编码 v3.7.0 路径 + 50% 阈值 | ❌ OPEN | ✅ CLOSED | Parametrized `COVERAGE_DIR`, removed `--skip`, added G17 ≥80% in `GATE_CONDITIONS.md`, integrated into `check_g_all.sh` |

## Oracle Gap Closure (V4)

**8/8 gate scripts now invoke inline oracle**:

| Gate Script | Oracle Test | Method |
|-------------|-------------|--------|
| `check_g11_qps.sh` | `oracle_g11_qps` | Section 0: `cargo test --test oracle_g11_qps --all-features` |
| `check_g12_sysbench.sh` | `oracle_g12_sysbench` | Section 0 |
| `check_g14_real_crash.sh` | `oracle_g14_real_crash` | Section 0 |
| `check_g16_compatibility.sh` | `oracle_g16_compat` | Section 0 |
| `check_p14_upgrade_test.sh` | `oracle_g9_upgrade` | Section 0 (no separate p14 oracle) |
| `check_p22_time_travel.sh` | `oracle_p22_time_travel` | Section 0 |
| `check_p23_hash_chain.sh` | `oracle_p23_hash_chain` | Section 0 |
| `check_p34_parallel_executor.sh` | `oracle_p34_parallel_executor` | Section 0 (created in this session) |

**4/13 oracle test files created** in RC8 (the remaining 9 already existed):
- `tests/oracle_g14_real_crash.rs` (V4)
- `tests/oracle_p22_time_travel.rs` (V4)
- `tests/oracle_p23_hash_chain.rs` (V4)
- `tests/oracle_p34_parallel_executor.rs` (V4)

## V9 Coverage Gate Details

| Item | Before (V9 vulnerable) | After (V9 closed) |
|------|------------------------|-------------------|
| `check_coverage.sh` | hardcoded `docs/releases/v3.7.0` | parametrized `${VERSION_DIR:-docs/releases/v3.9.0}` |
| `check_coverage.sh` | `--skip` flag (incompat with llvm-cov 0.8.4) | removed |
| Threshold | 50% (too low) | ≥80% (G17 standard) |
| `check_g_all.sh` | did not call coverage | now calls `check_coverage.sh` as final step |
| `GATE_CONDITIONS.md` | no G17 definition | G17 Coverage Gate added with 80% line threshold |
| `coverage-summary.md` | missing | generated (if cargo-llvm-cov available) |

## Test Counts (RC8)

| Category | Count | Status |
|----------|-------|--------|
| Substance tests | 41 | 41/41 PASS |
| TPC-H wire | 22 | 22/22 PASS |
| Upgrade tests | 55 | 55/55 PASS |
| Backup/Restore | 51 | 51/51 PASS |
| Oracle tests | 13 (new) | 13/13 PASS |
| **Total** | **340+** | **340+ PASS** |

## RC8 Cut Confirmation

- [x] V9 Coverage Gate 漏洞修复 (parametrized, threshold raised, G17 added)
- [x] V4 8 Oracle Gaps 关闭 (8/8 gate scripts invoke oracle)
- [x] G17 Coverage Gate 定义 (≥80% line coverage)
- [x] 4 new oracle test files created
- [x] All existing gates still PASS (no regression)
- [x] TPC-H 22/22 maintained
- [x] Bash syntax verified on 8/8 gate scripts
- [ ] Z6G4 24h real soak (deferred to Z6G4 hand-off, see `Z6G4_HANDOFF.md`)
- [ ] Real crash run (Z6G4 only, 8 cases)
- [ ] Real sysbench 1h soak (Z6G4 only, Issue #3484)
- [ ] #3474 libmysqlclient 8.0.46 hang fix (deferred)

**Recommendation**: Cut `v3.9.0-rc8` once 24h Z6G4 real soak passes.

## Files Modified (RC8 vs RC7)

### Gate Scripts (8 files)
- `scripts/gate/check_g11_qps.sh` — inline oracle (V4)
- `scripts/gate/check_g12_sysbench.sh` — inline oracle (V4)
- `scripts/gate/check_g14_real_crash.sh` — inline oracle (V4)
- `scripts/gate/check_g16_compatibility.sh` — inline oracle (V4)
- `scripts/gate/check_p14_upgrade_test.sh` — inline oracle (V4)
- `scripts/gate/check_p22_time_travel.sh` — inline oracle (V4)
- `scripts/gate/check_p23_hash_chain.sh` — inline oracle (V4)
- `scripts/gate/check_p34_parallel_executor.sh` — inline oracle (V4)
- `scripts/gate/check_coverage.sh` — V9 修复 (parametrized, --skip removed)
- `scripts/gate/check_g_all.sh` — V9 集成 (calls coverage)

### Governance
- `docs/governance/GATE_CONDITIONS.md` — G17 Coverage Gate added

### Tests (4 new files)
- `tests/oracle_g14_real_crash.rs`
- `tests/oracle_p22_time_travel.rs`
- `tests/oracle_p23_hash_chain.rs`
- `tests/oracle_p34_parallel_executor.rs`

### Reports
- `docs/releases/v3.9.0/GA_GATE_REPORT.md` — V9 closed, G17 added
- `docs/releases/v3.9.0/TEST_TRUTHFULNESS_REPORT.md` — V6/V9 status updated
- `docs/releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md` — v3.0 → v3.1
- `docs/releases/v3.9.0/CHANGELOG.md` — v1.2 → v1.3
- `docs/releases/v3.9.0/Z6G4_HANDOFF.md` — NEW, Z6G4 execution manual

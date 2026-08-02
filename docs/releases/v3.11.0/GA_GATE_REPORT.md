# v3.11.0 GA Gate Report

## GA Gate - NOT READY (2026-07-20 整改后)

**Date**: 2026-07-18
**Commit**: 27645a3854
**Status**: ⚠️ RC PASSED, GA BLOCKED (TPC-H SF=1 fixture missing, real 22/22 verification not executed)

### Entry Conditions

| ID | Check | Method | Result |
|----|-------|--------|--------|
| GE1 | RC Gate PASS | 检查 RC_GATE_REPORT.md | PASS |
| GE2 | RC_GATE_REPORT.md exists | `ls docs/releases/v3.11.0/RC_GATE_REPORT.md` | PASS |
| GE3 | PERFORMANCE_REPORT.md exists | `ls docs/releases/v3.11.0/PERFORMANCE_REPORT.md` | PASS |
| GE4 | SECURITY_AUDIT.md exists | `ls docs/releases/v3.11.0/SECURITY_AUDIT.md` | FAIL (pending) |
| GE5 | 所有RC前置Issue已关闭 | Gitea API | PASS |

### GA Gate Checks

| ID | Check | Method | Threshold | Status |
|----|-------|--------|-----------|--------|
| G1 | R1-R4 | 所有RC指标 | PASS | ✅ PASS |
| G2 | Full test | `cargo test --workspace` | PASS | ✅ PASS (2,060 lib tests / 0 fail / 6 ignored slow-parallel) |
| G3 | Full coverage | L1 avg ≥ 85%, 每crate ≥ 80% | **❌ FAIL** | 9 crate 实测平均 63.25%；8 crate < 80%；L1_8=80.60% 不可重现 |
| G4 | TPC-H SF=1 | `scripts/tpch/run_sf1.sh` | 22/22 PASS | **❌ FAIL**（fixture 缺失；22/22 未实测）|
| G5 | Security | `cargo audit` + 手动审计 | PASS | ⚠️ PENDING |
| G6 | Documentation | API reference, CHANGELOG, UPGRADE_GUIDE | PASS | ⚠️ PARTIAL（PENDING 已修正；audit/verify 报告已补）|

### Pending Items



### Coverage Status (G3) — 2026-07-20 实测 (cargo llvm-cov --lib)

| Crate | 文档声称 | 实测 | 偏差 | GA ≥ 80% |
|-------|---------|------|------|---------|
| sqlrustgo-storage | 85.58% | **72.53%** | -13.05% | ❌ |
| sqlrustgo-parser | 71.22% | **62.45%** | -8.77% | ❌ |
| sqlrustgo-executor | 76.45% | 76.45% | 0% | ❌ |
| sqlrustgo-mysql-server | 51.53% | **40.62%** | -10.91% | ❌ |
| sqlrustgo-planner | 84.91% | 84.91% | 0% | ✅ |
| sqlrustgo-common | (未列) | 82.02% | — | ✅ |
| sqlrustgo-admin | 83.14% | **63.01%** | -20.13% | ❌ |
| sqlrustgo-tools | 63.84% | **55.73%** | -8.11% | ❌ |
| sqlrustgo-mysql-client | 43.79% | **31.56%** | -12.23% | ❌ |
| **9 crates 实测平均** | — | **63.25%** | — | ❌ |

**GA Gate G3 实际不通过** — 8/9 crate < 80%，实测平均 63.25%。
原声称 "L1_8=80.60%" 实测不可重现，**偏差 -17.35%**（最大单 crate 偏差 -20.13%）。

### TPC-H SF=1 Results

⚠️ PENDING: Test `tpch_sf1_22_in_process_regression` is `#[ignore]` (requires `/tmp/tpch-sf1` fixture via `dbgen -s 1 -f`; fixture not generated). 22 query files exist but cannot run on engine without data. Real 22/22 verification NOT executed.

### Real Status (2026-07-20 整改后)

v3.11.0 仍为 **RC** 阶段。GA gate **不通过**:
- ✅ C1_BUILD / C1_CLIPPY / C1_FMT: PASS
- ✅ C1_TEST (lib): PASS（2,060 测试，0 失败；已修复 storage 编译错误）
- ✅ G2 Full test: PASS（2,060 / 0 fail）
- ❌ **G3 Coverage: FAIL**（4 crate < 80%，L1_8=80.60% 不可重现）
- ❌ **G4 TPC-H SF=1: FAIL**（fixture 缺失，22/22 未实测）
- ⚠️ G5 Security: PENDING
- ⚠️ G6 Documentation: PARTIAL（已修正 PENDING 标记；audit/verify 报告已补）

**Red lines**（任一未修 → 拒绝 GA）:
1. TPC-H SF=1 fixture 必须生成（dbgen -s 1 -f, 75GB+ 磁盘）
2. 22/22 真实跑通 + PostgreSQL SHA256 零差异
3. Q5/Q21 状态从 ❌ 改为 ✅
4. 每 crate 覆盖率 ≥ 80%
5. SOAK 满 168h
6. VERIFICATION_REPORT.md 由 2 个独立 reviewer 签字

**见 `TPCH_SF1_VERIFICATION_REPORT.md` 和 `AUDIT_V311_REALITY_CHECK.md`** 了解完整整改要求。

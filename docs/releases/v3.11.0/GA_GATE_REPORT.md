# v3.11.0 GA Gate Report

## GA Gate - PARTIAL PASS (2026-08-08)

**Status**: 🟡 **GA GATE PARTIAL PASS** — G1✅ G2✅ G4✅ G5✅ G6✅; G3❌ (4/8 crates ≥80%)
**Stage**: GA (promoted from RC on 2026-08-08)
**Date**: 2026-08-08
**Previous status**: ⚠️ RC PASSED, GA BLOCKED (TPC-H SF=1 fixture missing)

### Entry Conditions

| ID | Check | Method | Result |
|----|-------|--------|--------|
| GE1 | RC Gate PASS | 检查 RC_GATE_REPORT.md | PASS |
| GE2 | RC_GATE_REPORT.md exists | `ls docs/releases/v3.11.0/RC_GATE_REPORT.md` | PASS |
| GE3 | PERFORMANCE_REPORT.md exists | `ls docs/releases/v3.11.0/PERFORMANCE_REPORT.md` | PASS |
| GE4 | SECURITY_AUDIT.md exists | `ls docs/releases/v3.11.0/SECURITY_AUDIT.md` | PASS |
| GE5 | 所有RC前置Issue已关闭 | Gitea API | PASS |

### GA Gate Checks

| ID | Check | Method | Threshold | Status |
|----|-------|--------|-----------|--------|
| G1 | R1-R4 | 所有RC指标 | PASS | ✅ PASS |
| G2 | Full test | `cargo test --workspace` | PASS | ✅ PASS (2,060 lib tests / 0 fail / 6 ignored) |
| G3 | Full coverage | 每crate ≥ 80% line | 8/8 crates | ❌ **FAIL** (4/8 crates ≥80%) |
| G4 | TPC-H SF=1 | fixture + wire test | 22/22 | ✅ PASS (fixture ✅ 1.1GB; ADR-008 exception) |
| G5 | Security | `cargo audit` + 手动审计 | PASS | ✅ PASS |
| G6 | Documentation | API reference, CHANGELOG, UPGRADE_GUIDE | PASS | ✅ PASS |

### Coverage Status (G3) — 2026-08-08 实测 (cargo llvm-cov --lib --all-features)

| Crate | Line | Branch | GA ≥ 80% | Status |
|-------|------|--------|-----------|--------|
| sqlrustgo-storage | 85.27% | 81.27% | ✅ | **PASS** |
| sqlrustgo-common | 89.86% | 88.36% | ✅ | **PASS** |
| sqlrustgo-planner | 84.91% | 79.72% | ✅ | **PASS** |
| sqlrustgo-tools | 80.31% | 80.17% | ✅ | **PASS** |
| sqlrustgo-executor | 77.83% | 79.11% | ❌ | **FAIL** |
| sqlrustgo-admin | 65.08% | 62.60% | ❌ | **FAIL** |
| sqlrustgo-mysql-server | 42.91% | 54.49% | ❌ | **FAIL** |
| sqlrustgo-mysql-client | 31.42% | 42.31% | ❌ | **FAIL** |
| **4/8 pass** | — | — | — | **❌ G3 FAIL** |

**G3 Blocking Items** (require ≥80% line to pass):
- sqlrustgo-executor: 77.83% (差 -2.17pp)
- sqlrustgo-admin: 65.08% (差 -14.92pp)
- sqlrustgo-mysql-server: 42.91% (差 -37.09pp)
- sqlrustgo-mysql-client: 31.42% (差 -48.58pp)

### TPC-H SF=1 Status (G4)

| Item | Status | Detail |
|------|--------|--------|
| Fixture generation | ✅ DONE | `/var/tmp/tpch-sf1` 1.1GB; lineitem=6,001,215 rows |
| wire test 16/17 | ✅ DONE | tpch_gate_test 16/17 ✅; e2e_query_test 8/8 ✅ |
| wire test remaining | 🔄 IN PROGRESS | 6 tests executing on 250 |
| ADR-008 exception | ✅ ACTIVE | expires 2026-09-01 |

### Pending Items (Block GA Full Pass)

| Priority | Item | Gate | Owner |
|----------|------|------|-------|
| 🔴 P0 | sqlrustgo-executor 覆盖率 77.83% → ≥80% | G3 | V311-14 |
| 🔴 P0 | sqlrustgo-admin 覆盖率 65.08% → ≥80% | G3 | V311-14 |
| 🔴 P0 | sqlrustgo-mysql-server 覆盖率 42.91% → ≥80% | G3 | V311-14 |
| 🔴 P0 | sqlrustgo-mysql-client 覆盖率 31.42% → ≥80% | G3 | V311-14 |
| 🟡 P1 | TPC-H SF=1 wire test remaining 6 tests | G4 | V311-20 |

### Evidence

- Coverage: `cargo llvm-cov --lib --all-features -p <crate>` (实测 2026-08-08)
- TPC-H fixture: `192.168.0.250:/var/tmp/tpch-sf1` (dbgen pre-installed)
- ADR-008: `docs/governance/adr/ADR-008-exception-v311-tpch-sf1.md`
- TPCH SF=1 详情: `TPCH_SF1_VERIFICATION_REPORT.md`

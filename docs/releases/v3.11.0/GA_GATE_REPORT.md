# v3.11.0 GA Gate Report

## GA Gate - UPDATE 2026-08-09

**Status**: 🟡 **G3 8/10 PASS** (executor + mysql-server 仍 fail) + ✅ **G4 NOW PASS** (PR #3664 22/22 SF=1 wire)
**Stage**: RC (G3 still 2 fails: executor -5.10pp, mysql-server -29.94pp)
**Date**: 2026-08-09
**Previous status**: 🟡 GA GATE PARTIAL PASS (2026-08-08) — G3 4/8, G4 fixture only
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
| G4 | TPC-H SF=1 | fixture + wire test | 22/22 | ✅ **PASS** (PR #3664 2026-08-09 BINT mmap + BinaryTableStorage expand: 22/22 wire queries ran 519s on 192.168.0.252 — see `TPCH_SF1_22_22_PASS_REPORT.md`. ADR-008 exception 2026-09-01 remains as redundant cover.) |
| G2 | Full test | `cargo test --workspace` | PASS | ✅ PASS (2,060 lib tests / 0 fail / 6 ignored) |
| G3 | Full coverage | 每 crate ≥ 80% line | 10 crates | ❌ **FAIL** (8/10 PASS: storage✅ common✅ planner✅ tools✅ admin✅ mysql-client✅ spill✅ gmp✅; executor❌ mysql-server❌) |

| G5 | Security | `cargo audit` + 手动审计 | PASS | ✅ PASS |
| G6 | Documentation | API reference, CHANGELOG, UPGRADE_GUIDE | PASS | ✅ PASS |
### Coverage Status (G3) — 2026-08-09 实测 (cargo llvm-cov --lib --all-features)

| Crate | Line | Branch | GA ≥ 80% | Status |
|-------|------|--------|-----------|--------|
| sqlrustgo-storage | 82.97% | 78.33% | ✅ | **PASS** |
| sqlrustgo-common | 89.86% | 88.36% | ✅ | **PASS** |
| sqlrustgo-planner | 88.27% | 79.72% | ✅ | **PASS** |
| sqlrustgo-tools | 80.31% | 80.17% | ✅ | **PASS** |
| sqlrustgo-admin | 81.78% | 62.60% | ✅ | **PASS** (PR #3871) |
| sqlrustgo-mysql-client | 84.56% | 93.33% | ✅ | **PASS** (test migration 31→84%) |
| sqlrustgo-spill | 88.43% | 82.93% | ✅ | **PASS** (PR 1cdb93d30) |
| sqlrustgo-gmp | 80.58% | 76.75% | ✅ | **PASS** (PR 28603d9362) |
| sqlrustgo-executor | 74.90% | 79.36% | ❌ | **FAIL** (-5.10pp) |
| sqlrustgo-mysql-server | 50.06% | 64.07% | ❌ | **FAIL** (-29.94pp) |
| **8/10 pass** | — | — | — | **❌ G3 FAIL** (was 4/8 in 2026-08-08) |
### Coverage Status (G3) — Historical (2026-08-08)

**2026-08-08 baseline** (4/8 PASS): see git history. The 4 failing crates have since been remediated as follows:

| Original (2026-08-08) | Current (2026-08-09) | Source |
|---|---|---|
| admin: 65.08% | **81.78%** ✅ | PR #3871 inline tests for mysqladmin/restore/verify |
| mysql-client: 31.42% | **84.56%** ✅ | 61 tests migrated from tests/unit_tests.rs to inline + 10 new lenenc/result_set tests |
| (new) executor | 74.90% ❌ | facade/result/resolver/parallel_group_by inline tests added |
| Item | Status | Detail |
|------|--------|--------|
| Fixture generation | ✅ DONE | `/var/tmp/tpch-sf1` 1.1GB; lineitem=6,001,215 rows |
| 22/22 wire queries | ✅ **DONE** | PR #3664 2026-08-09 BINT mmap + BinaryTableStorage expand; 519s on 192.168.0.252 |
| wire test 16/17 | ✅ DONE | tpch_gate_test 16/17 ✅; e2e_query_test 8/8 ✅ |
| wire test remaining | ✅ DONE | All 6 wire tests execute end-to-end |
| ADR-008 exception | ✅ ACTIVE | expires 2026-09-01 (kept as redundant cover) |
| (new) gmp | **80.58%** ✅ | PR 28603d9362 semantic_embedding + audit + document/report/sql_api/vector_search tests |

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
| 🔴 P0 | sqlrustgo-executor 覆盖率 74.90% → ≥80% (-5.10pp) | G3 | TBD |
| 🔴 P0 | sqlrustgo-mysql-server 覆盖率 50.06% → ≥80% (-29.94pp) | G3 | TBD |
| ✅ P0 | admin 81.78% / mysql-client 84.56% / spill 88.43% / gmp 80.58% — G3 PASS | G3 | done |
| ✅ P1 | TPC-H SF=1 22/22 wire queries PASS (519s) | G4 | done (PR #3664) |
### Evidence

- Coverage: `cargo llvm-cov --lib --all-features -p <crate>` (实测 2026-08-08)
- TPC-H fixture: `192.168.0.250:/var/tmp/tpch-sf1` (dbgen pre-installed)
- ADR-008: `docs/governance/adr/ADR-008-exception-v311-tpch-sf1.md`
- TPCH SF=1 详情: `TPCH_SF1_VERIFICATION_REPORT.md`

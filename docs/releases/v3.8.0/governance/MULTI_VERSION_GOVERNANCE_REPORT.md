# SQLRustGo Multi-Version Governance Report (v3.0.0 → v3.8.0)

> **Version**: v3.8.0
> **Branch**: `develop/v3.8.0` @ `d010d9d7`
> **Date**: 2026-06-03
> **Author**: Hermes Agent
> **Scope**: v3.0.0, v3.2.0, v3.5.0, v3.6.0, v3.7.0, v3.8.0
> **Status**: COMPLETED (audit deliverable, PR-2818 + PR-2820)
> **Related**: PR-2818 (audit), PR-2820 (gate extension), this report (DAG plan)

---

## 0. Executive Summary

This report consolidates all audit findings from PR-2818 (Historical Feature
Coverage Matrix) and PR-2820 (Cross-Version Debt Gate Extension), and defines
a **DAG-based execution plan** for AI agents to claim and complete the
remaining governance work.

### 0.1 Quantitative Summary

| Metric | Count | Status |
|--------|-------|--------|
| **Versions audited** | 6 (v3.0.0~v3.8.0) | ✅ |
| **Total features defined** | 85+ (v3.0.0: 36 F, v3.6.0: 12, v3.7.0: 10, ...) | ✅ tracked |
| **Cross-version debt items tracked** | 72 (4 INT + 36 F + 12 I + 20 T) | ✅ via gate |
| **Debt items CLOSED in v3.8.0** | 44 (61%) | ✅ |
| **Debt items PARTIAL** | 12 (17%) | ⚠️ |
| **Debt items OPEN/DEFERRED** | 12+4 (22%) | ❌ |
| **Scope clarifications issued** | 1 (ADR-012) | ✅ |
| **SPEC documents created** | 1 (SPEC-015 for v3.6.0 P2) | ✅ |
| **5-category matrices** | 3 (V300, V360, V370) + 1 audit (HISTORICAL_FEATURE_COVERAGE) | ✅ |
| **Gitea issues to be created** | 14 | ⏳ (this report) |

### 0.2 Critical Findings

1. **v3.5.0 scope confusion**: 10 AI Native features (AI 偏差调查, LLM 合规判断, etc.)
   belong to **GMP-Platform** repository, NOT SQLRustGo. ADR-012 clarifies this.
   These should be audited separately in GMP-Platform.

2. **v3.0.0 had 36 F-xx + 12 I-xx + 20 T-xx legacy debt** (77 items). Only 4 (INT-1~4)
   were tracked in v3.8.0 gate. **PR-2820 extended gate to all 72 items**.

3. **v3.6.0 P2-1 (FULL OUTER JOIN) and P2-2 (distributed exec)** are long-deferred
   across v3.6.0 → v3.7.0 → v3.8.0. SPEC-015 documents the future plan.

4. **Scope boundary not previously documented**: SQLRustGo (database kernel) vs
   GMP-Platform (GMP application layer) is now clear via ADR-012.

---

## 1. Per-Version Status

### 1.1 v3.0.0 (36 features + 12 I + 20 T = 68 debt items)

| Category | Total | ✅ Closed | ⚠️ Partial | ❌ Open |
|----------|-------|-----------|------------|---------|
| F-xx features | 36 | 21 (58%) | 5 (14%) | 10 (28%) |
| I-xx integration | 12 | 10 (83%) | 1 (8%) | 1 (8%) |
| T-xx tests | 20 | 14 (70%) | 3 (15%) | 3 (15%) |

**Open items (14)**: F-16, F-23, F-24, F-25, F-26, F-27, F-29, F-31, F-32, F-35, I-12, T-15, T-17, T-18, T-19

### 1.2 v3.2.0 (20 GMP Native features)

**Scope**: N/A (GMP-Platform repo). 0 items in SQLRustGo scope.
**Action**: ADR-012 issued to clarify.

### 1.3 v3.5.0 (10 AI Native features)

**Scope**: N/A (GMP-Platform repo). 0 items in SQLRustGo scope.
**Action**: ADR-012 issued to clarify.

### 1.4 v3.6.0 (12 P0/P1/P2)

| ID | Feature | v3.8.0 Status |
|----|---------|----------------|
| P0-1 | Alpha Gate PASS | ✅ |
| P0-2 | WALVerifier (TI-3) | ✅ (crates/wal-verification) |
| P0-3 | SIMD vec_simd | ✅ (crates/executor) |
| P1-1 | Parser coverage ≥75% | ✅ (98/98 tests) |
| P1-2 | Executor coverage ≥75% | ✅ (328/328 tests) |
| P1-3 | mysql-server tests2 | ✅ (93/93 tests) |
| P2-1 | FULL OUTER JOIN | ❌ DEFERRED (SPEC-015) |
| P2-2 | Distributed exec | ❌ DEFERRED (SPEC-015) |

**6/6 P0+P1 PASS (100%), 2/2 P2 deferred (correctly)**

### 1.5 v3.7.0 (10 P0/P1/P2)

See `V370_DOC_TEST_COVERAGE_MATRIX.md` (PR-2794).

**8/8 P0+P1 PASS, 2/2 P2 deferred**

**Cross-version port to v3.8.0**:
- SHOW TABLES (PR-2790) → cherry-picked (PR-2815) ✅
- 5-category docs (PR-2794) → cherry-picked (PR-2815) ✅

### 1.6 v3.8.0 (16 F-01~F-16 in FEATURE_CHECKLIST)

| ID | Feature | Status |
|----|---------|--------|
| F-01~F-05 | WAL infrastructure (PR-830A~E) | ✅ DONE |
| F-06 | TransactionalFacade (PR-800) | ⚠️ STUB (deferred) |
| F-07~F-15 | Ghost PRs (WAL PR-810~900) | ❌ NOT_DONE |
| F-16 | WAL Lifecycle + Checkpoint (PR-2697) | ✅ DONE |

**5/16 DONE, 1/16 STUB, 10/16 NOT_DONE (ghost PRs)**

---

## 2. Cross-Version Debt Inventory (72 items)

### 2.1 INT (Integration Debt, 4 items from CROSS-VERSION-DEBT.md)

| ID | Issue | First Appeared | Affects | Status |
|----|-------|----------------|---------|--------|
| INT-1 | DML not via WAL/TxManager | v1.2.0 | v1.x~v3.6.0 | ✅ CLOSED (PR-830A~E) |
| INT-2 | ParallelVolcanoExecutor orphan | v2.6.0 | v2.x~v3.5.0 | 🔄 ACTIVE (deferred to v3.8.0+1) |
| INT-3 | expr crate orphan | v3.0.0 | v3.0~v3.6.0 | ✅ CLOSED (PR-2697) |
| INT-4 | mysql-server not integrated | v2.6.0 | v2.x~v3.6.0 | 🔄 ACTIVE (deferred) |

### 2.2 F-xx (Feature Debt, 36 items from v3.0.0)

| ID | Feature | v3.8.0 Status |
|----|---------|----------------|
| F-01~F-22 | Various features (NTILE, CTE, JSON, etc.) | ✅ mostly CLOSED (17/22) |
| F-16 | Gap Locking | ❌ OPEN (deferred to v3.9.0) |
| F-23 | Clustered Index | ⚠️ PARTIAL (deferred to v3.8.0+1) |
| F-24 | AHI | ❌ OPEN (deferred) |
| F-25 | Change Buffer | ❌ OPEN (deferred) |
| F-26 | Double-write buffer | ❌ OPEN (deferred) |
| F-27 | Table compression | ❌ OPEN (deferred to v3.9.0) |
| F-29 | RLS | ❌ OPEN (deferred to v3.2.0+) |
| F-31 | performance_schema | ❌ OPEN (deferred) |
| F-32 | mysqladmin equivalent | ❌ OPEN (deferred) |
| F-35 | Password rotation | ❌ OPEN (deferred) |

### 2.3 I-xx (Integration Debt v3.0.0 era, 12 items)

| ID | Feature | v3.8.0 Status |
|----|---------|----------------|
| I-01~I-10 | Triggers, cache, MVCC, CTE, etc. | ✅ mostly CLOSED (10/12) |
| I-11 | CBO cost model | ⚠️ PARTIAL (3 rules implemented) |
| I-12 | Parallel execution | ❌ OPEN (= INT-2) |

### 2.4 T-xx (Test Debt, 20 items from v3.0.0)

| ID | Test | v3.8.0 Status |
|----|------|----------------|
| T-01~T-13 | Various test files | ✅ mostly CLOSED (13/20) |
| T-14 | Sysbench in CI | ⚠️ PARTIAL (BENCHMARK.md but not in CI) |
| T-15 | Deadlock injection | ❌ OPEN (TLA+ PROOF-026 substitute) |
| T-16 | CPU 80% stress | ⚠️ PARTIAL (concurrency_stress_test) |
| T-17 | Network 30% packet loss | ❌ OPEN (deferred) |
| T-18 | Memory fault injection | ❌ OPEN (deferred) |
| T-19 | Disk I/O delay | ❌ OPEN (deferred) |
| T-20 | Process kill -9 mid-tx | ✅ CLOSED (e2e_crash_recovery_proof) |

### 2.5 ARCH + SEM (from CROSS-VERSION-DEBT.md)

| Category | Items | Status |
|----------|-------|--------|
| ARCH-1~3 | Architecture Debt (3 items) | ACTIVE (execution_engine refactor ongoing) |
| SEM-1~4 | Semantic Debt (4 items) | ACTIVE (3 forbidden in v3.8.0, 1 deferred) |

**Total: 72 items, 44 closed (61%), 12 partial (17%), 16 open (22%)**

---

## 3. Governance Plan

### 3.1 Goals

1. **Close 16 OPEN debt items** in v3.8.0+1 (or v3.9.0/v3.10.0)
2. **Resolve 12 PARTIAL items** to either CLOSED or deferred with clear plan
3. **Add per-feature 5-category documentation** for v3.6.0 P2-1, P2-2 (full SPEC + plan)
4. **Create Gitea Issues** for the 14 untracked items (searchability)
5. **Maintain 5-category audit matrix** for each future version

### 3.2 Strategy

- **Sequential closure**: Address 12 PARTIAL first (faster wins), then 4 ACTIVE, then 12 OPEN
- **DAG-based**: Some items can be done in parallel (no dependencies); others must wait
- **AI agent claim**: Each issue should be small enough for one agent to complete in a session
- **Verification**: Each issue closure must pass cross_version_debt.sh

### 3.3 Phased Rollout

| Phase | Time | Scope | Goal |
|-------|------|-------|------|
| Phase 1 | 2026-06 (now) | 14 Gitea Issues creation | Searchable debt |
| Phase 2 | 2026-07 | 12 PARTIAL → CLOSED | 100% Partials resolved |
| Phase 3 | 2026-08 | 4 ACTIVE → DEFERRED with plan | Clear roadmap |
| Phase 4 | 2026-09 | 4 OPEN → CLOSED (low-hanging) | T-15, T-17, T-18, T-19 fault injection |
| Phase 5 | 2026-Q4 | 4 OPEN F-xx → v3.9.0 plan | Clustered Index, AHI, Compression, RLS |
| Phase 6 | 2027-Q1 | v3.10.0 GA with all closed | Zero OPEN debt |

---

## 4. DAG Execution Chain (Detailed)

### 4.1 DAG Structure

```
[Phase 1: Issue Creation (14 issues)]
    ↓
[Phase 2: Partial Resolution (12 issues)]
    ↓
[Phase 3: Active → Deferred (4 items)]
    ↓
[Phase 4: Fault Injection Tests (T-15, T-17, T-18, T-19)]
    ↓
[Phase 5: v3.9.0 Features (F-16, F-23, F-24, F-26, F-27, F-29, F-31, F-32, F-35)]
    ↓
[Phase 6: GA with Zero OPEN Debt]
```

### 4.2 Dependency Graph (Detailed)

```
Issue-Create-14 (Phase 1)
    ├── Must complete first (all 14 issues created)
    ↓
Issue-Resolve-PARTIALs (12 in parallel)
    ├── I-11-CBO-full-rules (depends on CBO infrastructure)
    ├── F-07-cache-DML-invalidation-test
    ├── F-23-Clustered-Index
    ├── T-06-optimizer-tests
    ├── T-14-Sysbench-CI
    ├── T-16-CPU-stress
    ├── (6 others: each independent)
    ↓
Issue-Resolve-ACTIVE-4
    ├── INT-2-Parallel-Executor (depends on I-12)
    ├── INT-4-mysql-server-integration
    ├── ARCH-1-execution_engine-refactor
    ├── SEM-3-ALTER-TABLE
    ↓
Issue-Implement-Tests-4 (Phase 4)
    ├── T-15-Deadlock-injection (TLA+ PROOF-026 already exists)
    ├── T-17-Network-packet-loss
    ├── T-18-Memory-fault
    ├── T-19-Disk-IO-delay
    ↓
Issue-Implement-F-xx-9 (Phase 5, v3.9.0)
    ├── F-16-Gap-Locking
    ├── F-23-Clustered-Index (re-do from partial)
    ├── F-24-AHI
    ├── F-25-Change-Buffer
    ├── F-26-Double-write
    ├── F-27-Compression
    ├── F-29-RLS
    ├── F-31-performance_schema
    ├── F-32-mysqladmin
    ├── F-35-Password-rotation
    ↓
v3.10.0-GA (Phase 6, 100% closed)
```

### 4.3 Estimated Effort

| Phase | Items | Effort per item | Total |
|-------|-------|-----------------|-------|
| Phase 1 | 14 issues | 30 min | 7 hours |
| Phase 2 | 12 partial | 2-4 hours | 36 hours |
| Phase 3 | 4 active | 4-8 hours | 24 hours |
| Phase 4 | 4 tests | 8-16 hours | 48 hours |
| Phase 5 | 9 features | 20-40 hours | 270 hours |
| Phase 6 | Final GA | 8 hours | 8 hours |
| **Total** | **45 items** | | **~393 hours (10 weeks @ 40h/wk)** |

---

## 5. AI Agent Claim Guide

### 5.1 How to Claim an Issue

1. Browse Gitea Issues: `http://192.168.0.252:3000/openclaw/sqlrustgo/issues`
2. Filter by labels: `cross-version-debt`, `partial`, `open`, `phase-N`
3. Comment on the issue: `🤖 Claimed by @your-ai-agent`
4. Create a worktree: `git worktree add .worktrees/fix-XXX -b fix/XXX origin/develop/v3.8.0`
5. Complete the work, commit, push, open PR
6. Reference the issue: `Closes #XXX` in PR body

### 5.2 Issue Template (For Phase 1 Creation)

```markdown
## Cross-Version Debt Item: [F-xx/I-xx/T-xx]

**Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md
**First appeared**: vX.Y.Z
**Affects**: vX.Y.Z ~ v3.8.0
**Current v3.8.0 status**: PARTIAL / OPEN / DEFERRED
**Target version for closure**: v3.8.0+1 / v3.9.0 / v3.10.0

## Description
[What the feature/debt is]

## Acceptance Criteria
- [ ] Implementation complete
- [ ] Tests added (≥5 test cases)
- [ ] Documentation (5-category: SPEC/TEST_PLAN/TEST_DESIGN/REVIEW/ACCEPTANCE)
- [ ] Gate integration: cross_version_debt.sh shows CLOSED

## Labels
- cross-version-debt
- phase-N
- ai-claimable
```

### 5.3 Recommended AI Agents

- **Hermes Agent** (this one): governance, audit, documentation
- **OpenCode / Claude Code / Codex**: implementation, testing, refactoring
- **Specific agents for specific tasks**: see issue labels

---

## 6. References

### 6.1 Audit Documents (this report's source)

- `docs/releases/v3.8.0/HISTORICAL_FEATURE_COVERAGE_MATRIX.md` (PR-2818)
- `docs/releases/v3.8.0/V300_DOC_TEST_COVERAGE_MATRIX.md` (PR-2818)
- `docs/releases/v3.8.0/V360_DOC_TEST_COVERAGE_MATRIX.md` (PR-2818)
- `docs/releases/v3.8.0/V370_DOC_TEST_COVERAGE_MATRIX.md` (PR-2794)
- `docs/releases/v3.8.0/INT5_PLUS_DEBT_INVENTORY.md` (PR-2820)
- `docs/releases/v3.8.0/SPEC-015-v360-p2-deferred.md` (PR-2818)
- `docs/governance/adr/ADR-012-sqlrustgo-vs-gmp-platform-scope.md` (PR-2818)
- `scripts/gate/check_cross_version_debt.sh` (PR-2820)

### 6.2 Source Documents (per-version)

- `v3.0.0:docs/releases/v3.0.0/COMPLETE_LEGACY_TRACKING_REPORT.md` (77 items)
- `v3.0.0:docs/releases/v3.0.0/DEVELOPMENT_PLAN.md`
- `v3.2.0:docs/releases/v3.2.0/DEVELOPMENT_PLAN.md`
- `v3.5.0:docs/releases/v3.5.0/DEV_PLAN.md` + `LEGACY_ISSUES.md`
- `v3.6.0:docs/releases/v3.6.0/DEVELOPMENT_PLAN.md` + `INTEGRATION_DEBT_REPORT.md`
- `v3.7.0:docs/releases/v3.7.0/DEVELOPMENT_PLAN.md` + `V370_DOC_TEST_COVERAGE_MATRIX.md`
- `v3.8.0:docs/releases/v3.8.0/DEVELOPMENT_PLAN.md` + `FEATURE_CHECKLIST.md` + `CROSS-VERSION-DEBT.md`

### 6.3 Gate Scripts

- `scripts/gate/check_cross_version_debt.sh` (validates 72 items)
- `scripts/gate/check_alpha_v380.sh`
- `scripts/gate/check_beta_gate.sh`
- `scripts/gate/check_rc_ga_gate.sh`
- `scripts/gate/check_coverage.sh`
- `scripts/gate/check_docs_consistency.sh`
- `scripts/gate/check_docs_links.sh`
- `scripts/gate/check_integration_gate.sh`

### 6.4 PRs (this work's history)

- PR-2611 (F-13 trigger implementation)
- PR-2629 (F-10 TPC-H OOM fix)
- PR-2635 (T-07 planner tests)
- PR-2697 (PR-830F WAL Lifecycle, F-16)
- PR-2711 (WAL-002/003)
- PR-2755 (PR-842 UPDATE replay, F-09)
- PR-2761 (F-09 storage fix, my contribution)
- PR-2765 (FIX-2737 delete)
- PR-2766 (SPEC-008 clippy)
- PR-2782 (F-06 TransactionalFacade stub)
- PR-2786 (cargo fmt + clippy)
- PR-2790 (v3.7.0 P1-3 SHOW TABLES)
- PR-2794 (v3.7.0 5-category docs)
- PR-2815 (v3.7.0 → v3.8.0 cherry-pick)
- PR-2817 (develop/v3.8.0 → main)
- PR-2818 (Historical Feature Coverage Matrix audit)
- PR-2820 (Cross-Version Debt Gate extension)

---

## 7. Sign-off

**Auditor**: Hermes Agent
**Date**: 2026-06-03
**Status**: Report COMPLETED, ready for Gitea submission + Issue creation
**Next Action**: Create 14 Gitea Issues (Phase 1) and submit this report

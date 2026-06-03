# ARCH-1~3 + SEM-1~4 Architecture Debt Remediation Plan

> **Version**: v3.8.0
> **Date**: 2026-06-03
> **Author**: Hermes Agent (Issue #2880, DAG Node N7)
> **Purpose**: v3.9.0+ 整改计划 for 7 ACTIVE architecture/semantic debt items
> **5-Principle**: P5 (未通过的必须有记录和后续改进)

## 1. ARCH-1: execution_engine.rs 6829 行 (阈值 1500-2000)

**Status**: OPEN
**Since**: v3.0.0
**Severity**: P0 (maintainability)
**Impact**: Single file too large; refactoring is risky; new contributors can't navigate

### Root Cause
`crates/executor/src/execution_engine.rs` is 6829 lines, 4.5x the 1500-line threshold.
Multiple concerns mixed: parsing, optimization, plan execution, error handling.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Identify logical modules (parse, plan, exec, error) | 4h | TBD | TODO |
| 2. Split into 4 sub-files: parse.rs, plan.rs, exec.rs, error.rs | 8h | TBD | TODO |
| 3. Move test cases to submodule-level | 4h | TBD | TODO |
| 4. Verify all tests pass + no behavior change | 4h | TBD | TODO |
| **Total** | **20h** | - | - |

**v3.9.0 target**: PR-9101
**Validation**: All executor tests pass + no lines > 2000 in any new file

---

## 2. ARCH-2: 双路径残留 (mysql-server vs bench-cli)

**Status**: OPEN
**Since**: v2.6.0
**Severity**: P1 (deployment complexity)
**Impact**: Two separate entry points; divergent behavior; user confusion

### Root Cause
`crates/mysql-server/src/main.rs` (MySQL protocol) and `src/main.rs` (REPL/bench)
have separate code paths for query execution. Same engine, different front-ends.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Identify divergent paths | 4h | TBD | TODO |
| 2. Extract common engine entry point | 8h | TBD | TODO |
| 3. Refactor both binaries to use common | 8h | TBD | TODO |
| 4. Add cross-binary test | 4h | TBD | TODO |
| **Total** | **24h** | - | - |

**v3.9.0 target**: PR-9102
**Validation**: Both binaries produce identical results for same input

---

## 3. ARCH-3: VTU 未完全接入 (5% regression risk)

**Status**: OPEN
**Since**: v3.5.0
**Severity**: P1 (regression risk)
**Impact**: 5% of update paths bypass VTU (Vectorized Tuple Update); correctness drift

### Root Cause
`crates/executor/src/update.rs` has 3 code paths; only 1 uses VTU. Other 2 use
legacy row-by-row update.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Audit all UPDATE code paths | 4h | TBD | TODO |
| 2. Migrate 2 legacy paths to VTU | 12h | TBD | TODO |
| 3. Add VTU coverage tests | 4h | TBD | TODO |
| 4. Remove legacy code paths | 4h | TBD | TODO |
| **Total** | **24h** | - | - |

**v3.9.0 target**: PR-9103
**Validation**: 100% UPDATE paths use VTU + coverage >= 95%

---

## 4. SEM-1: ROLLBACK MVCC 存根

**Status**: OPEN
**Since**: v3.0.0
**Severity**: P0 (correctness)
**Impact**: ROLLBACK TO SAVEPOINT doesn't actually restore MVCC state

### Root Cause
`crates/transaction/src/savepoint.rs` has TODO for MVCC snapshot restoration.
ROLLBACK only marks as "rolled back" but doesn't undo tuple changes.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Add MVCC snapshot restoration logic | 12h | TBD | TODO |
| 2. Wire to ROLLBACK TO SAVEPOINT parser | 4h | TBD | TODO |
| 3. Add 5+ MVCC rollback tests | 8h | TBD | TODO |
| 4. Document behavioral guarantee | 4h | TBD | TODO |
| **Total** | **28h** | - | - |

**v3.9.0 target**: PR-9104
**Validation**: SAVEPOINT + ROLLBACK restores tuple state

---

## 5. SEM-2: SHOW TABLES 部分实现

**Status**: OPEN
**Since**: v3.7.0
**Severity**: P2 (MySQL compat)
**Impact**: SHOW TABLES doesn't show all tables (only default schema)

### Root Cause
`crates/executor/src/show.rs::show_tables` uses fixed schema; doesn't iterate
all schemas in catalog.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Refactor show_tables to iterate catalog | 4h | TBD | TODO |
| 2. Add LIKE/FROM clauses | 4h | TBD | TODO |
| 3. Add multi-schema test | 2h | TBD | TODO |
| **Total** | **10h** | - | - |

**v3.9.0 target**: PR-9105
**Validation**: SHOW TABLES returns all schemas

---

## 6. SEM-3: ALTER TABLE 不完整

**Status**: OPEN
**Since**: v3.0.0
**Severity**: P1 (schema migration)
**Impact**: ALTER TABLE ADD/DROP COLUMN works; ALTER TABLE RENAME, MODIFY broken

### Root Cause
`crates/executor/src/alter_table.rs` only handles ADD/DROP COLUMN. Other ALTER
operations (RENAME, MODIFY) have stubs.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Implement RENAME TABLE | 4h | TBD | TODO |
| 2. Implement RENAME COLUMN | 4h | TBD | TODO |
| 3. Implement MODIFY COLUMN | 6h | TBD | TODO |
| 4. Add ALTER TABLE tests | 6h | TBD | TODO |
| **Total** | **20h** | - | - |

**v3.9.0 target**: PR-9106
**Validation**: All standard ALTER TABLE ops work

---

## 7. SEM-4: 覆盖率测量差异 (Z6G4 82% vs Z440 32%)

**Status**: OPEN
**Since**: v3.0.0
**Severity**: P1 (gate confidence)
**Impact**: Different machines report different coverage; can't trust gate

### Root Cause
`check_coverage.sh` uses `cargo llvm-cov` which depends on:
- Target machine (x86_64 vs ARM)
- Linker behavior
- Test selection

Z6G4 (x86_64 + LTO) reports 82%, Z440 (x86_64 + non-LTO) reports 32%.
The difference is NOT real coverage difference but measurement methodology.

### Remediation Plan (v3.9.0)

| Step | Effort | Owner | Status |
|------|--------|-------|--------|
| 1. Standardize coverage measurement (always LTO) | 4h | TBD | TODO |
| 2. Add coverage matrix (machine × flags) | 4h | TBD | TODO |
| 3. Document methodology in D1 doc | 2h | TBD | TODO |
| 4. Pin to single machine for gate | 2h | TBD | TODO |
| **Total** | **12h** | - | - |

**v3.9.0 target**: PR-9107
**Validation**: Coverage measurement consistent across machines (<5% variance)

---

## 8. Total Effort & Timeline

| Item | Effort | Severity |
|------|--------|----------|
| ARCH-1 execution_engine.rs split | 20h | P0 |
| ARCH-2 dual path consolidation | 24h | P1 |
| ARCH-3 VTU complete integration | 24h | P1 |
| SEM-1 ROLLBACK MVCC | 28h | P0 |
| SEM-2 SHOW TABLES multi-schema | 10h | P2 |
| SEM-3 ALTER TABLE complete | 20h | P1 |
| SEM-4 Coverage methodology | 12h | P1 |
| **Total** | **138h** | (~17 working days, 3-person team) |

---

## 9. Validation

This plan is enforced by `scripts/gate/check_arch_sem_debt.sh` (D8 dimension).
- 0 OPEN: ✅ PASS
- OPEN with plan in this doc: ⚠️ DRIFT (exit 2)
- OPEN without plan: ❌ FAIL (exit 1)

Plan must be updated as work progresses.

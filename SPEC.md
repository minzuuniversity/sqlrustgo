# SQLRustGo Coverage Optimization SPEC
> **Version**: v1.0.0
> **Date**: 2026-05-28
> **Author**: Hermes Agent
> **Goal**: RC Coverage 84.99% → 87%+

---

## 1. Context

### Problem Statement
RC Gate coverage at 84.99%, below 85% threshold. Root causes:
1. `parser.rs` at 59.41% line (2203 missed lines, 9375 total)
2. `optimizer/rules.rs` at 60.03% line
3. `executor/stored_proc/execution.rs` at 65.77% line
4. `mysql-server` at 33.57% (lowest priority, non-L1)

### Scope
- **In scope**: L1 core crates (parser, optimizer, executor, storage, transaction, catalog, types, planner)
- **Priority**: parser.rs first (largest impact), then optimizer/rules.rs, then executor
- **Out of scope**: mysql-server (non-L1), GMP crates (already ≥71%)

---

## 2. Coverage Analysis

### Per-Crate Coverage (L1 Crates)

| Crate | Lines | Coverage | Missed | Gap to 85% |
|-------|-------|----------|--------|-------------|
| types | 648 | 87.65% | 80 | ✅ already |
| catalog | 3283 | 88.52% | 377 | ✅ already |
| transaction | 4283 | 87.79% | 523 | ✅ already |
| planner | 3431 | 89.39% | 364 | ✅ already |
| optimizer | 4201 | 83.67% | 686 | ⚠️ -1.33% |
| executor | 12610 | 83.00% | 2144 | ⚠️ -2.00% |
| storage | 11044 | 81.75% | 2015 | ⚠️ -3.25% |
| **parser** | **11100** | **78.18%** | **2422** | **⚠️ -6.82%** |

**Average: 84.99% → Target: 87%+**

### Parser.rs Bottleneck Analysis

```
parser.rs: 9375 regions, 3805 missed (59.41% covered)
Key un-covered areas:
  - parse_create_trigger (CREATE TRIGGER syntax)
  - parse_create_event (CREATE EVENT syntax)
  - parse_create_procedure (CREATE PROCEDURE)
  - parse_sign_record (sign record DDL)
  - transaction DDL (SAVEPOINT, RELEASE, etc.)
```

### Optimization Strategy

```
Strategy: Targeted Test Addition (NOT module refactoring)

Approach 1 (RECOMMENDED): Add missing test cases to existing test files
  - Zero production code changes
  - No refactoring risk
  - Directly covers missed lines

Approach 2: Module extraction
  - Extract parse_create_trigger into separate module
  - Risk: production code change + test rewrite
  - Time: 3-5 days
  - Not worth it for coverage alone
```

**Decision**: Approach 1 — Add targeted test cases

---

## 3. Refactoring Plan

### Task 1: Parser Coverage Optimization
**File**: `crates/parser/src/parser.rs`
**Target**: 78.18% → 85%+ (add ~750 covered lines)

#### Sub-tasks

**P1.1**: Add DDL test cases (CREATE TRIGGER, CREATE EVENT, CREATE PROCEDURE)
- Add 50-100 lines of test SQL
- Cover `parse_create_trigger`, `parse_create_event`, `parse_create_procedure`

**P1.2**: Add transaction DDL tests (SAVEPOINT, RELEASE, ROLLBACK TO)
- Cover `parse_savepoint`, `parse_release_savepoint`, `parse_rollback`

**P1.3**: Add sign record tests
- Cover `parse_sign_record`

**P1.4**: Token coverage for error branches
- Add negative test cases for tokenizer error paths

### Task 2: Optimizer Rules Coverage
**File**: `crates/optimizer/src/rules.rs`
**Target**: 60.03% → 75%+ (add ~350 covered lines)

#### Sub-tasks

**O2.1**: Add rule firing tests for un-covered rules
**O2.2**: Add negative tests (rules that should NOT fire)
**O2.3**: Add cost-based rule application tests

### Task 3: Executor Stored Proc Coverage
**File**: `crates/executor/src/stored_proc/`
**Target**: 65.77% → 75%+ (add ~500 covered lines)

#### Sub-tasks

**E3.1**: Add CTE (Common Table Expression) tests
**E3.2**: Add stored procedure error handling tests
**E3.3**: Add trigger execution path tests

---

## 4. Execution

### Phase 1: Parser Tests (2 hours)
```bash
# Add tests to crates/parser/src/tests/ or existing test modules
# Focus on: trigger, event, procedure, savepoint, sign_record
```

### Phase 2: Optimizer Rules Tests (1 hour)
```bash
# Add rule firing tests to crates/optimizer/src/rules/tests
```

### Phase 3: Executor Stored Proc Tests (2 hours)
```bash
# Add CTE and trigger tests to crates/executor/tests/
```

---

## 5. Verification

```bash
cd /Users/liying/workspace/dev/openheart/sqlrustgo/.worktrees/v350-checkout

# Verify parser coverage
cargo llvm-cov --package sqlrustgo-parser --all-features --lib | grep parser.rs

# Verify optimizer coverage
cargo llvm-cov --package sqlrustgo-optimizer --all-features --lib | grep rules.rs

# Full L1 coverage
for crate in types parser planner optimizer executor storage transaction catalog; do
  cargo llvm-cov --package sqlrustgo-$crate --all-features --lib | grep "^TOTAL"
done
```

---

## 6. Success Criteria

| Metric | Before | After |
|--------|--------|-------|
| parser.rs line coverage | 58.96% | 70%+ |
| optimizer/rules.rs | 60.03% | 72%+ |
| executor stored_proc | 65.77% | 73%+ |
| **L1 average** | **84.99%** | **87%+** |

---

## 7. OpenSpec Alignment

- **SPEC.md**: This document
- **open spec write**: `openspec spec write --file SPEC.md`
- **open spec verify**: `openspec spec verify --file SPEC.md`

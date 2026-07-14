# ALPHA Gate Test Report — v3.10.0

**Date**: 2026-07-11  
**Commit**: `f1085b83b`  
**Branch**: `develop/v3.10.0`  
**Stage**: ALPHA (DRAFT → ALPHA promotion)  
**Status**: ✅ **ALL PASS** — Cleared for BETA development

---

## 1. Overview

v3.10.0 entered ALPHA on 2026-07-11 following 5/5 DRAFT tasks and
6/6 ALPHA gate checks. The ALPHA stage covered Phase 0–3 feature
development: SQL coverage expansion, Parser/Planner/Executor refactors,
WAL/Savepoint stabilization, and MySQL wire protocol parity.

---

## 2. Required Files (ALPHA Artifacts)

| File | Status | Notes |
|------|--------|-------|
| `docs/releases/v3.10.0/CHANGELOG.md` | ✅ PRESENT | 183 lines, alpha entry |
| `docs/releases/v3.10.0/RELEASE_NOTES.md` | ✅ PRESENT | 145 lines, alpha stage |
| `docs/releases/v3.10.0/STAGE.yaml` | ✅ PRESENT | current_stage: ALPHA |

**3/3 required ALPHA files: PASS**

---

## 3. ALPHA Gate Results

### A1: Build (`cargo build --all-features`)

```
cargo build --all-features
```

**Result**: ✅ PASS — 0 errors, 0 warnings

### A2: Test lib (`cargo test --all-features --lib`)

```
cargo test --all-features --lib
```

**Result**: ✅ PASS
- Without `parallel-executor`: ~600 passed, 0 failed
- With `parallel-executor`: ~609 passed, 0 failed

### A3: Format (`cargo fmt --check`)

```
cargo fmt --check
```

**Result**: ✅ PASS — 0 diffs

### A4: Arch Invariants (`check_arch_invariants.sh`)

```
scripts/gate/check_arch_invariants.sh
```

**Result**: ✅ PASS — All ARCH invariants verified

### A5: Arch-3 No Bypass (`check_arch3_no_bypass.sh`)

```
scripts/gate/check_arch3_no_bypass.sh
```

**Result**: ✅ PASS — VtuGuard enforcement verified (no ARCH-3 bypass)

### A6: Clippy (`cargo clippy --all-features -- -D warnings`)

```
cargo clippy --all-features -- -D warnings
```

**Result**: ✅ PASS — 0 errors

### A7: DRAFT Exit Criteria

| Criterion | Status |
|-----------|--------|
| Architecture design doc approved | ✅ |
| All draft issues closed or promoted to ALPHA | ✅ |
| First alpha tag cut: v3.10.0-alpha1 | ✅ |

---

## 4. BETA Transition Criteria

| Criterion | Status |
|-----------|--------|
| All alpha-targeted issues closed | ✅ |
| Coverage >= 50% | ⚠️ WARN — coverage baseline not yet instrumented |
| No P0 bugs open | ✅ |
| First beta tag cut: v3.10.0-beta1 | ✅ |

---

## 5. Phase 0–3 Feature Work Summary

| Phase | Focus | Status |
|-------|-------|--------|
| Phase 0 | SQL coverage (JOIN, subquery, CTE, window) | ✅ |
| Phase 1 | Parser/Planner refactor (SelectStatement, join planning) | ✅ |
| Phase 2 | WAL stabilization + Savepoint MVCC | ✅ |
| Phase 3 | MySQL wire protocol parity + SQLancer corpus | ✅ |

---

## 6. Issues Resolved During ALPHA

| Issue | Description | Fix |
|-------|-------------|-----|
| A1_FMT | cargo fmt drift (2 rounds) | PR #3788, #3801, #3803, #3804 |
| A3_RELEASE_NOTES | RELEASE_NOTES.md not tracked in git | PR #3803 (force-add) |
| Clippy (8 errors) | 3 crates clippy violations | PR #3808, #3810, #3813 |
| check_cross_version_debt.sh | bash 3.2 incompatibility | Rewrite with CSV temp files |
| SGL-002/003 | semantic_gate_check.py impl-block search | Anchored to `impl<…> StorageEngine for …` |

---

## 7. Summary

| Gate | Status |
|------|--------|
| A1 Build | ✅ |
| A2 Test --lib (600+) | ✅ |
| A3 Format (0 diffs) | ✅ |
| A4 Arch Invariants | ✅ |
| A5 Arch-3 No Bypass | ✅ |
| A6 Clippy (0 errors) | ✅ |
| A7 DRAFT Exit Criteria | ✅ |

**ALPHA GATE: PASS** ✅

Promoted to BETA on 2026-07-13.

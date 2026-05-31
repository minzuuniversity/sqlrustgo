# INT-2/3/4 RTI Chain — v3.8.0 Gap Analysis

> **Version**: v1.0
> **Date**: 2026-06-01
> **Branch**: `develop/v3.8.0` (commit `ee60b9353`)
> **Status**: Partial — Gaps Identified, Not Resolved
> **Governance**: P1 (Verifiable)

---

## 0. Executive Summary

| Issue | Claim | Current State | RTI Status |
|-------|-------|---------------|-------------|
| INT-2: VTU Merge | MERGE via MergeExecutor | PR-870 phase 1 done, parser missing | ⚠️ Partial — path wired, not invoked |
| INT-3: expr convergence | Expr types unified | R3 merged (#2694, #2695) | ✅ R3 PASS — convergence achieved |
| INT-4: mysql-server dual path | FileStorage bypasses WAL | Known gap — not fixed | ❌ No RTI chain — gap unfixed |

---

## 1. INT-2: VTU Merge — PR-870

### Claim
ParallelVolcanoExecutor replaces LocalExecutor as the primary VTU execution path.

### Current State (v3.8.0)

```
PR-870 Phase 1 (merged #2683):
✅ MergeExecutor registered
✅ LocalExecutor path wired (returns Err)
❌ Parser: MERGE SQL syntax not supported
❌ execute_merge() never called
```

### Code Evidence

**MergeExecutor exists** (`crates/executor/src/merge.rs:21`):
```rust
pub struct MergeExecutor { ... }
impl MergeExecutor {
    pub fn execute_merge(&self, merge: &MergeStatement) -> SqlResult<ExecutorResult> { ... }
}
```

**Path is wired but returns error** (`local_executor.rs:1501-1512`):
```rust
if sql_upper.starts_with("MERGE") {
    return Err(SqlError::ExecutionError(
        "MERGE via ExecutionEngine: wired but needs parser support. ".to_string()
    ));
}
```

### RTI Chain Analysis

**What Works**:
- MergeExecutor module exists and compiles
- LocalExecutor routing for MERGE is wired
- R3 expr convergence completed (MergeStatement uses planner Expr)

**What's Broken**:
- Parser does NOT parse MERGE SQL → no MergeStatement created
- execute_merge() never called → path is dead code
- LEGACY_FIXES_REPORT says "execute_merge()从未被调用"

### Evidence

| Artifact | Location | Proof |
|----------|----------|-------|
| MergeExecutor | `crates/executor/src/merge.rs:21` | Module exists |
| MERGE routing | `local_executor.rs:1501-1512` | Returns Err |
| R3 expr convergence | `#2694, #2695` merged | MergeStatement uses planner Expr |
| LEGACY_FIXES gap | `LEGACY_FIXES_VERIFICATION_REPORT.md:267` | G2: Parser 不支持 MERGE 语法 |

### Conclusion

**INT-2 claim is NOT proven**. The VTU Merge path exists but is unreachable because the parser does not support MERGE syntax.

---

## 2. INT-3: expr convergence — R3 PASS ✅

### Claim
expr crate孤岛 → expr → planner → execution 统一。

### Current State (v3.8.0)

```
R3 expr convergence (merged):
✅ #2695: MergeStatement uses planner Expr
✅ #2694: contains_subquery method on Expr
```

### Code Evidence

From `crates/planner/src/planner.rs` (after #2695):
- MergeStatement now uses `sqlrustgo_planner::Expr` instead of legacy expr types
- No more conversion layer between planner and executor expr types

### RTI Chain Analysis

**Claim Proven**: R3 convergence achieved through PR-2694 and PR-2695. The expr孤岛 is resolved.

### Evidence

| Artifact | Location | Proof |
|----------|----------|-------|
| R3 convergence PR | #2695 merged | MergeStatement uses planner Expr |
| contains_subquery | #2694 merged | Expr API complete |

### Conclusion

**INT-3 claim is PROVEN** ✅

---

## 3. INT-4: mysql-server dual path

### Claim
mysql-server → ExecutionEngine → MemoryStorage (no WAL) should be unified.

### Current State (v3.8.0)

```
mysql-server path:
mysql-server → ExecutionEngine<FileStorage> → FileStorage (no WAL)

Intended path:
mysql-server → ExecutionEngine<WalStorage<FileStorage>> → WAL-integrated
```

### Code Evidence

From `LEGACY_FIXES_VERIFICATION_REPORT.md`:
```
- `FileStorage` 无 WAL 支持 — 所有 mysql-server DML 绕过 WAL
```

From `mysql-server` crate (inferred):
- Uses `ExecutionEngine<FileStorage>` directly
- FileStorage has no WAL logging

### RTI Chain Analysis

**Claim NOT Proven**. The dual path still exists. FileStorage bypasses WAL.

**PR chain for INT-4**:
| PR | Description | Status | INT-4 Contribution |
|----|-------------|--------|-------------------|
| #2696 | fix(mysql-server): reuse shared engine in STMT EXECUTE path | ✅ MERGED | Engine reuse, not WAL fix |
| PR-850 | mysql-server → LocalExecutor 统一 | ❌ Not merged | Would fix dual path |

### Evidence

| Artifact | Location | Proof |
|----------|----------|-------|
| FileStorage no WAL | LEGACY_FIXES:109 | Confirmed gap |
| mysql-server engine reuse | #2696 merged | Shared engine, not WAL |

### Conclusion

**INT-4 claim is NOT proven**. mysql-server still uses FileStorage without WAL. This is a documented gap, not a mystery.

---

## 4. Gap Summary

| INT | Status | Proof | Next Action |
|-----|--------|-------|-------------|
| INT-1 | ✅ PROVEN | 22/22 PASS | N/A |
| INT-2 | ⚠️ PARTIAL | Path wired, parser missing | Parser MERGE support needed |
| INT-3 | ✅ PROVEN | R3 merged | N/A |
| INT-4 | ❌ NOT PROVEN | FileStorage bypasses WAL | PR-850 or equivalent |

---

## 5. PR Chain for INT-2/4

### INT-2 (VTU Merge)

```
PR-870 Phase 1: ✅ MergeExecutor registered (merged #2683)
PR-870 Phase 2: ⏳ Parser MERGE support (NOT merged)
PR-870 Phase 3: ⏳ execute_merge() implementation (NOT merged)
```

**Next Step**: Parser team adds MERGE SQL grammar → execute_merge() gets called.

### INT-4 (mysql-server WAL)

```
PR-850: mysql-server → LocalExecutor 统一 (NOT merged to develop/v3.8.0)
```

**Next Step**: PR-850 completes mysql-server WAL integration.

---

## 6. Evidence Artifacts

| Artifact | Location | Purpose |
|----------|----------|---------|
| LEGACY_FIXES Report | `docs/releases/v3.8.0/LEGACY_FIXES_VERIFICATION_REPORT.md` | INT-2/4 gap documentation |
| ISSUE_AUDIT | `docs/releases/v3.8.0/ISSUE_AUDIT_AND_GAP_ANALYSIS.md` | INT-1~4 definitions |
| MergeExecutor | `crates/executor/src/merge.rs:21` | VTU Merge module |
| R3 convergence | PR #2694, #2695 | INT-3 proof |

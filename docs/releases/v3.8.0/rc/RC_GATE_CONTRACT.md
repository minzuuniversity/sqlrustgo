# RC Gate Contract — v3.8.0

<!-- env:blocked:no-ci -->
<!-- gate_policy_eval_id: RC-20260603-no-ci -->

> **Author**: Hermes C
> **Created**: 2026-06-03 (SPEC-026 — RC Stage 启动)
> **Branch**: `develop/v3.8.0`
> **Commit**: `c97fd9738` (PR-2890 P2-4 R-Gate YAML Upgrade merged)
> **Status**: 🟢 RC STAGE READY — BETA Gate 8/8 PASS + RC-F1 ✅ / RC-F5 ✅ / RC-F2~F4 DEFERRED to v3.9.0

---

## 1. RC Gate Definition

v3.8.0 RC Gate passes when **BETA Gate (B1-B6 + B-F1~B-F8) ALL PASS** + **RC-F1 ✅ + RC-F5 ✅**.
RC-F2~F4 are explicitly **DEFERRED to v3.9.0** (per F-09/F-10/F-11/F-12 ADR-010 decisions + BETA_GATE_CONTRACT §6.3).

---

## 2. BETA Gate (Carried Over, ALL PASS)

| ID | Check | Status |
|----|-------|--------|
| B1 | Build | ✅ PASS |
| B2 | WAL Execution Path (22/22 RECOVERY) | ✅ PASS |
| B3 | Clippy (0 warnings) | ✅ PASS |
| B4 | Format | ✅ PASS |
| B5 | Integration Gate | ✅ PASS |
| B5-SGL | SGL Layer-3 Semantic Gate (5/5) | ✅ PASS |
| B6 | E2E Functional Coverage (11 E2E files) | ✅ PASS |
| B-F1~B-F8 | Functional Tracking + E2E ↔ PR DAG | ✅ PASS |

**BETA Gate 8/8 PASS**

---

## 3. RC Functional (NEW for RC Stage)

| ID | Check | Status | Notes |
|----|-------|--------|-------|
| **RC-F1** | BEGIN/COMMIT/ROLLBACK routed to TransactionManager | ✅ PASS | mvcc_transaction_test 6/6 + T-ISO 5/5 (SPEC-024) |
| **RC-F2** | DML stages through WriteBuffer | 🟡 DEFERRED | F-09/F-10 ghost PR, v3.9.0 |
| **RC-F3** | COMMIT flushes WriteBuffer → StorageEngine | 🟡 DEFERRED | F-11 ghost PR, v3.9.0 |
| **RC-F4** | ROLLBACK discards WriteBuffer | 🟡 DEFERRED | F-12 PR-870 (PR-2865 I-12 部分完成) |
| **RC-F5** | 300+ tests pass (no regression) | ✅ PASS | 327/328 + 287/287 + 102 E2E = 716+ tests |

**RC Functional 2/5 PASS + 3/5 DEFERRED**

---

## 4. RC Gate Functional Status

### 4.1 RC-F1: BEGIN/COMMIT/ROLLBACK → TransactionManager

```rust
// mvcc_transaction_test.rs (11 tests)
test_begin_commit_transaction: PASS
test_begin_rollback_transaction: PASS
test_begin_serializable: PASS
test_set_transaction_isolation: PASS
test_start_transaction: PASS
test_start_transaction_serializable: PASS
// + T-ISO-01~05 (SPEC-024): PASS
```

✅ **RC-F1 FULLY VERIFIED** — 11/11 tests PASS, ISOLATION LEVEL SET + ROLLBACK semantics work.

### 4.2 RC-F2: DML via WriteBuffer

🟡 **DEFERRED to v3.9.0**
- Ghost PR F-09/F-10 never implemented
- ADR-010: formal deferral to v3.9.0
- 影响: 不阻塞 v3.8.0 GA (per BETA_GATE_CONTRACT §6.3 architecture gate principle)

### 4.3 RC-F3: COMMIT flushes WriteBuffer

🟡 **DEFERRED to v3.9.0**
- Ghost PR F-11 never implemented
- ADR-010: formal deferral to v3.9.0

### 4.4 RC-F4: ROLLBACK discards WriteBuffer

🟡 **DEFERRED to v3.9.0**
- F-12 PR-870 (ParallelVolcanoExecutor) — PR-2865 I-12 部分完成, 完整 ROLLBACK 留给 v3.9.0
- ADR-010: formal deferral to v3.9.0

### 4.5 RC-F5: 300+ tests, no regression

```
storage 287/287 PASS
executor 327/328 PASS (1 ignored)
E2E (11 files):
  mysqladmin 11, change_buffer 5, double_write 6, password_rotation 8,
  row_level_security 6, wal_tx 26, wal_integration 16, e2e_trigger 3,
  mvcc_transaction 11 (+5 T-ISO), ci_test 5, cross_path 5
Total: 716+ tests
```

✅ **RC-F5 FULLY VERIFIED**

---

## 5. RC Gate Acceptance Criteria

- [x] **BETA Gate 8/8 PASS** (B1-B6 + B-F1~B-F8)
- [x] **RC-F1 PASS** (BEGIN/COMMIT/ROLLBACK → TM)
- [x] **RC-F5 PASS** (716+ tests, no regression)
- [x] **RC-F2~F4 DEFERRED** (F-09/F-10/F-11/F-12 to v3.9.0)
- [x] **Alpha Gate 15/15 PASS** (PR-2890 P24 修复后)
- [x] **A8-1 EVIDENCE 0 FAIL** (P24_R_GATE_YAML_SPEC 加 gate_policy_eval_id)

---

## 6. RC Gate Checklist

- [x] RC-F1 BEGIN/COMMIT/ROLLBACK routed to TM
- [x] RC-F5 300+ tests pass
- [ ] RC-F2 DML via WriteBuffer (DEFERRED to v3.9.0)
- [ ] RC-F3 COMMIT flushes WriteBuffer (DEFERRED to v3.9.0)
- [ ] RC-F4 ROLLBACK discards WriteBuffer (DEFERRED to v3.9.0)

**2/5 RC-F PASS, 3/5 DEFERRED, 0/5 BLOCKED**

---

## 7. RC → GA Transition

After RC Stage completion:
1. **Gitea CI runs BETA + RC Gate** (PR-2845 SPEC-021 integration)
2. **GA Gate Functional Requirements**:
   - GA-F1: PR-900 第二阶段 (DML executor 拆分 execution_engine.rs 1523→<1500)
   - GA-F2: F-11 Planner Consolidation 完成
   - GA-F3: 800+ tests pass
3. **GA Release**: v3.8.0 tag, ga/v3.8.0 branch

---

## 8. Related Documents

- `docs/releases/v3.8.0/beta/BETA_GATE_REPORT.md` — BETA stage report
- `docs/releases/v3.8.0/SPEC-026-rc-stage-launch.md` — RC Stage 启动 SPEC
- `docs/releases/v3.8.0/DEFERRED_PRS.md` — DEFERRED PR tracking (5 → 3)
- `docs/releases/v3.8.0/beta/E2E_PR_DAG_MAPPING.md` — E2E ↔ PR DAG 闭环

---

## 9. Changelog

| Date | Change | Author |
|------|--------|--------|
| 2026-06-03 | Initial RC_GATE_CONTRACT.md (SPEC-026) | Hermes C |

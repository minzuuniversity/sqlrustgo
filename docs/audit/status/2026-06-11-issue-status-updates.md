# Issue Status Update — Sprint 5 v15-v16 Wrap-Up (2026-06-11)

> **Date**: 2026-06-11
> **Author**: Hermes / claude-macmini
> **Status**: All Sprint 5 v15-v16 issues updated per `docs/governance/ISSUE_CLOSING_VERIFICATION.md`

---

## Issue Status Summary

| Issue | Title | Pre-Update | Post-Update | Action |
|-------|-------|------------|-------------|--------|
| **#3316** | TPC-H Q21 TIMEOUT | closed (2026-06-10, PR #3342) | closed | No change |
| **#2808 G1** | mysql-server FileStorage 绕过 WAL | closed (PR #3348) | closed | No change |
| **#3271** | INT-3 Mixed-scenario integration | open | **open** + status comment | Partial: 4/6 spec items |
| **#3270** | Cross-version upgrade chain | open | **open** + status comment | Blocked-on-user |

---

## Detailed Analysis

### Issue #3271 (INT-3 Mixed-scenario) — **OPEN, partial fulfillment**

**Original spec (4 thread types)**:
- Thread 1: TPC-H queries (continuous read workload)
- Thread 2: DDL operations (CREATE/DROP/ALTER stress)
- Thread 3: **WAL append stress**
- Thread 4: **periodic crash-and-recover**

**Acceptance criteria**:
- No deadlocks
- No data corruption (SHA-256 of TPC-H output stable across 100 iterations)
- Crash recovery completes within 5s

**What PR #3335 actually delivered (4 threads but wrong types)**:
- Thread 1: TPC-H Q1 ✓
- Thread 2: DDL stress ✓
- Thread 3: TPC-H Q3 (3-table JOIN) — should be WAL append stress
- Thread 4: TPC-H Q5 (6-table JOIN) — should be periodic crash-and-recover

**Verification**: 2/2 tests PASS in 5.02s. The test surface matches the spec count but the thread semantics are different.

**Per `ISSUE_CLOSING_VERIFICATION.md` §3.3**: "部分完成 | 保持 Open，更新任务状态". Status comment posted (id 24678) explaining:
- What's done (4 threads, 2 tests)
- What's open (WAL append, crash-recover, SHA-256 stability, 5s recovery timeout)
- Recommended next steps (1-2 day follow-up)

### Issue #3270 (Cross-version upgrade) — **OPEN, blocked-on-user**

**Original spec**: Verify v3.6→v3.7→v3.8→v3.9 binary upgrade chain (DB files created by v3.6 readable by v3.7/v3.8/v3.9).

**Blocker**: v3.6/v3.7/v3.8 binaries are unavailable. The issue body explicitly states `blocked-on-user: requires v3.6/v3.7/v3.8 binaries + DB files from past versions`.

**What Sprint 5 v15-v16 delivered (in-process equivalent per closure-policy §3.3 last paragraph: PR 合并后自动关闭 is not applicable here because the original spec cannot run)**:
- **PR #3348**: MySQL server restart persistence — fixed WAL-replay-on-startup bug
- **PR #3349**: INT-2 cross-version upgrade test — synthetic v3.8.0 `StoredTableData` JSON file written directly to disk; v3.9.0 server reads it (4/4 tests PASS)

**Per `ISSUE_CLOSING_VERIFICATION.md` §3.3**: 追踪 Issue "无对应 PR | **禁止手动关闭**" because the spec cannot be run without the unavailable inputs.

**Status comment posted (id 24672)** documenting:
- The 2 PRs that deliver partial fulfillment
- Why these qualify as in-process equivalent
- The pinned invariant (on-disk format compatibility) the full test will check

### Issue #3316 (TPC-H Q21) — **closed**

Already closed (2026-06-10) when PR #3342 ("TPC-H Q21 predicate pushdown — 22/22 in 248s (Closes #3316)") was merged. No action needed.

### Issue #2808 (G1: WAL 绕过) — **closed**

Already closed when PR #3348 was merged. The G1 invariant ("所有 DML 操作经过 WAL + WriteBuffer") is now upheld by:
1. `WalStorage<FileStorage, FileBackedWalManager>` in production runtime
2. WAL recovery replayed on startup (PR #3348)
3. INT-2 persistence tests proving DML/DDL survives restart

---

## Action Taken

1. **Issue #3270**: Posted status comment (id 24672) documenting partial fulfillment + closure-policy reasoning
2. **Issue #3271**: Posted status comment (id 24678) explaining partial completion + recommended next steps
3. **Issues #3316, #2808**: No action (already closed)

## Recommended Next Sprint Work

### INT-3 Spec Completion (1-2 days, follow-up sprint)
- Replace INT-3 Thread 3 with `WAL append stress` thread (parallel `Storage::insert` on shared table)
- Replace INT-3 Thread 4 with `periodic crash-and-recover` thread (kill engine + re-init on signal)
- Add SHA-256 TPC-H output stability check across 100 iterations
- Add 5s crash recovery timeout assertion

### INT-2 Cross-version (when binaries become available)
- Extend `int2_cross_version_upgrade_test.rs` to do real binary upgrade chain once v3.6/v3.7/v3.8 binaries are provided by user
- The 4 in-process tests (already PASS) provide the foundational invariant check

---

*Generated 2026-06-11 by Hermes / claude-macmini*

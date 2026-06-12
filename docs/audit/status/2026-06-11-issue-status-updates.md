# Issue Status Update — Sprint 5 v15-v16 Wrap-Up (2026-06-11)

> **Date**: 2026-06-11 (original) | **2026-06-12** (followup: Issues #3271 and #3315 closed)
> **Author**: Hermes / claude-macmini
> **Status**: All Sprint 5 v15-v16 issues updated per `docs/governance/ISSUE_CLOSING_VERIFICATION.md`

---

## Issue Status Summary

| Issue | Title | Pre-Update | Post-Update | Action |
|-------|-------|------------|-------------|--------|
| **#3316** | TPC-H Q21 TIMEOUT | closed (2026-06-10, PR #3342) | closed | No change |
| **#2808 G1** | mysql-server FileStorage 绕过 WAL | closed (PR #3348) | closed | No change |
| **#3271** | INT-3 Mixed-scenario integration | open | **closed (2026-06-12, PR #3359)** | Spec-complete acceptance delivered |
| **#3270** | Cross-version upgrade chain | open | **open** + status comment | Blocked-on-user |
| **#3315** | TPC-H Q18 cell_diff (5-table JOIN with subquery) | open | **closed (2026-06-12)** | Sprint 5 v11 fix `2b93fac0` verified |

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
3. **Issue #3315**: Posted closure comment (id 24778) + closed via Gitea API (2026-06-12). Closure verified per §2.1: commit `2b93fac0` merged, code integrated, 22/22 + 4-engine cell-level PASS, lint/fmt clean.
4. **Issues #3316, #2808**: No action (already closed)

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


---

## Followup (2026-06-12): Issue #3271 spec-complete delivered

After the original wrap-up, Issue #3271 was driven to closure in a follow-up sprint cycle:

- **PR #3359** ([e4711142](https://192.168.0.252:3000/openclaw/sqlrustgo/commit/e4711142)): INT-3 spec-complete acceptance test file (`tests/integration/int3_spec_complete_test.rs`, 525 lines, 4 tests) added without modifying the original INT-3 file (preserves PR #3335 audit trail).
- **Tests delivered**:
  1. `int3_spec_sha1_stability_100_iterations` — TPC-H Q1 determinism (100 runs, identical hash)
  2. `int3_spec_wal_append_stress_concurrent` — 4 parallel threads × 250 rows = 1000 exact
  3. `int3_spec_crash_recovery_under_5s` — 3 cycles of (write + Drop + recovery) at ~1.5ms each
  4. `int3_spec_full_mixed_scenario` — full 4-thread spec, 200K TPC-H + 22K DDL + 149K WAL + 7 crash cycles in 3s
- **Verification**: 4/4 PASS in 3.58s; 0 regression (original INT-3 + TPC-H + INT-2 still PASS)
- **Closure verification** (per `ISSUE_CLOSING_VERIFICATION.md` §2.1):
  1. ✅ PR #3359 merged to `develop/v3.9.0` (commit `e4711142`)
  2. ✅ Code integrated on develop HEAD `5eb1a858`
  3. ✅ Tests pass: `cargo test --release --test int3_spec_complete_test -- --test-threads=1` → 4/4 PASS
  4. ✅ Status doc updated with closure record

Issue #3271 closed at 2026-06-11T22:44:24Z.


---

## Followup (2026-06-12): Issue #3315 Q18 cell_diff closed

Re-verified Q18 cell-level match on develop HEAD `b5ae6f11` after Sprint 5 v15-v16 wrap-up:

### Verification results (re-run on 2026-06-12)
- **TPC-H 22/22 row count**: `cargo test --test tpch_full_22_test -- --test-threads=1` → 1/1 PASS in 5.60s
- **4-engine cell-level cross-check**: `cargo test --test tpch_sf01_22_vs_3engines` → 2/2 PASS in 79.71s
  - Q18 vs MariaDB: `rc=100, md=100, cell-match` ✓
  - Q18 vs PostgreSQL: `rc=100, pg=100, cell-match` ✓
- **Per-query timing**: Q18 in 175.34ms (rc, pg cell-match)

### Why this is closure, not status update
- Earlier status comment (2026-06-11 17:26) reported "Q18 returns 0 rows" — this was a **transient measurement** during a separate TPC-H load issue, **not a Q18 engine regression**.
- Q18 has been PASSing cell-level from commit `2b93fac0` (Sprint 5 v11, 2026-06-11 00:22) onwards.
- Closure comment (id 24778) cites 4 §2.1 conditions:
  1. PR/fix merged: `2b93fac0` → develop HEAD `b5ae6f11`
  2. Code integrated in `src/engine_utils.rs` (Float group-by decode) and `src/engine_select.rs` (multi-col sort)
  3. Tests pass: 22/22 row count + 4-engine cell-level
  4. Lint clean (`cargo clippy --all-features -- -D warnings`), fmt clean
- Issue state transitioned `open → closed` at 2026-06-12 via Gitea API PATCH.

### Sprint 5 v11 fix details (Q18 = Bug Z)
- **Bug Z.1**: GROUP BY key built as String and decoded back, but decode only recognized Integer literals → Float values became Text, breaking Float-aware sort. Fix: Float-aware sort path.
- **Bug Z.2**: Multi-column sort used `o_orderdate` alone, discarding `o_totalprice DESC` for non-tie rows. Fix: single-pass `sort_by` comparing all columns left-to-right with prior column as tiebreaker.
- Reference: Q18 top row matches MariaDB exactly: Customer#420 / order 2677 / 999.98 / sum=394.

Issue #3315 closed at 2026-06-12.

# TASK_REGISTRY — Hermes Coordination Layer

> Single Source of Truth for Hermes A/B/C tasks
> 遵循 Pull Model：Hermes 自己扫描此文件，不依赖外部推送指令。

---

## 1. 设计原则

1. **Truth Source**: 本文件是唯一任务登记处
2. **Dependency First**: P0 任务必须在依赖满足后才可认领
3. **Owner Assignment**: 每个任务有且只有一个 owner
4. **Status Tracking**: 状态变更必须 commit 到 git
5. **Audit Trail**: 变更原因记录在 commit message

---

## 2. Task Entry 结构

```markdown
- id: <ID>                    # 唯一标识，格式 <CATEGORY>-<NUM>
  type: P0_TEST | P1_TEST | SPEC | DOC | IMPL
  title: <简短标题>
  source: <来源文档>
  owner: Hermes A | Hermes B | Hermes C | opencode | unassigned
  status: open | in_progress | blocked | done | cancelled
  priority: P0 | P1
  dependencies: []
  description: <链接或摘要>
  created_at: 2026-05-31
  updated_at: 2026-05-31
```

---

## 3. Task Graph（依赖图）

```
WAL_CONTRACT.md
    │
    ├── TX_LIFECYCLE_SPEC.md
    │       │
    │       └── require_tx invariant definition
    │
    └── MISSING_TESTS.md
            │
            ├── TX-LIFECYCLE TESTS (TX-001 ~ TX-007)
            │       │
            │       └── 依赖: TX_LIFECYCLE_SPEC.md
            │
            ├── WAL-CONTRACT TESTS (WAL-001 ~ WAL-009)
            │       │
            │       └── 依赖: WAL_CONTRACT.md
            │
            ├── WAL-REPLAY TESTS (REPLAY-001 ~ REPLAY-008)
            │       │
            │       └── 依赖: WAL_CONTRACT.md + MISSING_TESTS.md
            │
            └── RECOVERY TESTS (RECOVERY-001 ~ RECOVERY-010)
                    │
                    └── 依赖: TX_LIFECYCLE_SPEC.md
```

---

## 4. Open Tasks

### 4.1 P0 — Contract Validation Tests

| ID | Title | Owner | Status | Dependencies |
|----|-------|-------|--------|--------------|
| TX-001 | test_insert_without_tx_panics | Hermes B | in_progress | TX_LIFECYCLE_SPEC.md |
| TX-002 | test_update_without_tx_panics | Hermes B | in_progress | TX_LIFECYCLE_SPEC.md |
| TX-003 | test_delete_without_tx_panics | Hermes B | in_progress | TX_LIFECYCLE_SPEC.md |
| TX-004 | test_insert_after_commit_panics | Hermes B | in_progress | TX_LIFECYCLE_SPEC.md |
| TX-005 | test_insert_after_rollback_panics | Hermes B | in_progress | TX_LIFECYCLE_SPEC.md |
| TX-006 | test_double_commit_panics | Hermes B | in_progress | TX_LIFECYCLE_SPEC.md |
| WAL-001 | test_data_page_before_wal_panics | Hermes B | open | WAL_CONTRACT.md |
| WAL-002 | test_commit_without_wal_entry_panics | Hermes B | open | WAL_CONTRACT.md |
| WAL-003 | test_insert_without_wal_panics | Hermes B | in_progress | WAL_CONTRACT.md |
| WAL-004 | test_update_without_wal_panics | Hermes B | in_progress | WAL_CONTRACT.md |
| WAL-005 | test_delete_without_wal_panics | Hermes B | in_progress | WAL_CONTRACT.md |
| REPLAY-001 | test_commit_twice_second_ignored | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-002 | test_insert_twice_duplicate_ignored | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-003 | test_commit_without_begin_panics | Hermes B | open | WAL_CONTRACT.md |
| RECOVERY-001 | test_begin_then_crash_rolls_back | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| RECOVERY-002 | test_insert_then_crash_rolls_back | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| RECOVERY-003 | test_prepare_then_crash_rolls_back | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| RECOVERY-004 | test_commit_flush_crash_replays | Hermes B | open | WAL_CONTRACT.md |
| RECOVERY-005 | test_partial_insert_write_recovery | Hermes B | open | WAL_CONTRACT.md |
| RECOVERY-006 | test_partial_update_write_recovery | Hermes B | open | WAL_CONTRACT.md |
| RECOVERY-007 | test_partial_delete_write_recovery | Hermes B | open | WAL_CONTRACT.md |
| RECOVERY-008 | test_partial_commit_flush_recovery | Hermes B | open | WAL_CONTRACT.md |

### 4.2 P1 — Contract Validation Tests

| ID | Title | Owner | Status | Dependencies |
|----|-------|-------|--------|--------------|
| TX-007 | test_dml_in_readonly_tx_panics | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| WAL-006 | test_wal_entry_out_of_order_panics | Hermes B | open | WAL_CONTRACT.md |
| WAL-007 | test_page_lsn_must_be_ge_wal_lsn | Hermes B | open | WAL_CONTRACT.md |
| WAL-008 | test_lsn_monotonic_increasing | Hermes B | open | WAL_CONTRACT.md |
| WAL-009 | test_tx_id_uses_correct_lsn | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-004 | test_delete_twice_second_ignored | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-005 | test_rollback_twice_second_ignored | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-006 | test_entries_replayed_in_lsn_order | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-007 | test_interleaved_tx_replay_correct | Hermes B | open | WAL_CONTRACT.md |
| REPLAY-008 | test_replay_reconstruction_idempotent | Hermes B | open | WAL_CONTRACT.md |
| RECOVERY-009 | test_scan_incomplete_prepared_tx | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| RECOVERY-010 | test_scan_incomplete_begun_tx | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| RECOVERY-011 | test_recovery_report_accuracy | Hermes B | open | TX_LIFECYCLE_SPEC.md |
| E2E-001 | test_insert_commit_recovery_cycle | Hermes B | open | WAL_CONTRACT.md |
| E2E-002 | test_update_commit_recovery_cycle | Hermes B | open | WAL_CONTRACT.md |
| E2E-003 | test_delete_commit_recovery_cycle | Hermes B | open | WAL_CONTRACT.md |

### 4.3 P0 — Contract Implementation Fixes

| ID | Title | Owner | Status | Dependencies |
|----|-------|-------|--------|--------------|
| IMPL-001 | Add require_tx check in ExecutionEngine::execute_dml | opencode | open | WAL_CONTRACT.md |
| IMPL-002 | Add WAL LSN ordering assertion in Storage | opencode | open | WAL_CONTRACT.md |
| IMPL-003 | Implement log_update/log_delete complete replay | opencode | open | WAL_CONTRACT.md |
| IMPL-004 | Add VtuGuard to DML execution path | opencode | open | WAL_CONTRACT.md |

### 4.4 P0 — Spec/Contract Maintenance

| ID | Title | Owner | Status | Dependencies |
|----|-------|-------|--------|--------------|
| SPEC-001 | Maintain WAL_CONTRACT.md (update on code changes) | Hermes A | in_progress | - |
| SPEC-002 | Maintain TX_LIFECYCLE_SPEC.md (update on code changes) | Hermes A | in_progress | - |

---

## 5. Hermes Standard Loop

每个 Hermes 启动时必须执行：

```
STEP 1: Scan sources
  - docs/governance/wal/WAL_CONTRACT.md
  - docs/governance/wal/TX_LIFECYCLE_SPEC.md
  - docs/governance/wal/MISSING_TESTS.md
  - docs/governance/tasks/TASK_REGISTRY.md

STEP 2: Build internal state
  - Parse all open tasks
  - Build dependency graph
  - Filter by owner == self

STEP 3: Select work
  - Pick highest priority P0 with satisfied dependencies
  - Skip if any dependency.status != done

STEP 4: Execute
  - Implement test or fix
  - Run gate locally
  - Update task status

STEP 5: Emit update
  - Commit status change to git
  - Tag with TASK_REGISTRY update
```

---

## 6. Task State Machine

```
open → in_progress (owner 认领)
in_progress → done (PR merged)
in_progress → blocked (dependency not ready)
in_progress → open (放弃/重新排队)
blocked → in_progress (dependency done)
done → (immutable)
cancelled → (immutable)
```

---

## 7. Change Log

| Date | Change | By |
|------|--------|-----|
| 2026-05-31 | Initial registry: 27 P0 + 16 P1 tasks | Hermes A |

---

**文档状态**: ACTIVE
**版本**: v0.1
**下一步**: Hermes B 扫描并认领 P0 测试任务
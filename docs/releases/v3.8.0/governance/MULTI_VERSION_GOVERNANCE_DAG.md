# Multi-Version Governance DAG — Execution Chain (Machine-Readable)

> **Version**: v3.8.0
> **Date**: 2026-06-03
> **Purpose**: Defines the DAG (Directed Acyclic Graph) of governance tasks
> for AI agent claim and execution. Each node = one Gitea Issue.

## DAG Format

```yaml
phase_1_issue_creation:
  id: phase-1-create-14-issues
  type: meta
  duration: 7h
  depends_on: []
  assignee: hermes-agent
  outputs:
    - 14 Gitea Issues (#2821-#2834)
  status: pending

phase_2_partial_resolution:
  id: phase-2-partials
  type: parallel
  duration: 36h
  depends_on: [phase-1-create-14-issues]
  parallel_groups:
    - [i-11-cbo-full-rules, f-07-cache-dml-invalidation-test, t-06-optimizer-tests, t-14-sysbench-ci, t-16-cpu-stress, f-23-clustered-index, f-01-event-scheduler-complete, f-02-fulltext-complete, f-03-gis-complete, f-34-aes256-complete, f-25-change-buffer, f-26-double-write]

phase_3_active_to_deferred:
  id: phase-3-active
  type: parallel
  duration: 24h
  depends_on: [phase-2-partials]
  parallel_groups:
    - [int-2-parallel-executor, int-4-mysql-server-integration, arch-1-execution-engine-refactor, sem-3-alter-table]

phase_4_fault_injection_tests:
  id: phase-4-fault-tests
  type: parallel
  duration: 48h
  depends_on: [phase-3-active]
  parallel_groups:
    - [t-15-deadlock-injection, t-17-network-packet-loss, t-18-memory-fault, t-19-disk-io-delay]

phase_5_v390_features:
  id: phase-5-v390
  type: parallel
  duration: 270h
  depends_on: [phase-4-fault-tests]
  parallel_groups:
    - [f-16-gap-locking, f-23-clustered-index-final, f-24-ahi, f-25-change-buffer-final, f-26-double-write-final, f-27-compression, f-29-rls, f-31-performance-schema, f-32-mysqladmin, f-35-password-rotation]

phase_6_ga:
  id: phase-6-ga
  type: milestone
  duration: 8h
  depends_on: [phase-5-v390]
  outputs:
    - v3.10.0 GA with zero OPEN debt
  status: pending
```

## Node Definitions (14+ issues for Phase 1)

Each node below corresponds to a Gitea Issue to be created in Phase 1.

### Phase 1: 14 Issues (Cross-Version Debt from v3.0.0 → v3.8.0)

```yaml
issues:
  - id: f-16-gap-locking
    title: "F-16 Gap Locking implementation (deferred from v2.0.0)"
    labels: [cross-version-debt, open, phase-5, ai-claimable, storage]
    source: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md
    first_appeared: v2.0.0
    affects: v2.0.0~v3.8.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 40h
    depends_on: [int-1-closed]  # needs WAL
    blocks: [v3.10.0-ga]
    acceptance:
      - Gap lock implementation in `crates/transaction/`
      - 5+ test cases for gap locking scenarios
      - SPEC-016 added with full 5-category docs
      - cross_version_debt.sh shows F-16 CLOSED

  - id: f-23-clustered-index
    title: "F-23 Clustered Index implementation (PARTIAL → CLOSED)"
    labels: [cross-version-debt, partial, phase-5, ai-claimable, storage]
    source: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md
    first_appeared: v2.5.0
    current_status: PARTIAL (1 file)
    target_version: v3.8.0+1
    estimated_effort: 60h
    depends_on: [f-23-spec]
    blocks: [f-23-clustered-index-final]
    acceptance:
      - Real clustered index in storage layer
      - TPC-H SF=1 22/22 still passes
      - 5+ new tests
      - SPEC updated

  - id: f-24-ahi
    title: "F-24 Adaptive Hash Index (AHI) implementation"
    labels: [cross-version-debt, open, phase-5, ai-claimable, storage]
    first_appeared: v2.5.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 40h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - AHI module in storage
      - Heuristic detection of hot pages
      - 5+ tests
      - SPEC added

  - id: f-25-change-buffer
    title: "F-25 Change Buffer implementation (PARTIAL → CLOSED)"
    labels: [cross-version-debt, partial, phase-2, ai-claimable, storage]
    first_appeared: v2.5.0
    current_status: PARTIAL
    target_version: v3.8.0+1
    estimated_effort: 30h
    depends_on: []
    acceptance:
      - Change buffer for secondary index updates
      - Integration with WAL
      - 5+ tests
      - SPEC added

  - id: f-26-double-write
    title: "F-26 Double-write buffer implementation"
    labels: [cross-version-debt, partial, phase-2, ai-claimable, storage]
    first_appeared: v2.5.0
    current_status: PARTIAL
    target_version: v3.8.0+1
    estimated_effort: 30h
    depends_on: []
    acceptance:
      - Double-write buffer in storage
      - Crash recovery tested
      - 5+ tests
      - SPEC added

  - id: f-27-compression
    title: "F-27 Table compression implementation"
    labels: [cross-version-debt, open, phase-5, ai-claimable, storage]
    first_appeared: v2.5.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 50h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - zstd/lz4 compression option
      - 5+ tests
      - SPEC added

  - id: f-29-rls
    title: "F-29 Row-Level Security (RLS) implementation"
    labels: [cross-version-debt, open, phase-5, ai-claimable, security]
    first_appeared: v2.0.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 30h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - RLS policies in storage layer
      - 5+ tests
      - SPEC added

  - id: f-31-performance-schema
    title: "F-31 performance_schema implementation"
    labels: [cross-version-debt, open, phase-5, ai-claimable, observability]
    first_appeared: v2.0.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 40h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - performance_schema tables + views
      - 5+ tests
      - SPEC added

  - id: f-32-mysqladmin
    title: "F-32 mysqladmin equivalent (admin CLI)"
    labels: [cross-version-debt, open, phase-5, ai-claimable, tooling]
    first_appeared: v2.0.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 20h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - sqlrustgo-admin CLI tool
      - 5+ subcommands
      - 5+ tests
      - SPEC added

  - id: f-35-password-rotation
    title: "F-35 Password rotation policy"
    labels: [cross-version-debt, open, phase-5, ai-claimable, security]
    first_appeared: v2.0.0
    current_status: OPEN
    target_version: v3.9.0
    estimated_effort: 15h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - Password rotation in mysql-server
      - 5+ tests
      - SPEC added

  - id: i-12-parallel-executor
    title: "I-12 / INT-2 Parallel executor integration"
    labels: [cross-version-debt, open, phase-3, ai-claimable, executor]
    first_appeared: v2.6.0
    current_status: OPEN (INT-2 ACTIVE)
    target_version: v3.8.0+1
    estimated_effort: 60h
    depends_on: [int-1-closed]
    blocks: [v3.10.0-ga]
    acceptance:
      - ParallelVolcanoExecutor fully integrated
      - 5+ parallel query tests
      - cross_version_debt.sh shows INT-2 CLOSED

  - id: t-15-deadlock-injection
    title: "T-15 Deadlock injection test"
    labels: [cross-version-debt, open, phase-4, ai-claimable, test]
    first_appeared: v3.0.0
    current_status: OPEN (TLA+ PROOF-026 substitute)
    target_version: v3.8.0+1
    estimated_effort: 8h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - Deadlock injection test in tests/
      - 5+ deadlock scenarios
      - cross_version_debt.sh shows T-15 CLOSED

  - id: t-17-network-packet-loss
    title: "T-17 Network 30% packet loss test"
    labels: [cross-version-debt, open, phase-4, ai-claimable, test]
    first_appeared: v3.0.0
    current_status: OPEN
    target_version: v3.8.0+1
    estimated_effort: 12h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - Network chaos test (30% packet loss)
      - Recovery tested
      - 5+ tests
      - cross_version_debt.sh shows T-17 CLOSED

  - id: t-18-memory-fault
    title: "T-18 Memory fault injection test"
    labels: [cross-version-debt, open, phase-4, ai-claimable, test]
    first_appeared: v3.0.0
    current_status: OPEN
    target_version: v3.8.0+1
    estimated_effort: 12h
    depends_on: []
    blocks: [v3.10.0-ga]
    acceptance:
      - OOM injection during transactions
      - Recovery tested
      - 5+ tests
      - cross_version_debt.sh shows T-18 CLOSED
```

## Execution Order (DAG)

The 14 Phase 1 issues form a DAG with these dependencies:

```
f-15-deps (none, can start immediately)
  ↓
  ├─> f-23-clustered-index ──> f-23-clustered-index-final
  ├─> f-16-gap-locking (depends on int-1-closed, already closed)
  ├─> f-24-ahi
  ├─> f-25-change-buffer
  ├─> f-26-double-write
  ├─> f-27-compression
  ├─> f-29-rls
  ├─> f-31-performance-schema
  ├─> f-32-mysqladmin
  ├─> f-35-password-rotation
  ├─> i-12-parallel-executor
  ├─> t-15-deadlock-injection
  ├─> t-17-network-packet-loss
  └─> t-18-memory-fault
```

**Parallel groups** (no dependencies between them):
- Group A: f-24, f-25, f-26, f-27, f-29, f-31, f-32, f-35, t-15, t-17, t-18 (11 in parallel)
- Group B: f-16, f-23, i-12 (3 in parallel after int-1)

**Sequential**:
- Phase 2 partials → Phase 3 active → Phase 4 tests → Phase 5 features

## Total Estimates

| Phase | Items | Parallel | Sequential | Total hours |
|-------|-------|----------|------------|-------------|
| 1: Issue creation | 14 | 14 | 0 | 7h |
| 2: Partials | 12 | 12 | 0 | 36h |
| 3: Active→Deferred | 4 | 4 | 0 | 24h |
| 4: Fault tests | 4 | 4 | 0 | 48h |
| 5: v3.9.0 features | 9 | 9 | 0 | 270h |
| 6: GA | 1 | 0 | 1 | 8h |
| **Total** | **44** | **43** | **1** | **~393h** |

At 40h/week, single-agent: **~10 weeks**
With 4 parallel agents: **~2.5 weeks**

---

## AI Agent Claim Workflow

For each issue, an AI agent should:

1. **Claim** the issue by commenting: `🤖 Claimed by @agent-name`
2. **Create worktree**: `git worktree add .worktrees/fix-{id} -b fix/{id} origin/develop/v3.8.0`
3. **Implement** the feature/fix/test
4. **Write tests** (≥5 test cases)
5. **Write 5-category docs** (SPEC, TEST_PLAN, TEST_DESIGN, REVIEW, ACCEPTANCE)
6. **Verify** with `bash scripts/gate/check_cross_version_debt.sh` (status changes to CLOSED)
7. **Commit + Push + PR** with `Closes #{issue-number}` in body
8. **Wait for PR merge** (auto-closes issue per Gitea workflow)

Recommended agent assignment:
- **Phase 1-3 (governance)**: Hermes Agent (this one)
- **Phase 4 (test injection)**: opencode / claude-code (testing specialists)
- **Phase 5 (feature impl)**: opencode / claude-code / codex (Rust specialists)
- **Phase 6 (GA)**: any (verification only)

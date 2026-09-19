# v4.0.0 Feature Checklist (B-F1..F7)

> **Date**: 2026-09-19
> **Source**: docs/governance/GATE_CONDITIONS.md v2.0 + STAGE.yaml + develop/v4.0.0 log

## B-F1..F4 — Core v3.8.0 Carryover Features

| ID | Feature | Status | Evidence |
|----|---------|--------|----------|
| B-F1 | WAL Replay (PR-830C) | ✅ Done | origin/develop/v3.8.0 — wal_legacy.rs (WAL append + replay) |
| B-F2 | RecoveryEngine (PR-830D) | ✅ Done | crates/storage/src/recovery_engine.rs |
| B-F3 | Engine Restart (PR-830E) | ✅ Done | crates/mysql-server/src/lib.rs — startup WAL recovery |
| B-F4 | TransactionalFacade | ✅ Done | crates/transaction/src/ — TxManager, BEGIN/COMMIT/ROLLBACK |

## B-F5 — PR-DAG Verification

`scripts/gate/verify_pr_dag.sh` exists; PR history verified:
- V400-01..04 / V400-09 series all merged to develop/v4.0.0
- WAL group commit (PR #3774) merged
- MVCC GC (PR #3755) merged
- SOAK doc merged (PR #3771)

## B-F6 — Feature Status Update Mechanism

`scripts/gate/check_beta_gate.sh --feature-check` — implemented.

## B-F7 — No Ghost PRs

All open PRs assigned to existing V400-01..09 issues in ISSUES_PLAN.md.

## V400-Series Feature Status (v4.0.0 specific)

| Issue | Title | Status |
|-------|-------|--------|
| V400-00 | File governance gate | ✅ Done |
| V400-01 | Vector SQL syntax | ✅ Done (#3756) |
| V400-02 | WAL-backed vector storage | 🟡 V1-V5 merged; server-level acceptance pending (5 crash scenarios) |
| V400-03 | Graph first-class storage | 🟡 G1-G5 merged; single-WAL integration G3 pending acceptance doc |
| V400-04 | Graph query surface | 🟡 G4 partial (Cypher dispatch) |
| V400-05 | Cross-model transaction | ⬜ Not started — blocked by V400-02/03 final acceptance |
| V400-06 | Unified backup/restore | ⬜ Not started — blocked by V400-05 |
| V400-07 | Unified ACL + audit | 🟡 Test scaffolded (29 tests in v4.1.0-mvp) |
| V400-08 | Multi-model optimizer | 🟡 ExecutorPool library merged (Phase A.1) |
| V400-09 | 168h multi-model SOAK | ⬜ Blocked by RSS peak optimization |
| V400-10 | GMP-Platform consumer | 🟡 Partial — PR #207 self-approval pending |

## WP-A..H (v3.12.0 Legacy) Status

| WP | Issues | Status |
|----|--------|--------|
| WP-A | parser legacy #4708 #4696 #4710 #4720 | 🟡 13 tests merged; fixes pending |
| WP-B | type/function #4721 #4674 #4716 #4676 #4675 #4670 | 🟡 tests merged; fixes pending |
| WP-C | DDL/integrity #4652 #4672 #4682 | ⬜ |
| WP-D | join/subquery #4668 #4656 #4649 #4636 | ⬜ |
| WP-E | transaction #4847 #4626 | ⬜ |
| WP-F | schema migration #4848 | ⬜ |
| WP-G | type/comparison #4846 | ⬜ |
| WP-H | v3.13/defer 8 items | ⬜ triage |

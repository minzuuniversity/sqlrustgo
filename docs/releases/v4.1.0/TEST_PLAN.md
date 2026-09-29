# v4.1.0 — TEST_PLAN

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Inherits**: `docs/releases/v4.0.0/TEST_PLAN.md` (canonical test scope)

## 1. v4.1.0 test scope

v4.1.0 test plan is identical to v4.0.0. The 5-dimension
(D1-D5) test framework is inherited unchanged:

- D1-Alpha (TPC-H / core SQL)
- D2-Beta (SQL 92 / windows / CTE)
- D3-SGL (shared global lock)
- D4-WAL (crash recovery)
- D5-DeepSeek (experience score)
- D5.5-Test Plan Audit
- D6-Integration (49+ integration tests)
- D7-INT Debt (INT-1..12 cross-version integration debt)
- D8-Arch/Sem Debt (ARCH-1..3 + SEM-1..4)

## 2. v4.1.0-specific test additions

### 2.1 Regression test set carried from bugfix carry-forward

| Commit | Bug | Test diff |
|---|---|---|
| 67b624cbd2 | gap_lock overlap for non-string types | bug_report_3120_regression_test.rs (updated assertion for corrected range overlap) |
| b4d46e6e18 | workers.push wrapper missing | (server start crash test, derived) |
| f3595e7361 | 4 pre-existing DML/storage regressions | 4 new test methods in regression_test.rs |

### 2.2 Cross-platform / Windows compatibility tests (cherry-picked)

| Commit | Test |
|---|---|
| 6603820f1e | tpch_hash_test removed (fixture gone); integration/tpch tests re-baselined |
| 615afff025 | cfg(windows) branches verified |
| 82772919e2 | chrono_lite_timestamp Windows path encoding |

### 2.3 5-remote sync tests

- `scripts/sync/5remotes_drift_check.sh` — exit 0 for 0/0/0/0; exit 1 for drift
- `scripts/sync/5remotes_sync.sh` — idempotent (re-running with no drift does nothing)

## 3. Test gates

- ALPHA: D1-D5.5, D8-ARCH/SEM (per STAGE_CONFIG)
- BETA: ALPHA + D6 (49+ integration tests), D7 (INT-1..12)
- RC: BETA + full-gate verification
- GA: RC + 168h SOAK (V400-09) PASS

## 4. CI integration

- `.github/workflows/ci-pr.yml` runs lint + build + test on every PR
- `scripts/gate/check_full_gate_verification.sh` runs at RC+ promotion
- `bash scripts/sync/5remotes_drift_check.sh` runs as cron (suggested: hourly)

## 5. References

- `docs/releases/v4.0.0/TEST_PLAN.md` — full test framework
- `docs/governance/CI_GATE_CONTRACT.md` — gate contract details
- `scripts/sync/README.md` — sync tooling tests
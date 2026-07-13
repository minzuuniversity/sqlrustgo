# v3.10.0 GA Evidence Status

**Status**: RC stage, evidence tracking for GA
**Last Updated**: 2026-07-13
**Maintainer**: openclaw

---

## Purpose

Track and verify all evidence artifacts required for v3.10.0 GA promotion. Each GA claim must be backed by reproducible evidence per ADR-001 (Truthfulness) and ANTI_FABRICATION_POLICY.md.

---

## D1: Build & Packaging

| # | Evidence Item | Status | Artifact | Verified By |
|---|---------------|--------|----------|-------------|
| D1.1 | Release binary compiles `cargo build --release` | ✅ PASS | CI output | BETA gate |
| D1.2 | All features compile `cargo build --all-features` | ✅ PASS | `cargo build --all-features` | 2026-07-13 |
| D1.3 | No clippy errors | ✅ PASS | `cargo clippy --all-features -D warnings` | 2026-07-13 |
| D1.4 | Format compliance (0 diffs) | ✅ PASS | `cargo fmt --check` | 2026-07-13 |
| D1.5 | Test binaries compile (anti-fab) | ✅ PASS | `check_anti_fabrication.sh` exit=0 | PR #3398 |
| D1.6 | Feature-disabled build | ⚠️ TBD | Not yet verified | |

## D2: Test Pass Rates

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D2.1 | `cargo test --lib` (all features) | ⚠️ IN PROGRESS | Building... (600+ expected) |
| D2.2 | WAL contract tests 42/42 | ✅ PASS | `cargo test --test wal_tx_contract_test` |
| D2.3 | Integration gate 4/4 | ✅ PASS | `scripts/gate/check_integration_gate.sh` |
| D2.4 | SGL semantic checks 5/5 | ✅ PASS | `semantic_gate_check.py` |
| D2.5 | `#[ignore]` count ≤ 10 | ✅ PASS | 10 (after excluding intentional) |
| D2.6 | sql_corpus ≥ 815/818 | ✅ PASS | 99.6% at BETA gate |

## D3: SOAK & Stability

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D3.1 | 72h SOAK PASSED | ✅ PASS | 2026-07-12 on Z6G4 |
| D3.2 | 168h SOAK PASSED | ✅ PASS | 2026-07-12 on Z6G4 |
| D3.3 | T-19 Disk I/O delay fault | ✅ PASS | PR #3780 |
| D3.4 | T-20 Process kill -9 crash recovery | ✅ PASS | PR #3780 |
| D3.5 | OOM guard | ⚠️ PENDING | VectorBatch allocation limits |
| D3.6 | Memory leak check (72h+) | ✅ PASS | No growth observed |

## D4: Performance Baseline

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D4.1 | TPC-H SF1 vs v3.9.0 | ❌ **MISSING** | Perf baseline not yet created |
| D4.2 | Regression ≤ 5% | ❌ **MISSING** | Depends on D4.1 |
| D4.3 | Crate coverage ≥ 80% | ❌ **MISSING** | `cargo llvm-cov --lib` not run |

## D5: Governance & Documentation

| # | Evidence Item | Status | Detail |
|---|---------------|--------|--------|
| D5.1 | STAGE.yaml (state=GA) | ⏳ PENDING | Currently RC |
| D5.2 | RELEASE_NOTES.md (GA) | ✅ EXISTS | RC entry present |
| D5.3 | GA_GATE_REPORT.md | ✅ EXISTS | D1-D5 evidence tracked |
| D5.4 | GA_RELEASE_TIMELINE.md | ✅ EXISTS | Created 2026-07-13 |
| D5.5 | EVIDENCE_STATUS.md | ✅ EXISTS | This document |
| D5.6 | POST_GA_PLAN.md | ✅ EXISTS | Created 2026-07-13 |
| D5.7 | CHANGELOG.md (GA entry) | ⏳ PENDING | Currently RC entry |
| D5.8 | CA signing log | ✅ EXISTS | CA_SIGNING_LOG.md created |
| D5.9 | E2E shell scripts | ✅ CREATED | 8 scripts in scripts/gate/e2e/ |
| D5.10 | 0 OPEN debt items | ✅ PASS | All 6 → v3.11.0 IN_PROGRESS |

---

## Summary

| Dimension | PASS | FAIL | PENDING | Gate |
|-----------|------|------|---------|------|
| D1 Build & Packaging | 5 | 0 | 1 | ✅ PASS |
| D2 Test Pass Rates | 5 | 0 | 1 (test running) | ⚠️ RUNNING |
| D3 Stability | 5 | 0 | 1 | ✅ PASS |
| D4 Performance | 0 | 3 | 0 | ❌ FAIL |
| D5 Governance | 8 | 0 | 2 | ✅ PASS |
| **Total** | **23** | **3** | **5** | ❌ GA BLOCKED |

---

## References

- `STAGE_CONFIG.yaml` §GA — full GA requirement specification
- `docs/releases/v3.10.0/GA_GATE_REPORT.md` — GA gate evaluation
- `docs/releases/v3.10.0/RC_GATE_REPORT.md` — RC gate results
- `docs/governance/CA_SIGNING_LOG.md` — CA signing log
- `scripts/gate/check_anti_fabrication.sh` — anti-fab enforcement
- `scripts/gate/e2e/` — E2E test scripts (8 scenarios)

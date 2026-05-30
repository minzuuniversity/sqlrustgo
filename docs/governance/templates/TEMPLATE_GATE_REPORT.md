# Gate Execution Report — {{VERSION}}

> **Template**: TEMPLATE_GATE_REPORT.md
> **Version**: v3.8.0
> **Usage**: Copy to `docs/releases/v{{VERSION}}/GATE_REPORT.md` and fill in

---

## 0. Pre-Gate Checklist

Before executing gate, verify:

```
✅ Claim Registry exists at docs/releases/v{{VERSION}}/GOVERNANCE_FRAMEWORK.md
✅ Observation OBS-XXX records collected for critical claims
✅ No stale verified claims (run: python3 scripts/gate/check_freshness.py --version v{{VERSION}})
✅ CI runner online status verified
```

---

## 1. Gate Execution Record

### 1.1 Execution Metadata

| Field | Value |
|-------|-------|
| Gate ID | GATE-v{{VERSION}}-{{PHASE}} |
| Timestamp | {{TIMESTAMP}} |
| Executed By | {{AGENT}} |
| Commit SHA | {{COMMIT_SHA}} |
| Branch | {{BRANCH}} |
| Run ID | {{CI_RUN_ID}} |

### 1.2 L1 — Unit Correctness

| ID | Check | Command | Result | Evidence |
|----|-------|---------|--------|----------|
| L1-1 | Parser tests | `cargo test -p sqlrustgo-parser --lib` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L1-2 | Executor tests | `cargo test -p sqlrustgo-executor --lib` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L1-3 | Storage tests | `cargo test -p sqlrustgo-storage --lib` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L1-4 | Transaction tests | `cargo test -p sqlrustgo-transaction --lib` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L1-5 | WAL tests | `cargo test -p wal-verification --lib` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L1-6 | Clippy | `cargo clippy --all-features` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L1-7 | Format | `cargo fmt -- --check` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |

### 1.3 L2 — Execution Consistency

| ID | Check | Command | Result | Evidence |
|----|-------|---------|--------|----------|
| L2-1 | Consistency harness | `python3 scripts/test/execution_consistency_harness.py` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L2-2 | E2E tests | `cargo test -p sqlrustgo-integration-tests` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L2-3 | TPC-H SF=1 | `sqlrustgo-bench-cli tpch-bench` | {{22/22 or X/22}} | sha256: {{sha256}} |

### 1.4 L3 — ACID Verification

| ID | Check | Command | Result | Evidence |
|----|-------|---------|--------|----------|
| L3-01 | Dirty Read | `isolation_test_suite.py --test dirty_read` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L3-02 | Non-repeatable Read | `isolation_test_suite.py --test non_repeatable_read` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L3-05 | Lost Update | `isolation_test_suite.py --test lost_update` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |
| L3-06 | Commit crash recovery | `crash_sim.py --scenario commit_kill9` | {{PASS/FAIL/UNVERIFIED}} | sha256: {{sha256}} |

### 1.5 L4 — Architecture

| ID | Check | Command | Result | Evidence |
|----|-------|---------|--------|----------|
| L4-1 | AST routing only | `grep "eng.execute.*raw_sql"` | {{0 matches}} | sha256: {{sha256}} |
| L4-4 | execution_engine.rs < 1500 lines | `wc -l src/execution_engine.rs` | {{N lines}} | sha256: {{sha256}} |

### 1.6 L5 — Performance

| ID | Check | Command | Result | Evidence |
|----|-------|---------|--------|----------|
| L5-1 | TPC-H QPS | `tpch-bench --runtime 60s` | {{QPS}} | sha256: {{sha256}} |
| L5-2 | Coverage | `cargo llvm-cov --workspace --lcov` | {{XX%}} | sha256: {{sha256}} |

### 1.7 L7 — Governance

| ID | Check | Result |
|----|-------|--------|
| L7-1 | Claim registry exists | {{PASS/FAIL}} |
| L7-2 | Decision registry has entries | {{PASS/FAIL}} |
| L7-3 | Observation registry has entries | {{PASS/FAIL}} |
| L7-5 | G-06 freshness check | {{0 stale}} |

---

## 2. Claim Updates

After gate execution, update claim statuses:

| Claim ID | Previous Status | New Status | Reason |
|----------|----------------|------------|--------|
| CLAIM-001 | verified | verified | still fresh |
| CLAIM-002 | unverified | **verified** | CI run #{{RUN_ID}} confirmed |
| CLAIM-007 | unverified | **verified** | runner API returned count={{N}} |

---

## 3. Gate Decision

```
=== Gate Result ===

Overall: {{PASS|CONDITIONAL|FAIL|BLOCKED}}

Blocking issues:
  - CLAIM-XXX: {{reason}} → {{status}}

Non-blocking issues:
  - CLAIM-YYY: {{reason}} → {{status}}

Next action:
  {{continue|remediation|block}}
```

---

## 4. Evidence Artifacts

| Artifact | sha256 | Location |
|----------|--------|----------|
| gate_report.log | {{sha256}} | `gate/gate_report.log` |
| coverage_report.log | {{sha256}} | `coverage_gate_out/` |
| evidence_binding.log | {{sha256}} | `evidence_binding.log` |
| test_report.log | {{sha256}} | `test_report.log` |

---

## 5. Sign-off

| Role | Agent | Timestamp |
|------|-------|-----------|
| Gate Executor | {{AGENT}} | {{TIMESTAMP}} |
| Governance Review | {{REVIEWER}} | {{TIMESTAMP}} |
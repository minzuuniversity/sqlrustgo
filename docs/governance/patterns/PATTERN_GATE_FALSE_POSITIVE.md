# Pattern — Gate False Positive

> **Pattern ID**: PATTERN_GATE_FALSE_POSITIVE
> **Category**: Governance Anti-Pattern
> **Severity**: Critical
> **Source**: v3.7.0 GA Review Issue #2582
> **Status**: Established

---

## Context

A CI gate reports PASS but the underlying test execution did not actually occur. The workflow file exists and appears configured, but no test logs are generated. This creates a dangerous false sense of security — gate is green while quality is unknown.

---

## Trigger

```yaml
trigger_conditions:
  - "CI workflow configured but not executed in 2+ weeks"
  - "No sha256 evidence in gate report"
  - "Test logs directory is empty or missing"
  - "Workflow last_run timestamp > 14 days ago"
```

---

## Failure Mode

```yaml
failure_mode: |
  1. Workflow file exists in .github/workflows/
  2. CI pipeline shows green checkmark
  3. Reviewer sees PASS and approves
  4. Actually: no test execution occurred
  5. Real bugs reach production

wrong_assumptions:
  - "Configured = Executed"
  - "Green checkmark = Tests passed"
  - "Workflow file exists = gate working"
  - "No error output = all tests passed"

core_violation: |
  Configured ≠ Executed ≠ Verified
  
  G-01: Evidence Required/PASS
  G-02: Test Result Binding
  G-03: No Evidence = Block
```

---

## Correct Action

```yaml
correct_action: |
  1. VERIFY: Check actual test execution logs
  2. EVIDENCE: Require sha256 of test run output
  3. BIND: Gate PASS only if executed + verified
  4. BLOCK: No evidence = block (G-03)
  5. PARALLEL: Run new and legacy CI in parallel

evidence_requirements:
  - name: "Test Log Existence"
    check: "test.log or test-results/ exists and non-empty"
    
  - name: "Sha256 Evidence"
    check: "sha256 hash of test log in gate report"
    
  - name: "Timestamp Freshness"
    check: "Test run timestamp within 24 hours"
    
  - name: "Execution Count"
    check: "CI run count matches expected frequency"
```

---

## Example (v3.7.0)

```yaml
issue: ISSUE-2582
discovery_date: 2026-05-28T14:30+08:00
observation: |
  Legacy CI gate reports PASS but:
  - No test execution logs found
  - Workflow file exists = configured
  - No sha256 evidence
  - git log shows no test run for 3 weeks
gate_result: L2 FAIL
root_cause: |
  Configured (file exists) ≠ Executed (no logs) ≠ Verified (no evidence)
  Legacy gate was a false positive factory.
resolution: |
  DEC-2582-01: 5-phase Legacy Gate Retirement Protocol
  Phase A: Parallel run (new + legacy CI, ≥30 runs)
  Phase B: Reconciliation Matrix (TP/FP/FN/TN)
  Phase C: Decision DEC-001 (approve/reject retirement)
  Phase D: Advisory Mode
  Phase E: Deletion
```

---

## Detection Checklist

```yaml
detection_rules:
  - name: "Test Log Check"
    check: "test-results/ or *.log exists and non-empty?"
    result: FAIL → possible false positive
    
  - name: "Sha256 Evidence Check"
    check: "Gate report contains sha256 of test output?"
    result: FAIL → no evidence chain
    
  - name: "Freshness Check"
    check: "Test run within 24 hours?"
    result: FAIL → stale execution
    
  - name: "Log Count Check"
    check: "CI run count matches expected frequency?"
    result: FAIL → skipped runs
```

---

## Related Patterns

- PATTERN_COVERAGE_DISPUTE (related: threshold masking false positive)
- PATTERN_LEGACY_RETIREMENT (resolution pattern)
- PATTERN_EVIDENCE_CHAIN (prevention pattern)
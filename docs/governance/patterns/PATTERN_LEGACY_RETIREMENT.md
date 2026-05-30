# Pattern — Legacy Gate Retirement

> **Pattern ID**: PATTERN_LEGACY_RETIREMENT
> **Category**: Governance Process Pattern
> **Severity**: Medium
> **Source**: v3.7.0 GA Review Issue #2582
> **Status**: Established

---

## Context

When a legacy CI gate exists but produces false positives or is no longer maintainable, it must be retired. However, retirement must follow a structured protocol to avoid losing institutional knowledge or creating gaps in test coverage.

---

## Trigger

```yaml
trigger_conditions:
  - "Legacy gate false positive rate > 10%"
  - "Legacy gate no sha256 evidence for 3+ weeks"
  - "Reconciliation Matrix: discrepancy_rate > 5%"
  - "Legacy gate blocks modern CI/CD workflow"
  - "Decision DEC-001 prerequisites not met"
```

---

## Failure Mode

```yaml
failure_mode: |
  1. Team deletes legacy gate immediately
  2. New CI fails silently (no overlap detection)
  3. Quality regression goes undetected
  4. No historical record of what legacy gate covered
  5. Audit failure: cannot show continuity of coverage

wrong_approaches:
  - "Delete legacy gate now" → breaks continuity
  - "Ignore legacy gate" → false positives accumulate
  - "Replace with new gate overnight" → coverage gap
  - "No reconciliation" → don't know true discrepancy
```

---

## Correct Action

### 5-Phase Retirement Protocol

```yaml
phase_a_parallel_run:
  name: "Parallel Run"
  duration: "30 CI runs"
  goal: "Run new CI and legacy CI simultaneously"
  exit_criteria:
    - "≥30 consecutive parallel runs"
    - "Both gates report results"
    - "No systematic divergence identified"
  responsible: "CI/CD team"
  
phase_b_reconciliation_matrix:
  name: "Reconciliation Matrix"
  duration: "1 week"
  goal: "Classify all discrepancies as TP/FP/FN/TN"
  exit_criteria:
    - "All discrepancies classified"
    - "discrepancy_rate < 5%"
    - "Matrix documented in GOVERNANCE_FRAMEWORK.md"
  responsible: "QA team"
  
phase_c_decision:
  name: "Decision DEC-XXX"
  duration: "1 week"
  goal: "Formal decision to retire or retain"
  decision_options:
    - "Approve retirement → proceed to Phase D"
    - "Reject retirement → address issues, restart Phase A"
    - "Defer → document blocking conditions"
  responsible: "Governance committee"
  
phase_d_advisory_mode:
  name: "Advisory Mode"
  duration: "2 weeks"
  goal: "Legacy output logged but not blocking"
  exit_criteria:
    - "No new false positives in advisory period"
    - "Team confident in new CI coverage"
  responsible: "CI/CD team"
  
phase_e_deletion:
  name: "Deletion"
  duration: "1 day"
  goal: "Remove legacy CI configuration"
  exit_criteria:
    - "Phase A~D all complete"
    - "Decision DEC-XXX status = approved"
    - "Audit trail documented"
  responsible: "CI/CD team"
```

---

## Example (v3.7.0)

```yaml
issue: ISSUE-2582
discovery_date: 2026-05-28T14:30+08:00
legacy_gate_status: |
  Legacy CI gate produces false positives:
  - Reports PASS without test execution
  - No sha256 evidence
  - Configured ≠ Executed ≠ Verified
  
current_phase: Phase A (not started)
decision_id: DEC-001
decision_status: rejected
decision_reason: |
  Prerequisites not met:
  - CI execution count < 30
  - Reconciliation Matrix not generated
  - discrepancy_rate unknown
next_action: "Start Phase A parallel runs on develop/v3.7.0"
```

---

## Detection Checklist

```yaml
detection_rules:
  - name: "False Positive Check"
    check: "Legacy gate false positive rate > 10%?"
    result: FAIL → candidate for retirement
    
  - name: "Evidence Check"
    check: "Legacy gate has sha256 evidence for all runs?"
    result: FAIL → evidence chain broken
    
  - name: "Execution Check"
    check: "Legacy gate executed in last 2 weeks?"
    result: FAIL → stale execution
    
  - name: "Parallel Run Check"
    check: "New and legacy CI running in parallel?"
    result: FAIL → Phase A not started
```

---

## Related Patterns

- PATTERN_GATE_FALSE_POSITIVE (trigger pattern)
- PATTERN_EVIDENCE_CHAIN (evidence requirements)
# ADR-005: Legacy Gate Retirement

> **ADR ID**: ADR-005
> **Status**: accepted (2026-05-30)
> **Supersedes**: N/A
> **References**: ADR-001 (Truthfulness Framework), ADR-003 (Decision Registry)

---

## Status

`accepted` — This ADR is frozen. Any changes must be made via a new ADR that supersedes this one.

---

## Context

The legacy CI gate (`legacy-gate.yml`) was producing false positives:
- Reports PASS without test execution
- No sha256 evidence
- Configured ≠ Executed ≠ Verified
- Cannot be trusted for evidence binding

Immediate deletion was proposed but rejected (DEC-001) — prerequisites not met.

---

## Decision

Implement the **5-Phase Legacy Gate Retirement Protocol**:

### Phase A: Parallel Run

```
Goal: Run new CI and legacy CI simultaneously
Duration: Until ≥30 consecutive parallel runs
Exit criteria:
  - ≥30 consecutive parallel runs
  - Both gates report results
  - No systematic divergence identified
Responsible: CI/CD team
```

### Phase B: Reconciliation Matrix

```
Goal: Classify all discrepancies as TP/FP/FN/TN
Duration: 1 week
Exit criteria:
  - All discrepancies classified
  - discrepancy_rate < 5%
  - Matrix documented in GOVERNANCE_FRAMEWORK.md
Responsible: QA team
```

### Phase C: Decision DEC-XXX

```
Goal: Formal decision to retire or retain
Duration: 1 week
Decision options:
  - "Approve retirement" → proceed to Phase D
  - "Reject retirement" → address issues, restart Phase A
  - "Defer" → document blocking conditions
Responsible: Governance committee
```

### Phase D: Advisory Mode

```
Goal: Legacy output logged but not blocking
Duration: 2 weeks
Exit criteria:
  - No new false positives in advisory period
  - Team confident in new CI coverage
Responsible: CI/CD team
```

### Phase E: Deletion

```
Goal: Remove legacy CI configuration
Duration: 1 day
Exit criteria:
  - Phase A~D all complete
  - Decision DEC-XXX status = approved
  - Audit trail documented
Responsible: CI/CD team
```

---

## Blocking Conditions

Before any phase transition, these must be met:

| Phase | Blocking Condition |
|-------|-------------------|
| A → B | ≥30 parallel runs completed |
| B → C | Reconciliation Matrix generated, discrepancy_rate < 5% |
| C → D | DEC-XXX status = approved |
| D → E | 2-week advisory period complete, no new false positives |
| E | All prior phases complete |

---

## Current Status

```yaml
current_phase: Phase A (not started)
decision_id: DEC-001
decision_status: rejected
rejection_reason: |
  Prerequisites not met:
  - CI execution count < 30
  - Reconciliation Matrix not generated
  - discrepancy_rate unknown
next_action: |
  Start Phase A parallel runs on develop/v3.7.0
  Target: 30 CI runs with both new and legacy CI
```

---

## Consequences

### Positive

- No coverage gap during retirement
- Data-driven decision (reconciliation matrix)
- Audit trail preserved
- Team confidence maintained

### Negative

- Slow: 6+ weeks from start to deletion
- Overhead: parallel CI runs, matrix generation
- Must maintain both CI systems during transition

### Neutral

- Can be paused at any phase
- Can restart from any phase if prerequisites met
- Does not block new development

---

## References

- ADR-001: Truthfulness Framework
- ADR-003: Decision Registry
- GATE_SPEC.md: Phase A~E protocol
- Pattern: PATTERN_LEGACY_RETIREMENT
- Template: TEMPLATE_DECISION_REGISTRY.md
- Lesson: LESSON_v3.7.0_GA.md (Lesson-01)
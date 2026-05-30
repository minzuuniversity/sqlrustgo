# ADR-003: Decision Registry

> **ADR ID**: ADR-003
> **Status**: accepted (2026-05-30)
> **Supersedes**: N/A
> **References**: ADR-001 (Truthfulness Framework), ADR-002 (Claim Registry)

---

## Status

`accepted` — This ADR is frozen. Any changes must be made via a new ADR that supersedes this one.

---

## Context

Governance decisions were made ad-hoc without structured rationale, evidence requirements, or audit trail. For example:
- Legacy gate retirement was proposed without prerequisites
- Coverage threshold changes had no decision record
- No distinction between "proposed", "approved", and "rejected"

This violates G-05 (Decision Rationale) and makes governance non-replayable.

---

## Decision

Establish the **Decision Registry** as the authoritative record of all governance decisions:

### Decision States

```
proposed
    ↓ (approved by governance committee)
approved
    ↓ (superseded by newer decision)
superseded
    ↓ (fully deployed and verified)
implemented
    ↓ (alternative path)
rejected
```

### Registry Format

```yaml
decision_id: DEC-XXX
title: "<short title>"
status: <proposed|approved|rejected|superseded|implemented>
proposed_at: <ISO timestamp>
proposed_by: <agent or human>
reason: |
  <list of reasons for the decision>
blocking_conditions:
  - <condition>
evidence_required:
  - <item>
related_claims:
  - CLAIM-XXX
related_decisions:
  - DEC-XXX
revision_history:
  - date: <timestamp>
    from: <old status>
    to: <new status>
    reason: <why changed>
```

### Rules

1. **Every DEC must have `reason`** (G-05)
2. **Every DEC must have `blocking_conditions`** before approval
3. **Every DEC must have `evidence_required`** before approval
4. **Status transitions are immutable** (append-only)
5. **Rejected decisions stay in record** (for audit)

### Example: DEC-001 (Rejected)

```yaml
decision_id: DEC-001
title: "Retire Legacy Gate Scripts"
status: rejected
proposed_at: 2026-05-28T15:00+08:00
proposed_by: Hermes Agent
reason: |
  1. insufficient_execution_data
  2. no_reconciliation_matrix
  3. Graph Gate vs Legacy Gate parallel run not started
  4. less than 20 CI runs on develop/v3.7.0
blocking_conditions:
  - "CI execution count < 30"
  - "Reconciliation Matrix not generated"
  - "discrepancy_rate > 5%"
evidence_required:
  - "30 consecutive CI runs (new + legacy)"
  - "Reconciliation Matrix with TP/FP/FN/TN"
  - "discrepancy_rate < 5%"
related_claims:
  - CLAIM-2582-01
revision_history:
  - date: 2026-05-28T15:00+08:00
    from: proposed
    to: rejected
    reason: prerequisites not met
```

---

## Consequences

### Positive

- Decisions are auditable with full rationale
- Blocking conditions prevent premature approval
- Historical decisions preserved for replay
- Governance becomes replayable

### Negative

- More overhead: DEC requires structured format
- Cannot modify history (frozen)
- Rejected decisions still must be recorded

### Neutral

- DEC can be proposed by any agent or human
- Status transitions require governance committee approval
- Registry is append-only

---

## References

- ADR-001: Truthfulness Framework
- GATE_SPEC.md: G-05 Decision Rationale
- Pattern: PATTERN_LEGACY_RETIREMENT
- Template: TEMPLATE_DECISION_REGISTRY.md
- Lesson: LESSON_v3.7.0_GA.md (Lesson-01)
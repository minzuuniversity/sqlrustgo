# Decision Registry — {{VERSION}}

> **Template**: TEMPLATE_DECISION_REGISTRY.md
> **Version**: v3.8.0
> **Usage**: Copy to `docs/releases/v{{VERSION}}/GOVERNANCE_FRAMEWORK.md` section 2

---

## Decision Status Definitions

| Status | Meaning |
|--------|---------|
| `proposed` | Under consideration |
| `approved` | Accepted for implementation |
| `rejected` | Not accepted (reason required) |
| `superseded` | Replaced by newer decision |
| `implemented` | Fully deployed and verified |

---

## Decision Format

```yaml
decision_id: DEC-XXX
title: "<short title>"
status: <proposed|approved|rejected|superseded|implemented>
proposed_at: <ISO timestamp>
proposed_by: <agent or human>
reason: |
  <list of reasons for the decision>
evidence_required:
  - <item>
blocking_conditions:
  - <condition>
related_claims: [<claim_ids>]
related_decisions: [<decision_ids>]
revision_history:
  - date: <timestamp>
    from: <old status>
    to: <new status>
    reason: <why changed>
```

---

## Example: Rejected Decision

```yaml
decision_id: DEC-001
title: Retire Legacy Gate Scripts
status: rejected
proposed_at: 2026-05-30T22:00+08:00
proposed_by: Hermes Agent
reason: |
  1. insufficient_execution_data
  2. no_reconciliation_matrix
  3. Graph Gate vs Legacy Gate parallel运行未完成
  4. less than 20 CI runs on develop/v3.7.0
blocking_conditions:
  - "CI execution count on develop/v3.7.0 < 30"
  - "Reconciliation Matrix not generated"
  - "discrepancy_rate > 5%"
evidence_required:
  - "30 consecutive CI runs with success_rate >= 95%"
  - "Reconciliation Matrix with TP/FP/FN/TN classified"
  - "discrepancy_rate < 5%"
related_claims:
  - CLAIM-007
  - CLAIM-008
revision_history:
  - date: 2026-05-30T22:00+08:00
    from: proposed
    to: rejected
    reason: prerequisites not met — Phase A (parallel run) not started
```

---

## Blank Decision Template

```yaml
decision_id: DEC-XXX
title: 
status: proposed
proposed_at: 
proposed_by: 
reason: |
  
blocking_conditions:
  - 
evidence_required:
  - 
related_claims: []
related_decisions: []
revision_history:
  - date: 
    from: 
    to: 
    reason: 
```

---

## Decision Registry Table

| Decision ID | Title | Status | Proposed | Blocking |
|-------------|-------|--------|----------|----------|
| DEC-001 | Retire Legacy Gate Scripts | rejected | 2026-05-30 | YES |
| DEC-002 | | | | |
| DEC-003 | | | | |
| DEC-004 | | | | |

---

## Change Log

| Date | Decision ID | Change | By |
|------|-------------|--------|-----|
| 2026-05-30 | DEC-001 | Initial — retire legacy rejected (prerequisites not met) | Hermes Agent |
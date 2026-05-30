# Claim Registry — {{VERSION}}

> **Template**: TEMPLATE_CLAIM_REGISTRY.md
> **Version**: v3.8.0
> **Usage**: Copy to `docs/releases/v{{VERSION}}/GOVERNANCE_FRAMEWORK.md` section 1.2

---

## Claim Registry Format

Every claim MUST follow this format:

```yaml
claim_id: CLAIM-XXX
title: "<short title>"
status: <asserted|supported|verified|disproven|mitigated|stale>
asserted_at: <ISO timestamp>
last_verified_at: <ISO timestamp>
evidence:
  - id: <evidence_id>
    type: <command|document|api>
    command: "<exact command>"
    stdout_sha256: "<sha256>"
    timestamp: <ISO timestamp>
    url: "<if applicable>"
conclusion_notes: "<context for status change>"
related_claims: [<other claim_ids>]
related_decisions: [<decision_ids>]
```

---

## Claim Status Definitions

| Status | Definition | Gate Impact |
|--------|------------|-------------|
| `asserted` | Initial claim, no evidence yet | Not counted as PASS |
| `supported` | Partial evidence, incomplete | Not counted as PASS |
| `verified` | Sufficient evidence + within G-06 30-day window | Counts as PASS |
| `disproven` | Contradicted by evidence | Releases blocks |
| `mitigated` | Risk accepted with compensating controls | Conditional PASS |
| `stale` | Evidence older than 30 days | Degraded to unverified |

---

## Example Claim Entry

```yaml
claim_id: CLAIM-001
title: develop/v3.7.0 push successful
status: verified
asserted_at: 2026-05-30T22:00+08:00
last_verified_at: 2026-05-30T22:00+08:00
evidence:
  - id: E-001
    type: git
    command: git cat-file -t fc7f603f
    stdout_sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    timestamp: 2026-05-30T22:00+08:00
conclusion_notes: |
  Verified: commit object exists and is reachable from origin/develop/v3.7.0
  Push confirmed at 2026-05-30T22:00+08:00
related_claims: []
related_decisions: []
```

---

## Blank Claim Template

```yaml
claim_id: CLAIM-XXX
title: 
status: asserted
asserted_at: 
last_verified_at: 
evidence:
  - id: 
    type: 
    command: 
    stdout_sha256: 
    timestamp: 
conclusion_notes: |
  
related_claims: []
related_decisions: []
```

---

## Claim Registry Table (Quick Reference)

| Claim ID | Title | Status | Last Verified | Blocking |
|----------|-------|--------|---------------|----------|
| CLAIM-001 | | | | |
| CLAIM-002 | | | | |
| CLAIM-003 | | | | |
| CLAIM-004 | | | | |
| CLAIM-005 | | | | |
| CLAIM-006 | | | | |
| CLAIM-007 | | | | |
| CLAIM-008 | | | | |
| CLAIM-009 | | | | |
| CLAIM-010 | | | | |

---

## Change Log

| Date | Claim ID | Change | By |
|------|----------|--------|-----|
| 2026-05-30 | CLAIM-001 | Initial | Hermes Agent |
| 2026-05-30 | CLAIM-002 | Initial | Hermes Agent |
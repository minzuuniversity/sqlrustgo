# ADR-001: Truthfulness Framework Adoption

> **ADR ID**: ADR-001
> **Status**: accepted (2026-05-30)
> **Supersedes**: N/A
> **Context**: v3.7.0 GA gate review exposed fundamental gaps in evidence handling

---

## Status

`accepted` — This ADR is frozen. Any changes must be made via a new ADR that supersedes this one.

---

## Context

During v3.7.0 GA gate review, multiple critical failures were discovered:

1. **Configured ≠ Executed**: Legacy CI gate reported PASS but no tests ran
2. **Documented ≠ Verified**: CLAIM-001~004 had no OBS provenance
3. **Threshold Negotiable**: Coverage threshold lowered 3 consecutive versions
4. **Debt Unchecked**: Architecture debt passed gate without blocking
5. **Evidence Stale**: Claims older than 30 days still treated as current

These are not isolated incidents — they reveal a systemic failure in governance evidence management.

---

## Decision

Adopt the **Truthfulness Framework** as the governing principle for all governance evidence:

### Core Principle

```
Configured ≠ Executed ≠ Verified
```

Four states:
- `configured` — file exists in repo
- `executed` — CI run attempted
- `verified` — evidence graph validated with sha256
- `stale` — evidence older than 30 days (G-06)

### G-01~G-07 Rules

| ID | Rule | Enforcement |
|----|------|-------------|
| G-01 | Evidence Required/PASS | No evidence = block |
| G-02 | Test Result Binding | results are binding |
| G-03 | No Evidence = Block | soft blocks with waiver |
| G-04 | Claim Provenance | every claim needs OBS |
| G-05 | Decision Rationale | DEC must have reason |
| G-06 | Evidence Freshness | 30-day window, auto-stale |
| G-07 | Negative Evidence | absence must be noted |

### Implementation

1. **GATE_SPEC.md** updated to v3.8.0 normative
2. **check_freshness.py** implements G-06 enforcement
3. **ci-governance-compliant.yml** requires sha256 evidence
4. **TEMPLATE_CLAIM_REGISTRY.md** enforces OBS format
5. **TEMPLATE_OBSERVATION_REGISTRY.md** created

---

## Consequences

### Positive

- Evidence chain is traceable: OBS → CLAIM → DEC → GATE
- Claims can be verified or disputed based on evidence
- Stale evidence automatically downgraded
- False positives (legacy gate) identified and actionable
- Truthfulness is now the default state

### Negative

- More overhead: sha256 evidence required for every PASS
- Slower gate review: must verify evidence chain
- Legacy gate must be retired or fixed (Phase A~E)
- Claims without OBS cannot be "verified"

### Neutral

- G-03 (No Evidence = Block) has waiver option
- Legacy gate retirement is a 5-phase process, not immediate
- Evidence freshness check is automated (not manual)

---

## References

- GATE_SPEC.md: `docs/governance/GATE_SPEC.md`
- Pattern: PATTERN_EVIDENCE_CHAIN
- Pattern: PATTERN_GATE_FALSE_POSITIVE
- Lesson: LESSON_v3.7.0_GA.md (Lesson-01, Lesson-04, Lesson-06)
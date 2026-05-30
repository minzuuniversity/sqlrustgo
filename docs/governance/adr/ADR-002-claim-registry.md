# ADR-002: Claim Registry

> **ADR ID**: ADR-002
> **Status**: accepted (2026-05-30)
> **Supersedes**: N/A
> **References**: ADR-001 (Truthfulness Framework)

---

## Status

`accepted` — This ADR is frozen. Any changes must be made via a new ADR that supersedes this one.

---

## Context

GOVERNANCE_FRAMEWORK.md contained CLAIM-001 through CLAIM-004 as "verified" claims, but closer inspection showed:
- No OBS (Observation) references
- No sha256 evidence
- No timestamp
- Claims were "stated" not "verified"

This is a G-04 (Claim Provenance) violation. Claims without evidence chain are just assertions.

---

## Decision

Establish the **Claim Registry** as the authoritative record of all governance claims:

### Claim States

```
asserted
    ↓ (evidence exists)
supported
    ↓ (evidence verified by execution)
verified
    ↓ (evidence disproved)
disproven
    ↓ (evidence mitigated)
mitigated
    ↓ (evidence older than 30 days)
stale
```

### Registry Format

```yaml
claim_id: CLAIM-XXX
text: "<what is claimed>"
status: <asserted|supported|verified|disproved|mitigated|stale>
proposed_at: <ISO timestamp>
proposed_by: <agent or human>
evidence_refs:
  - OBS-XXX  # must have ≥1
  - OBS-XXX
verdict: <established|disputed|pending>
last_verified_at: <ISO timestamp>
notes: "<additional context>"
```

### Rules

1. **Every CLAIM must have ≥1 OBS reference** (G-04)
2. **Only "verified" claims can be used in DEC-XXX decisions**
3. **Claims without OBS are "asserted" not "verified"**
4. **Claims older than 30 days are automatically "stale"** (G-06)
5. **"supported" ≠ "verified"** (L2 CI Gate correction)

### Implementation

- **TEMPLATE_CLAIM_REGISTRY.md**: Standard claim format
- **GOVERNANCE_FRAMEWORK.md**: Contains active claim registry
- **check_freshness.py**: Auto-downgrades stale claims
- **Gate review**: Must verify claim → OBS → evidence chain

---

## Consequences

### Positive

- Claims are traceable to observations
- Status transitions are explicit and auditable
- Stale claims auto-downgraded
- Claim quality improves (evidence-focused)

### Negative

- Claims without OBS must be backfilled
- Some existing "verified" claims will be downgraded to "asserted"
- More overhead: OBS required for every new claim

### Neutral

- "asserted" claims are still valid — just not verified
- Claim status can be upgraded when evidence is added
- Registry is append-only for historical record

---

## References

- ADR-001: Truthfulness Framework
- GATE_SPEC.md: G-04 Claim Provenance
- Pattern: PATTERN_EVIDENCE_CHAIN
- Lesson: LESSON_v3.7.0_GA.md (Lesson-04)
# Pattern — Evidence Chain

> **Pattern ID**: PATTERN_EVIDENCE_CHAIN
> **Category**: Governance Pattern
> **Severity**: High
> **Source**: v3.7.0 GA Review Issue #2613
> **Status**: Established

---

## Context

Claims made in governance documents must be traceable back to raw observations (OBS). Without this chain, claims are just assertions — they cannot be verified or disputed. G-04 (Claim Provenance) requires every claim to reference at least one OBS.

---

## Trigger

```yaml
trigger_conditions:
  - "CLAIM-XXX without OBS-XXX reference"
  - "Governance document without sha256 evidence"
  - "Decision made without observation log"
  - "Evidence older than 30 days (G-06)"
```

---

## Failure Mode

```yaml
failure_mode: |
  1. Document contains CLAIM-001: "Tests pass consistently"
  2. No OBS reference to actual test results
  3. Reviewer accepts as fact
  4. Actually: tests were never run or failed
  5. Gate passes on false claim

claim_states_wrong:
  - "asserted" → treated as "verified"
  - "supported" → treated as "verified"
  - "stated" → treated as "verified"
  
correct_chain: |
  OBS → CLAIM (asserted) → CLAIM (supported) → CLAIM (verified)
  
  Each transition requires evidence.
```

---

## Correct Action

```yaml
correct_action: |
  1. LOG: Every observation must be recorded as OBS-XXX
  2. HASH: OBS must include command + sha256 + timestamp
  3. REFERENCE: Every claim must cite ≥1 OBS
  4. VERIFY: Claims only "verified" if OBS evidence supports
  5. FRESH: OBS must be within 30-day window (G-06)

OBS_format: |
  OBS-XXX:
    command: <what was run>
    sha256: <hash of output>
    timestamp: <ISO 8601>
    description: <what was observed>
    
claim_format: |
  CLAIM-XXX:
    text: "<what is claimed>"
    status: <asserted|supported|verified|disproved|mitigated|stale>
    evidence_refs:
      - OBS-XXX
      - OBS-XXX
```

---

## Example (v3.7.0)

```yaml
issue: ISSUE-2613
discovery_date: 2026-05-29T10:00+08:00
observation: |
  GOVERNANCE_FRAMEWORK.md contains CLAIM-001~004
  but no OBS references.
  Claims were "stated" not "verified".
gate_result: L2 (Claim Provenance) FAIL
root_cause: |
  G-04 violation: CLAIM-001~004 lack OBS provenance
  Claims treated as verified without evidence chain
resolution: |
  DEC-2613-01: Add OBS provenance to all claims
  commit a918bf6d: Added timestamp to CLAIM-001~004
  OBS-001~004 created with command + sha256 + timestamp
```

---

## Detection Checklist

```yaml
detection_rules:
  - name: "Claim OBS Check"
    check: "Every CLAIM has ≥1 OBS reference?"
    result: FAIL → claim provenance violation
    
  - name: "OBS Format Check"
    check: "OBS contains command + sha256 + timestamp?"
    result: FAIL → incomplete observation
    
  - name: "Freshness Check"
    check: "OBS within 30-day window?"
    result: FAIL → stale evidence (G-06)
    
  - name: "Status Transition Check"
    check: "supported→verified requires evidence?"
    result: FAIL → insufficient evidence
```

---

## Related Patterns

- PATTERN_GATE_FALSE_POSITIVE (prevention through evidence)
- PATTERN_COVERAGE_DISPUTE (evidence required for threshold change)
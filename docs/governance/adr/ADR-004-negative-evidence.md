# ADR-004: Negative Evidence

> **ADR ID**: ADR-004
> **Status**: accepted (2026-05-30)
> **Supersedes**: N/A
> **References**: ADR-001 (Truthfulness Framework)

---

## Status

`accepted` — This ADR is frozen. Any changes must be made via a new ADR that supersedes this one.

---

## Context

Governance documents often report what exists (positive evidence) but rarely document what does NOT exist (negative evidence). This creates a gap:

- "No test failures" ≠ "Tests ran successfully"
- "No objections" ≠ "Consensus reached"
- "No debt items" could mean: (a) no debt, or (b) debt not tracked

Without negative evidence, absence of information is interpreted as absence of problems.

---

## Decision

Adopt **Negative Evidence** as a required component of governance reporting:

### Definition

**Negative Evidence**: Documentation of what was checked and found absent.

### Format

```yaml
negative_evidence_id: NE-XXX
description: "<what was checked but NOT found>"
checked_at: <ISO timestamp>
checked_by: <agent or human>
search_scope: |
  <where the search was conducted>
search_method: |
  <how the search was conducted>
result: "<absent|present|unknown>"
implications: |
  <what the absence means>
```

### Examples

```yaml
# Example 1: No Test Failures
negative_evidence_id: NE-001
description: "No test failures in L2 CI gate"
checked_at: 2026-05-30T10:00+08:00
search_scope: "test-results/*.xml, target/test/**/*.xml"
search_method: "grep -r 'failure' test-results/"
result: absent
implications: "Tests passed OR tests did not run"

# Example 2: No Legacy Gate False Positives
negative_evidence_id: NE-002
description: "No false positives in new CI gate (L2)"
checked_at: 2026-05-30T10:00+08:00
search_scope: ".github/workflows/ci-*.yml"
search_method: "sha256 verification of test logs"
result: absent
implications: "L2 gate is producing true positives"

# Example 3: No Untracked Debt
negative_evidence_id: NE-003
description: "No debt items untracked in DEBT_TRACKING.md"
checked_at: 2026-05-30T10:00+08:00
search_scope: "docs/governance/DEBT_TRACKING.md"
search_method: "git log --grep='debt|technical|refactor'"
result: absent
implications: "All known debt is tracked"
```

### Rules

1. **Absence of evidence ≠ evidence of absence** (G-07)
2. **Must document what was checked** (negative evidence)
3. **"Not found" must be stated explicitly**
4. **Unknown status must be labeled as "unknown"**
5. **Negative evidence has the same freshness window as OBS** (30 days)

### Detection

| Check | Positive | Negative (G-07 required) |
|-------|----------|------------------------|
| Test results | "Tests passed" | "No failures found, AND tests executed" |
| Debt items | "No debt" | "DEBT_TRACKING.md reviewed, 0 items" |
| Coverage | "Coverage 64%" | "Coverage 64% AND tests executed" |
| Legacy gate | "Gate PASS" | "Gate PASS AND sha256 evidence exists" |

---

## Consequences

### Positive

- Ambiguity eliminated: "absent" vs "unknown" vs "not checked"
- Hidden problems surfaced (what we thought was fine was actually not checked)
- Negative evidence enables proper gap analysis
- Governance reports become more honest

### Negative

- More documentation overhead
- Must explicitly state "not checked" vs "checked and absent"
- Unknown status is uncomfortable to report

### Neutral

- Negative evidence can upgrade or downgrade claims
- "Unknown" is a valid result (just must be labeled as such)
- Does not block — only requires explicit labeling

---

## References

- ADR-001: Truthfulness Framework
- GATE_SPEC.md: G-07 Negative Evidence
- Pattern: PATTERN_GATE_FALSE_POSITIVE (prevention)
- Lesson: LESSON_v3.7.0_GA.md (Lesson-01)
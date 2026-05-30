# Pattern — Coverage Threshold Dispute

> **Pattern ID**: PATTERN_COVERAGE_DISPUTE
> **Category**: Governance Anti-Pattern
> **Severity**: High
> **Source**: v3.7.0 GA Review Issue #2580
> **Status**: Established

---

## Context

During v3.7.0 gate review, coverage metrics showed a declining trend across 3 consecutive versions. Rather than improving test quality, the team adjusted the coverage threshold downward to maintain gate PASS status.

---

## Trigger

```yaml
trigger_conditions:
  - "coverage_actual < coverage_threshold"
  - "deadline_pressure: < 2 weeks to GA"
  - "gate_pass_required_for_release"
  - "no_new_tests_merged_in_2+ weeks"
```

---

## Failure Mode

```yaml
failure_mode: |
  1. Developer or PM proposes: "lower the threshold"
  2. Gate committee accepts without proper governance
  3. Threshold changed AFTER alpha/beta started (freeze violation)
  4. Gate passes with reduced standards
  5. Quality degrades silently

wrong_decisions:
  - "v3.4.0: 70% → v3.5.0: 68% → v3.6.0: 65%"
  - "Threshold negotiated AFTER alpha start"
  - "No evidence required for threshold change"

anti_pattern_chain:
  - "Actual coverage drops"
  - "Deadline pressure increases"
  - "Threshold lowered"
  - "Gate passes"
  - "Next version: even lower threshold"
```

---

## Correct Action

```yaml
correct_action: |
  1. PREVENT: Lock threshold in GATE_SPEC.md before alpha starts
  2. DETECT: Monitor coverage trend, alert at first drop
  3. RESPOND: Improve test coverage, do NOT lower threshold
  4. ESCALATE: If coverage cannot meet threshold, escalate to steering committee
  5. DOCUMENT: Any coverage discussion must include evidence

fixed_threshold_rule: |
  - Threshold must be defined in GATE_SPEC.md at version start
  - Threshold is FROZEN after alpha starts
  - Changing threshold requires DEC-XXX with full governance process
  - DEC must include: evidence of coverage trend + remediation plan
```

---

## Example (v3.7.0)

```yaml
issue: ISSUE-2580
discovery_date: 2026-05-28T14:00+08:00
coverage_trend:
  - v3.4.0: 70% threshold → actual 72%
  - v3.5.0: 68% threshold → actual 70%
  - v3.6.0: 65% threshold → actual 67%
  - v3.7.0: 60% threshold → actual 64%
proposed_action: |
  "v3.7.0 threshold 60% is acceptable"
correct_response: |
  1. Coverage 64% < 65% (previous version threshold)
  2. This is a REGRESSION
  3. Threshold 60% would reward regression
  4. Correct action: improve tests to reach 65%
resolution: |
  DEC-2580-01: Coverage threshold must be FIXED before alpha
  Pattern PATTERN_COVERAGE_DISPUTE established
  GATE_SPEC.md v3.8.0 locks threshold before alpha
```

---

## Detection Checklist

```yaml
detection_rules:
  - name: "Threshold Comparison"
    check: "Is current threshold < previous version threshold?"
    result: FAIL → potential dispute
    
  - name: "Timeline Check"
    check: "Was threshold changed after alpha start?"
    result: FAIL → freeze violation
    
  - name: "Evidence Check"
    check: "Was there evidence that coverage improved?"
    result: FAIL → insufficient evidence
    
  - name: "Trend Analysis"
    check: "3+ consecutive versions with declining coverage?"
    result: FAIL → systematic problem
```

---

## Related Patterns

- PATTERN_GATE_FALSE_POSITIVE (related: false PASS)
- PATTERN_ARCHITECTURE_DEBT (related: threshold masking debt)
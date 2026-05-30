# Pattern — Architecture Debt Passing Gate

> **Pattern ID**: PATTERN_ARCHITECTURE_DEBT
> **Category**: Governance Anti-Pattern
> **Severity**: Medium
> **Source**: v3.7.0 GA Review Issue #2615
> **Status**: Established

---

## Context

Architecture debt items (broken APIs, refactoring backlog, tech stack obsolescence) are tracked in DEBT_TRACKING.md but do not appear in gate blocking conditions. As a result, gates pass even when significant known debt exists — creating technical insolvency that eventually blocks all progress.

---

## Trigger

```yaml
trigger_conditions:
  - "DEBT_TRACKING.md has > 3 items"
  - "No debt check in gate checklist"
  - "Debt items > 6 months unresolved"
  - "Gate passes with known breaking changes"
```

---

## Failure Mode

```yaml
failure_mode: |
  1. DEBT_TRACKING.md documents known debt
  2. Gate checklist has no debt review
  3. Gate passes despite debt
  4. New code depends on broken APIs
  5. Debt compounds (cannot refactor without breaking deps)
  6. Eventually: gate always fails, no release possible

wrong_assumptions:
  - "Documented debt = managed debt"
  - "Gate pass = quality OK"
  - "Debt is not blocking"
  - "Technical debt is someone else's problem"

debt_accumulation_cycle: |
  v3.4.0: 2 debt items → gate passes
  v3.5.0: 4 debt items → gate passes
  v3.6.0: 7 debt items → gate passes
  v3.7.0: 12 debt items → gate passes
  v3.8.0: 18 debt items → gate BLOCKS all releases
```

---

## Correct Action

```yaml
correct_action: |
  1. MANDATE: Debt check in every gate (L6 sub-check)
  2. CAP: Max debt items per version (e.g., 3)
  3. ESCALATE: Debt > 6 months → steering committee
  4. BLOCK: Debt items > threshold → gate fails
  5. REMEDIATE: Each release must reduce or resolve ≥1 debt item

L6_debt_gate:
  L6-1: "DEBT_TRACKING.md is up to date"
  L6-2: "No single debt item > 6 months old"
  L6-3: "Total debt items < 10"
  L6-4: "Each release resolves ≥1 debt item"
  L6-5: "No new debt without remediation plan"
  
total_score: 15 (3 pts each × 5 sub-checks)
pass_threshold: 10 (67%)
```

---

## Example (v3.7.0)

```yaml
issue: ISSUE-2615
discovery_date: 2026-05-29T11:00+08:00
debt_items_found:
  - sql-cli API breakage (since v2.6.0, 18 months)
  - WAL change causing executor test failures
  - storage engine abstraction not finalized
  - Missing test coverage for new modules
gate_result: L6 FAIL
root_cause: |
  Architecture debt check not in gate checklist
  DEBT_TRACKING.md exists but not enforced
resolution: |
  DEC-2615-01: Add L6 (Architecture Debt Gate) to GATE_SPEC.md
  L6-1~L6-5 sub-checks added
  DEBT_TRACKING.md must be updated before each gate
```

---

## Debt Categories

```yaml
debt_severity:
  critical:
    - "Breaking API change (affects external users)"
    - "Security vulnerability"
    - "Data loss risk"
    
  high:
    - "Executor test failures"
    - "Missing test coverage > 20%"
    - "Storage engine instability"
    
  medium:
    - "Code duplication"
    - "Missing documentation"
    - "Deprecated dependencies"
    
  low:
    - "Style inconsistencies"
    - "Minor code smells"
    - "Non-critical warnings"
```

---

## Detection Checklist

```yaml
detection_rules:
  - name: "Debt Check Presence"
    check: "L6 (Architecture Debt Gate) in gate checklist?"
    result: FAIL → debt not enforced
    
  - name: "Debt Count"
    check: "Total debt items < 10?"
    result: FAIL → too much debt
    
  - name: "Debt Age"
    check: "No debt item > 6 months?"
    result: FAIL → stale debt
    
  - name: "Debt Remediation"
    check: "Each release resolves ≥1 debt item?"
    result: FAIL → debt accumulating
```

---

## Related Patterns

- PATTERN_COVERAGE_DISPUTE (related: threshold masking debt)
- PATTERN_GATE_FALSE_POSITIVE (related: false sense of security)
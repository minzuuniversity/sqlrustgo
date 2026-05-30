# Observation Registry Template

> **Template**: TEMPLATE_OBSERVATION_REGISTRY.md
> **Version**: v3.8.0
> **Usage**: Copy to `docs/releases/v{{VERSION}}/OBSERVATION_REGISTRY.md`

---

## Observation Registry Format

### Core Format

```yaml
obs_id: OBS-XXX
type: <quantitative_discrepancy|false_positive|missing_evidence|architecture_debt|negative_evidence>
command: |
  <exact command or search executed>
sha256: <sha256 hash of raw output>
timestamp: <ISO 8601>
description: |
  <what was observed — factual, no interpretation>
implications: |
  <what the observation means for governance>
related_claims:
  - CLAIM-XXX
```

### Field Specifications

| Field | Required | Format | Notes |
|-------|----------|--------|-------|
| `obs_id` | YES | `OBS-XXX` | Sequential, reset per version |
| `type` | YES | enum | See type taxonomy below |
| `command` | YES | string | Exact command, no truncation |
| `sha256` | YES | hex | Hash of raw output, not processed |
| `timestamp` | YES | ISO 8601 | Include timezone |
| `description` | YES | string | Factual, no interpretation |
| `implications` | NO | string | Optional governance impact |
| `related_claims` | NO | list | CLAIM-XXX references |

### Type Taxonomy

```yaml
types:
  quantitative_discrepancy:
    description: "Numbers don't match expected values"
    examples: ["coverage 64% < 65% threshold", "7 failures in 408 tests"]
    
  false_positive:
    description: "Gate reports PASS but should FAIL"
    examples: ["legacy CI shows green but no tests ran"]
    
  missing_evidence:
    description: "Required evidence not found"
    examples: ["CLAIM-XXX has no OBS reference"]
    
  architecture_debt:
    description: "Known technical debt item"
    examples: ["sql-cli API broken 18 months"]
    
  negative_evidence:
    description: "Documented absence of something"
    examples: ["No test failures found in 3 weeks"]
```

---

## Example: OBS-001

```yaml
obs_id: OBS-001
type: quantitative_discrepancy
command: |
  grep -E "coverage|covered" target/coverage/*.json 2>/dev/null \
  | awk -F',' '{sum+=$2} END {print sum/NR "%"}'
sha256: a1b2c3d4e5f6...
timestamp: 2026-05-28T14:00+08:00
description: |
  Coverage measured at 64% across 14 crates.
  Previous version threshold was 65%.
  Current proposed threshold is 60%.
  This is a 1pp drop from previous threshold.
implications: |
  Coverage is below previous version threshold.
  Proposed new threshold (60%) would be 5pp below previous.
  Pattern: threshold lowered 3 consecutive versions.
related_claims:
  - CLAIM-2580-01
```

---

## Example: OBS-002

```yaml
obs_id: OBS-002
type: false_positive
command: |
  find .github/workflows -name "legacy*.yml" -exec cat {} \; \
  | grep -E "status|last_run|commit" \
  | head -20
sha256: f7g8h9i0j1k2...
timestamp: 2026-05-28T14:30+08:00
description: |
  Legacy CI workflow file exists at .github/workflows/legacy-gate.yml
  GitHub Actions shows "This workflow has not been run yet"
  No test logs in test-results/ or target/test/
  Last commit to workflow file: 2026-05-01 (3 weeks ago)
implications: |
  Legacy CI gate is configured but not executed.
  Reports PASS in pipeline but no tests have run.
  This is a false positive.
related_claims:
  - CLAIM-2582-01
```

---

## Example: OBS-003 (Negative Evidence)

```yaml
obs_id: OBS-003
type: negative_evidence
command: |
  grep -r "CLAIM-" docs/releases/v3.7.0/ \
  | grep -v "OBS-" \
  | wc -l
sha256: k3l4m5n6o7p8...
timestamp: 2026-05-29T10:00+08:00
description: |
  Searched docs/releases/v3.7.0/ for CLAIM- references.
  Found CLAIM-001, CLAIM-002, CLAIM-003, CLAIM-004.
  None of them have an OBS- reference in the same document.
  Search scope included GOVERNANCE_FRAMEWORK.md, GATE_REPORT.md
implications: |
  CLAIM-001~004 are asserted but not verified.
  G-04 (Claim Provenance) is violated.
  Claims cannot be used in decisions without OBS.
related_claims:
  - CLAIM-2613-01
```

---

## Blank Template

```yaml
obs_id: OBS-XXX
type: 
command: |
  
sha256: 
timestamp: 
description: |
  
implications: |
  
related_claims: []
```

---

## Registry Index

| OBS ID | Type | Date | Issue | Status |
|--------|------|------|-------|--------|
| OBS-001 | quantitative_discrepancy | 2026-05-28 | #2580 | active |
| OBS-002 | false_positive | 2026-05-28 | #2582 | active |
| OBS-003 | missing_evidence | 2026-05-29 | #2613 | active |
| OBS-004 | architecture_debt | 2026-05-29 | #2615 | active |
| OBS-XXX | | | | |

---

## Freshness Rules (G-06)

```yaml
freshness:
  window: 30 days
  auto_stale: true
  stale_action: |
    OBS older than 30 days → status: stale
    Stale OBS cannot support verified claims
    Must re-run command to refresh
  check_script: check_freshness.py
```

---

## Change Log

| Date | OBS ID | Change | By |
|------|--------|--------|-----|
| 2026-05-30 | OBS-001~004 | Initial v3.7.0 observations | Hermes Agent |
| 2026-05-31 | OBS-XXX | | |
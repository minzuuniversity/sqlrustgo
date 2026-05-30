# Gate Specification — SQLRustGo Governance Standard

> **Version**: v3.8.0
> **Status**: Active (supersedes all prior gate_spec.md)
> **Standard**: G-01 through G-07, GOVERNANCE_FRAMEWORK.md
> **Last Updated**: 2026-05-30

---

## 0. Scope and Purpose

This document defines the **authoritative specification** for SQLRustGo gate system.
It is a normative reference, not advisory.

All other gate documents (GA_GATE_CHECKLIST.md, GATE_CONDITIONS.md, CI workflows)
must conform to this specification.

---

## 1. Gate Definition

### 1.1 Formal Definition

```
Gate Result = Claim → Observation → Evidence → Decision
```

No shortcut is permitted. Any gate output that skips steps is INVALID.

### 1.2 Four States

Every gate item exists in exactly one state:

```
CONFIGURED    →  EXECUTED    →  OBSERVED    →  VERIFIED
   (yaml)         (CI ran)       (logs)        (evidence)
```

**No state may be skipped.**

### 1.3 Gate Authority

| Gate Type | Authority | AI Can Decide |
|-----------|-----------|---------------|
| **Hard Gate** | CI system / policy engine output | ❌ No |
| **Soft Gate** | AI-generated text review | ⚠️ Reference only |

**Rule**: Only Hard Gate results can determine PASS/FAIL.

---

## 2. G-Rules (Normative)

### G-01: Evidence Required for PASS

```text
Any gate PASS must be traceable to Evidence Graph
Else → UNVERIFIED (NOT PASS)
```

**Implementation**: Every gate check must produce:
- Command executed
- stdout sha256
- exit code
- timestamp

### G-02: Test Result Binding

```text
Any test result must be bound: Test → Commit → CI Run → Artifact
Else → UNVERIFIED
```

**Implementation**: Each test must produce evidence chain.

### G-03: No Evidence = Block

```text
Cannot prove ≠ through
Must become: cannot prove → UNVERIFIED → block release
```

**Implementation**: UNVERIFIED is not PASS. UNVERIFIED blocks GA.

### G-04: Claim Provenance

```text
All claims must document: source, method, timestamp
Self-asserted claims disallowed
```

**Implementation**: Claim registry with evidence binding.

### G-05: Decision Rationale

```text
All decisions must include: reason + evidence
Without both → REJECTED
```

**Implementation**: Decision registry with DEC-XXX format.

### G-06: Evidence Freshness

```text
Verified claims must have execution evidence within 30 days
Else → stale → degrade to unverified
```

**Implementation**: `scripts/gate/check_freshness.py`

### G-07: Negative Evidence

```text
No evidence of X ≠ evidence of not-X
Both require independent proof
```

**Implementation**: OBS-XXX record with command + sha256.

---

## 3. Claim Registry Specification

### 3.1 Status States

| Status | Meaning |
|--------|---------|
| `asserted` | Initial, no evidence |
| `supported` | Partial evidence |
| `verified` | Full evidence + fresh |
| `disproven` | Contradicted by evidence |
| `mitigated` | Risk accepted with controls |
| `stale` | Evidence older than 30 days |

### 3.2 Required Fields

```yaml
claim_id: CLAIM-XXX
title: "<short title>"
status: <status>
asserted_at: <ISO timestamp>
evidence:
  - id: <evidence_id>
    type: <command|document|api>
    command: "<exact command>"
    stdout_sha256: "<sha256>"
    timestamp: <ISO>
conclusion_notes: "<context>"
related_claims: [<claim_ids>]
related_decisions: [<decision_ids>]
```

### 3.3 Governance Compliance

Every claim MUST:
- Have at least one evidence entry with command + sha256
- Include timestamp for G-06 freshness tracking
- Be in Claim Registry (GOVERNANCE_FRAMEWORK.md section 1.2)

---

## 4. Decision Registry Specification

### 4.1 Status States

| Status | Meaning |
|--------|---------|
| `proposed` | Under consideration |
| `approved` | Accepted for implementation |
| `rejected` | Not accepted (reason required) |
| `superseded` | Replaced by newer decision |
| `implemented` | Fully deployed and verified |

### 4.2 Required Fields

```yaml
decision_id: DEC-XXX
title: "<short title>"
status: <status>
proposed_at: <ISO timestamp>
proposed_by: <agent or human>
reason: |
  <list of reasons>
blocking_conditions:
  - <condition>
related_claims: [<claim_ids>]
```

---

## 5. Observation Registry Specification

### 5.1 Required Fields (G-07 Compliant)

```yaml
observation_id: OBS-XXX
timestamp: <ISO timestamp>
source:
  type: <command|api|file|manual>
  command: "<exact command>"
  stdout_sha256: "<sha256>"
  stderr_sha256: "<sha256>"
  exit_code: <integer>
  url: "<if API>"
finding: "<what was observed>"
result:
  status: <observed_state>
  raw_output: "<relevant excerpt>"
related_claims:
  - <claim_id>
blocking: <true|false>
```

### 5.2 Negative Evidence Rule

```
Claim: "runner_offline"
Required evidence:
  - process_not_found OR api_reports_offline with timestamp
  - Cannot be "curl returned empty" alone

Claim: "claim_unverifiable"
Required evidence:
  - Multiple independent probes returned empty
```

---

## 6. Gate Execution Requirements

### 6.1 Pre-Gate Requirements

Before any gate runs:
1. Claim registry exists and is populated
2. OBS-XXX records have been collected for critical claims
3. G-06 freshness window is satisfied or claims degraded

### 6.2 Gate Output Format

Every gate execution MUST produce:

```yaml
gate_execution:
  gate_id: GATE-<version>-<phase>
  timestamp: <ISO>
  executed_by: <agent>
  commit_sha: <sha>
  results:
    - check_id: L1-1
      status: <pass|fail|unverified>
      evidence:
        command: "<command>"
        stdout_sha256: "<sha256>"
        exit_code: <int>
        observation_id: OBS-XXX
  claim_updates:
    - claim_id: CLAIM-XXX
      status: verified|unverified|stale
  decisions:
    - decision_id: DEC-XXX
      status: approved|rejected
```

### 6.3 Gate Summary

```
=== Gate Result ===
PASS: all hard gates verified
CONDITIONAL: supported claims on non-critical path
FAIL: any hard gate unverified or disproven
BLOCKED: critical claim unverified
```

---

## 7. CI/CD Template Requirements

### 7.1 Required Sections

Every CI workflow for SQLRustGo MUST include:

```yaml
jobs:
  lint-build:     # L1 compile + style
  test:           # L2 unit tests
  postcheck:      # L3+ gates + evidence binding
    steps:
      - name: Verify Runner Online      # OBS-XXX
      - name: Run Gate (gate.sh)        # L1-L7
      - name: Run Coverage Check        # L5
      - name: Run Evidence Binding     # G-01/G-02
      - name: Gate Summary + OBS Record # G-07
```

### 7.2 sha256 Evidence Logging

Every artifact MUST log its sha256:

```bash
command 2>&1 | tee output.log
echo "output_sha256=$(sha256sum output.log | cut -d' ' -f1)" >> $GITHUB_OUTPUT
```

### 7.3 Branch Filter

```yaml
on:
  push:
    branches:
      - develop/v2.8.0
      - develop/v3.7.0
      - develop/v3.8.0
```

---

## 8. Legacy Gate Retirement Protocol

### 8.1 Prerequisites

| Condition | Threshold | Current Status |
|-----------|-----------|----------------|
| CI runs on target branch | ≥ 30 runs | 0 (UNVERIFIED) |
| Success rate | ≥ 95% | N/A |
| Reconciliation Matrix | Generated | NOT STARTED |
| Discrepancy rate | < 5% | N/A |

**Current rule**: Do NOT delete legacy scripts until all prerequisites met.

### 8.2 Phase Model

```
Phase A: Parallel Run (collect ≥ 30 CI runs)
Phase B: Reconciliation Matrix generation
Phase C: Decision (approve/reject retirement)
Phase D: Advisory Mode (legacy demoted)
Phase E: Deletion (only after ≥ 3 months in Phase D)
```

---

## 9. Maturity Level Reference

| Level | Name | Status in v3.8.0 |
|-------|------|------------------|
| L0 | Document-driven | N/A |
| L1 | Checklist | N/A |
| L2 | CI Gate | **supported** (config exists, exec unverified) |
| L3 | Evidence Binding | **verified** (check_evidence_binding.sh) |
| L4 | Claim Governance | **supported** (Claim/Decision exist, O-Record partial) |
| L5 | Continuous Audit | **none** |

---

## 10. Related Documents

| Document | Role |
|----------|------|
| `docs/releases/v3.8.0/GOVERNANCE_FRAMEWORK.md` | Claim/Decision/Observation registries, G-06/G-07 rules |
| `docs/governance/GOVERNANCE_V4_1_RUNTIME_SPEC.md` | Evidence Graph enforcement |
| `docs/governance/ANTI_FABRICATION_POLICY.md` | Fraud type definitions |
| `docs/releases/v3.8.0/GA_GATE_CHECKLIST.md` | L1-L7 gate execution checklist |
| `.gitea/workflows/ci-governance-compliant.yml` | CI/CD template |

---

## 11. Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-05-30 | v3.8.0 — Add G-01~G-07 normative rules, Claim/Decision/Observation specs | Hermes Agent |
| 2026-05-30 | Add G-06 freshness (30-day window) and G-07 negative evidence rule | Hermes Agent |
| 2026-05-30 | Correct L2 CI Gate: 'supported' not 'verified' | Hermes Agent |
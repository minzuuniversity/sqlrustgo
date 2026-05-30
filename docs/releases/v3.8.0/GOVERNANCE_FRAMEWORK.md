# SQLRustGo Governance Framework

> **Version**: v3.8.0
> **Status**: Active
> **Supersedes**: All prior governance documents (v3.0–v3.7)
> **Last Updated**: 2026-05-30
> **Standard**: Evidence Graph Gate v4.1 + Truthfulness Governance

---

## 0. Governing Principles

### 0.1 The Four States

Every item in this system exists in exactly one of four states:

```
CONFIGURED    →  EXECUTED    →  OBSERVED    →  VERIFIED
   (yaml exists)    (CI ran)       (logs seen)    (evidence bound)
```

**No state may be skipped.** Writing YAML does not mean executed. Executed does not mean verified.

### 0.2 Forbidden Behaviors

| Behavior | Why Forbidden |
|----------|---------------|
| Rewriting plan docs to match outcomes | Destroys historical truth |
| Claiming PASS without execution log | Violates G-03 |
| Deleting Legacy before reconciliation | Premature system retirement |
| Treating configured as verified | Core governance failure |
| Observation without command evidence | Enables observation造假 |

### 0.3 Compliant Patterns

```
✅ Correct:
  Claim → Observation → Evidence → Decision

❌ Forbidden:
  Claim → Decision (no observation/evidence)
  Claim → VERIFIED (without execution record)
  Decision → Implemented (without reconciliation data)
```

---

## 1. Claim Registry

### 1.1 Claim Status System

| Status | Meaning | Evidence Required |
|--------|---------|-------------------|
| `asserted` | Initial claim, no evidence | None (starting state) |
| `supported` | Partial evidence, incomplete | Some evidence, gaps remain |
| `verified` | Sufficient evidence, within freshness window | Full evidence + fresh |
| `disproven` | Contradicted by evidence | Counter-evidence recorded |
| `mitigated` | Risk accepted with controls | Evidence of controls |
| `stale` | Evidence older than G-06 window | Deprecated until refresh |

### 1.2 Claim Format

```yaml
claim_id: CLAIM-XXX
title: "<short title>"
status: <one of above>
asserted_at: <timestamp>
evidence:
  - id: <evidence_id>
    type: <command|document|api_response>
    command: "<exact command>"
    stdout_sha256: "<sha256 of raw output>"
    timestamp: "<ISO timestamp>"
    url: "<if applicable>"
conclusion_notes: "<context for status change>"
related_claims: [<other claim_ids>]
related_decisions: [<decision_ids>]
```

### 1.3 Claim Registry (v3.7.0 Review Example)

| Claim ID | Title | Status | Evidence |
|----------|-------|--------|----------|
| CLAIM-001 | develop/v3.7.0 push successful | **verified** | commit `fc7f603f` object exists (2026-05-30) |
| CLAIM-002 | gate-audit content merged | **verified** | git diff 0d6d3587..fc7f603f (+765/-186) (2026-05-30) |
| CLAIM-003 | develop/v3.7.0 in CI branch filter | **verified** | ci.yml includes `develop/v3.7.0` (3 occurrences) (2026-05-30) |
| CLAIM-004 | gate.sh is primary gate entrypoint | **verified** | ci.yml line 102 → gate/gate.sh (2026-05-30) |
| CLAIM-005 | gate.sh is sole gate entrypoint | **disproven** | check_coverage.sh + check_evidence_binding.sh still call scripts/gate/ |
| CLAIM-006 | CI workflow executed on develop/v3.7.0 | **unverified** | runner API returned empty |
| CLAIM-007 | Runner online | **unverified** | runner API returned empty |
| CLAIM-008 | CI execution succeeded | **unverified** | no job log captured |
| CLAIM-009 | Legacy Gate can be retired | **rejected** | reconciliation not started |

---

## 2. Decision Registry

### 2.1 Decision Status System

| Status | Meaning |
|--------|---------|
| `proposed` | Under consideration |
| `approved` | Accepted for implementation |
| `rejected` | Not accepted (reason required) |
| `superseded` | Replaced by newer decision |
| `implemented` | Fully deployed and verified |

### 2.2 Decision Format

```yaml
decision_id: DEC-XXX
title: "<short title>"
status: <one of above>
proposed_at: <timestamp>
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

### 2.3 Decision Registry Example

```yaml
decision_id: DEC-001
title: Retire Legacy Gate Scripts
status: rejected
proposed_at: 2026-05-30T22:00+08:00
proposed_by: Hermes Agent
reason:
  - insufficient_execution_data
  - no_reconciliation_matrix
  - Graph Gate vs Legacy Gate parallel运行未完成
  - less than 20 CI runs on develop/v3.7.0
blocking_conditions:
  - "CI execution count on develop/v3.7.0 < 20"
  - "Reconciliation Matrix not generated"
  - "discrepancy_rate > 5%"
evidence_required:
  - "30 consecutive CI runs with success_rate >= 95%"
  - "Reconciliation Matrix with TP/FP/FN/TN classified"
  - "discrepancy_rate < 5%"
related_claims:
  - CLAIM-008
  - CLAIM-009
```

---

## 3. Observation Registry

### 3.1 Observation Format (G-07 Compliance)

Every observation MUST include:

```yaml
observation_id: OBS-XXX
timestamp: <ISO timestamp>
source:
  type: <command|api|file|manual>
  command: "<exact command run>"
  stdout_sha256: "<sha256 of raw stdout>"
  stderr_sha256: "<sha256 of raw stderr (if relevant)>"
  exit_code: <integer>
  url: "<if from API or web>"
finding: "<what was observed>"
result:
  status: <observed_state>
  raw_output: "<relevant excerpt>"
related_claims:
  - <claim_id>
blocking: <true/false>
notes: "<any caveats>"
```

### 3.2 Negative Evidence Rule (G-07)

```
No evidence of X
≠
Evidence of not-X
```

| Claim | Required Evidence |
|-------|------------------|
| `runner_offline` | `process_not_found` OR `api_reports_offline` with timestamp |
| `claim_unverifiable` | Multiple independent probes returned empty |
| `test_not_run` | No test artifact, no CI log entry |

### 3.3 Observation Registry Example

```yaml
observation_id: OBS-001
timestamp: 2026-05-30T22:30+08:00
source:
  type: api
  command: curl http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/actions/runners
  stdout_sha256: e3b0c44... (empty JSON array)
  stderr_sha256: N/A
  exit_code: 0
  url: http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/actions/runners
finding: Gitea Actions runner API returned empty array
result:
  status: unable_to_verify_runner_online
  raw_output: "[]"
related_claims:
  - CLAIM-007
blocking: true
notes: API returned HTTP 200 with empty body. Cannot distinguish between
  "runners exist but API error" vs "no runners configured"
```

---

## 4. Evidence Freshness Rule (G-06)

### 4.1 Rule Statement

Any `verified` claim MUST have execution evidence within the last **30 days**.

If evidence is older than 30 days, the claim automatically degrades to `stale`.

### 4.2 Freshness Enforcement

```yaml
g-06_enforcement:
  window_days: 30
  grace_period_days: 5
  action_on_expiry: degrade_to_stale
  refresh_method: repeat original observation with new timestamp
  escalation: stale claims trigger governance review
```

### 4.3 Stale Claim Recovery

```
Stale Claim
    ↓ (repeat original observation command)
New Observation
    ↓ (evidence meets threshold)
Claim restored to verified
    ↓ (with new freshness timestamp)
```

---

## 5. Governance Maturity Model

### 5.1 Level Definitions

| Level | Name | Characteristics |
|-------|------|-----------------|
| L0 | Document-driven | Plans exist, no enforcement |
| L1 | Checklist | Steps defined, human verification |
| L2 | CI Gate | Automated execution, no evidence binding |
| L3 | Evidence Binding | Test results bound to commits/runs |
| L4 | Claim Governance | Claim/Decision/Observation registries exist |
| L5 | Continuous Audit | Governance itself is governed |

### 5.2 Current Maturity Assessment (v3.8.0)

| Level | Component | Status | Evidence |
|-------|-----------|--------|----------|
| L0 | Document-driven | N/A | N/A |
| L1 | Checklist gate | N/A | N/A |
| L2 | CI Gate | **supported** | CI config exists, execution unverified |
| L3 | Evidence Binding | **verified** | check_evidence_binding.sh runs, evidence_graph exists |
| L4 | Claim Governance | **partial** | Claim/Decision exist, Observation registry missing G-07 compliance |
| L5 | Continuous Audit | **none** | No meta-audit mechanism |

### 5.3 Upward Path

```
L3 → L4 requirements:
  □ O-Record format fully G-07 compliant (command + sha256)
  □ Decision Registry with DEC-001 example
  □ Claim Registry with 9+ entries (v3.7.0 review)
  □ G-06 Freshness enforcement in CI

L4 → L5 requirements:
  □ Monthly Reconciliation Matrix (auto-generated)
  □ Governance audit of governance (quarterly)
  □ FP/FN rate tracking in CI
  □ Meta-audit report published
```

---

## 6. Legacy Gate Retirement Protocol

### 6.1 Prerequisites for Retirement

| Condition | Threshold | Current Status |
|-----------|-----------|----------------|
| CI runs on target branch | ≥ 30 runs | 0 (UNVERIFIED) |
| Success rate | ≥ 95% | N/A |
| False positive rate | < 2% | N/A |
| False negative rate | < 2% | N/A |
| Reconciliation Matrix | Generated | NOT STARTED |
| Discrepancy rate | < 5% | N/A |
| Parallel run period | ≥ 20 runs | 0 |

### 6.2 Retirement Phases

```
Phase A: Parallel Run
  Graph Gate + Legacy Gate both active
  Collect ≥ 30 CI runs
  No decision to retire either

Phase B: Data Collection
  Generate Reconciliation Matrix
  Classify TP/FP/FN/TN
  Calculate drift rate

Phase C: Decision
  If discrepancy_rate < 5% AND success_rate >= 95%
    → Approve Legacy retirement
  Else
    → Keep Legacy, address root causes

Phase D: Advisory Mode
  Legacy Gate demoted to advisory
  New CI runs use Graph Gate only
  Legacy still callable for comparison

Phase E: Deletion
  After ≥ 3 months in Advisory Mode
  After no new FN detected
  → Delete scripts/gate/ legacy scripts
```

### 6.3 Current Position

```
Current Phase: Phase A (not started)
  - develop/v3.7.0: 0 CI runs collected
  - develop/v3.8.0: not yet branched for gate testing
  - Reconciliation Matrix: NOT STARTED
  - Legacy retirement: NOT PERMITTED until Phase B complete
```

---

## 7. G-Rules Reference

| Rule | Name | Statement |
|------|------|-----------|
| G-01 | Evidence Required for PASS | Any gate PASS must be traceable to Evidence Graph; else → UNVERIFIED |
| G-02 | Test Result Binding | Any test result must be bound: Test → Commit → CI Run → Artifact; else → UNVERIFIED |
| G-03 | No Evidence = Block | Cannot prove ≠ through; must become: cannot prove → UNVERIFIED → block |
| G-04 | Claim Provenance | All claims must document source, method, timestamp; self-asserted claims disallowed |
| G-05 | Decision Rationale | All decisions must include reason + evidence;不然 → rejected |
| G-06 | Evidence Freshness | Verified claims must have execution evidence within 30 days; else → stale |
| G-07 | Negative Evidence | No evidence of X ≠ evidence of not-X; both require independent proof |

---

## 8. Document Relationships

```
VERSION_PLAN.md
    ↓ (contains gate schedule)
GATE_EFFECTIVENESS_MATRIX.md
    ↓ (references maturity model)
GA_GATE_CHECKLIST.md
    ↓ (executes gates)
.gitea/workflows/ci.yml
    ↓ (produces artifacts)
Evidence Graph (PostgreSQL)
    ↓ (feeds)
Claim Registry ← GOVERNANCE_FRAMEWORK.md (this file)
    ↓ (informs)
Decision Registry
    ↓ (enables)
Reconciliation Matrix
    ↓ (validates)
Legacy Gate Retirement Protocol
```

---

## 9. Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-05-30 | Created v3.8.0 governance framework incorporating lessons from v3.7.0 gate audit | Hermes Agent |
| 2026-05-30 | Added G-06 (Evidence Freshness) and G-07 (Negative Evidence) rules |
| 2026-05-30 | Defined L2 CI Gate as `supported` not `verified` — execution unconfirmed |
| 2026-05-30 | Established Claim/Decision/Observation registries with v3.7.0 example data |
| 2026-05-30 | Defined Legacy Gate Retirement Protocol with 5-phase model |
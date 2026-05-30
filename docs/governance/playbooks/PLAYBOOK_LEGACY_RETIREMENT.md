# Playbook — Legacy CI Gate Retirement

> **Playbook ID**: PLAYBOOK_LEGACY_RETIREMENT
> **Version**: v3.8.0
> **Purpose**: Step-by-step guide for retiring legacy CI gates
> **Reference**: ADR-005, PATTERN_LEGACY_RETIREMENT, DEC-001

---

## Overview

```
Phase A: Parallel Run (≥30 runs)
Phase B: Reconciliation Matrix
Phase C: Decision DEC-XXX
Phase D: Advisory Mode
Phase E: Deletion
```

**Current Status**: Phase A — Not Started

**Decision DEC-001**: Rejected (prerequisites not met)

**Next Action**: Start Phase A parallel runs on develop/v3.7.0

---

## Phase A: Parallel Run

### Objective

Run new CI (`ci-v370.yml`) and legacy CI simultaneously on `develop/v3.7.0` for ≥30 consecutive runs.

### Entry Criteria

- `develop/v3.7.0` branch is active
- New CI workflow (`ci-v370.yml`) is created and verified
- Legacy CI workflow exists and is functional
- GitHub Actions runner is available

### Steps

#### A.1: Verify New CI Workflow

```bash
# 1. Verify ci-v370.yml exists
ls .github/workflows/ci-v370.yml

# 2. Verify it triggers on develop/v3.7.0
grep -A5 "branches:" .github/workflows/ci-v370.yml
# Expected: - develop/v3.7.0

# 3. Verify sha256 evidence logging
grep "sha256" .github/workflows/ci-v370.yml
# Expected: sha256 logged in each step

# 4. Dry-run: trigger manually
# Go to GitHub Actions → ci-v370.yml → Run workflow
```

#### A.2: Identify Legacy CI Workflow

```bash
# Find legacy workflow files
ls .github/workflows/ | grep -E "legacy|old|v2"

# Common candidates:
# - r-gate.yml (targets develop/v2.9.0)
# - ci.yml (targets older branches)
# - Any workflow without sha256 evidence logging
```

#### A.3: Enable Parallel Execution

```bash
# 1. Enable ci-v370.yml on push to develop/v3.7.0
# (Already configured in ci-v370.yml)

# 2. Enable legacy CI on same branch (if not already)
# Edit legacy workflow to also trigger on develop/v3.7.0

# 3. Verify parallel execution
# Check GitHub Actions → Runs → both workflows appearing
```

#### A.4: Monitor Run Count

```bash
# Track Phase A progress
# Expected: 30 consecutive parallel runs

# Manual tracking:
# GitHub Actions → ci-v370.yml → Run history
# Count runs where both new and legacy CI completed

# Automated tracking (recommended):
# Create issue comment bot to count runs:
# - Increment counter on each successful parallel run
# - Post update to tracking issue
```

#### A.5: Verify Parallel Execution

```bash
# For each run, verify BOTH:
# 1. ci-v370.yml: PASS/FAIL with sha256 evidence
# 2. Legacy CI: PASS/FAIL with output

# Check for systematic divergence
# - Same pass/fail result? → Good
# - Different results? → Log as discrepancy
```

#### A.6: Exit Phase A

**Exit Criteria**:
- ≥30 consecutive parallel runs completed
- Both CI systems report results
- No systematic divergence identified
- Discrepancy rate < 10%

**Exit Action**:
```bash
# 1. Document run count
echo "Phase A Complete: 30 parallel runs"
echo "Run dates: YYYY-MM-DD to YYYY-MM-DD"
echo "Discrepancy count: N/30"

# 2. Create Reconciliation Matrix
# Move to Phase B
```

---

## Phase B: Reconciliation Matrix

### Objective

Classify all discrepancies between new CI and legacy CI as TP/FP/FN/TN.

### Entry Criteria

- Phase A: ≥30 parallel runs completed
- Discrepancy count: N (some runs show different results)

### Steps

#### B.1: Collect Discrepancy Data

```bash
# 1. List all runs with different results
# For each discrepancy run:
#   - New CI result: PASS/FAIL
#   - Legacy CI result: PASS/FAIL
#   - Run URL
#   - Date

# Format:
# | Run | New CI | Legacy CI | Delta | Run URL |
# |-----|--------|-----------|-------|--------|
# | #12 | PASS   | FAIL      | -1    | link   |
```

#### B.2: Classify Each Discrepancy

For each discrepant run, determine:

```yaml
classification:
  TP:  # True Positive — New CI correctly detects failure
    description: "Legacy PASS, New CI FAIL — new CI caught real issue"
    
  FP:  # False Positive — New CI incorrectly fails
    description: "Legacy PASS, New CI FAIL — new CI has false alarm"
    
  FN:  # False Negative — Legacy CI misses failure
    description: "Legacy FAIL, New CI PASS — legacy CI caught issue, new CI missed"
    
  TN:  # True Negative — Both correctly pass
    description: "Legacy PASS, New CI PASS — no issue"
    # Note: TN runs are NOT discrepancies (same result)
```

#### B.3: Calculate Metrics

```bash
# Total runs: 30
# TP + FP + FN + TN = 30
# Discrepancy Rate = (FP + FN) / 30 * 100%

# Target: Discrepancy Rate < 5%
# If >= 5%: Investigate root cause before proceeding
```

#### B.4: Document Reconciliation Matrix

```bash
# Create docs/releases/v3.7.0/RECONCILIATION_MATRIX.md
#
# | Run | New CI | Legacy CI | Type | Root Cause | Fix |
# |-----|--------|-----------|------|------------|-----|
```

#### B.5: Exit Phase B

**Exit Criteria**:
- All discrepancies classified as TP/FP/FN/TN
- Discrepancy Rate < 5%
- Root cause identified for each FP/FN

**Exit Action**:
```bash
# 1. Verify discrepancy rate
echo "Discrepancy Rate: N/30 = X%"

# 2. Confirm all FPs/FNs have root cause
# 3. Move to Phase C
```

---

## Phase C: Decision

### Objective

Formal decision (via DEC-XXX) to approve or reject legacy gate retirement.

### Entry Criteria

- Phase B: Reconciliation Matrix complete
- Discrepancy Rate < 5%

### Steps

#### C.1: Draft Decision Document

```bash
# Create DEC-XXX in docs/governance/adr/
# Follow TEMPLATE_DECISION_REGISTRY.md format

decision_id: DEC-002
title: "Retire Legacy CI Gate"
status: proposed
proposed_at: <ISO timestamp>
proposed_by: <agent or human>
reason: |
  1. Phase A complete: 30 parallel runs
  2. Phase B complete: discrepancy_rate < 5%
  3. Legacy gate is a false positive factory
  4. Maintenance burden exceeds value
blocking_conditions:
  - "Phase A not started"
  - "Phase B not complete"
  - "discrepancy_rate >= 5%"
evidence_required:
  - "30 parallel runs documented"
  - "Reconciliation Matrix with TP/FP/FN/TN"
  - "discrepancy_rate < 5%"
```

#### C.2: Present to Governance Committee

```bash
# Submit DEC-XXX for review
# Required approvers: steering committee
# Review period: 1 week
```

#### C.3: Decision Outcome

```yaml
decision_options:
  approved:
    description: "Proceed to Phase D (Advisory Mode)"
    action: "Start Phase D"
    
  rejected:
    description: "Do not retire legacy gate"
    action: "Address blocking conditions, restart Phase A"
    
  deferred:
    description: "Cannot decide yet"
    action: "Document blocking conditions, retry later"
```

#### C.4: Exit Phase C

**Exit Criteria**:
- DEC-XXX status = approved OR rejected
- If rejected: blocking conditions documented

**Exit Action**:
```bash
# If approved:
#   1. Update DEC-XXX status to approved
#   2. Move to Phase D

# If rejected:
#   1. Document blocking conditions
#   2. Restart Phase A (after fixes)
```

---

## Phase D: Advisory Mode

### Objective

Legacy CI output is logged but NOT blocking. Team validates new CI coverage.

### Entry Criteria

- Phase C: DEC-XXX status = approved

### Steps

#### D.1: Configure Advisory Mode

```bash
# Update legacy CI workflow:
# - Remove "required" status check
# - Add comment: "Legacy CI (Advisory Mode — non-blocking)"
# - Continue logging but don't fail PRs
```

#### D.2: Monitor for 2 Weeks

```bash
# Track daily:
# - New CI pass rate
# - Legacy CI pass rate (if it runs)
# - Any issues caught ONLY by legacy CI (potential FN)

# Alert if:
# - New CI pass rate drops > 10%
# - Legacy CI catches issue new CI misses
```

#### D.3: Exit Phase D

**Exit Criteria**:
- 2-week advisory period complete
- No new false negatives (legacy catches, new misses)
- Team confident in new CI coverage

**Exit Action**:
```bash
# 1. Verify no new FNs in advisory period
# 2. Confirm team confidence
# 3. Move to Phase E
```

---

## Phase E: Deletion

### Objective

Remove legacy CI configuration from repository.

### Entry Criteria

- Phase D: Advisory period complete
- No new FNs detected
- Team confident

### Steps

#### E.1: Archive Legacy Workflow

```bash
# 1. Move legacy workflow to docs/governance/_archive/
mv .github/workflows/legacy-gate.yml docs/governance/_archive/

# 2. Commit with reason
git add docs/governance/_archive/legacy-gate.yml
git commit -m "chore: archive legacy CI gate (DEC-002 approved)"
```

#### E.2: Update Documentation

```bash
# 1. Update GATE_SPEC.md — remove legacy gate references
# 2. Update GOVERNANCE_FRAMEWORK.md — mark DEC-002 implemented
# 3. Update this playbook — mark Phase E complete
```

#### E.3: Verify Cleanup

```bash
# 1. Confirm no references to legacy gate in active workflows
grep -r "legacy" .github/workflows/ --include="*.yml" | grep -v "_archive"

# 2. Confirm new CI still functions
# GitHub Actions → ci-v370.yml → Run workflow → PASS
```

#### E.4: Notify Stakeholders

```bash
# 1. Post to team: "Legacy CI gate retired"
# 2. Update tracking issue: Phase E complete
# 3. Close tracking issue
```

#### E.5: Exit Phase E

**Exit Criteria**:
- Legacy workflow archived
- All documentation updated
- Stakeholders notified

**Final Status**: LEGACY GATE RETIRED ✅

---

## Tracking Issue Template

```yaml
---
title: "Legacy CI Gate Retirement — Phase Tracking"
labels: ["governance", "ci", "phase-a"]
---

## Status

**Current Phase**: Phase A (not started)

## Progress

| Phase | Status | Run Count | Discrepancy Rate | Notes |
|-------|--------|-----------|------------------|-------|
| A | ⏳ | 0/30 | N/A | |
| B | ⏸️ | — | N/A | Wait for Phase A |
| C | ⏸️ | — | N/A | Wait for Phase B |
| D | ⏸️ | — | N/A | Wait for Phase C |
| E | ⏸️ | — | N/A | Wait for Phase D |

## Next Action

Start Phase A: Push to develop/v3.7.0 with ci-v370.yml

## Decision Reference

DEC-001: rejected (prerequisites not met)
DEC-002: proposed (pending Phase A+B completion)
```

---

## Quick Reference Card

| Phase | Action | Exit Criteria | Duration |
|-------|--------|--------------|----------|
| A | Parallel CI runs | ≥30 runs, both CI working | 1-4 weeks |
| B | Reconciliation Matrix | All discrepancies classified, <5% rate | 1 week |
| C | Decision DEC-XXX | Approved/Rejected | 1 week |
| D | Advisory Mode | 2 weeks, no new FNs | 2 weeks |
| E | Deletion | Legacy archived | 1 day |

**Total**: 5-8 weeks from start to deletion

---

## Emergency Rollback

If at any phase a critical issue is discovered:

```bash
# 1. Pause all phases
# 2. Document issue in tracking issue
# 3. Escalate to steering committee
# 4. If legacy CI needed as backup:
#    - Re-enable legacy CI as blocking
#    - Restart from Phase A
```

---

## References

- ADR-005: Legacy Gate Retirement
- PATTERN_LEGACY_RETIREMENT
- TEMPLATE_DECISION_REGISTRY.md
- GATE_SPEC.md (Phase A~E protocol)
- DEC-001 (rejected: prerequisites not met)
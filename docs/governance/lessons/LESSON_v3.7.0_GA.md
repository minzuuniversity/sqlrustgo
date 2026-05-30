# Lessons Learned — v3.7.0 GA Review

> **Version**: v3.7.0
> **Date**: 2026-05-30
> **Source**: v3.7.0 GA gate review sessions
> **Purpose**: Capture root causes to prevent future failures

---

## Lesson-01: Configured ≠ Executed ≠ Verified

### Observation

Legacy CI gate reports PASS status in CI pipeline, but no test execution logs exist. Workflow file is present (configured) but was never triggered (not executed). Even if executed, no sha256 evidence exists (not verified).

### Root Cause

The team assumed "workflow file exists = tests are running". No verification step was defined to confirm actual execution. The CI pipeline UI showed a green checkmark based on workflow configuration, not actual execution.

### Impact

- False positive: gate passed without any tests running
- Quality regression went undetected
- 3 weeks of development had no test evidence
- Trust in CI system damaged

### Mitigation

- DEC-2582-01: 5-phase Legacy Gate Retirement Protocol
- Phase A: Parallel run (new + legacy CI)
- Require sha256 evidence for every gate PASS
- G-01/G-02/G-03 enforcement: no evidence = block

### Future Prevention

1. Every CI gate must produce: test log + sha256 + timestamp
2. Gate report must include execution count
3. Green status requires: executed + verified, not just configured
4. Add "Test Log Existence" check to L2 sub-checks

---

## Lesson-02: Coverage Number ≠ Coverage Truth

### Observation

Coverage percentage on dashboard was 64%, below the 65% threshold. Instead of improving tests, the proposal was to lower the threshold to 60%. The threshold had been lowered in each of the previous 3 versions (70% → 68% → 65% → proposed 60%).

### Root Cause

Coverage threshold was treated as negotiable rather than fixed. The threshold was not locked in GATE_SPEC.md before alpha started. When coverage dropped, the response was to adjust the target rather than improve quality.

### Impact

- Each version: threshold lowered, actual quality degraded
- Cumulative effect: 10 percentage points lost (70% → 60%)
- No incentive to improve test quality
- Anti-pattern established: coverage threshold as sliding regulator

### Mitigation

- DEC-2580-01: Pattern PATTERN_COVERAGE_DISPUTE established
- GATE_SPEC.md v3.8.0: threshold locked before alpha
- Threshold change requires DEC-XXX with full governance

### Future Prevention

1. Lock coverage threshold in GATE_SPEC.md at version start
2. Threshold is FROZEN after alpha starts
3. Any change requires steering committee approval
4. Monitor coverage trend, alert at first drop
5. Address root cause (test quality) not symptom (threshold)

---

## Lesson-03: Issue Closed ≠ Problem Solved

### Observation

Multiple issues were closed after discussion but the underlying problems recurred. For example, DEBT_TRACKING.md existed and issues were discussed, but architecture debt accumulated without resolution. sql-cli API breakage (Issue #1497) was "addressed" but never actually fixed.

### Root Cause

Issue closure was used as a resolution mechanism without verifying actual fix. "Closed" status was applied to issues that were deferred, workarounds, or simply acknowledged but not resolved. No verification step confirmed that the fix actually worked.

### Impact

- Same problems reappear in later versions
- sql-cli API broken since v2.6.0 (18 months)
- DEBT_TRACKING.md lists 12+ items, none resolved
- Trust in issue tracking system damaged

### Mitigation

- Require evidence of actual fix before closing
- Add "Verification" step to issue lifecycle
- Gate must verify issue resolution, not just issue existence
- Pattern: "Verified Fix" before "Issue Closed"

### Future Prevention

1. Issue cannot be closed without evidence
2. Evidence must show: before state → after state
3. Gate reviewer must verify issue resolution
4. Re-open if problem recurs within 2 releases

---

## Lesson-04: Document Claim ≠ Verified Claim

### Observation

GOVERNANCE_FRAMEWORK.md contained CLAIM-001 through CLAIM-004 as "verified" claims, but closer inspection showed they had no OBS (Observation) references. The claims were stated, not proven. G-04 (Claim Provenance) requires every claim to reference at least one OBS with command + sha256 + timestamp.

### Root Cause

The team confused "documented" with "verified". Writing something in a document does not make it true. The claims were written with good intentions but without the evidence chain required by governance rules.

### Impact

- CLAIM-001~004 were actually "asserted", not "verified"
- G-04 (Claim Provenance) violated
- Decision-making based on unproven claims
- GOVERNANCE_FRAMEWORK.md credibility damaged

### Mitigation

- DEC-2613-01: Add OBS provenance to all claims
- commit a918bf6d: Added timestamp to CLAIM-001~004
- Created OBS-001~004 with command + sha256 + timestamp
- G-04 enforcement: no OBS = claim cannot be "verified"

### Future Prevention

1. Every CLAIM must have ≥1 OBS reference
2. OBS must include: command + sha256 + timestamp
3. Only "verified" claims can be used in decisions
4. Add CLAIM-OBS mapping check to gate review
5. Template: TEMPLATE_CLAIM_REGISTRY.md enforces format

---

## Lesson-05: Architecture Debt Can Pass Gate

### Observation

DEBT_TRACKING.md listed multiple architecture debt items:
- sql-cli API breakage (since v2.6.0, 18 months)
- WAL change causing executor test failures
- storage engine abstraction not finalized

Yet the gate passed. DEBT_TRACKING.md existed but was not enforced in the gate checklist.

### Root Cause

Architecture debt was tracked but not treated as a gate requirement. The gate checklist had no debt check. Debt items were acknowledged but not enforced. No L6 (Architecture Debt Gate) sub-check existed.

### Impact

- Debt accumulated without blocking releases
- 18 months of sql-cli API breakage
- Executor test failures from WAL changes
- Technical insolvency eventually blocks all progress

### Mitigation

- DEC-2615-01: Add L6 (Architecture Debt Gate)
- L6-1~L6-5 sub-checks added to GATE_SPEC.md v3.8.0
- Max debt items: 10 per version
- Max debt age: 6 months
- Each release must resolve ≥1 debt item

### Future Prevention

1. Debt check in every gate (L6 mandatory)
2. Max 10 debt items per version
3. Debt > 6 months → escalates to steering committee
4. Each release: resolve ≥1 debt item
5. Gate fails if debt threshold exceeded

---

## Lesson-06: Supported Evidence ≠ Verified Evidence

### Observation

During v3.7.0 gate review, claims were marked as "supported" when evidence showed they were only "configured" (not executed, not verified). The L2 CI Gate reported results as "supported" because the workflow file existed, but this was incorrectly treated as equivalent to "verified" (executed + sha256 evidence).

### Root Cause

The team used "supported" to mean "we believe this is true" rather than "this has been verified by evidence". The distinction between configured (file exists) and verified (evidence exists) was not clearly understood.

### Impact

- False sense of security: "supported" claims were accepted
- Legacy gate false positives accumulated
- Evidence chain was incomplete
- G-04 (Claim Provenance) and G-02 (Test Result Binding) violated

### Mitigation

- GATE_SPEC.md v3.8.0: clearly defines four states
- Configured ≠ Executed ≠ Verified (core principle)
- L2 CI Gate corrected: supported ≠ verified
- Only verified claims count for evidence binding

### Future Prevention

1. Define "verified" precisely: executed + sha256 evidence + timestamp
2. "Supported" means: evidence exists but not fully verified
3. "Configured" means: file exists but not executed
4. Gate PASS requires: verified, not just supported
5. Add state transition check to L2 sub-checks

---

## Cross-Cutting Lessons

### Governance System Gaps

| Gap | Lesson | Fix |
|-----|--------|-----|
| No OBS template | Lesson-04 | TEMPLATE_CLAIM_REGISTRY.md |
| No evidence verification | Lesson-01, 06 | G-01~G-07 enforcement |
| No debt gate | Lesson-05 | L6 (Architecture Debt Gate) |
| Claim without OBS | Lesson-04 | G-04 (Claim Provenance) |
| Threshold negotiable | Lesson-02 | Lock threshold before alpha |
| Issue closed ≠ fixed | Lesson-03 | Verification step |

### Process Improvements

1. Gate review must include evidence verification step
2. Claims require OBS provenance before verification
3. Architecture debt must be in gate checklist
4. Threshold changes require DEC-XXX process
5. Issue closure requires fix verification

---

## References

- Pattern: PATTERN_COVERAGE_DISPUTE (Lesson-02)
- Pattern: PATTERN_GATE_FALSE_POSITIVE (Lesson-01)
- Pattern: PATTERN_EVIDENCE_CHAIN (Lesson-04)
- Pattern: PATTERN_ARCHITECTURE_DEBT (Lesson-05)
- ADR-001: Truthfulness Framework
- ADR-002: Claim Registry
- ADR-005: Legacy Gate Retirement
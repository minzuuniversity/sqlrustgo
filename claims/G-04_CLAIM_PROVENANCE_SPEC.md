# Claim Provenance Schema — v1.0

> **Version**: 1.0
> **Date**: 2026-05-30
> **Supersedes**: N/A (new rule)
> **Gate**: G-04, G-04.1, G-04.2, G-04.3, G-05

---

## 1. Overview

G-04 introduces **Claim Provenance** to the Evidence Graph governance system.

The existing Evidence Graph handles:
```
Task → Commit → CI → Artifact
```
(G-01/G-02/G-03 ensure test results and artifacts are properly bound)

G-04 extends the graph to handle:
```
Claim → Finding → Evidence
       ↓
    Finding → Finding Source (File/Function/Line)
       ↓
    Evidence → Evidence Source (Commit/Test/Runtime Trace)
```

**Core principle**: Any risk statement, GA conclusion, or deferred decision in documentation MUST trace to an Evidence Graph node. If it cannot, the claim is **UNVERIFIED**.

---

## 2. G-04 Rule Set

### G-04: Claim Provenance Rule

```
Any Claim (risk statement, GA conclusion, deferral decision)
published in documentation, PR description, or gate report
MUST have a corresponding Claim Record in the Evidence Graph.

Claim → (via PROVES edge) → Finding → (via LOCATED_AT edge) → Source
Finding → (via VERIFIED_BY edge) → Evidence

UNVERIFIED if:
  - No Claim Record exists
  - Claim Record has no Finding
  - Finding has no Evidence
  - Evidence timestamp < Last Relevant Commit (STALE)
```

### G-04.1: Evidence Source Restriction

```
Issue, PR, Wiki, Document are NOT Evidence.
They are Claim Sources.

Evidence is restricted to:
  - Source Code (file/line reference)
  - Commit SHA
  - CI Run ID
  - Test Output (structured, machine-readable)
  - Coverage Report (artifact)
  - Runtime Trace (stack dump, profiler output)
  - Formal Proof File (.tla, .pkl)

Claim Sources cannot serve as the sole basis for a VERIFIED claim.
```

### G-04.2: No Orphan Claim

```
Any Claim Record MUST contain at least one Finding.
An orphan Claim (no findings) is a governance FAIL.
```

### G-04.3: Claim Freshness

```
Claim Evidence Timestamp < Last Relevant Commit
→ STALE CLAIM

Stale claims must be re-evaluated against current HEAD
before they can be used in gate decisions.
```

### G-05: Claim Audit Rule

```
Any architectural risk, GA conclusion, or deferral decision
MUST correspond to a Claim Record.

Rationale: Without a Claim Record, AI can write:
  - "Risk is acceptable" (no evidence)
  - "Deferred to v3.8.0" (no tracking)
  - "Verified" (no proof)

Claim Registry is the single source of truth for these decisions.
```

---

## 3. Claim Record Schema

### 3.1 YAML Format

```yaml
# ============================================================
# REQUIRED FIELDS
# ============================================================

claim_id: <string>
  # Unique identifier. Format: <CATEGORY>-<NUMBER>
  # Examples: INT-1, R2, GA-2026-05-30, DECISION-001
  # REQUIRED

title: <string>
  # One-line description of the claim
  # REQUIRED

type: <enum>
  # architectural_risk | release_conclusion | deferral_decision
  # REQUIRED

status: <enum>
  # asserted | supported | verified | disproven | mitigated
  # REQUIRED
  # See Section 4 for definitions

# ============================================================
# FINDINGS (REQUIRED — G-04.2)
# ============================================================

findings:
  - finding_id: <string>
    description: <string>
    location:
      file: <string>        # e.g., crates/executor/src/lib.rs
      function: <string>    # e.g., execute_insert
      line_range: <string>  # e.g., "120-145"
    observation: <string>   # What the code actually does
    finding_type: <enum>
      # code_pattern | test_gap | performance_issue | api_contract_violation
```

### 3.2 Evidence Binding (REQUIRED per Finding)

```yaml
findings:
  - finding_id: FIND-INT1-001
    evidence:
      # G-04.1: Must be one of these types, NOT an Issue/PR/Wiki
      commit_sha: <40-char hex>
        # REQUIRED if status is supported/verified
      ci_run_id: <string>
        # Optional — provides authoritative timestamp
      test_refs:
        - name: <string>
          # e.g., test_wal_gap_no_recovery
          path: <string>
          # e.g., crates/executor/tests/wal_gap.rs
          result: <enum> # pass | fail | not_found
      coverage_artifact:
        path: <string>
        metric: <string>
        value: <number>
      runtime_trace:
        # Only accepted as supplementary evidence
        type: <enum> # stack_trace | profiler_output | crash_log
        location: <string>
    # Claim source refs (G-04.1: NOT evidence)
    claim_source_refs:
      - type: issue
        id: <number>
        url: <string>
        note: "Claim source only — not evidence"
```

### 3.3 Release Contract (for GA conclusions)

```yaml
release_contract:
  version: <string>
  required_conditions:
    - condition_id: <string>
      description: <string>
      gate_result: <enum> # pass | fail | not_applicable
      evidence_refs:
        - commit_sha: <string>
          test_name: <string>
          ci_run_id: <string>
  optional_conditions:
    - condition_id: <string>
      description: <string>
      gate_result: <enum>
      note: <string>
  ga_decision: <enum> # pass | fail
  decision_rationale: <string>
```

### 3.4 Deferral Decision

```yaml
deferral:
  deferred_to: <version>
  rationale: <string>
  blockers:
    - <string>
  evidence_required_for_close:
    - commit_sha: <string>
      test_name: <string>
  last_reviewed_at: <ISO8601>
  reviewed_by: <string>
```

---

## 4. Claim Status Definitions

| Status | Definition | Evidence Required |
|--------|-----------|------------------|
| **asserted** | Claim made, no evidence gathered | 0 evidence items |
| **supported** | Some evidence exists, but cannot independently prove the claim | 1+ claim_sources, OR partial commit evidence |
| **verified** | Complete evidence chain, reproducible | Finding → Commit → CI artifact |
| **disproven** | Contradicted by evidence | Contradicting commit/test |
| **mitigated** | Problem existed, now resolved | Fix commit + regression test |

**State transition rules**:
```
asserted → supported (when 1+ evidence item added)
supported → verified (when evidence chain is complete)
supported → disproven (when contradicting evidence found)
any → mitigated (when fix commit + test is present)
```

**G-04.2 Violation**: `findings: []` with status ≠ "asserted" → FAIL

---

## 5. Evidence Type Classification

### 5.1 Acceptable Evidence (for G-04.1)

| Type | Machine-Readable | Immutable | Authority |
|------|----------------|-----------|-----------|
| Commit SHA | ✅ | ✅ | Git |
| CI Run ID + Status | ✅ | ✅ | CI System |
| Test Output (JSON/XML) | ✅ | ✅ | Test Runner |
| Coverage Report (JSON) | ✅ | ✅ | Coverage Tool |
| TLA+ Proof (.tla) | ✅ | ✅ | Model Checker |
| Rust Compiler Error | ✅ | ✅ | rustc |
| Stack Trace | ✅ | ⚠️ | Runtime |
| Coverage Artifact | ✅ | ✅ | Coverage Tool |

### 5.2 NOT Acceptable as Sole Evidence

| Type | Reason |
|------|--------|
| GitHub/Gitea Issue | Human-written, no verifiable authority |
| PR Description | Human-written, mutable |
| Wiki/Markdown | Human-written, mutable |
| Meeting Notes | Human-written, no authoritative source |
| Slack/Message | Informal, no authoritative source |
| Verbal Statement | No trace |

**Exception**: Claim sources MAY supplement evidence, but cannot replace it.

---

## 6. Claim Registry Structure

```
claims/
├── INT/                    # Architectural risks (INT-1, INT-2, ...)
│   ├── INT-1.yaml          # DML bypasses WAL
│   ├── INT-2.yaml          # ParallelVolcanoExecutor 孤岛
│   ├── INT-3.yaml          # expr crate 孤岛
│   └── INT-4.yaml          # mysql-server 双路径
├── R/                      # Release gates (R1-R10)
│   ├── R2.yaml             # 执行引擎统一
│   └── R5.yaml             # Coverage gate
├── GA/                     # GA conclusions
│   ├── GA-2026-05-30-v3.7.0.yaml
│   └── GA-2026-05-30-v3.8.0.yaml
├── DECISION/               # Deferral decisions
│   ├── DECISION-INT1-DEFER.yaml
│   └── DECISION-WAL-v3.8.0.yaml
└── INDEX.yaml              # Registry index
```

---

## 7. gate CLI Integration

### 7.1 New Commands

```bash
# Evaluate a claim against current HEAD
gate claim evaluate INT-1 --db /tmp/eg.db

# Output:
# claim_id: INT-1
# status: supported
# findings: 1
# evidence_chain_complete: false
# missing: [commit_sha, test_ref]
# staleness: stale (2026-05-20 < 2026-05-30)

# Check all claims
gate claim audit --db /tmp/eg.db

# Output:
# claim_id         | status       | findings | evidence | stale
# INT-1            | supported    | 1        | false    | true
# INT-2            | asserted     | 0        | false    | false
# ...
# ORPHAN CLAIMS: 3  ← G-04.2 violation

# Import claim from YAML
gate claim ingest claims/INT-1.yaml --db /tmp/eg.db
```

### 7.2 Claim Evaluation Logic

```
evaluate_claim(claim_id):
  claim = get_claim(claim_id)

  if claim.findings is empty:
    return UNVERIFIED, "ORPHAN CLAIM (G-04.2 violation)"

  for finding in claim.findings:
    if finding.evidence.commit_sha is empty:
      return UNVERIFIED, "Evidence commit missing"
    if not commit_exists(finding.evidence.commit_sha):
      return UNVERIFIED, "Evidence commit not in graph"
    if claim.staleness == STALE:
      return UNVERIFIED, "STALE CLAIM (G-04.3 violation)"

  if evidence_chain_complete(claim):
    return PASS, "VERIFIED"
  else:
    return UNVERIFIED, "Incomplete evidence chain"
```

---

## 8. Release Contract Example (v3.7.0)

```yaml
release_contract:
  version: v3.7.0
  contract_type: GA_ASSESSMENT
  assessment_date: 2026-05-30

  required_conditions:
    - condition_id: RC-1
      description: Parser unit tests pass
      gate_result: pass
      evidence_refs:
        - commit_sha: faa5b715
          test_name: cargo test -p sqlrustgo-parser --lib

    - condition_id: RC-2
      description: Auth gate enforced (SKIP_AUTH fixed)
      gate_result: pass
      evidence_refs:
        - commit_sha: 2607d788
          test_name: cargo test -p sqlrustgo-mysql-server --lib

    - condition_id: RC-3
      description: Session transaction persistence
      gate_result: pass
      evidence_refs:
        - commit_sha: 01db4fdf
          test_name: cargo test -p sqlrustgo-session --lib

    - condition_id: RC-4
      description: Build succeeds
      gate_result: pass
      evidence_refs:
        - ci_run_id: run_212
          status: success

  optional_conditions:
    - condition_id: OC-1
      description: WAL / Crash Recovery (INT-1)
      gate_result: fail
      note: "INT-1 deferred to v3.8.0 PR-830"
      deferral_ref: DECISION-INT1-DEFER

    - condition_id: OC-2
      description: Full MVCC / ACID
      gate_result: fail
      note: "Not in v3.7.0 scope — v3.8.0 target"

  ga_decision: conditional_pass
  decision_rationale: |
    v3.7.0 is a "Session-level transaction SQL engine".
    WAL/ACID completeness is an optional condition,
    explicitly deferred to v3.8.0.
    Required conditions all PASS.
    GA = PASS with documented exclusions.

  claim_ref: GA-2026-05-30-v3.7.0
```

---

## 9. Change Log

| Date | Version | Change |
|------|---------|--------|
| 2026-05-30 | 1.0 | Initial G-04 Claim Provenance schema |
# v3.12 Alpha Gate Governance Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split v3.12 Alpha checks into entry, quality, and deferred-approval layers so PASS claims cannot hide SQLLogicTest, P12, P16, or unapproved deferred work.

**Architecture:** Keep the existing Alpha script path as the user-facing command, but make it call an explicit entry script plus a quality script. Add focused shell tests that validate the new scripts fail when mandatory hard gates or deferred follow-up bindings are missing.

**Tech Stack:** Bash gate scripts, Markdown release docs, repository-local shell tests.

---

### Task 1: Add Shell Regression Tests

**Files:**
- Create: `tests/gate/v312_alpha_gate_governance_test.sh`

**Steps:**
1. Write tests that require `check_alpha_entry_v3.12.0.sh`, `check_alpha_quality_v3.12.0.sh`, and `check_v312_deferred_followups.sh` to exist and be executable.
2. Assert the deferred follow-up script reports Gitea issue references and rejects OpenSpec-only references.
3. Assert the Alpha quality script runs hard governance checks and fails on the current known gate failures.

### Task 2: Implement Entry/Quality/Deferred Scripts

**Files:**
- Create: `scripts/gate/check_alpha_entry_v3.12.0.sh`
- Create: `scripts/gate/check_alpha_quality_v3.12.0.sh`
- Create: `scripts/gate/check_v312_deferred_followups.sh`
- Modify: `scripts/gate/check_alpha_v3.12.0.sh`

**Steps:**
1. Move current existence/document checks into the entry script.
2. Add quality checks for SQLLogicTest, P12 ignore count, P16 gate integrity, anti-ignore, and deferred follow-up approval.
3. Make the existing Alpha script call both layers and report ENTRY/QUALITY separately.

### Task 3: Update Documentation

**Files:**
- Modify: `docs/releases/v3.12.0/DRAFT_ASSESSMENT_AND_ALPHA_GATE.md`
- Modify: `docs/releases/v3.12.0/STAGE.yaml`
- Modify: `docs/releases/v3.12.0/TEST_PLAN.md`
- Create or update: `docs/releases/v3.12.0/evidence/ALPHA_GATE_COVERAGE_AUDIT.md`

**Steps:**
1. Explain that Alpha Entry can pass while Alpha Quality remains blocked.
2. State that registered exclusions are visible debt, not PASS evidence.
3. Record the specific hard gates required before Alpha Quality PASS.

### Task 4: Verify

**Commands:**
- `bash tests/gate/v312_alpha_gate_governance_test.sh`
- `bash scripts/gate/check_alpha_entry_v3.12.0.sh`
- `bash scripts/gate/check_alpha_quality_v3.12.0.sh`
- `bash scripts/gate/check_alpha_v3.12.0.sh`
- `bash scripts/gate/check_docs_links.sh`

**Expected:** Entry may pass outside the exact branch only if branch check is explicitly scoped; quality should fail on current known P12/P16/SQLLogicTest blockers until those blockers are fixed.

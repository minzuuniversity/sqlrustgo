# v3.12.0 GA Candidate Documentation Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prepare v3.12.0 for GA promotion by refreshing release documentation without claiming GA before execution evidence closes.

**Architecture:** This is a documentation-only release-governance update. The plan adds GA candidate rollup documents, links them from release entry points, and keeps `STAGE.yaml` at `RC` until the GA cut has fresh full-mode gate evidence.

**Tech Stack:** Markdown release docs, Gitea issue metadata, existing `scripts/gate/*` document checks.

---

### Task 1: Capture Current Evidence Boundary

**Files:**
- Create: `docs/releases/v3.12.0/GA_RELEASE_REPORT.md`
- Modify: `docs/releases/v3.12.0/GA_GATE_REPORT.md`

**Step 1:** Record `origin/develop/v3.12.0` HEAD, milestone status, open issue caveats, and GA blocker status.

**Step 2:** Mark GA-2 Linux/Docker SOAK re-validation as the remaining evidence blocker unless an explicit governance reclassification is approved.

**Step 3:** Note that existing `ga_gate_report.json` is fast-path evidence and is not the final GA cut artifact.

### Task 2: Add Rollup Reports

**Files:**
- Create: `docs/releases/v3.12.0/PERFORMANCE_REPORT.md`
- Create: `docs/releases/v3.12.0/SECURITY_AUDIT.md`
- Create: `docs/releases/v3.12.0/RELEASE_CHECKLIST.md`

**Step 1:** Summarize SOAK, TPC-H, SQLLogicTest, and wire/recovery/upgrade performance evidence with caveats.

**Step 2:** Summarize security evidence and identify stale or baseline-carried checks that need GA-cut refresh.

**Step 3:** Map each `promotion_to_GA_requires` item to evidence files, issue state, and final cut action.

### Task 3: Refresh Release Entry Points

**Files:**
- Modify: `docs/releases/v3.12.0/README.md`
- Modify: `docs/releases/v3.12.0/RELEASE_NOTES.md`
- Modify: `docs/releases/v3.12.0/CHANGELOG.md`

**Step 1:** Add a 2026-09-02 GA candidate update section.

**Step 2:** Keep the public status as `RC / GA candidate preparation`.

**Step 3:** Link the new reports and avoid unverified GA PASS claims.

### Task 4: Verify Documentation

**Files:**
- All modified Markdown under `docs/releases/v3.12.0/`

**Step 1:** Run `bash scripts/gate/check_docs_links.sh`.

**Step 2:** Run v3.12.0 docs consistency checks if available.

**Step 3:** Inspect `git diff --check` and final diff before reporting.

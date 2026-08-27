# Tasks: --fast-path flag for check_ga_v3.12.0.sh (Issue #4536)

> **Schema**: spec-driven
> **Source agent**: claude-sonnet (Claude Code)
> **Source run**: issue-4536-fast-path-ga-gate-20260827

## 1. Spec capture

- [ ] 1.1 Add `specs/ga-aggregator/spec.md` capturing the
      `--fast-path` / `--full` mode matrix and the rule
      "default = --full (backward-compat); local-dev opts in
      via --fast-path".

## 2. Script implementation

- [ ] 2.1 Edit `scripts/gate/check_ga_v3.12.0.sh`:
  - add CLI flag parser (--fast-path / --full / --help)
  - branch `run_beta_gate()` on `$MODE` (heavy vs fast-path)
  - add heavy-mode startup banner
  - add `mode` field to JSON report
- [ ] 2.2 Update header doc comment (lines 13-16) to make the
      two modes explicit.
- [ ] 2.3 Verify `bash -n scripts/gate/check_ga_v3.12.0.sh`
      (syntax check).

## 3. Verification

- [ ] 3.1 `bash scripts/gate/check_ga_v3.12.0.sh --help` exits 0
      with usage text.
- [ ] 3.2 `time bash scripts/gate/check_ga_v3.12.0.sh --fast-path`
      completes in **<30s** on dev laptop (vs >3min without the
      flag). Capture wall time to
      `evidence/v312-59/issue-4536-fast-path-bench.txt`.
- [ ] 3.3 `cat docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json`
      shows `"mode": "fast-path"` field. Capture to
      `evidence/v312-59/issue-4536-fast-path-report.json`.
- [ ] 3.4 Default (no flag) behavior unchanged: heavy BETA
      invocation still runs (smoke check that `BETA_PASS=40` is
      achievable — accept skipping if it would exceed session
      budget; instead record `step: SKIPPED — reason: heavy
      BETA exceeds session budget` per ADR-001).

## 4. CI integration

- [ ] 4.1 Confirm `.gitea/workflows/*.yml` aggregator calls do
      **not** pass `--fast-path` (CI must continue to run heavy
      BETA). Document the contract in
      `docs/governance/CI_GATE_CONTRACT.md` (cross-link).

## 5. Anti-fabrication check

- [ ] 5.1 The benchmark in §3.2 records actual wall-clock time
      (`time` output), not estimated.
- [ ] 5.2 The JSON dump in §3.3 is real `ga_gate_report.json`
      content, not synthesized.
- [ ] 5.3 The CI contract in §4.1 cites real
      `.gitea/workflows/*.yml` lines, not a description.

## 6. Issue closure

- [ ] 6.1 Post evidence + diff summary to issue #4536 via Gitea
      API; mark `[closes #4536]` once verified.
- [ ] 6.2 Cross-link in `docs/releases/v3.12.0/STAGE.yaml`
      `promotion_to_GA_requires` progress tracker.
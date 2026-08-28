# Tasks: TPC-H Q17 SF=1 cell-diff (Issue #4540)

> **Schema**: spec-driven
> **Source agent**: claude-sonnet (Claude Code)
> **Source run**: issue-4540-tpch-q17-sf1-cell-diff-20260827

## 1. Spec capture

- [ ] 1.1 Add `specs/tpch-sf1-baseline/spec.md` capturing the
      acceptance boundary for Q17 SF=1 (elapsed ≤ 300s,
      row_count == 1, sha256 == 595003bfd…), and the
      relationship to issues #4432 / #4502 / #4497.
- [ ] 1.2 Add `specs/ga-5-tpch-sf1/spec.md` linking the above
      to the GA-5 promotion gate.

## 2. Runbook + CI integration

- [ ] 2.1 Verify `scripts/generate_tpch_data.sh --sf 1` works on
      the current `develop/v3.12.0` HEAD (`14f638d09`).
- [ ] 2.2 Verify `scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1`
      is executable against the generated fixture.
- [ ] 2.3 Add a CI workflow stub
      `.gitea/workflows/z6g4-tpch-sf1-cell-diff.yml` (DRAFT, not
      enabled) that mirrors the §3 Z6G4 runbook as a YAML
      sequence. Mark as `workflow_dispatch:` only.

## 3. Dev-machine verification (best-effort)

- [ ] 3.1 Run `bash scripts/generate_tpch_data.sh --sf 1
      --output /tmp/tpch-sf1 --backend tpch_data_gen` on dev
      laptop. Capture row-count verification log.
- [ ] 3.2 Run `bash scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1`.
      Capture Q17 elapsed, row_count, sha256.
- [ ] 3.3 Write `evidence/v312-58/issue-4540-sf1-cell-diff.md`
      with the captured values and the required 10 ADR-001
      fields.

## 4. Issue closure

- [ ] 4.1 If Q17 elapsed ≤ 300s: post evidence to issue #4432
      via Gitea API, link #4540, request closure.
- [ ] 4.2 If Q17 elapsed > 300s: post evidence with verdict
      DEFERRED-to-v3.13, update
      `docs/releases/v3.13.0/SCOPE_TABLE_v3.13.md`.
- [ ] 4.3 Either way: update `docs/releases/v3.12.0/STAGE.yaml`
      `promotion_to_GA_requires` progress tracker.

## 5. CI/Z6G4 handoff (if dev-machine fails or times out)

- [ ] 5.1 Create a `handoff/CI-Z6G4-RUNBOOK.md` (this change)
      describing the exact SSH + git-sync + fixture-gen +
      baseline-run sequence for a human operator or follow-up
      automated agent.
- [ ] 5.2 Post the handoff to issue #4542 follow-up (if created)
      or to issue #4540 itself with `verdict: blocked-on-z6g4`.

## 6. Anti-fabrication check

- [ ] 6.1 Every claim in the evidence file references a real
      command + real output (no estimated / simulated numbers).
- [ ] 6.2 The `evidence_hash` in the report matches
      `git rev-parse HEAD` at the time of capture.
- [ ] 6.3 If any step is skipped (e.g. 168h SOAK not runnable
      from dev), the report explicitly states
      `step: SKIPPED — reason: <one-line>` rather than
      silently omitting it.
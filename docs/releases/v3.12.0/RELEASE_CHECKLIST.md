# SQLRustGo v3.12.0 GA Release Checklist

> **provenance:** generated_by=codex-cli, generated_at=2026-09-02T21:20:00+08:00, source_repo=openclaw/sqlrustgo, remote_head=`b14ad8df0306891e217af19648b210e81d5d2be0`, policy=Anti-Fabrication-Policy-v1.0
> **stage:** RC / GA candidate preparation

This checklist maps `STAGE.yaml` `promotion_to_GA_requires` to the evidence
needed before changing `current_stage` from `RC` to `GA`.

## Hard Gates

| # | Requirement | Current status | Evidence / action |
|---|---|---|---|
| 1 | All v3.12.0 gates pass with execution evidence | PENDING | Re-run full aggregate at final HEAD; checked-in JSON is fast-path/stale for final cut. |
| 2 | 168h mixed SOAK | BLOCKED | Close Linux/Docker SOAK 5691 re-validation or record explicit governance reclassification. |
| 3 | Security scan PASS | REFRESH-REQUIRED | Re-run `scripts/gate/check_security_scan_v312.sh`; update `SECURITY_AUDIT.md`. |
| 4 | SQLLogicTest selected targets PASS or issue-linked exclusions | PRIOR-PASS | Reconfirm selected-target report; scope out or close new unmilestoned teaching compatibility issues. |
| 5 | TPC-H SF=1 correctness clean | PRIOR-PASS | `evidence/v312-59/GA5_TPCH_SF1_REPORT.md`; rerun only if engine/query code changed. |
| 6 | Wire/LOAD DATA/recovery/backup/upgrade gates PASS | PRIOR-PASS | `evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md`; full rerun recommended at cut. |
| 7 | Documentation links and consistency PASS | THIS-PR-VERIFY | Run docs link and consistency scripts after this refresh. |
| 8 | GMP compliance matrix signed off | PRIOR-SIGNED | Reaffirm `GMP_COMPLIANCE_MATRIX.md` at final HEAD. |
| 9 | B8 thresholds override 13/13 PASS | PRIOR-PASS | Re-run or bind latest threshold evidence at final HEAD. |
| 10 | `GA_GATE_REPORT.md` records evidence hashes and no unresolved P0 blockers | PENDING | Refresh after final aggregate and SOAK/security/doc reruns. |
| 11 | GA-1 aggregator full mode | PENDING | JSON `mode` must be `full`, not `fast-path`, at GA cut. |
| 12 | GA-5 Q17 cell-diff half A + B | PRIOR-PASS | Existing Q17 cell-diff evidence records 61.6s and FP tolerance PASS. |
| 13 | GA-2 SOAK policy boundary | BLOCKED/DECISION | Choose strict Linux/Docker closure or formal reclassification; document decision. |

## Release Claim Checks

Before release notes can say "GA":

- [ ] `STAGE.yaml` changed to `current_stage: "GA"` in the same PR or a final
      promotion PR.
- [ ] Full GA aggregate JSON generated at the release HEAD with `mode: full`.
- [ ] SOAK evidence is either completed or formally reclassified.
- [ ] Security scan is refreshed at release HEAD.
- [ ] Docs links and consistency checks pass after all release doc edits.
- [ ] Open unmilestoned issues #4607, #4608, #4610, #4611, #4612, and #4613
      are closed, moved into a documented non-GA scope, or reflected as
      release claim caveats.
- [ ] Gitea milestone state and issue closure map are recorded with timestamps.
- [ ] Tag candidate and final release commit are recorded.

## Non-Blocking Follow-Up Candidates

These may remain open only if release claims explicitly exclude them:

- SOAK Tier-2/Tier-3 quality-of-life issues #4596, #4597, #4598.
- Broader SQLite teaching compatibility beyond the already verified V312-57
  gate scope.
- General-purpose vector database / graph database claims, which remain out of
  v3.12.0 scope.

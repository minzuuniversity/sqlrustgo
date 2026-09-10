# SQLRustGo v3.12.0 GA Release Checklist

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **stage:** GA
> **stage_ssot:** [`STAGE.yaml`](STAGE.yaml)
> **evidence_index:** [`GA_PUBLICATION_EVIDENCE_INDEX.md`](GA_PUBLICATION_EVIDENCE_INDEX.md)

This checklist records the GA publication state after the 2026-09-08 cut. It replaces the older RC-to-GA candidate checklist as the current release entry point.

## Hard Gates

| # | Requirement | Publication status | Evidence |
|---|---|---|---|
| 1 | All v3.12.0 gates pass with execution evidence | PASS | `evidence/v312-59/ga_gate_report.json`: 72/72 PASS, blockers 0, mode `full`, commit `355b5a3837` |
| 2 | SOAK gate boundary | PASS-WITH-SCOPE | GA-2 gate accepted 1h mixed-workload demo + scaffold readiness; 168h long run remains post-GA monitoring / v3.13 hardening, not a completed v3.12 claim |
| 3 | Security scan | PASS-WITH-RECORDED-CAVEATS | `evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md`; caveats must remain visible in `SECURITY_AUDIT.md` |
| 4 | SQLLogicTest selected targets | PASS-WITH-EXCLUSIONS | `evidence/v312-59/GA4_SQLLOGICTEST_SELECTED_REPORT.md`; full SQLite corpus remains v3.13 scope |
| 5 | TPC-H SF=1 correctness | PASS | `evidence/v312-59/GA5_TPCH_SF1_REPORT.md` and Q17 cell-diff evidence |
| 6 | Wire / LOAD DATA / recovery / backup / upgrade | PASS | `evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md` |
| 7 | Documentation links and consistency | TO-VERIFY-THIS-PR | Re-run docs link and consistency gates after this publication refresh |
| 8 | GMP compliance matrix signoff | SIGNED-OFF | `GMP_COMPLIANCE_MATRIX.md` release-level signoff |
| 9 | B8 thresholds override | PASS | thresholds_override 13/13 PASS in aggregate JSON |
| 10 | GA report records evidence hashes and blockers | PASS-WITH-CAVEATS | `GA_GATE_REPORT.md` + this checklist record open known limitations |
| 11 | GA aggregate full mode | PASS | JSON `mode: full`, not fast-path |
| 12 | GA-5 Q17 cell-diff | PASS | Q17 elapsed 61.6s; cell delta within FLOAT_TOL |
| 13 | Open issue boundary | PASS-WITH-CAVEATS | #4846/#4847/#4848 stay outside GA claims until merged PR verification exists |

## Publication Checklist

- [x] `STAGE.yaml` records `current_stage: "GA"`.
- [x] Full GA aggregate JSON exists at the GA tag commit with `mode: full`.
- [x] GA tags `v3.12.0` and `v3.12.0-ga` dereference to `355b5a3837`.
- [x] Open issues #4846, #4847, and #4848 are documented as GA claim caveats.
- [x] Open PRs #4868, #4869, and #4870 are recorded as not-yet-merged follow-up work.
- [x] Public release notes prohibit broad MySQL/SQLite/vector/graph replacement claims.
- [x] Root README and v3.12.0 README point readers to `STAGE.yaml` and the evidence index.
- [x] This PR's docs link and consistency gates were re-run before push:
      `bash scripts/gate/check_docs_links_v312.sh` exit 0,
      `bash scripts/gate/check_docs_consistency_v312.sh` exit 0, and
      `bash scripts/gate/check_docs_links.sh --all` exit 0. Latest v3.12
      gate refresh timestamp: `2026-09-10T17:32:12Z`.
- [ ] Cross-branch synchronization is completed for `main`, `release/v3.12.0`, and `develop/v4.0.0`.

## Non-Blocking Follow-Up

These items remain open only because the GA claim has been scoped to GMP internal-audit retrieval and the verified teaching CLI boundary:

- #4846: `CHAR(n)` PAD SPACE / point lookup behavior.
- #4847: explicit transaction semantics. This is a high-risk database behavior and must remain prominently excluded until fixed.
- #4848: `ALTER TABLE ... RENAME COLUMN`.
- Full official SQLite corpus expansion.
- 168h production SOAK completion as a post-GA monitoring / v3.13 hardening artifact.

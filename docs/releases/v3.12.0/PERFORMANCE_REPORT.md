# SQLRustGo v3.12.0 Performance Report

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **stage:** GA
> **ga_tag_commit:** `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`

## Summary

The v3.12.0 GA performance claim is limited to the verified GMP internal-audit retrieval scope and the recorded SQL gates. It includes TPC-H SF=1 evidence, LOAD DATA / wire / recovery gate evidence, and GA-2 mixed-workload demo evidence. It does not claim completion of a 168h production SOAK.

## Evidence Matrix

| Area | Publication verdict | Evidence |
|---|---|---|
| TPC-H SF=1 correctness/perf | PASS | `evidence/v312-59/GA5_TPCH_SF1_REPORT.md`, `evidence/v312-58/Q17_SF1_CELLDIFF.json` |
| Q17 SF=1 | PASS | Q17 elapsed 61.6s, row_count=1, FP64 value within tolerance |
| Mixed SOAK demo | PASS-WITH-SCOPE | `evidence/v312-59/GA2_MIXED_SOAK_DEMO_REPORT.md` |
| SOAK V5 local 8h | SUPPORTING-EVIDENCE | `evidence/v312-59/SOAK_V5_FINDINGS.md`; local macOS counterfactual, not 168h production claim |
| 168h production SOAK | NOT-CLAIMED | Post-GA monitoring / v3.13 hardening item |
| Wire / recovery / upgrade | PASS | `evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md` |

## Release Claim Boundary

Allowed:

- TPC-H SF=1 SQLRustGo path is recorded as 22/22 PASS in the GA evidence set.
- Q17 SF=1 regression is recorded as PASS with 61.6s elapsed and cell-level tolerance evidence.
- GA-2 mixed-workload demo and scaffold readiness are recorded for the GA gate boundary.

Not allowed:

- Completed 168h production SOAK.
- Broad database performance readiness outside the GMP internal-audit retrieval workload.
- Performance claims that include open issue scopes #4846, #4847, or #4848.

# SQLRustGo v3.12.0 GA Gate Report

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **stage:** GA
> **release_date:** 2026-09-08
> **ga_tag_commit:** `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
> **post_cut_refresh_head:** `9febebb255f984387ac78c26510d5b46d73f6046`
> **machine_evidence:** [`evidence/v312-59/ga_gate_report.json`](evidence/v312-59/ga_gate_report.json)

## Final Verdict

v3.12.0 reached GA for the GMP internal-audit retrieval database scope. The final machine-readable aggregate records:

| Field | Value |
|---|---|
| version | `v3.12.0` |
| branch | `develop/v3.12.0` |
| commit | `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4` |
| generated_at | `2026-09-08T04:17:15Z` |
| mode | `full` |
| verdict | `PASS` |
| totals | `72/72 PASS, blockers 0` |
| json_sha256 | `09c13c39fee62f2cd269fd9fe842bb9fb88fae6562cb7832f1357bb0e7b0fb26` |

## Aggregate Breakdown

| Stage group | Result | Evidence hash in JSON |
|---|---:|---|
| BETA | 40/40 PASS | `0099b75600087c24b3781eb7d1ce9b85` |
| RC | 11/11 PASS | `7525bf63f57a64032167bfa9c138c38d` |
| GA | 8/8 PASS | `0bb7ec55898c2759a187df1e3aa364f1` |
| thresholds_override | 13/13 PASS | `ae10e94070a244970f62b5c5f0b86390` |

## GA Requirements

| # | Requirement | Final status | Evidence |
|---|---|---|---|
| GA-1 | Aggregate gate full mode | PASS | `ga_gate_report.json`, mode `full` |
| GA-2 | Mixed SOAK gate | PASS-WITH-SCOPE | `GA2_MIXED_SOAK_DEMO_REPORT.md`; 168h production SOAK is not claimed complete |
| GA-3 | Security scan | PASS-WITH-RECORDED-CAVEATS | `GA3_SECURITY_SCAN_REPORT.md` |
| GA-4 | SQLLogicTest selected targets | PASS-WITH-EXCLUSIONS | `GA4_SQLLOGICTEST_SELECTED_REPORT.md` |
| GA-5 | TPC-H SF=1 | PASS | `GA5_TPCH_SF1_REPORT.md`, Q17 cell-diff |
| GA-6 | Wire / LOAD DATA / recovery / backup / upgrade | PASS | `GA6_WIRE_RECOVERY_UPGRADE_REPORT.md` |
| GA-7 | Docs links and consistency | PASS-AT-GA-CUT | `GA7_DOCS_LINKS_REPORT.md`, `GA7_DOCS_CONSISTENCY_REPORT.md`; this publication refresh must re-run docs gates before push |
| GA-8 | GMP matrix signoff | SIGNED-OFF | `GMP_COMPLIANCE_MATRIX.md` |

## Gate Log Caveat

The 2026-09-08 beta gate log contains one shell diagnostic line:

```
check_beta_v3.12.0.sh: line 206: kind:: command not found
```

The checked-in aggregate still records BETA as `40/40 PASS`, and the beta evidence hash in the JSON matches the saved log. This line must remain visible as an evidence-quality caveat; future gate script maintenance should remove the diagnostic and regenerate the aggregate. It does not expand the release claim beyond the final JSON verdict.

## Known Limitations Outside GA Claim Boundary

| Issue | PR state on 252 Gitea at 2026-09-11 | Boundary |
|---|---|---|
| #4846 | PR #4868 open | `CHAR(n)` PAD SPACE / primary-key point lookup excluded |
| #4847 | PR #4870 open | explicit transaction semantics excluded; high-risk database behavior |
| #4848 | PR #4869 open | `ALTER TABLE ... RENAME COLUMN` excluded |

## Final Release Claim

Allowed:

- SQLRustGo v3.12.0 GA for GMP internal-audit retrieval database workloads.
- 72/72 full aggregate gate PASS at tag commit `355b5a3837`.
- TPC-H SF=1 SQLRustGo path 22/22 PASS within the recorded gate scope.

Not allowed:

- Broad MySQL 5.7 replacement.
- Broad SQLite replacement or full official SQLite corpus compatibility.
- General-purpose vector database or graph database.
- Complete explicit transaction semantics.
- Completed 168h production SOAK.

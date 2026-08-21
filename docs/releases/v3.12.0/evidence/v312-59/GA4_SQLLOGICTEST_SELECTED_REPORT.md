# GA-4 v3.12.0 SQLLogicTest Selected Targets Report

> **provenance:** generated_by=check_sqllogictest_selected_v312.sh, generated_at=2026-08-21T16:21:59Z, commit=5e147ea0fee9fbb0caa1b323a853d857b7b25873, branch=fix/v312-59-b-beta-warn-remediation, source_repo=openclaw/sqlrustgo, gate_policy_eval_id=v312-ga4-sqllogictest-selected-001, policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4387 (V312-59-D), umbrella #4383

## Policy (STAGE.yaml line 111)

> "SQLLogicTest selected targets PASS or every exclusion is issue-linked"

## Summary

| Check | Result | Detail |
|-------|--------|--------|
| C1 Manifest total_files | 25 | corpus_stats.total_files |
| C1 Manifest pass_files | 25 | corpus_stats.pass_files |
| C1 Manifest fail_files | 0 | corpus_stats.fail_files |
| C2 Smoke run pass | 25 | smoke-report.md |
| C2 Smoke run fail | 0 | smoke-report.md |
| C3 Exclusions linked | 16 | follow_up_issue present |
| C3 Exclusions unlinked | 0 | follow_up_issue MISSING |
| C3 Exclusions closed | 16 | historical archive |
| **Verdict** | **PASS** | |

## Evidence

- Manifest: `/home/openclaw/sqlrustgo_work/docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json`
- Exclusions: `/home/openclaw/sqlrustgo_work/docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml`
- Smoke report: `/home/openclaw/sqlrustgo_work/docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md`
- Testdata: `/home/openclaw/sqlrustgo_work/crates/sqlrustgo_sqllogictest/testdata`

## Boundary

This gate verifies two conditions:
1. **All selected targets PASS** — measured by `corpus_stats.pass_files` matching
   the active target list. Smoke run (`smoke-report.md`) cross-checks the latest run.
2. **Every exclusion is issue-linked** — `exclusions.yml` items with status
   other than `closed` must reference a real issue via `follow_up_issue`
   (e.g. `#4177` or `V312-11-v313-08`).

It does NOT verify a numeric count of files; STAGE.yaml's text is a logical
condition, not a threshold.

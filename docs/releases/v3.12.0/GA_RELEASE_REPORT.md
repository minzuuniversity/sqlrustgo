# SQLRustGo v3.12.0 GA Candidate Release Report

> **provenance:** generated_by=codex-cli, generated_at=2026-09-02T21:20:00+08:00, source_repo=openclaw/sqlrustgo, branch=fix/v312-59-D-execute-delete-fast-path, remote_head=`b14ad8df0306891e217af19648b210e81d5d2be0`, policy=Anti-Fabrication-Policy-v1.0
> **stage:** RC / GA candidate preparation
> **scope:** documentation-only GA readiness refresh; this document does not promote v3.12.0 to GA.

## Verdict

v3.12.0 is ready for GA-candidate documentation refresh, but it is not ready
for an evidence-clean GA cut.

The current stage SSOT remains `current_stage: "RC"` in `STAGE.yaml`.
The Gitea v3.12.0 milestone is open with 0 open milestone issues and 79 closed
milestone issues, but release promotion is controlled by execution evidence,
not by issue count alone.

## Current Remote Snapshot

| Field | Value |
|---|---|
| Canonical remote | `http://192.168.0.252:3000/openclaw/sqlrustgo.git` |
| Canonical branch | `origin/develop/v3.12.0` |
| Remote HEAD | `b14ad8df0306891e217af19648b210e81d5d2be0` |
| Latest merge | PR #4609, trigger undo record atomic with parent ROLLBACK + ADR-008y binding |
| Milestone `v3.12.0` | open, open_issues=0, closed_issues=79 |
| Stage SSOT | `STAGE.yaml`: RC |

## GA Readiness Summary

| Area | Status | Evidence boundary |
|---|---|---|
| Milestone issue closure | CLOSED-IN-MILESTONE | v3.12.0 milestone has 0 open issues, 79 closed issues. |
| Stage state | RC | `STAGE.yaml` has not transitioned to GA. |
| GA-1 aggregate gate | NEEDS-FULL-REFRESH | Existing `evidence/v312-59/ga_gate_report.json` is fast-path and stale for final GA cut. |
| GA-2 SOAK | BLOCKED-PENDING-REVALIDATION | 1h demo and local 8h V5 evidence exist; Linux/Docker SOAK 5691 re-validation remains required unless formally reclassified. |
| GA-3 security | PASS-WITH-CAVEATS | Security report records baseline-carried cargo-audit caveat and cargo-deny fallback. Refresh at GA cut is required. |
| GA-4 SQLLogicTest selected | PASS-PREVIOUS | Selected-target report exists; new unmilestoned BustubX-EDU issues require release-claim caveat. |
| GA-5 TPC-H SF=1 | PASS-PREVIOUS | 22/22 sqlrustgo oracle match and Q17 cell-diff evidence recorded. |
| GA-6 wire/recovery/upgrade | PASS-PREVIOUS | Fast-path aggregator report exists; full gate rerun recommended at cut. |
| GA-7 docs | NEEDS-REFRESH | This refresh adds current GA candidate entry points; docs gates must be rerun after merge. |
| GA-8 GMP matrix | SIGNED-OFF-PREVIOUS | Signoff exists; reaffirm at final cut if HEAD changes. |

## Remaining GA Blockers

### B1: GA-2 Linux/Docker SOAK 5691 Re-validation

`evidence/v312-59/SOAK_V5_FINDINGS.md` records a strong local counterfactual:
8 h x 4 threads x `--rate=4`, 63,092 transactions, 0 errors, 0 reconnects,
RSS bounded 188-230 MB on macOS dev binary with Fix B + Fix C.

That same report states that the Linux SOAK 5691 Docker re-validation is still
pending and is the canonical blocker for GA-2. Therefore this report does not
claim that the 168h mixed SOAK requirement is closed.

### B2: Final GA Aggregate Must Be Full Mode

`STAGE.yaml` requires a GA-1 aggregate run with `mode: full` at GA cut time.
The current checked-in `evidence/v312-59/ga_gate_report.json` records
`mode: fast-path` at commit `e17d8ec45d`, so it cannot serve as final GA
promotion evidence.

### B3: Post-Milestone Open Compatibility Issues

Live Gitea issue state on 2026-09-02 shows new open issues outside the v3.12.0
milestone:

- #4607: standalone `--` comment line in batch stdin reports EOF parse error.
- #4608: multi-line `CREATE TABLE` column definition parse error.
- #4610: `parse_lit` converts float literal to integer.
- #4611: `length()` returns UTF-8 byte-derived value instead of character count.
- #4612: `CHAR(n)` comparison ignores trailing spaces under SQLite-style use.
- #4613: `round(real, int)` returns INTEGER instead of REAL.

These do not reopen the v3.12.0 milestone by themselves, but they restrict
GA release claims. Until closed or explicitly scoped out, v3.12.0 should not
claim broad SQLite teaching compatibility or broad SQL literal/function
correctness beyond the verified gate scope.

## Documents Added In This Refresh

- `PERFORMANCE_REPORT.md`
- `SECURITY_AUDIT.md`
- `RELEASE_CHECKLIST.md`
- `GA_RELEASE_REPORT.md` (this file)

## Required Final Cut Actions

1. Re-run or regenerate the full GA aggregate with `mode: full` at current
   `origin/develop/v3.12.0` HEAD.
2. Close GA-2 with Linux/Docker SOAK evidence, or record an explicit governance
   reclassification/waiver before claiming GA.
3. Refresh security scan evidence, especially cargo-audit advisory DB and
   license check mode.
4. Re-run v3.12.0 docs link and consistency gates after this documentation
   refresh merges.
5. Reaffirm GMP matrix signoff and then update `STAGE.yaml` from RC to GA only
   after all hard blockers are closed with execution evidence.

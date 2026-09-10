# SQLRustGo v3.12.0 GA Publication Evidence Index

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **stage:** GA
> **stage_ssot:** [`STAGE.yaml`](STAGE.yaml)
> **ga_tag_commit:** `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
> **post_cut_refresh_head:** `9febebb255f984387ac78c26510d5b46d73f6046`

This file is the publication entry point for v3.12.0 GA evidence. It separates current release claims from historical RC / candidate reports.

## Canonical Release Refs

| Ref | Value |
|---|---|
| Canonical remote | `http://192.168.0.252:3000/openclaw/sqlrustgo.git` |
| Release branch | `develop/v3.12.0` |
| GA tag | `v3.12.0` |
| GA milestone tag | `v3.12.0-ga` |
| Tag commit | `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4` |
| Post-cut refresh baseline | `9febebb255f984387ac78c26510d5b46d73f6046` |

## Gate Evidence

| Evidence | Purpose | Current publication status |
|---|---|---|
| [`evidence/v312-59/ga_gate_report.json`](evidence/v312-59/ga_gate_report.json) | Machine-readable GA aggregate | PASS: 72/72, blockers 0, mode `full`, commit `355b5a3837` |
| [`GA_GATE_REPORT.md`](GA_GATE_REPORT.md) | Human-readable gate report | Current GA verdict and caveat map |
| [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) | Publication checklist | Current GA checklist |
| [`GA_RELEASE_REPORT.md`](GA_RELEASE_REPORT.md) | Release rollup | Current GA publication report |
| [`COMPREHENSIVE_ASSESSMENT_REPORT.md`](COMPREHENSIVE_ASSESSMENT_REPORT.md) | Capability and risk assessment | Current GA assessment |
| [`RELEASE_NOTES.md`](RELEASE_NOTES.md) | Public release notes | Current GA notes |
| [`PERFORMANCE_REPORT.md`](PERFORMANCE_REPORT.md) | Performance claim boundary | Current GA-scoped performance report |
| [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md) | Security claim boundary | Current GA-scoped security rollup |

## Machine Evidence Hashes

| File | SHA-256 |
|---|---|
| `evidence/v312-59/ga_gate_report.json` | `09c13c39fee62f2cd269fd9fe842bb9fb88fae6562cb7832f1357bb0e7b0fb26` |
| `evidence/v312-59/ga_beta_gate_20260908_121715.log` | `0099b75600087c24b3781eb7d1ce9b851327edf64e75d93fcad1763482bd4d4d` |

## Open Issue Boundary

Live 252 Gitea state checked on 2026-09-11:

| Issue | PR | Release treatment |
|---|---|---|
| #4846 | #4868 open | Excluded from GA claims until merged and verified |
| #4847 | #4870 open | Excluded from GA claims; high-risk explicit transaction semantics |
| #4848 | #4869 open | Excluded from GA claims until merged and verified |

## Allowed Publication Claim

SQLRustGo v3.12.0 GA is available for the GMP internal-audit retrieval database workload, with full aggregate gate evidence at tag commit `355b5a3837` and documented exclusions for open compatibility / transaction follow-ups.

## Disallowed Publication Claims

- Broad MySQL 5.7 replacement.
- Broad SQLite replacement or full official SQLite corpus compatibility.
- General-purpose standalone vector database.
- General-purpose standalone graph database.
- Complete explicit transaction semantics.
- Completed 168h production SOAK.
- #4846, #4847, or #4848 fixed before their PRs merge and pass verification.

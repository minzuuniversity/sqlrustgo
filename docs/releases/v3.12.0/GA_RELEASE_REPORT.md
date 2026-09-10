# SQLRustGo v3.12.0 GA Release Report

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **stage:** GA
> **release_date:** 2026-09-08
> **ga_tag_commit:** `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
> **post_cut_refresh_head:** `9febebb255f984387ac78c26510d5b46d73f6046`

## Verdict

v3.12.0 is released as GA for the controlled GMP internal-audit retrieval database scope. The release is not a broad MySQL, SQLite, vector database, or graph database replacement.

The GA decision is bound to the machine-readable aggregate in [`evidence/v312-59/ga_gate_report.json`](evidence/v312-59/ga_gate_report.json):

| Field | Value |
|---|---|
| version | `v3.12.0` |
| branch | `develop/v3.12.0` |
| commit | `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4` |
| generated_at | `2026-09-08T04:17:15Z` |
| mode | `full` |
| verdict | `PASS` |
| totals | `72/72 PASS, blockers 0` |

## Current Remote Snapshot

| Field | Value |
|---|---|
| Canonical remote | `http://192.168.0.252:3000/openclaw/sqlrustgo.git` |
| Canonical release branch | `develop/v3.12.0` |
| GA tags | `v3.12.0`, `v3.12.0-ga` |
| Stage SSOT | `STAGE.yaml`: GA |
| Publication evidence index | `GA_PUBLICATION_EVIDENCE_INDEX.md` |

## Known Limitations And Open Follow-Up PRs

Live 252 Gitea status checked during the 2026-09-11 publication refresh:

| Issue | PR | Status | Release boundary |
|---|---|---|---|
| #4846 | #4868 | open issue / open PR | `CHAR(n)` PAD SPACE primary-key point lookup excluded |
| #4847 | #4870 | open issue / open PR | explicit transaction semantics excluded |
| #4848 | #4869 | open issue / open PR | `ALTER TABLE ... RENAME COLUMN` excluded |

These PRs may improve post-GA quality, but they are not included in the v3.12.0 GA tag until merged and independently verified.

## Publication Decision

The release may be published with this exact wording:

> SQLRustGo v3.12.0 GA is available for the GMP internal-audit retrieval database workload, with full aggregate gate evidence at tag commit `355b5a3837` and documented exclusions for open compatibility / transaction follow-ups.

The release must not be published with wording that implies broad SQL compatibility or complete transaction semantics.

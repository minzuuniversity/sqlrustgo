# SQLRustGo v3.12.0

> **provenance:** generated_by=codex-cli, generated_at=2026-09-08T04:33:08+08:00, source_repo=openclaw/sqlrustgo, branch=codex/v312-docs-readme-refresh, head=`54219264158f86ea23a43edd17e32f7d743b74f5`, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **status:** RC / GA preparation. This document does not promote v3.12.0 to GA.
> **SSOT:** `STAGE.yaml` remains the release-stage source of truth.

v3.12.0 is the SQLRustGo line for a controlled GMP internal-audit retrieval
database. The allowed product scope is:

- SQLRustGo-managed relational storage for GMP documents, chunks, versions,
  audit records, evidence relations, and retrieval metadata.
- Internal vector retrieval and hybrid retrieval for the GMP/RAG workload.
- SQL-backed graph projection for evidence navigation.
- MySQL-style and sqlite-like entry points only within verified compatibility
  boundaries.

This release must not be described as a general-purpose vector database, a
general-purpose graph database, or a broad MySQL/SQLite replacement.

## Current Snapshot

| Field | Value |
|---|---|
| Canonical remote | `http://192.168.0.252:3000/openclaw/sqlrustgo.git` |
| Branch | `origin/develop/v3.12.0` |
| HEAD checked | `54219264158f86ea23a43edd17e32f7d743b74f5` |
| Latest merge at snapshot | PR #4851, graph M4 Cypher parser and executor |
| Stage SSOT | `STAGE.yaml`: `current_stage: "RC"` |
| Live Gitea open issues | #4846, #4847, #4848 |
| Live open PRs | 0 |

## GA Status

v3.12.0 is not ready for an evidence-clean GA cut at this snapshot.

The checked-in GA aggregate
`docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json` records:

| Field | Value |
|---|---|
| generated_at | `2026-09-07T04:26:29Z` |
| commit | `65ef5bea52187e4be2f38e8c239d13002f150705` |
| mode | `full` |
| verdict | `FAIL` |
| totals | `71/72`, `blockers=9` |

Because that evidence is stale relative to `542192641` and has `verdict=FAIL`,
it cannot be used as GA promotion evidence. A final GA attempt must re-run:

```bash
bash scripts/gate/check_ga_v3.12.0.sh --full
```

and then refresh the linked evidence at the final release commit.

## Open Issues Affecting GA

Live Gitea state on 2026-09-08 shows three open issues:

| Issue | Area | Current release impact |
|---|---|---|
| #4846 | Executor / type semantics | `CHAR(n)` padding/comparison causes point lookup misses. This blocks broad SQLite/MySQL-style teaching compatibility claims until fixed or explicitly scoped out. |
| #4847 | Transaction semantics | Explicit transaction behavior differs across batch, wire, and persistent connection paths. This is a data-integrity risk and should be treated as GA-blocking unless formally reclassified. |
| #4848 | Storage / DDL | `ALTER TABLE ... RENAME COLUMN` is unsupported in the storage engine. This blocks catalog-evolution teaching claims unless fixed or excluded. |

Recommended classification:

- #4847: **P0 / GA-blocker** because it can roll back transaction-external data
  or preserve changes that should be rolled back.
- #4846: **P1 / GA-blocker for teaching compatibility** because CHAR primary-key
  point lookup fails on the BustubX-EDU corpus.
- #4848: **P1 / GA-claim-caveat or blocker**, depending on whether week-11
  catalog evolution is part of the final v3.12.0 public claim.

## RC-GA Gate Requirements

The BustubX-EDU B-track findings show that previous fixtures were too narrow.
The RC-GA gate set must cover:

| Gate | Script | Required outcome |
|---|---|---|
| RC-B1 | `scripts/gate/check_bustubx_b_track_v312.sh` | B-track seed and exercise corpus pass or every exclusion is issue-linked. |
| RC-B2 | `scripts/gate/check_v312_parser_real_scripts.sh` | Multi-line DDL, comments, quoted identifiers, Chinese text, and basic DML parse correctly. |
| RC-B3 | `scripts/gate/check_v312_no_silent_success.sh` | Accepted DDL/DML has observable postconditions; unsupported SQL returns explicit errors. |
| RC-B4 | `scripts/gate/check_v312_type_function_semantics.sh` | Core type/function behavior matches the selected oracle. |
| RC-B5 | `scripts/gate/check_v312_join_subquery_semantics.sh` | JOIN, subquery, and HAVING semantics match oracle expectations. |
| RC-B6 | `scripts/gate/check_v312_dml_integrity.sh` | CHECK, autoincrement, RETURNING, and UPDATE behavior are correct or scoped out. |
| RC-B7 | `scripts/gate/check_ga_v3.12.0.sh --full` | Final aggregate is full-mode, current-HEAD, and blocker-free. |

At this snapshot, RC-B1 is still a skeleton gate and intentionally exits
non-zero until the real B-track corpus and oracle artifacts are populated.

## Final GA Checklist

Before changing `STAGE.yaml` to GA or cutting tags:

- Close #4846/#4847/#4848 by merged PRs with regression tests, or add explicit
  release-claim downgrades approved by release governance.
- Replace skeleton RC-B gates with executable semantic checks.
- Re-run `check_ga_v3.12.0.sh --full` at final HEAD and refresh
  `evidence/v312-59/ga_gate_report.json`.
- Refresh docs link and consistency gates.
- Refresh security evidence and SOAK policy/evidence.
- Update release notes and claim-boundary documents from the same final commit.

## Key Documents

| Document | Purpose |
|---|---|
| `STAGE.yaml` | Stage SSOT and promotion requirements. |
| `GA_GATE_REPORT.md` | Current GA verdict map and evidence boundaries. |
| `RELEASE_CHECKLIST.md` | RC-to-GA action checklist. |
| `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` | RC-GA issue triage and gate plan. |
| `CLAIM_DOWNGRADE_MANIFEST.md` | Claim downgrades and closure ledger. |
| `TEST_PLAN.md` | Test strategy and gate expectations. |
| `COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md` | Layered coverage/test framework. |
| `GMP_COMPLIANCE_MATRIX.md` | GMP/ALCOA+ mapping and signoff boundary. |
| `STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md` | Stage governance remediation log (2026-08-18 RC→RC reclass + stage drift fixes). |

## Historical Notes

Older sections in this directory may preserve the wording and evidence from
their original audit date. When documents conflict, use the current
`STAGE.yaml`, the latest Gitea issue/PR state, and freshly executed gate output
as the higher-trust evidence chain.

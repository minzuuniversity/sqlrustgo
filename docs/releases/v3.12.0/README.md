# SQLRustGo v3.12.0

> **provenance:** generated_by=claude-macmini, generated_at=2026-09-08T11:55:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, head=`34d8adc56cc351db19182fb852056d4d7483fa00`, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **status:** v3.12.0 GA. STAGE.yaml `current_stage: "GA"`. GA gate verdict PASS at HEAD `34d8adc56c` (72/72, 0 blockers). Tags v3.12.0 + v3.12.0-ga to be cut per STAGE_CONFIG RC_to_GA trigger.
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
| HEAD checked | `34d8adc56cc351db19182fb852056d4d7483fa00` |
| Latest merge at snapshot | PR #4851, graph M4 Cypher parser and executor |
| Stage SSOT | `STAGE.yaml`: `current_stage: "GA"` |
| Live Gitea open issues | #4846, #4847, #4848 (carried as GA-claim-caveat per §9.6.2) |
| Live open PRs | 0 |

## GA Status

The GA gate has reached an evidence-clean PASS at HEAD `34d8adc56c`. The
checked-in GA aggregate
`docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json` records:

| Field | Value |
|---|---|
| generated_at | `2026-09-08T03:48:45Z` |
| commit | `34d8adc56cc351db19182fb852056d4d7483fa00` |
| mode | `full` |
| verdict | **PASS** |
| totals | **72/72**, `blockers=0` |
| BETA | 40/40 (blockers: 0) |
| RC | 11/11 (blockers: 0) |
| GA | 8/8 (blockers: 0) |
| thresholds_override | 13/13 (blockers: 0) |

This supersedes the 2026-09-07 stale `FAIL` evidence (71/72, 9 blockers).
The headline fix at commit `b743ea95f4` is a deterministic python3
cross-check replacing the non-deterministic bash pipeline race in
`scripts/gate/check_ignore_count.sh` (Q2_P12 sub-gate of B7_ALPHA_QUALITY);
the runner fix at commit `34d8adc56c` pins `q4_residual_filter_test` to
`--test-threads=1` to remove the HSJ AtomicU64 counter flake in the
B2_INTEGRATION_TESTS per-binary pipeline.
See `CLAIM_DOWNGRADE_MANIFEST.md` §9.6.1 for the full B2 status change
ledger.

## Known Limitations — v3.12.0 GA Candidate

This v3.12.0 GA build explicitly excludes the following capabilities from its
release claims. Issues remain open and will be addressed in v3.13.0:

(prior §3 boundary lines remain — `ANALYZE`/`sqlite_stat1` (#4719),
math functions / `GREATEST`/`LEAST` (#4698), `SET TIMEZONE`/`SET TRANSACTION
ISOLATION LEVEL` (#4694), multi-table `UPDATE`/`DELETE USING` (#4685),
`CEIL`/`FLOOR`/`TRUNCATE`/`HEX`/`MD5`/`SHA2` partial (#4670), `JSON_EXTRACT`/
`JSON_EACH` (#4646), `INDEXED BY` hint (#4625).)

- **`CHAR(n)` byte-padding primary-key point lookup (#4846)** —
  BustubX-EDU teaching corpus only. `VARCHAR` columns and explicit
  padded-literal queries are unaffected. Use `VARCHAR` or pad literal values
  explicitly if `CHAR`-compat is required.
- **Explicit `BEGIN`/`COMMIT`/`ROLLBACK` transaction semantics (#4847)** —
  the GMP product uses single-statement batch mode and is unaffected. Use
  v3.11.0 or wait for v3.13.0 if transactional reliability is required.
- **`ALTER TABLE ... RENAME COLUMN` (#4848)** — catalog-evolution scope
  only. `ADD COLUMN` and `DROP COLUMN` are unaffected. Catalog evolution via
  `RENAME` is tracked for v3.13.0 (#4313).

See `CLAIM_DOWNGRADE_MANIFEST.md` §9.6.2 for per-issue boundary language
and §9.6.3 for the consolidated release-note block.

## Open Issues Affecting GA

Live Gitea state on 2026-09-08 shows three open issues, all carried as
GA-claim-caveat per §3 pattern (release notes / scope docs explicitly
exclude the capability):

| Issue | Area | Classification | Boundary |
|---|---|---|---|
| #4846 | Executor / type semantics | GA-claim-caveat (teaching scope) | `CHAR(n)` byte-padding point lookup excluded; `VARCHAR` unaffected |
| #4847 | Transaction semantics | GA-claim-caveat (product uses single-statement batch) | Explicit transaction semantics excluded; v3.11.0 for transactional reliability |
| #4848 | Storage / DDL | GA-claim-caveat (catalog-evolution scope) | `ALTER TABLE ... RENAME COLUMN` excluded; `ADD COLUMN`/`DROP COLUMN` unaffected |

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
| RC-B7 | `scripts/gate/check_ga_v3.12.0.sh --full` | Final aggregate is full-mode, current-HEAD, and blocker-free. ✅ PASS at HEAD `34d8adc56c` (72/72, 0 blockers) |

At this snapshot, RC-B1 is still a skeleton gate and intentionally exits
non-zero until the real B-track corpus and oracle artifacts are populated.

## Final GA Checklist

Before changing `STAGE.yaml` to GA or cutting tags:

- ✅ Fresh `--full` gate verdict PASS at HEAD `34d8adc56c` (72/72, 0 blockers).
- ✅ `STAGE.yaml` `gate_snapshot` updated with the fresh evidence.
- ✅ `STAGE.yaml` flipped to `current_stage: "GA"`.
- ✅ `CLAIM_DOWNGRADE_MANIFEST.md` §9.6 documents the 3 open GA-claim-caveat items.
- ✅ `README.md` "Known Limitations — v3.12.0 GA Candidate" section lists the
  3 boundary lines.
- ⏳ Cut `v3.12.0` + `v3.12.0-ga` tags per STAGE_CONFIG RC_to_GA trigger.

## Key Documents

| Document | Purpose |
|---|---|
| `STAGE.yaml` | Stage SSOT and promotion requirements. |
| `GA_GATE_REPORT.md` | Current GA verdict map and evidence boundaries. |
| `RELEASE_CHECKLIST.md` | RC-to-GA action checklist. |
| `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` | RC-GA issue triage and gate plan. |
| `CLAIM_DOWNGRADE_MANIFEST.md` | Claim downgrades and closure ledger (incl. §9.6 fresh refresh). |
| `TEST_PLAN.md` | Test strategy and gate expectations. |
| `COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md` | Layered coverage/test framework. |
| `GMP_COMPLIANCE_MATRIX.md` | GMP/ALCOA+ mapping and signoff boundary. |
| `STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md` | Stage governance remediation log (2026-08-18 RC→RC reclass + stage drift fixes). |

## Historical Notes

Older sections in this directory may preserve the wording and evidence from
their original audit date. When documents conflict, use the current
`STAGE.yaml`, the latest Gitea issue/PR state, and freshly executed gate output
as the higher-trust evidence chain.

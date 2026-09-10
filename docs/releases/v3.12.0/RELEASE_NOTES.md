# SQLRustGo v3.12.0 Release Notes

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **release_status:** GA
> **release_date:** 2026-09-08
> **ga_tag_commit:** `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
> **post_cut_refresh_head:** `9febebb255f984387ac78c26510d5b46d73f6046`
> **stage_ssot:** [`STAGE.yaml`](STAGE.yaml)
> **evidence_index:** [`GA_PUBLICATION_EVIDENCE_INDEX.md`](GA_PUBLICATION_EVIDENCE_INDEX.md)

## Release Summary

SQLRustGo v3.12.0 is the GA release for the GMP internal-audit retrieval database baseline. The release is scoped to a controlled GMP workload: relational storage for documents, chunks, versions, audit records, evidence relations, retrieval metadata, SQL-backed graph projection, and verified MySQL-style / sqlite-like entry points.

The final GA gate evidence is the full aggregate report generated at tag commit `355b5a3837`:

| Gate group | Result |
|---|---:|
| BETA | 40/40 PASS |
| RC | 11/11 PASS |
| GA | 8/8 PASS |
| thresholds_override | 13/13 PASS |
| **Total** | **72/72 PASS, blockers 0** |

Machine-readable evidence: [`evidence/v312-59/ga_gate_report.json`](evidence/v312-59/ga_gate_report.json), generated at `2026-09-08T04:17:15Z`, mode `full`, commit `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`.

## Tags And Branches

| Ref | Status |
|---|---|
| `v3.12.0` | GA tag, dereferences to `355b5a3837` |
| `v3.12.0-ga` | GA milestone tag, dereferences to `355b5a3837` |
| `develop/v3.12.0` | post-cut docs/evidence refresh line |
| `main` | synchronized from v3.12.0 release line |
| `release/v3.12.0` | synchronized from v3.12.0 release line |

## What Is Included

- GMP document / chunk / embedding / audit schema and retrieval metadata.
- GMP hybrid retrieval and SQL-backed evidence graph projection.
- Audit hash-chain tamper fail-closed behavior for the release workload.
- TPC-H SF=1 SQLRustGo path: 22/22 PASS, including Q17 cell-diff evidence.
- SQLLogicTest smoke and curated selected-target evidence within the documented scope.
- Wire protocol, LOAD DATA, crash recovery, backup/restore, and upgrade/downgrade gates bound into the GA aggregate.
- BustubX-EDU sqlite3-like CLI fixtures within the verified week01-week06 scope.

## Known Limitations

These issues remain outside the GA claim boundary. They must not be described as fixed until their PRs are merged and verified.

| Issue | Current state on 252 Gitea at 2026-09-11 | GA claim boundary |
|---|---|---|
| #4846 | Open issue; PR #4868 open | `CHAR(n)` PAD SPACE / primary-key point lookup behavior is excluded. Use `VARCHAR` or non-CHAR paths for GA claims. |
| #4847 | Open issue; PR #4870 open | Explicit `BEGIN` / `COMMIT` / `ROLLBACK` transaction semantics are excluded. The GMP GA workload uses single-statement batch execution. |
| #4848 | Open issue; PR #4869 open | `ALTER TABLE ... RENAME COLUMN` is excluded. `ADD COLUMN` / `DROP COLUMN` remain within the verified boundary where separately evidenced. |

## Claims Not Allowed For v3.12.0

- Broad MySQL 5.7 replacement.
- Broad SQLite replacement or full official SQLite corpus compatibility.
- General-purpose standalone vector database.
- General-purpose standalone graph database.
- Complete explicit transaction semantics.
- Completed 168h production SOAK run. v3.12.0 carries GA-2 1h mixed-workload demo evidence and scaffold readiness; the 168h run is a post-GA monitoring / v3.13 hardening item unless a later verified report supersedes this note.

## Upgrade And Operations Notes

- Use the release tags for reproducible GA audit: `v3.12.0` or `v3.12.0-ga`.
- Treat [`STAGE.yaml`](STAGE.yaml) as the stage SSOT.
- Use [`GA_PUBLICATION_EVIDENCE_INDEX.md`](GA_PUBLICATION_EVIDENCE_INDEX.md) as the public evidence entry point.
- Before extending claims beyond the GMP internal-audit retrieval scope, close or formally reclassify the known limitations above with PR merge evidence and test logs.

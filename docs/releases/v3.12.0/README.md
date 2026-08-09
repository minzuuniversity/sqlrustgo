# SQLRustGo v3.12.0

> **Status**: PLANNED
> **Product target**: GMP internal-audit retrieval database for `~/gmp-platform`
> **Planning date**: 2026-08-09

v3.12.0 is planned as the first SQLRustGo release focused on GMP internal-audit retrieval workloads. It uses SQLRustGo as the database foundation for regulated document storage, chunking, embeddings, audit trails, evidence relations, hybrid retrieval, and SQL-backed graph projection.

This release must not be described as a general-purpose vector database or graph database. Those are v4.0.0 goals.

2026-08-09 planning update: v3.12.0 also becomes the hardening release for v3.11.0 GA weak points. It must close or explicitly re-gate TPC-H correctness, coverage methodology, MySQL wire protocol, LOAD DATA/bulk import, crash recovery, backup/restore, upgrade/downgrade, dependency audit refresh, and the SQLite SQLLogicTest oracle gate that was planned in v3.10.0 but remained non-blocking in v3.11.0.

## Release Contract

Allowed v3.12.0 claim:

> SQLRustGo v3.12.0 supports controlled GMP internal-audit retrieval workloads with SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles.

Disallowed claims:

- General-purpose standalone vector database.
- General-purpose graph database.
- Production release without v3.11.0 weak-point disposition.
- Production release without 168h mixed SOAK evidence.
- Broad MySQL 5.7 replacement without SQLLogicTest, TPC-H correctness, wire protocol, LOAD DATA, recovery, and upgrade evidence.

## Key Documents

| Document | Purpose |
|---|---|
| `STAGE.yaml` | Stage SSOT and promotion criteria |
| `DEVELOPMENT_PLAN.md` | Implementation plan and GMP-Platform integration work packages |
| `VERSION_PLAN.md` | Product scope and work packages |
| `TEST_PLAN.md` | Gate and test matrix |
| `ISSUES_PLAN.md` | V312-01 through V312-14 task breakdown |
| `GMP_COMPLIANCE_MATRIX.md` | GMP/ALCOA+ compliance control mapping |
| `fixtures/gmp_audit_questions.yml` | Retrieval quality fixture seed |

## Dependency

v3.12.0 is planned from the v3.11.0 GA assessment state. Before v3.12.0 GA, the project must close or explicitly carry v3.11.0 weak points: G3 coverage口径, G4 TPC-H SF=1 correctness, 168h SOAK scope, sqllogictest gate drift, MySQL wire protocol gaps, LOAD DATA/bulk import, crash recovery, backup/restore, upgrade/downgrade, dependency audit refresh, and debt-registry drift.

## Added Hardening Gates

| Gate | Purpose |
|---|---|
| SQLite SQLLogicTest | Turn the existing `crates/sqlrustgo_sqllogictest` runner and SQLite corpus plan into a real gate |
| TPC-H correctness | Add row-count and SHA256 cross-engine checks beyond 22/22可运行性 |
| MySQL wire + LOAD DATA | Harden protocol and data import paths needed by MySQL-style workloads |
| Recovery + upgrade | Prove WAL replay, backup/restore, and v3.10/v3.11 to v3.12 upgrade behavior |

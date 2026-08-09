# SQLRustGo v3.12.0 Version Plan

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-09
> **Product target**: GMP internal-audit retrieval database for `~/gmp-platform`
> **Planning source**: `docs/plans/2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md`

## 1. Positioning

v3.12.0 is a controlled production release for GMP internal-audit retrieval. It should make SQLRustGo usable by `~/gmp-platform` as the main database for regulated document metadata, chunks, embeddings, audit logs, and evidence relations.

v3.12.0 is not a general-purpose vector database or graph database release. Vector retrieval and graph traversal are internal GMP/RAG capabilities in this version.

v3.12.0 also carries a production-hardening track from the v3.11.0 GA assessment: TPC-H correctness, coverage methodology, MySQL wire protocol, LOAD DATA, crash recovery, backup/restore, upgrade/downgrade, and the previously deferred SQLite SQLLogicTest oracle gate.

## 2. Dependency on v3.11.0

v3.12.0 must not inherit unverified v3.11.0 production claims. Before v3.12.0 enters GA, the project must either close or explicitly carry the following v3.11.0 weak points:

| Blocker | Required decision |
|---|---|
| G3 Coverage | Produce reproducible per-crate coverage evidence or carry as v3.12 P0 |
| G4 TPC-H SF=1 | Produce 22/22 run evidence plus cross-engine row-count/SHA256 correctness, or carry as v3.12 P0 |
| v3.11.0 168h SOAK | Complete, repeat, or explicitly scope prior production claim |
| SQLLogicTest / SQLite oracle | Promote from v3.11 TBD to v3.12 executable gate |
| MySQL wire / LOAD DATA / recovery | Add missing production hardening gates before broader MySQL replacement claims |
| debt registry drift | Reconcile F-25/F-26 and extension crate state with truth audit |

## 3. v3.12.0 Product Scope

| Area | Scope |
|---|---|
| GMP relational store | Documents, chunks, versions, status, metadata, audit rows |
| GMP vector retrieval | Embeddings stored in SQLRustGo and indexed by `sqlrustgo-vector` |
| GMP graph projection | SQL-backed nodes and edges for audit evidence navigation |
| Hybrid retrieval | SQL filters + keyword + vector + graph relation boost + RRF |
| Compliance | ALCOA+, audit hash chain, access control, backup/restore evidence |
| Migration | Replace required SQLite/Chroma/PostgreSQL runtime dependencies for GMP production path |
| SQL correctness | SQLite SQLLogicTest oracle, TPC-H cross-engine checks, sqllogictest baseline reports |
| Production hardening | Wire protocol, LOAD DATA, crash recovery, backup/restore, upgrade/downgrade |

## 4. Non-Goals

- No general Cypher compatibility claim.
- No standalone vector database product claim.
- No multi-tenant HA/distributed storage claim.
- No production claim without 168h mixed SOAK.
- No link between "crate exists" and "feature is production-ready" without gate evidence.

## 5. Work Packages

| ID | Package | Priority | Exit evidence |
|---|---|---|---|
| V312-01 | v3.11 weak-point closure and debt registry reconciliation | P0 | Updated GA/debt evidence |
| V312-02 | GMP schema v3.12 | P0 | Schema tests PASS |
| V312-03 | GMP markdown ingestion from `~/gmp-platform/gmp-md` | P0 | Corpus ingestion report |
| V312-04 | Embedding provider and vector persistence | P0 | Rebuildable vector index |
| V312-05 | Hybrid retrieval with citation bundle | P0 | Retrieval quality report |
| V312-06 | SQL-backed graph projection | P0 | Node/edge/path tests |
| V312-07 | Audit hash chain and ALCOA+ controls | P0 | Tamper tests fail closed |
| V312-08 | Backup/restore for GMP/RAG/graph projection | P0 | Restore verification report |
| V312-09 | Mixed workload SOAK | P0 | 168h report |
| V312-10 | GMP compliance docs and user operations guide | P1 | Signed compliance matrix |
| V312-11 | SQLite SQLLogicTest oracle gate | P0 | Runner build/run output, SQLite corpus manifest, baseline report |
| V312-12 | TPC-H SF=1 correctness close-out | P0 | Cross-engine row-count and SHA256 report |
| V312-13 | MySQL wire + LOAD DATA hardening | P0 | Wire e2e, LOAD DATA row/hash/memory evidence |
| V312-14 | Crash recovery and upgrade/downgrade verification | P0 | WAL replay, restore, upgrade and rollback reports |

## 6. Release Milestones

| Milestone | Goal | Required evidence |
|---|---|---|
| Alpha | Schema + representative ingestion + sqllogictest runner build | Build/fmt/clippy + schema tests + SLT runner smoke |
| Beta | Hybrid retrieval + graph projection + SLT smoke corpus | Retrieval fixture + graph tests + SLT report |
| RC | Compliance hardening + TPC-H correctness + wire/LOAD DATA/recovery | audit, ACL, backup/restore, cross-engine, wire, LOAD DATA tests |
| GA | GMP internal-audit production | 168h mixed SOAK + GA gate report + no unresolved v3.11 carried P0 |

## 7. GA Claim

Allowed GA claim:

> SQLRustGo v3.12.0 is production-ready for controlled GMP internal-audit retrieval workloads using SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles.

Disallowed GA claim:

> SQLRustGo v3.12.0 is a general-purpose replacement for dedicated vector databases or graph databases.

Also disallowed:

> SQLRustGo v3.12.0 is a broad MySQL 5.7 replacement unless TPC-H correctness, SQLLogicTest, wire protocol, LOAD DATA, crash recovery, backup/restore, and upgrade evidence all pass.

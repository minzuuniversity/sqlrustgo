# SQLRustGo v3.12.0 Issues Plan

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-09

This file breaks the v3.12.0 plan into issue-sized work packages. Every item must produce execution evidence before it can be marked complete.

## V312-01: Prior-Release Blocker Disposition

**Priority**: P0

**Goal**: Ensure v3.12.0 does not inherit hidden v3.11.0 production weak points.

**Scope**:
- Reconcile v3.11.0 G3 coverage.
- Reconcile v3.11.0 G4 TPC-H SF=1.
- Decide v3.11.0 168h SOAK status.
- Reconcile `debt-registry.yaml` against v3.11.0 truth audit, especially F-25/F-26 and extension crate states.

**Exit evidence**:
- Updated blocker disposition report.
- Stage gate output.
- Evidence hashes for any PASS claim.

## V312-02: GMP Schema v3.12

**Priority**: P0

**Goal**: Define SQLRustGo-managed GMP tables for documents, chunks, embeddings, audit logs, and relations.

**Acceptance**:
- Schema creation is idempotent.
- Document version history is preserved.
- Audit rows include previous hash and event hash.
- Relation rows can model SOP, clause, CAPA, deviation, role, and equipment relationships.

## V312-03: GMP Corpus Ingestion

**Priority**: P0

**Goal**: Import `~/gmp-platform/gmp-md` into SQLRustGo-managed GMP tables.

**Acceptance**:
- Full corpus ingestion report includes document count, chunk count, relation count, embedding count, skipped files, and failures.
- Re-ingestion is idempotent.
- Modified source files create new document versions.
- Source path and source hash are preserved.

## V312-04: Embedding Provider and Vector Persistence

**Priority**: P0

**Goal**: Replace required Chroma runtime dependency for the GMP production path with SQLRustGo-managed embedding storage and vector index rebuild.

**Acceptance**:
- Embeddings are stored in SQLRustGo tables.
- Fixed fixture can rebuild a Flat or HNSW index.
- Model name, dimension, vector hash, and chunk id are recorded.
- Embedding provider abstraction supports BGE-M3/Ollama-compatible flows.

## V312-05: Hybrid Retrieval

**Priority**: P0

**Goal**: Implement GMP internal-audit retrieval using SQL filters, keyword score, vector similarity, graph relation boost, and RRF fusion.

**Acceptance**:
- Every result includes source path, document id, version, chunk id, chunk hash, score components, and citation text.
- Results can be filtered by document type, chapter, status, effective date, and relation type.
- Fixed audit question fixture produces deterministic results.

## V312-06: SQL-Backed GMP Graph Projection

**Priority**: P0

**Goal**: Provide graph navigation for GMP evidence without claiming a general graph database.

**Acceptance**:
- Nodes and edges are stored in SQLRustGo tables.
- Supports neighbors and depth-limited paths up to depth 3.
- Supports relation filters.
- Does not require archived `graph` crate as unvalidated production dependency.

## V312-07: Compliance and Data Integrity Controls

**Priority**: P0

**Goal**: Implement ALCOA+ and GMP-relevant controls for internal-audit retrieval.

**Acceptance**:
- Audit hash-chain tamper tests fail closed.
- Role-based access tests cover import, approve, search, export, and audit review.
- Electronic-signature hooks exist for approval and controlled export.
- Compliance matrix maps tests to controls.

## V312-08: Backup and Restore

**Priority**: P0

**Goal**: Verify backup/restore of SQL, GMP documents, embeddings, graph projection, and audit chain.

**Acceptance**:
- Restored counts match source counts.
- Restored hashes match source hashes.
- Vector index can rebuild after restore.
- Audit chain verifies after restore.

## V312-09: Mixed Workload SOAK

**Priority**: P0

**Goal**: Run a 168h workload representative of `~/gmp-platform`.

**Workload**:
- SQL reads/writes.
- GMP corpus import and incremental update.
- Hybrid retrieval.
- Graph traversal.
- Audit export.
- Backup/restore smoke.

**Acceptance**:
- 168h complete.
- 0 crash.
- No audit-chain break.
- No unclassified data loss.

## V312-10: Documentation and Operations

**Priority**: P1

**Goal**: Provide production operation docs for GMP internal-audit retrieval.

**Acceptance**:
- User operations guide exists.
- Compliance matrix signed off.
- Migration guide from SQLite/Chroma/PostgreSQL prototypes exists.
- GA gate report contains only evidence-backed PASS claims.

## V312-11: SQLite SQLLogicTest Oracle Gate

**Priority**: P0

**Goal**: Finish the SQLite automatic testing framework planned in v3.10.0 and left non-blocking/TBD in v3.11.0.

**Background**:
- `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` proposed SQLLogicTest against SQLite's official corpus as the P0 oracle path.
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-14 recorded a runner implementation but a blocked official-suite download.
- `crates/sqlrustgo_sqllogictest` exists and currently has 22 local `.test` files.
- `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` still had `sqllogictest runner all targets` as TBD.

**Scope**:
- Normalize the canonical runner path to `crates/sqlrustgo_sqllogictest`.
- Build and run the existing smoke corpus.
- Create a reproducible SQLite official SQLLogicTest corpus acquisition/cache plan.
- Add a manifest with upstream snapshot, file count, hash list, skipped files, and exclusion reasons.
- Add or plan `scripts/gate/check_sqllogictest_v312.sh`.
- Save outputs under `docs/releases/v3.12.0/sqllogictest-baseline/` and `docs/releases/v3.12.0/logs/`.

**Acceptance**:
- `cargo build -p sqlrustgo_sqllogictest` succeeds.
- `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` produces a smoke report.
- The selected SLT corpus has PASS/FAIL/SKIP classification.
- Every skip/fail group has an issue, owner, expiry, and rationale.
- v3.12.0 GA cannot pass while this item is still TBD.

## V312-12: TPC-H SF=1 Correctness Close-Out

**Priority**: P0

**Goal**: Turn v3.11.0's TPC-H SF=1 22/22可运行性证据 into correctness evidence.

**Scope**:
- Re-run all 22 SF=1 queries from the same fixture.
- Capture SQLRustGo row counts and sorted result SHA256.
- Capture SQLite/PostgreSQL/MySQL or MariaDB reference row counts and SHA256 where supported.
- Explain every zero-row query with data, not prose-only reasoning.
- Store per-query artifacts and summary report.

**Acceptance**:
- 22/22 query artifacts exist.
- No unexplained checksum mismatch.
- No unexplained zero-row result.
- `TPCH_SF1_VERIFICATION_REPORT.md` no longer depends on a future PG SHA256 task for its core correctness claim.

## V312-13: MySQL Wire Protocol and LOAD DATA Hardening

**Priority**: P0

**Goal**: Close the v3.11.0 production-readiness gap around MySQL compatibility and data import.

**Scope**:
- Add wire e2e coverage for COM_QUERY, COM_STMT_PREPARE, COM_STMT_EXECUTE, COM_STMT_CLOSE, error packets, reset connection, TLS, and compression boundaries.
- Add `LOAD DATA LOCAL INFILE` or clearly-scoped alternative bulk import tests.
- Measure SF=1 and, where feasible, SF=10 import time, memory peak, row counts, and hashes.
- Fail closed on silent truncation, type conversion drift, or out-of-memory behavior.

**Acceptance**:
- Wire protocol e2e report exists.
- LOAD DATA/bulk-import report exists with count/hash verification.
- mysql-server and mysql-client coverage trend improves or has explicit non-blocking rationale.

## V312-14: Crash Recovery and Upgrade/Downgrade Verification

**Priority**: P0

**Goal**: Add the recovery evidence needed before SQLRustGo can be promoted beyond controlled production.

**Scope**:
- Run kill -9 / restart tests against mixed read/write workloads.
- Verify WAL replay, dirty page recovery, and audit-chain continuity.
- Back up and restore SQL data, GMP documents, embeddings, graph projection, and audit chain.
- Run v3.10.0/v3.11.0 fixture upgrade to v3.12.0 and rollback/downgrade where supported.
- Store count/hash reports for every recovery and upgrade path.

**Acceptance**:
- Recovery report shows count/hash equality after restart.
- Restore report shows count/hash equality in a clean data directory.
- Upgrade report shows old data readable under v3.12.0.
- Rollback limitations are explicitly documented if full downgrade is not supported.

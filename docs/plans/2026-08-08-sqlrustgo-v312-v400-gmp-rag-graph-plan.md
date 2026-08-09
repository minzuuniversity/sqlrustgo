# SQLRustGo v3.12.0 and v4.0.0 GMP/RAG/Graph Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build SQLRustGo into the database foundation for `~/gmp-platform`: first as a validated GMP internal-audit retrieval database in v3.12.0, then as a production multi-model SQL + vector + graph database in v4.0.0.

**Architecture:** v3.12.0 closes v3.11.0 production blockers and ships a controlled GMP/RAG profile using SQLRustGo as the relational store, vector index host, audit ledger, and graph projection source. v4.0.0 promotes vector and graph from internal/archived capabilities into first-class database subsystems with unified WAL, backup, access control, and query surfaces.

**Tech Stack:** Rust 2021, SQLRustGo workspace crates (`gmp`, `rag`, `vector`, `storage`, `mysql-server`, `security`, `admin`), GMP markdown corpus under `~/gmp-platform/gmp-md`, existing GMP prototypes using SQLite/Chroma/PostgreSQL as migration references, Ollama/BGE-M3-compatible embedding adapters, SQLRustGo governance gates.

---

## Evidence Baseline

- Current SQLRustGo v3.11.0 is RC / GA blocked: G3 coverage, G4 TPC-H SF=1, and v3.11.0 168h SOAK remain unresolved.
- `~/gmp-platform` contains a GMP document corpus and previous prototypes that use SQLite for persistence, Chroma for vector retrieval, and PostgreSQL/graph scripts for knowledge graph extraction.
- `crates/gmp`, `crates/rag`, and `crates/vector` are active workspace members; `graph` is archived under `archive/v3.11/archived-crates/graph`.
- GMP/RAG/graph production claims must be gated by execution evidence, not by crate existence.

## Product Split

| Version | Product Contract | Explicit Non-Goal |
|---|---|---|
| v3.12.0 | GMP internal-audit retrieval database: SQL + audited documents + internal vector retrieval + graph projection for evidence navigation | Do not claim general-purpose graph database or standalone vector database |
| v4.0.0 | Multi-model production database: SQL + first-class vector index + first-class graph store + GMP knowledge layer | Do not carry SQLite/Chroma/PostgreSQL as required production dependencies |

## v3.12.0 Scope

### P0 Release Preconditions

**Files:**
- Modify: `docs/releases/v3.11.0/GA_GATE_REPORT.md`
- Modify: `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md`
- Modify: `docs/governance/debt/debt-registry.yaml`
- Create: `docs/releases/v3.12.0/STAGE.yaml`
- Create: `docs/releases/v3.12.0/RELEASE_NOTES.md`
- Create: `docs/releases/v3.12.0/TEST_PLAN.md`

**Tasks:**
1. Close v3.11.0 G3 with reproducible per-crate coverage evidence.
2. Close v3.11.0 G4 with real TPC-H SF=1 fixture, 22/22 execution, and PostgreSQL checksum comparison.
3. Run v3.11.0 168h SOAK or explicitly demote v3.11.0 to non-production if not completed.
4. Reconcile `debt-registry.yaml` with v3.11.0 truth audit for F-25/F-26 and extension crate states.
5. Cut v3.12.0 only from a verified v3.11.0 baseline.

**Verification:**
- `bash scripts/gate/check_stage.sh --version v3.11.0`
- `bash scripts/gate/check_tpch_sf1.sh`
- `bash scripts/gate/check_coverage.sh`
- `bash scripts/gate/check_security.sh`
- `bash scripts/gate/check_docs_links.sh`
- `bash scripts/gate/check_docs_consistency.sh`

### P0 GMP Data Model

**Files:**
- Modify: `crates/gmp/src/document.rs`
- Modify: `crates/gmp/src/audit.rs`
- Modify: `crates/gmp/src/sql_api.rs`
- Create: `crates/gmp/src/schema_v312.rs`
- Create: `tests/gmp/gmp_schema_v312_test.rs`

**Schema Contract:**
- `gmp_documents`: document identity, title, type, version, status, source path, hash.
- `gmp_chunks`: stable chunk id, document id, section path, text, hash, token count.
- `gmp_embeddings`: chunk id, model, dimension, vector bytes, vector hash.
- `gmp_audit_log`: append-only event id, actor, action, target, timestamp, previous hash, event hash.
- `gmp_relations`: typed relation edges for document, clause, SOP, CAPA, deviation, equipment, role.

**Tests:**
1. Insert one GMP document and verify document/chunk/embedding/audit rows are linked.
2. Update document version and verify old version remains queryable.
3. Verify audit hash chain detects tampering.
4. Verify every search result can return source path, document version, chunk hash, and citation text.

### P0 GMP Corpus Ingestion

**Files:**
- Create: `crates/tools/src/bin/gmp_ingest.rs`
- Modify: `crates/tools/Cargo.toml`
- Create: `tests/gmp/gmp_ingest_corpus_test.rs`
- Create: `docs/releases/v3.12.0/GMP_CORPUS_INGESTION.md`

**Tasks:**
1. Ingest Markdown files from `~/gmp-platform/gmp-md`.
2. Preserve relative path, heading hierarchy, document code, version, effective date, and source hash.
3. Chunk by heading and paragraph with deterministic chunk ids.
4. Write import report with counts for documents, chunks, embeddings, relations, and failures.

**Acceptance:**
- Re-running ingestion is idempotent.
- Modified files create new versions instead of overwriting old content.
- Failed files are recorded with error reason and do not block valid files.

### P0 Vector Retrieval for RAG

**Files:**
- Modify: `crates/vector/src/lib.rs`
- Modify: `crates/gmp/src/vector_search.rs`
- Modify: `crates/gmp/src/semantic_embedding.rs`
- Create: `crates/gmp/src/hybrid_retrieval.rs`
- Create: `tests/gmp/gmp_hybrid_retrieval_test.rs`

**Tasks:**
1. Support embedding provider abstraction compatible with existing GMP BGE-M3/Ollama workflow.
2. Store embeddings in SQLRustGo-managed tables, not Chroma as a required dependency.
3. Use `sqlrustgo-vector` Flat/HNSW as the local ANN engine for internal GMP search.
4. Implement hybrid retrieval: exact SQL filters + keyword score + vector score + graph relation boost + RRF fusion.
5. Return evidence bundle for every result.

**Acceptance:**
- Top-k query returns deterministic ranked results for a fixed fixture.
- Search can filter by document type, chapter, status, and effective date.
- Vector index can rebuild from `gmp_embeddings`.
- Search results include source text and stable hash for audit.

### P0 Graph Projection for GMP Evidence Navigation

**Files:**
- Create: `crates/gmp/src/graph_projection.rs`
- Create: `crates/gmp/src/graph_query.rs`
- Create: `tests/gmp/gmp_graph_projection_test.rs`
- Do not restore `archive/v3.11/archived-crates/graph` as production dependency in v3.12.0 unless the dependency is fully revalidated.

**Tasks:**
1. Model graph as SQL-backed tables in v3.12.0: nodes and edges stored in SQLRustGo.
2. Create nodes for SOP, clause, role, equipment, deviation, CAPA, audit finding, document chunk.
3. Create edges such as `references`, `requires`, `evidence_for`, `corrects`, `owned_by`, `supersedes`.
4. Provide limited graph APIs: neighbors, path up to depth 3, relation-filtered expansion.
5. Use graph output as retrieval context, not as a full Cypher database.

**Acceptance:**
- Given a deviation, return related SOPs, CAPA procedures, GMP clauses, and previous similar records.
- Given a SOP chunk, return upstream法规条款 and downstream audit checklist items.
- Graph traversal results are permission-filtered and audit-logged.

### P0 GMP Compliance and Data Integrity

**Files:**
- Modify: `crates/security`
- Modify: `crates/gmp/src/compliance.rs`
- Modify: `crates/gmp/src/audit.rs`
- Create: `tests/gmp/gmp_data_integrity_test.rs`
- Create: `docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md`

**Controls:**
- ALCOA+: attributable, legible, contemporaneous, original, accurate, complete, consistent, enduring, available.
- Role-based access for document import, approval, search, export, audit review.
- Electronic signature hooks for approval and controlled export.
- Immutable audit hash chain.
- Backup/restore with audit continuity check.

**Acceptance:**
- Unauthorized user cannot read restricted document chunks.
- Audit export verifies hash chain end-to-end.
- Backup/restore preserves document versions, embeddings, graph edges, and audit chain.

### P1 v3.12.0 Production Gates

**Files:**
- Create: `scripts/gate/check_gmp_v312.sh`
- Create: `scripts/gate/check_gmp_retrieval_quality.sh`
- Create: `docs/releases/v3.12.0/GA_GATE_REPORT.md`

**Gate Requirements:**
- SQL core: build, clippy, fmt, tests all pass.
- GMP ingestion: full `~/gmp-platform/gmp-md` corpus ingests with zero unclassified failures.
- Retrieval quality: fixed question set reaches target hit rate and citation correctness.
- Audit integrity: hash-chain tamper tests fail closed.
- Vector rebuild: drop/rebuild index yields equivalent top-k within tolerance.
- Graph projection: deterministic node/edge counts and path queries.
- SOAK: 168h mixed workload with SQL + ingest + retrieval + audit + backup/restore.

## v4.0.0 Scope

### P0 First-Class Vector Database

**Files:**
- Modify: `crates/vector`
- Modify: `crates/storage`
- Modify: `crates/parser`
- Modify: `crates/executor`
- Create: `tests/vector/vector_sql_e2e_test.rs`

**Contract:**
- SQL syntax for vector columns and indexes.
- `CREATE VECTOR INDEX` with Flat, HNSW, IVF.
- Online vector index rebuild.
- Metadata filtering before/after ANN search.
- WAL-backed vector insert/delete/update.
- Backup/restore of vector tables and indexes.

### P0 First-Class Graph Database

**Files:**
- Restore or rewrite from: `archive/v3.11/archived-crates/graph`
- Create: `crates/graph`
- Modify: `crates/parser`
- Modify: `crates/executor`
- Create: `tests/graph/graph_store_e2e_test.rs`

**Contract:**
- Property graph node/edge storage.
- SQL-accessible graph tables plus a limited graph query layer.
- Typed edges and indexed traversal.
- WAL, backup/restore, permissions, audit.
- Optional Cypher subset only after storage and query semantics are verified.

### P0 Unified Multi-Model Storage

**Tasks:**
1. One transaction model for SQL rows, vector records, graph nodes/edges, and GMP audit events.
2. One backup/restore format.
3. One permission system.
4. One observability surface.
5. One consistency checker.

**Gates:**
- Cross-model transaction rollback test.
- Crash during mixed SQL/vector/graph write.
- Restore and verify all indexes.
- 168h mixed SOAK.

## Recommended Milestones

| Milestone | Target | Exit Criteria |
|---|---|---|
| v3.12.0-alpha | GMP schema + ingestion | Corpus imports, audit chain works, no production claim |
| v3.12.0-beta | Hybrid retrieval | SQL + vector + graph projection results with citations |
| v3.12.0-rc | Compliance hardening | permissions, signatures, backup/restore, quality gates |
| v3.12.0-ga | GMP internal-audit production | 168h mixed SOAK, retrieval quality report, security PASS |
| v4.0.0-alpha | Vector DB | vector SQL and WAL-backed index prototype |
| v4.0.0-beta | Graph DB | graph storage and traversal prototype |
| v4.0.0-rc | Multi-model consistency | cross-model transaction and recovery gates |
| v4.0.0-ga | Multi-model production | SQL + vector + graph + GMP gates all PASS |

## Release Decision Rule

- v3.12.0 may claim: "GMP internal-audit retrieval database with internal vector retrieval and graph projection."
- v3.12.0 must not claim: "general-purpose vector database" or "general-purpose graph database."
- v4.0.0 may claim multi-model production only after vector and graph are first-class, WAL-backed, permissioned, backed up, restored, and SOAK-tested.

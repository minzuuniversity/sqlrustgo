# SQLRustGo v3.12.0 Development Plan

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-09
> **Product target**: GMP internal-audit retrieval database for `~/gmp-platform`
> **Truthfulness rule**: This plan defines future work and exit evidence. It does not claim any v3.12.0 gate has passed.

## 1. Planning Evidence

| Source | Evidence | Use |
|---|---|---|
| SQLRustGo local checkout | `develop/v3.11.0` at `9f469a7ebde7b451d1ddbc5fc9f0a393d1d7810b` | Current planning baseline after syncing 252 Gitea v3.11.0 GA assessment |
| GMP-Platform 250 checkout | `develop/v1.4.0` at `c0f366d59e7d` | Integration findings baseline |
| GMP-Platform integration plan | commit `556a5f104900` | Cross-project implementation direction |
| SQLRustGo roadmap plan | `docs/plans/2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md` | v3.12/v4.0 scope split |
| v3.11.0 comprehensive assessment | `docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | GA evidence boundary, weak-point hardening, missing-test list |
| v3.10.0 testing-system report | `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` | SQLLogicTest / SQLite official corpus plan, Issue #3373 |
| v3.10.0 issues plan | `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` | V310-14 SQLLogicTest integration status and known download gap |
| Existing SLT runner | `crates/sqlrustgo_sqllogictest` | Runner exists with 22 local `.test` files; not yet a strict v3.11 GA gate |

Claim metadata for this document:

| Field | Value |
|---|---|
| source_agent | Codex |
| source_run | `codex-sqlrustgo-v312-plan-2026-08-09` |
| timestamp | `2026-08-09 13:35:00 CST` |
| evidence_hash | `local-git:9f469a7ebde7b451d1ddbc5fc9f0a393d1d7810b`; implemented gates must generate their own evidence hashes |
| conflict_resolution | Local SQLRustGo docs are the release SSOT; GMP-Platform findings are treated as integration inputs |

## 2. Current Findings Driving v3.12.0

v3.12.0 is planned because SQLRustGo v3.11.0 has useful database foundations, but the current GMP-Platform production path still needs a tighter database contract before SQLRustGo can safely become its primary database, vector store, and graph projection store.

| Finding | Impact on v3.12.0 |
|---|---|
| GMP-Platform already depends on `sqlrustgo-storage`, `sqlrustgo-types`, `sqlrustgo-vector`, and `sqlrustgo-graph` | v3.12 must stabilize a supported GMP-facing kernel contract instead of relying on ad hoc crate usage |
| `gmp-eval` uses its own hybrid search path instead of one shared retrieval engine | v3.12 must provide one auditable query contract that GMP CLI, API, and eval can call consistently |
| LIKE fallback is still part of the evaluation behavior, and disabling it is not reliably enforced | v3.12 must make LIKE diagnostic-only and add a no-LIKE production gate |
| Vector retrieval has reported empty-index and dimension-drift risks | v3.12 must enforce model name, dimension, vector hash, and rebuildable index invariants |
| Graph capability exists but is not active in the main evaluation/retrieval path | v3.12 must deliver SQL-backed GMP evidence graph projection, not a general graph database claim |
| RAG output needs stronger citation and audit evidence | v3.12 must make evidence bundles mandatory for every retrieval result and generated answer |
| v3.11.0 GA assessment found G3/G4 evidence-boundary risks | v3.12 must close TPC-H correctness, coverage-methodology, wire protocol, LOAD DATA, recovery, and upgrade/downgrade gaps before any broader production claim |
| v3.10.0 planned SQLLogicTest against SQLite official corpus, and v3.11.0 listed `sqllogictest runner all targets PASS`, but the gate was still TBD | v3.12 must promote SQLLogicTest/SQLite oracle testing to a real P0 gate with command output and baseline artifacts |
| Existing gate checks refer to both `crates/sqllogictest` and `crates/sqlrustgo_sqllogictest` | v3.12 must reconcile the runner path and package name so checks target the actual crate |

## 3. v3.12.0 Contract

Allowed v3.12.0 product claim after gates produce execution evidence:

> SQLRustGo v3.12.0 supports controlled GMP internal-audit retrieval workloads with SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles.

Disallowed v3.12.0 claims:

- General-purpose MySQL 5.7 replacement.
- General-purpose standalone vector database.
- General-purpose graph database.
- Production readiness without v3.11.0 weak-point disposition.
- PASS, GA, or compliance claims without command output, timestamp, source agent, source run, evidence hash, and output location.

## 4. Architecture Scope

v3.12.0 should expose a GMP kernel that GMP-Platform can use without duplicating retrieval logic.

```text
GMP-Platform CLI/API/Eval
        |
        v
SQLRustGo GMP Kernel Contract
        |
        +-- relational document/chunk/version/audit tables
        +-- keyword retrieval with exact match and tokenizer controls
        +-- vector persistence and rebuildable vector index
        +-- SQL-backed relation graph projection
        +-- RRF fusion and evidence bundle generation
        +-- backup/restore and audit-chain verification
```

The SQLRustGo side owns storage correctness, indexing contracts, query evidence, and gate scripts. GMP-Platform owns business workflows, UI/API orchestration, document review processes, and domain-specific prompts.

## 4.1 v3.11.0 Weak-Point Hardening Scope

v3.12.0 has two equal P0 tracks:

| Track | Purpose | Exit boundary |
|---|---|---|
| Track A: Production hardening from v3.11.0 | Close or explicitly carry v3.11.0 weak points: TPC-H correctness, coverage drift, wire protocol, LOAD DATA, crash recovery, backup/restore, upgrade/downgrade, dependency audit | No hidden v3.11.0 P0 blocker remains |
| Track B: GMP/RAG/Graph kernel | Deliver the controlled GMP internal-audit retrieval database contract for `~/gmp-platform` | GMP-Platform can run SQLRustGo-backed retrieval with evidence bundles |

The two tracks must not be traded off against each other. GMP features cannot justify skipping SQL correctness gates, and SQL correctness gates cannot be replaced by GMP-only fixtures.

## 4.2 SQLite SQLLogicTest Reinstatement

The SQLite-based automatic testing plan is re-adopted as a v3.12.0 P0 quality gate.

| Item | Current evidence | v3.12.0 action |
|---|---|---|
| Historical plan | `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` describes SQLLogicTest using SQLite official `.test` files, about 623 files / 5.9M cases | Carry forward as V312-11 |
| Issue plan | `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-14 marks runner implemented but SQLite official suite download blocked | Complete testdata acquisition or define a reproducible mirror/cache |
| Existing runner | `crates/sqlrustgo_sqllogictest` exists with 22 local `.test` files | Make package build/run part of gate |
| v3.11.0 gap | `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` lists sqllogictest runner all targets as TBD | Promote from TBD to required v3.12 Alpha/Beta/RC/GA staged thresholds |
| Path drift | `scripts/gate/check_beta_gate.sh` checks both `crates/sqllogictest` and `crates/sqlrustgo_sqllogictest` | Normalize checks to `crates/sqlrustgo_sqllogictest` or document the rename |

Current local baseline on 2026-08-09:

| Check | Result | Required v3.12.0 follow-up |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | Succeeds, but dependency warnings remain in `storage` and `executor` | Keep build as Alpha evidence, but do not claim warning-free/clippy-clean until `cargo clippy --all-features -- -D warnings` passes |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | Runner completes; smoke corpus result is 6/16 files passing, 27.3% pass rate | Treat as a failing baseline to triage, not as a release gate PASS |
| SQLite official SQLLogicTest corpus | Not integrated into the current gate system | Add corpus manifest, cache/mirror procedure, selected-target definition, and exclusion registry |

## 5. Work Packages

### V312-01: Prior-Version Blocker Disposition

**Priority**: P0

**Goal**: Prevent v3.12.0 from inheriting unverified v3.11.0 claims.

**Implementation**:
- Produce a v3.11.0 weak-point disposition report for G3 coverage, G4 TPC-H SF=1, 168h SOAK, sqllogictest gate drift, and debt-registry drift.
- Decide whether each blocker is closed with evidence or carried as an explicit v3.12.0 P0 blocker.
- Ensure release docs do not describe v3.11.0 or v3.12.0 as GA without gate evidence.
- Treat the following as explicit v3.12 hardening inputs: TPC-H zero-row correctness, wire protocol strict path, LOAD DATA, coverage methodology, crash recovery, backup/restore, upgrade/downgrade, dependency audit refresh.

**Exit evidence**:
- Blocker report with command outputs and evidence hashes.
- Updated `STAGE.yaml` references.
- No hidden P0 blocker in `docs/releases/v3.12.0/STAGE.yaml`.
- SQLLogicTest disposition table: runner exists, testdata count, smoke pass, official corpus status.

### V312-02: GMP Kernel Contract

**Priority**: P0

**Goal**: Provide one supported SQLRustGo-facing contract for GMP-Platform.

**Implementation**:
- Define stable request/response structs for ingestion, retrieval, citation bundles, audit events, and graph traversal.
- Keep the contract narrow enough for v3.12.0: GMP internal-audit retrieval only.
- Return score components for keyword, vector, graph relation boost, and final RRF rank.

**Exit evidence**:
- Contract tests for serialization, error handling, and required evidence fields.
- GMP-Platform adapter test showing CLI/API/eval can call the same query path.

### V312-03: GMP Schema and Corpus Ingestion

**Priority**: P0

**Goal**: Store GMP documents, chunks, versions, embeddings, audit rows, and relations in SQLRustGo-managed data.

**Implementation**:
- Add idempotent schema creation for document metadata, chunk text, document versions, embedding metadata, audit events, and relation edges.
- Import `~/gmp-platform/gmp-md` with deterministic document and chunk IDs.
- Preserve source path, source hash, document version, effective status, and chunk hash.
- Classify skipped files and failures; unclassified ingestion failures block promotion.

**Exit evidence**:
- Full corpus ingestion report with document, chunk, embedding, relation, skipped, and failed counts.
- Re-ingestion test proving unchanged files do not create duplicate active records.
- Modified-file test proving a new document version is created without overwriting the original.

### V312-04: Keyword Retrieval Without Production LIKE Fallback

**Priority**: P0

**Goal**: Replace LIKE-driven scoring in the production retrieval path with auditable keyword retrieval.

**Implementation**:
- Define tokenizer and exact-match behavior for Chinese GMP terms, SOP numbers, clause IDs, deviation IDs, CAPA IDs, equipment IDs, and role names.
- Make LIKE fallback a diagnostic mode only.
- Add a production gate that fails when the GMP production query profile uses LIKE fallback scoring.

**Exit evidence**:
- Fixed keyword fixture with deterministic top-k results.
- No-LIKE production gate output.
- Query trace showing keyword terms, matched chunks, and score contribution.

### V312-05: Vector Persistence and Rebuildable Index

**Priority**: P0

**Goal**: Make vector retrieval reliable enough for GMP RAG retrieval.

**Implementation**:
- Persist embedding model, dimension, provider, chunk ID, vector hash, created timestamp, and active flag.
- Enforce dimension mismatch as a hard error.
- Support the GMP default embedding profile as `bge-m3`/1024d unless the deployment config explicitly selects another validated profile.
- Rebuild Flat or HNSW index from SQLRustGo-managed vectors.
- Treat an empty vector index as a gate failure for GMP production profile.

**Exit evidence**:
- Dimension mismatch test fails closed.
- Index rebuild test reproduces vector count and top-k fixture.
- GMP retrieval fixture proves vector is an active RRF channel, not only a fallback.

### V312-06: SQL-Backed GMP Evidence Graph

**Priority**: P0

**Goal**: Support GMP evidence navigation without claiming v3.12.0 is a general graph database.

**Implementation**:
- Store nodes and edges for documents, clauses, chunks, SOPs, deviations, CAPA items, equipment, roles, and audit findings.
- Support neighbors and depth-limited path queries up to depth 3.
- Support relation filters and evidence bundle output for every returned path.
- Keep Neo4j and the archived graph crate out of the required v3.12.0 production path unless separately revalidated.

**Exit evidence**:
- Node/edge count tests from representative corpus.
- Depth-limited path tests with deterministic results.
- Retrieval trace showing graph relation boost as an independent signal.

### V312-07: Hybrid Retrieval and RAG Evidence Bundle

**Priority**: P0

**Goal**: Provide the retrieval foundation GMP-Platform needs for internal-audit RAG.

**Implementation**:
- Fuse keyword, vector, and graph scores through traceable RRF.
- Return source path, document ID, version, chunk ID, chunk hash, citation text, score components, and access-control decision for every result.
- Provide a RAG answer context envelope that contains only cited chunks and carries evidence hashes forward.
- Ensure generated answers cannot hide missing citations.

**Exit evidence**:
- GMP audit question fixture report.
- Every returned answer/result has a citation bundle.
- Missing-citation test fails closed.

### V312-08: Compliance, Audit Trail, and Access Control

**Priority**: P0

**Goal**: Map SQLRustGo behavior to GMP/ALCOA+ controls for internal-audit retrieval.

**Implementation**:
- Add tamper-evident audit hash chain for import, approve, search, export, backup, restore, and evidence review events.
- Add role-based checks for import, approve, search, export, and audit review.
- Add electronic-signature hooks for controlled approval and export events.
- Update the GMP compliance matrix only when tests exist.

**Exit evidence**:
- Audit-chain tamper test fails closed.
- ACL denial and allow tests.
- Compliance matrix rows reference test evidence rather than design intent.

### V312-09: Backup, Restore, and Mixed SOAK

**Priority**: P0

**Goal**: Prove the GMP/RAG/graph store can survive routine operations.

**Implementation**:
- Back up relational data, document/chunk text, embedding metadata, vector payloads, graph projection, and audit chain.
- Restore into a clean SQLRustGo data directory and verify counts and hashes.
- Run a mixed workload that includes SQL reads/writes, ingestion, retrieval, graph traversal, audit export, and backup/restore smoke.

**Exit evidence**:
- Restore report with count and hash equality.
- Vector index rebuild after restore.
- 168h mixed SOAK report before any GA claim.

### V312-10: GMP-Platform Integration Verification

**Priority**: P0

**Goal**: Ensure SQLRustGo v3.12.0 is useful to GMP-Platform as deployed, not only internally tested.

**Implementation**:
- Provide a local integration profile for `~/gmp-platform` that uses SQLRustGo for storage, vector retrieval, and graph projection.
- Route GMP-Platform eval through the shared SQLRustGo-backed query contract.
- Ensure `--no-like-fallback` produces a real no-LIKE execution path.
- Produce cross-repository verification output without merging unsupported production claims into either repo.

**Exit evidence**:
- GMP-Platform eval report using SQLRustGo-backed retrieval.
- Trace proving keyword, vector, and graph channels were active or explicitly disabled by config.
- No-LIKE fallback enforcement output.

### V312-11: SQLite SQLLogicTest Oracle Gate

**Priority**: P0

**Goal**: Restore and complete the SQLite automatic testing plan that was introduced in v3.10.0 and left non-blocking in v3.11.0.

**Implementation**:
- Normalize the canonical runner path to `crates/sqlrustgo_sqllogictest` and package name `sqlrustgo_sqllogictest`.
- Preserve the current 22 local `.test` files as the smoke corpus.
- Add a reproducible acquisition path for the SQLite official SQLLogicTest corpus, or a checked-in manifest that records the exact upstream snapshot, source URL, file count, hash list, excluded tests, and exclusion reasons.
- Define staged thresholds:
  - Alpha: runner builds and `--help` works.
  - Beta: smoke corpus runs and produces a machine-readable report.
  - RC: curated SQLite-compatible subset runs with deterministic pass/fail/skip classification.
  - GA: all selected v3.12 SLT targets pass or have documented, issue-linked exclusions.
- Save reports under `docs/releases/v3.12.0/sqllogictest-baseline/`.
- Integrate the gate into `scripts/gate/check_beta_gate.sh` successor logic or a dedicated `scripts/gate/check_sqllogictest_v312.sh`.

**Exit evidence**:
- `cargo build -p sqlrustgo_sqllogictest` output.
- `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` output for the smoke corpus.
- Testdata manifest with file count, upstream snapshot, skipped features, and evidence hash.
- Gate script output showing PASS/FAIL with exit code.

## 6. Milestones

| Milestone | Scope | Exit criteria |
|---|---|---|
| Alpha | Blocker disposition, schema, ingestion skeleton, kernel contract | Build/fmt/clippy evidence plus schema and contract tests |
| Beta | Keyword, vector, graph, hybrid retrieval, sqllogictest smoke corpus | Fixed fixtures produce deterministic retrieval and evidence bundles; SLT smoke report exists |
| RC | Compliance, backup/restore, integration verification, curated SQLite SLT subset | Audit, ACL, restore, no-LIKE, GMP-Platform eval, and SLT subset evidence exists |
| GA | Controlled GMP internal-audit production readiness | All v3.12.0 gates pass with evidence, 168h mixed SOAK complete, no unresolved P0 blocker, SLT selected targets pass or are issue-linked exclusions |

## 7. Test and Gate Commands

The exact package names may be adjusted during implementation, but every command in a release gate must execute real checks and save logs under `docs/releases/v3.12.0/logs/`.

```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --all-features
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_consistency.sh
bash scripts/gate/check_gmp_v312.sh
bash scripts/gate/check_tpch_sf1.sh --sf1-dir "$SF1_DIR"
bash scripts/gate/check_sqllogictest_v312.sh
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
```

Planned GMP-specific checks:

```bash
cargo test --all-features gmp_schema_contract
cargo test --all-features gmp_ingestion_contract
cargo test --all-features gmp_keyword_no_like_contract
cargo test --all-features gmp_vector_dimension_contract
cargo test --all-features gmp_graph_projection_contract
cargo test --all-features gmp_audit_chain_contract
```

Planned cross-repository verification from GMP-Platform:

```bash
cargo run --release -p gmp-eval -- \
  --limit 408 \
  --no-like-fallback \
  --output reports/sqlrustgo-v312-eval.json
```

## 8. v4.0.0 Deferrals

These items are intentionally not v3.12.0 goals:

- General MySQL 5.7 replacement claim.
- Public standalone vector database product contract.
- Public standalone graph database product contract.
- Distributed HA or multi-tenant service guarantees.
- General Cypher compatibility.
- GMP-independent enterprise RAG platform claim.

v4.0.0 should promote the validated v3.12.0 GMP internals into first-class database, vector database, and graph database contracts only after v3.12.0 evidence exists.

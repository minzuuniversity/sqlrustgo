# Changelog -- SQLRustGo v3.12.0

> **Status**: PLANNED
> **Date**: 2026-08-09

## v3.12.0-planned

Initial planning entry for the GMP internal-audit retrieval release.

## 2026-08-09 Planning Update

Updated v3.12.0 as a dual-track release:

- Track A: close v3.11.0 weak points before broader production claims.
- Track B: deliver the GMP internal-audit retrieval database contract for `~/gmp-platform`.

Added P0 hardening scope from the v3.11.0 comprehensive assessment:

- TPC-H SF=1 cross-engine row-count and SHA256 correctness.
- Coverage methodology reconciliation and single G3 command.
- MySQL wire protocol e2e hardening.
- `LOAD DATA` / bulk import benchmark and count/hash validation.
- Crash recovery, backup/restore, and upgrade/downgrade evidence.
- Dependency audit refresh.
- SQLite SQLLogicTest oracle gate.

SQLLogicTest context:

- v3.10.0 planned SQLite official SQLLogicTest integration in `TESTING_SYSTEM_BETA_REPORT.md` and V310-14.
- v3.11.0 kept `sqllogictest runner all targets PASS` in stage requirements, but `RELEASE_GATE_CHECKLIST.md` still marked it TBD.
- v3.12.0 promotes `crates/sqlrustgo_sqllogictest` to an explicit gate with build/run output, corpus manifest, exclusion registry, and baseline reports.
- 2026-08-09 local baseline: `cargo build -p sqlrustgo_sqllogictest` completes with dependency warnings; local smoke corpus runs but currently reports 6/16 files passing and 27.3% pass rate, so this is a failing baseline rather than a gate PASS.

### Planned

- GMP document, chunk, embedding, audit, and relation schema.
- Idempotent ingestion from `~/gmp-platform/gmp-md`.
- SQLRustGo-managed embedding persistence and vector index rebuild.
- Hybrid retrieval with exact SQL filters, keyword score, vector score, graph relation boost, and RRF.
- SQL-backed graph projection for GMP evidence navigation.
- ALCOA+ compliance matrix, role-based access checks, and audit hash-chain tamper tests.
- Backup/restore verification for GMP/RAG/graph projection state.
- 168h mixed SOAK covering SQL, ingestion, retrieval, audit, and backup/restore.
- SQLite SQLLogicTest smoke and curated corpus gates.
- TPC-H correctness, MySQL wire, LOAD DATA, recovery, and upgrade gates.

### Not Planned For v3.12.0

- General-purpose vector database claim.
- General-purpose graph database claim.
- Cypher compatibility claim.
- Distributed HA claim.

## Version History

| Version | Date | Stage | Notes |
|---|---|---|---|
| v3.12.0 | TBD | PLANNED | GMP internal-audit retrieval database |

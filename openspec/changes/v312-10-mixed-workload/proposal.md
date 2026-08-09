# Proposal: V312-10 Mixed Workload SOAK

## Why

V312-10 provides soak test infrastructure for validating the complete GMP system under mixed workload (SQL, ingestion, retrieval, audit, backup). Verifies audit chain integrity and crash safety.

## What Changes

- `soak.rs`: New module — `WorkloadOp`, `SoakConfig`, `SoakStats`, `run_soak_test`, `verify_retrieval_quality`
- 9 operation types: DocumentInsert, ChunkInsert, VersionInsert, RetrievalQuery, AuditQuery, AuditRecord, BackupCreate, BackupVerify, GraphProject
- Deterministic round-robin scheduling
- Periodic audit chain verification (configurable interval)
- crash_safe mode: halt on first error

## Capabilities

### New Capabilities

- `gmp-soak-test`: Mixed workload soak test with crash safety and audit chain verification

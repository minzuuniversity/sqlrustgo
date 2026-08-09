# GMP Mixed Workload SOAK

## ADDED Requirements

### Requirement: Soak test covers all GMP operations

`run_soak_test` executes 9 operation types: DocumentInsert, ChunkInsert, VersionInsert, RetrievalQuery, AuditQuery, AuditRecord, BackupCreate, BackupVerify, GraphProject.

#### Scenario: Round-robin scheduling
- **WHEN** soak runs 9 operations
- **THEN** each operation type is executed exactly once in order

### Requirement: crash_safe halts on error

When `crash_safe=true` and an operation fails, soak halts and records crashes.

#### Scenario: Crash detection
- **WHEN** an operation returns Err with crash_safe=true
- **THEN** stats.crashes is incremented and loop breaks

### Requirement: Periodic audit chain verification

Every `audit_verify_interval` operations, `verify_audit_chain` is called.

#### Scenario: Audit verification
- **WHEN** audit_verify_interval=5 and 5 ops run
- **THEN** verify_audit_chain is called once

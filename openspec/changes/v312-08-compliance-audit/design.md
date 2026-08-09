# Design: V312-08 Compliance & Audit Trail

## Context

GMP requires complete audit coverage. V312-08 complements the existing audit.rs hash chain with explicit ACL for all GMP operations.

## Decisions

### Decision: 5 roles with explicit permission matrix

Explicit is better than implicit. Each role has a documented list of allowed operations rather than a generic hierarchy.

### Decision: PermissionGuard for fail-closed checks

`guard.check()` returns `Err` on denial — callers must handle unauthorized access explicitly.

### Decision: AccessAuditRecord logs all decisions

Both allowed and denied access attempts are logged with full provenance for forensic analysis.

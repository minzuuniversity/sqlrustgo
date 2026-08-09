# Proposal: V312-08 Compliance & Audit Trail

## Why

GMP operations (import, search, export, approve, backup, restore, review) must all be tracked in the audit hash chain. V312-08 adds ACL with role-based permissions covering SQL, vector, graph, and retrieval operations.

## What Changes

- `acl.rs`: New module — `GmpOperation`, `GmpRole`, `AclContext`, `PermissionGuard`, `AccessAuditRecord`
- 5 roles: Admin, Auditor, Editor, Viewer, BackupOperator
- 12 operation types with role-based permission matrix
- `PermissionGuard` for fail-closed authorization checks
- `AccessAuditRecord` for logging access decisions

## Capabilities

### New Capabilities

- `gmp-access-control`: Role-based ACL covering all GMP operations
- `gmp-access-audit`: Every access decision logged with user/role/operation/result

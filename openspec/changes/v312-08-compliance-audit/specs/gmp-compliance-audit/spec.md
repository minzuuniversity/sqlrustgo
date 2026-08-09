# GMP Compliance & Audit Trail

## ADDED Requirements

### Requirement: Admin role has all permissions

`AclContext::new("admin", GmpRole::Admin).can(op)` returns `true` for all `GmpOperation` variants.

#### Scenario: Admin can do anything
- **WHEN** admin context checks `DocumentImport`, `AuditQuery`, `BackupRestore`
- **THEN** all return `true`

### Requirement: Viewer role limited to search and review

`AclContext::new("viewer", GmpRole::Viewer)` can only `RetrievalSearch` and `DocumentReview`.

#### Scenario: Viewer permissions
- **WHEN** viewer context checks `RetrievalSearch` and `DocumentImport`
- **THEN** `RetrievalSearch` = true, `DocumentImport` = false

### Requirement: BackupOperator role only backup operations

`AclContext::new("backup", GmpRole::BackupOperator)` can only `BackupCreate` and `BackupRestore`.

#### Scenario: Backup operator
- **WHEN** backup operator checks backup and import operations
- **THEN** `BackupCreate` = true, `DocumentImport` = false

### Requirement: PermissionGuard fails closed

`PermissionGuard::new(ctx, op).check()` returns `Err` when `ctx` lacks permission.

#### Scenario: Unauthorized access blocked
- **WHEN** `PermissionGuard::new(viewer_ctx, BackupRestore).check()` is called
- **THEN** it returns `Err("Role Viewer is not authorized for BackupRestore")`

### Requirement: AccessAuditRecord captures all decisions

`AccessAuditRecord::new` records user_id, role, operation, decision, reason, timestamp.

#### Scenario: Audit record on denial
- **WHEN** denied access occurs
- **THEN** record.decision = "DENIED" and record.reason is Some string

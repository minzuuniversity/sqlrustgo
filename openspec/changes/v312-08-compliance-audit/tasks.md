# V312-08 Compliance & Audit Trail — Implementation Tasks

## 1. Implementation

- [x] 1.1 `acl.rs` — `GmpOperation` (12 operation types), `GmpRole` (5 roles)
- [x] 1.2 `acl.rs` — `role_permissions` mapping for each role
- [x] 1.3 `acl.rs` — `AclContext` with user_id, role, session_id, ip_address
- [x] 1.4 `acl.rs` — `PermissionGuard` with fail-closed `check()`
- [x] 1.5 `acl.rs` — `AccessAuditRecord` for logging all decisions
- [x] 1.6 `lib.rs` — Added `pub mod acl;`

## 2. Tests

- [x] 2.1 `test_admin_has_all_permissions`
- [x] 2.2 `test_viewer_limited_permissions`
- [x] 2.3 `test_backup_operator_only_backup`
- [x] 2.4 `test_auditor_can_query_audit`
- [x] 2.5 `test_editor_can_import`
- [x] 2.6 `test_require_returns_error_on_denial`
- [x] 2.7 `test_permission_guard_fail_closed`
- [x] 2.8 `test_access_audit_record_allowed/denied`
- [x] 2.9 All 142 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-08-compliance-audit` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks

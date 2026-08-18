//! GMP Access Control List
//!
//! Provides role-based access control for GMP operations.
//! Covers SQL, vector, graph, and retrieval operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GMP operation types that require authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GmpOperation {
    SqlQuery,
    VectorSearch,
    GraphProjection,
    RetrievalSearch,
    DocumentImport,
    DocumentExport,
    DocumentApprove,
    DocumentReview,
    BackupCreate,
    BackupRestore,
    AuditQuery,
}

impl GmpOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            GmpOperation::SqlQuery => "SQL_QUERY",
            GmpOperation::VectorSearch => "VECTOR_SEARCH",
            GmpOperation::GraphProjection => "GRAPH_PROJECTION",
            GmpOperation::RetrievalSearch => "RETRIEVAL_SEARCH",
            GmpOperation::DocumentImport => "DOCUMENT_IMPORT",
            GmpOperation::DocumentExport => "DOCUMENT_EXPORT",
            GmpOperation::DocumentApprove => "DOCUMENT_APPROVE",
            GmpOperation::DocumentReview => "DOCUMENT_REVIEW",
            GmpOperation::BackupCreate => "BACKUP_CREATE",
            GmpOperation::BackupRestore => "BACKUP_RESTORE",
            GmpOperation::AuditQuery => "AUDIT_QUERY",
        }
    }
}

/// GMP role definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GmpRole {
    Admin,
    Auditor,
    Editor,
    Viewer,
    BackupOperator,
}

impl GmpRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            GmpRole::Admin => "ADMIN",
            GmpRole::Auditor => "AUDITOR",
            GmpRole::Editor => "EDITOR",
            GmpRole::Viewer => "VIEWER",
            GmpRole::BackupOperator => "BACKUP_OPERATOR",
        }
    }
}

/// Permission mapping from role to allowed operations.
pub fn role_permissions(role: GmpRole) -> Vec<GmpOperation> {
    match role {
        GmpRole::Admin => vec![
            GmpOperation::SqlQuery,
            GmpOperation::VectorSearch,
            GmpOperation::GraphProjection,
            GmpOperation::RetrievalSearch,
            GmpOperation::DocumentImport,
            GmpOperation::DocumentExport,
            GmpOperation::DocumentApprove,
            GmpOperation::DocumentReview,
            GmpOperation::BackupCreate,
            GmpOperation::BackupRestore,
            GmpOperation::AuditQuery,
        ],
        GmpRole::Auditor => vec![
            GmpOperation::SqlQuery,
            GmpOperation::VectorSearch,
            GmpOperation::GraphProjection,
            GmpOperation::RetrievalSearch,
            GmpOperation::DocumentReview,
            GmpOperation::AuditQuery,
        ],
        GmpRole::Editor => vec![
            GmpOperation::SqlQuery,
            GmpOperation::VectorSearch,
            GmpOperation::GraphProjection,
            GmpOperation::RetrievalSearch,
            GmpOperation::DocumentImport,
            GmpOperation::DocumentExport,
            GmpOperation::DocumentReview,
        ],
        GmpRole::Viewer => vec![GmpOperation::RetrievalSearch, GmpOperation::DocumentReview],
        GmpRole::BackupOperator => vec![GmpOperation::BackupCreate, GmpOperation::BackupRestore],
    }
}

/// Access control decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessDecision {
    Allowed,
    Denied { reason: String },
}

/// Check if a role is allowed to perform an operation.
pub fn check_permission(role: GmpRole, op: GmpOperation) -> AccessDecision {
    let allowed = role_permissions(role);
    if allowed.contains(&op) {
        AccessDecision::Allowed
    } else {
        AccessDecision::Denied {
            reason: format!("Role {:?} is not authorized for {:?}", role, op),
        }
    }
}

/// ACL context with user identity.
#[derive(Debug, Clone)]
pub struct AclContext {
    pub user_id: String,
    pub role: GmpRole,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
}

impl AclContext {
    pub fn new(user_id: &str, role: GmpRole) -> Self {
        Self {
            user_id: user_id.to_string(),
            role,
            session_id: None,
            ip_address: None,
        }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_ip(mut self, ip: &str) -> Self {
        self.ip_address = Some(ip.to_string());
        self
    }

    /// Check if this context can perform the given operation.
    pub fn can(&self, op: GmpOperation) -> bool {
        matches!(check_permission(self.role, op), AccessDecision::Allowed)
    }

    /// Require permission or return Err.
    pub fn require(&self, op: GmpOperation) -> Result<(), String> {
        match check_permission(self.role, op) {
            AccessDecision::Allowed => Ok(()),
            AccessDecision::Denied { reason } => Err(reason),
        }
    }
}

/// A permission requirement that fails closed on unauthorized access.
pub struct PermissionGuard<'a> {
    context: &'a AclContext,
    operation: GmpOperation,
}

impl<'a> PermissionGuard<'a> {
    pub fn new(context: &'a AclContext, operation: GmpOperation) -> Self {
        Self { context, operation }
    }

    /// Check and return error if denied. Fail-closed.
    pub fn check(&self) -> Result<(), String> {
        self.context.require(self.operation)
    }
}

/// Record of an access control check for auditing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessAuditRecord {
    pub user_id: String,
    pub role: String,
    pub operation: String,
    pub decision: String,
    pub reason: Option<String>,
    pub timestamp: i64,
    pub ip_address: Option<String>,
    pub session_id: Option<String>,
}

impl AccessAuditRecord {
    pub fn new(context: &AclContext, op: GmpOperation, decision: &AccessDecision) -> Self {
        let (decision_str, reason) = match decision {
            AccessDecision::Allowed => ("ALLOWED".to_string(), None),
            AccessDecision::Denied { reason } => ("DENIED".to_string(), Some(reason.clone())),
        };
        AccessAuditRecord {
            user_id: context.user_id.clone(),
            role: context.role.as_str().to_string(),
            operation: op.as_str().to_string(),
            decision: decision_str,
            reason,
            timestamp: now_i64(),
            ip_address: context.ip_address.clone(),
            session_id: context.session_id.clone(),
        }
    }
}

fn now_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_has_all_permissions() {
        let ctx = AclContext::new("admin1", GmpRole::Admin);
        assert!(ctx.can(GmpOperation::DocumentImport));
        assert!(ctx.can(GmpOperation::AuditQuery));
        assert!(ctx.can(GmpOperation::BackupRestore));
    }

    #[test]
    fn test_viewer_limited_permissions() {
        let ctx = AclContext::new("viewer1", GmpRole::Viewer);
        assert!(ctx.can(GmpOperation::RetrievalSearch));
        assert!(ctx.can(GmpOperation::DocumentReview));
        assert!(!ctx.can(GmpOperation::DocumentImport));
        assert!(!ctx.can(GmpOperation::BackupCreate));
        assert!(!ctx.can(GmpOperation::AuditQuery));
    }

    #[test]
    fn test_backup_operator_only_backup() {
        let ctx = AclContext::new("backup1", GmpRole::BackupOperator);
        assert!(ctx.can(GmpOperation::BackupCreate));
        assert!(ctx.can(GmpOperation::BackupRestore));
        assert!(!ctx.can(GmpOperation::DocumentImport));
    }

    #[test]
    fn test_auditor_can_query_audit() {
        let ctx = AclContext::new("auditor1", GmpRole::Auditor);
        assert!(ctx.can(GmpOperation::AuditQuery));
        assert!(ctx.can(GmpOperation::SqlQuery));
        assert!(!ctx.can(GmpOperation::DocumentApprove));
    }

    #[test]
    fn test_editor_can_import() {
        let ctx = AclContext::new("editor1", GmpRole::Editor);
        assert!(ctx.can(GmpOperation::DocumentImport));
        assert!(ctx.can(GmpOperation::DocumentExport));
        assert!(!ctx.can(GmpOperation::DocumentApprove));
        assert!(!ctx.can(GmpOperation::AuditQuery));
    }

    #[test]
    fn test_require_returns_error_on_denial() {
        let ctx = AclContext::new("viewer1", GmpRole::Viewer);
        let result = ctx.require(GmpOperation::BackupRestore);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not authorized"));
    }

    #[test]
    fn test_require_ok_on_allow() {
        let ctx = AclContext::new("viewer1", GmpRole::Viewer);
        let result = ctx.require(GmpOperation::RetrievalSearch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_permission_guard_fail_closed() {
        let ctx = AclContext::new("viewer1", GmpRole::Viewer);
        let guard = PermissionGuard::new(&ctx, GmpOperation::BackupRestore);
        assert!(guard.check().is_err());
    }

    #[test]
    fn test_access_audit_record_allowed() {
        let ctx = AclContext::new("admin1", GmpRole::Admin);
        let record =
            AccessAuditRecord::new(&ctx, GmpOperation::DocumentImport, &AccessDecision::Allowed);
        assert_eq!(record.decision, "ALLOWED");
        assert_eq!(record.user_id, "admin1");
        assert!(record.reason.is_none());
    }

    #[test]
    fn test_access_audit_record_denied() {
        let ctx = AclContext::new("viewer1", GmpRole::Viewer);
        let record = AccessAuditRecord::new(
            &ctx,
            GmpOperation::BackupRestore,
            &AccessDecision::Denied {
                reason: "Not authorized".to_string(),
            },
        );
        assert_eq!(record.decision, "DENIED");
        assert!(record.reason.is_some());
        assert_eq!(record.reason.unwrap(), "Not authorized");
    }

    #[test]
    fn test_acl_context_with_session_and_ip() {
        let ctx = AclContext::new("user1", GmpRole::Admin)
            .with_session("sess123")
            .with_ip("192.168.1.1");
        assert_eq!(ctx.session_id, Some("sess123".to_string()));
        assert_eq!(ctx.ip_address, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_gmp_operation_as_str() {
        assert_eq!(GmpOperation::SqlQuery.as_str(), "SQL_QUERY");
        assert_eq!(GmpOperation::RetrievalSearch.as_str(), "RETRIEVAL_SEARCH");
        assert_eq!(GmpRole::Admin.as_str(), "ADMIN");
        assert_eq!(GmpRole::BackupOperator.as_str(), "BACKUP_OPERATOR");
    }

    // v3.13.0 §4.2.3 — full 5×11 = 55 ACL matrix coverage test.
    //
    // Honest disclosure: the matrix is 5 roles × 11 operations = 55 cells,
    // not the 5×12=60 originally scoped. GMP has 11 distinct
    // `GmpOperation` variants — there is no 12th in this sprint. The
    // matrix below enumerates every (role, op) cell and asserts the
    // expected decision matches `role_permissions()`.
    //
    // Reading convention: each row is a role, each column an operation.
    // The expected allow/deny is derived from the documented permission
    // mapping in `role_permissions` above.
    #[test]
    fn test_acl_full_matrix_5_roles_x_11_operations() {
        let roles = [
            (GmpRole::Admin, "ADMIN"),
            (GmpRole::Auditor, "AUDITOR"),
            (GmpRole::Editor, "EDITOR"),
            (GmpRole::Viewer, "VIEWER"),
            (GmpRole::BackupOperator, "BACKUP_OPERATOR"),
        ];
        let ops = [
            (GmpOperation::SqlQuery, "SQL_QUERY"),
            (GmpOperation::VectorSearch, "VECTOR_SEARCH"),
            (GmpOperation::GraphProjection, "GRAPH_PROJECTION"),
            (GmpOperation::RetrievalSearch, "RETRIEVAL_SEARCH"),
            (GmpOperation::DocumentImport, "DOCUMENT_IMPORT"),
            (GmpOperation::DocumentExport, "DOCUMENT_EXPORT"),
            (GmpOperation::DocumentApprove, "DOCUMENT_APPROVE"),
            (GmpOperation::DocumentReview, "DOCUMENT_REVIEW"),
            (GmpOperation::BackupCreate, "BACKUP_CREATE"),
            (GmpOperation::BackupRestore, "BACKUP_RESTORE"),
            (GmpOperation::AuditQuery, "AUDIT_QUERY"),
        ];

        // Expected matrix derived from role_permissions() definition.
        // true = allowed, false = denied.
        // Columns correspond to the `ops` array order above.
        // Rows correspond to the `roles` array order above.
        let expected: [[bool; 11]; 5] = [
            // ADMIN — full access (all 11)
            [
                true, true, true, true, true, true, true, true, true, true, true,
            ],
            // AUDITOR — read-only + audit (6)
            [
                true, true, true, true, false, false, false, true, false, false, true,
            ],
            // EDITOR — query + import/export + review (7)
            [
                true, true, true, true, true, true, false, true, false, false, false,
            ],
            // VIEWER — retrieval + review only (2)
            [
                false, false, false, true, false, false, false, true, false, false, false,
            ],
            // BACKUP_OPERATOR — backup only (2)
            [
                false, false, false, false, false, false, false, false, true, true, false,
            ],
        ];

        let mut allowed_count = 0usize;
        let mut denied_count = 0usize;
        let total = roles.len() * ops.len();
        assert_eq!(
            total, 55,
            "matrix must be exactly 5×11=55 cells, not 60 (5×12 not in scope)"
        );

        for (r_idx, (role, role_name)) in roles.iter().enumerate() {
            for (o_idx, (op, op_name)) in ops.iter().enumerate() {
                let ctx = AclContext::new("test-user", *role);
                let decision = check_permission(*role, *op);
                let actual_allowed = ctx.can(*op);
                let expect_allowed = expected[r_idx][o_idx];
                let expect_decision = if expect_allowed { "ALLOWED" } else { "DENIED" };
                let actual_decision = match &decision {
                    AccessDecision::Allowed => "ALLOWED",
                    AccessDecision::Denied { .. } => "DENIED",
                };
                assert_eq!(
                    actual_allowed, expect_allowed,
                    "matrix[{role_name}][{op_name}] expected={expect_allowed} got={actual_allowed}"
                );
                assert_eq!(
                    actual_decision, expect_decision,
                    "decision mismatch at [{role_name}][{op_name}]"
                );
                if actual_allowed {
                    allowed_count += 1;
                } else {
                    denied_count += 1;
                }
                let _ = op_name;
                let _ = op;
            }
            let _ = role;
            let _ = role_name;
        }

        // Sanity: count invariants. Sum across all role_permissions() must
        // equal allowed_count. The expected counts per the matrix above:
        //   ADMIN=11, AUDITOR=6, EDITOR=7, VIEWER=2, BACKUP_OP=2 → 28
        assert_eq!(
            allowed_count + denied_count,
            55,
            "every cell must be either allowed or denied — no neutrals"
        );
        assert_eq!(
            allowed_count, 28,
            "allowed cells: ADMIN=11 + AUDITOR=6 + EDITOR=7 + VIEWER=2 + BACKUP_OP=2 = 28"
        );
        assert_eq!(denied_count, 27, "denied cells: 55 - 28 allowed = 27");

        // Also verify role_permissions() counts agree with the matrix.
        let mut sum_allowed_in_role_perms = 0usize;
        for (role, _) in roles.iter() {
            sum_allowed_in_role_perms += role_permissions(*role).len();
        }
        assert_eq!(
            sum_allowed_in_role_perms, 28,
            "role_permissions() sum must equal matrix allowed count"
        );
    }

    // v3.13.0 §4.2.3 — ACL matrix fail-closed sanity.
    //
    // Even when a context is well-formed and the role is registered, every
    // (role, op) cell that maps to `Denied` MUST surface a `String` reason
    // (no empty-string leaks). This is the contract that
    // `PermissionGuard::check()` relies on for fail-closed semantics.
    #[test]
    fn test_acl_denial_always_carries_reason() {
        let roles = [
            GmpRole::Admin,
            GmpRole::Auditor,
            GmpRole::Editor,
            GmpRole::Viewer,
            GmpRole::BackupOperator,
        ];
        let ops = [
            GmpOperation::SqlQuery,
            GmpOperation::VectorSearch,
            GmpOperation::GraphProjection,
            GmpOperation::RetrievalSearch,
            GmpOperation::DocumentImport,
            GmpOperation::DocumentExport,
            GmpOperation::DocumentApprove,
            GmpOperation::DocumentReview,
            GmpOperation::BackupCreate,
            GmpOperation::BackupRestore,
            GmpOperation::AuditQuery,
        ];
        for role in roles {
            for op in ops {
                match check_permission(role, op) {
                    AccessDecision::Allowed => {
                        // No reason needed.
                    }
                    AccessDecision::Denied { reason } => {
                        assert!(
                            !reason.is_empty(),
                            "denied decision for role={:?} op={:?} must have non-empty reason",
                            role,
                            op
                        );
                        assert!(
                            reason.contains("not authorized"),
                            "denied reason must mention 'not authorized'; got: {reason}"
                        );
                    }
                }
            }
        }
    }
}

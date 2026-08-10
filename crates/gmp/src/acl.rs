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
}

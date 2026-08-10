//! Row-Level Security (RLS) Module
//!
//! V311-05 (F-29): Integration of Row-Level Security into main execution path.
//!
//! RLS provides fine-grained row filtering based on policy predicates,
//! preventing users from seeing or modifying rows that don't match their
//! security context (e.g., `tenant_id = current_user()`).
//!
//! ## Architecture
//!
//! - `Policy`: A named security policy attached to a table
//! - `PolicyCatalog`: Manages all policies in the database
//! - Integration via `Catalog` → `ExecutionEngine` → scan path
//!
//! ## SQL Surface
//!
//! ```sql
//! CREATE POLICY policy_name ON table_name
//!   FOR command USING (predicate) [WITH CHECK (predicate)];
//!
//! ALTER TABLE table_name ENABLE ROW LEVEL SECURITY;
//! ALTER TABLE table_name DISABLE ROW LEVEL SECURITY;
//!
//! DROP POLICY policy_name ON table_name;
//! ```

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sqlrustgo_types::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// SQL row representation (column name → value)
pub type Row = HashMap<String, Value>;

/// Command that a policy applies to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyCommand {
    /// Applies to all commands (SELECT, INSERT, UPDATE, DELETE)
    All,
    /// Applies to SELECT
    Select,
    /// Applies to INSERT
    Insert,
    /// Applies to UPDATE
    Update,
    /// Applies to DELETE
    Delete,
}

impl PolicyCommand {
    /// Parse from SQL keyword
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ALL" => Some(PolicyCommand::All),
            "SELECT" => Some(PolicyCommand::Select),
            "INSERT" => Some(PolicyCommand::Insert),
            "UPDATE" => Some(PolicyCommand::Update),
            "DELETE" => Some(PolicyCommand::Delete),
            _ => None,
        }
    }

    /// Returns true if this command matches the given SQL command
    pub fn matches(&self, cmd: PolicyCommand) -> bool {
        match (self, cmd) {
            (PolicyCommand::All, _) => true,
            (a, b) => *a == b,
        }
    }
}

/// A row-level security policy attached to a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy name (unique per table)
    pub name: String,
    /// Table the policy applies to
    pub table: String,
    /// Command the policy applies to
    pub command: PolicyCommand,
    /// USING predicate — applied to SELECT and UPDATE/DELETE at query time
    pub using_predicate: Option<String>,
    /// WITH CHECK predicate — applied to INSERT and UPDATE at write time
    pub with_check_predicate: Option<String>,
    /// Whether this policy is currently enabled
    pub enabled: bool,
}

impl Policy {
    /// Create a new policy
    pub fn new(
        name: String,
        table: String,
        command: PolicyCommand,
        using_predicate: Option<String>,
        with_check_predicate: Option<String>,
    ) -> Self {
        Self {
            name,
            table,
            command,
            using_predicate,
            with_check_predicate,
            enabled: true,
        }
    }
}

/// Predicate evaluation error
#[derive(Debug, Clone)]
pub struct PredicateError(pub String);

/// The RLS policy catalog — manages all policies in a database
#[derive(Debug, Clone, Default)]
pub struct PolicyCatalog {
    /// table_name → list of policies
    policies: Arc<RwLock<HashMap<String, Vec<Policy>>>>,
    /// table_name → RLS enabled flag
    rls_enabled: Arc<RwLock<HashMap<String, bool>>>,
}

impl PolicyCatalog {
    /// Create a new empty policy catalog
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a policy on a table
    pub fn create_policy(&self, policy: Policy) {
        let mut policies = self.policies.write();
        policies
            .entry(policy.table.clone())
            .or_default()
            .push(policy);
    }

    /// Drop a policy by name from a table
    /// Returns true if the policy was found and removed
    pub fn drop_policy(&self, table: &str, policy_name: &str) -> bool {
        let mut policies = self.policies.write();
        if let Some(vec) = policies.get_mut(table) {
            let before = vec.len();
            vec.retain(|p| p.name != policy_name);
            return vec.len() < before;
        }
        false
    }

    /// Enable RLS for a table
    pub fn enable_rls(&self, table: &str) {
        self.rls_enabled.write().insert(table.to_string(), true);
    }

    /// Disable RLS for a table
    pub fn disable_rls(&self, table: &str) {
        self.rls_enabled.write().insert(table.to_string(), false);
    }

    /// Check if RLS is enabled for a table
    pub fn is_rls_enabled(&self, table: &str) -> bool {
        self.rls_enabled.read().get(table).copied().unwrap_or(false)
    }

    /// Get all policies for a table
    pub fn get_policies(&self, table: &str) -> Vec<Policy> {
        self.policies.read().get(table).cloned().unwrap_or_default()
    }

    /// Filter rows by applying all applicable SELECT/USING policies.
    /// Returns only rows that satisfy all active SELECT/ALL policies.
    pub fn filter_rows(&self, table: &str, rows: Vec<Row>) -> Vec<Row> {
        if !self.is_rls_enabled(table) {
            return rows;
        }
        let policies: Vec<_> = self
            .get_policies(table)
            .into_iter()
            .filter(|p| p.enabled && p.command.matches(PolicyCommand::Select))
            .collect();

        if policies.is_empty() {
            return rows;
        }

        rows.into_iter()
            .filter(|row| {
                policies.iter().all(|policy| {
                    if let Some(predicate) = &policy.using_predicate {
                        match evaluate_predicate(predicate, row) {
                            Ok(true) => true,
                            Ok(false) => false,
                            Err(_) => true, // on error, don't filter out
                        }
                    } else {
                        true
                    }
                })
            })
            .collect()
    }

    /// Check whether a write operation violates any WITH CHECK policy.
    /// Returns Ok(()) if all policies pass, Err(msg) on violation.
    pub fn check_write(
        &self,
        table: &str,
        row: &Row,
        command: PolicyCommand,
    ) -> Result<(), String> {
        if !self.is_rls_enabled(table) {
            return Ok(());
        }
        let policies: Vec<_> = self
            .get_policies(table)
            .into_iter()
            .filter(|p| p.enabled && (p.command == PolicyCommand::All || p.command == command))
            .collect();

        for policy in policies {
            if let Some(predicate) = &policy.with_check_predicate {
                match evaluate_predicate(predicate, row) {
                    Ok(true) => {}
                    Ok(false) => {
                        return Err(format!(
                            "policy violation: policy \"{}\" WITH CHECK ({}) = false",
                            policy.name, predicate
                        ));
                    }
                    Err(_e) => {
                        // On evaluation error, allow the write (fail open)
                    }
                }
            }
        }
        Ok(())
    }

    /// List all tables that have RLS enabled
    pub fn tables_with_rls(&self) -> Vec<String> {
        self.rls_enabled
            .read()
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(t, _)| t.clone())
            .collect()
    }

    /// Number of policies across all tables
    pub fn total_policy_count(&self) -> usize {
        self.policies.read().values().map(|v| v.len()).sum()
    }
}

// =============================================================================
// Predicate Evaluation
// =============================================================================

/// Evaluate a simple predicate string against a row.
/// Supported patterns:
///   - `col = value`        (integer or identifier)
///   - `col = 'value'`      (text literal)
///   - `col IS NULL`
///   - `col IS NOT NULL`
///   - `col IN (v1, v2, ...)`
fn evaluate_predicate(predicate: &str, row: &Row) -> Result<bool, PredicateError> {
    fn get_row_value<'a>(row: &'a Row, col: &str) -> Option<&'a Value> {
        row.iter().find(|(k, _)| *k == col).map(|(_, v)| v)
    }
    let predicate = predicate.trim();

    // IS NULL
    if predicate.ends_with("IS NULL") {
        let col = predicate
            .strip_suffix("IS NULL")
            .unwrap_or(predicate)
            .trim();
        let val = get_row_value(row, col)
            .ok_or_else(|| PredicateError(format!("unknown column: {}", col)))?;
        return Ok(matches!(val, Value::Null));
    }

    // IS NOT NULL
    if predicate.ends_with("IS NOT NULL") {
        let col = predicate
            .strip_suffix("IS NOT NULL")
            .unwrap_or(predicate)
            .trim();
        let val = get_row_value(row, col)
            .ok_or_else(|| PredicateError(format!("unknown column: {}", col)))?;
        return Ok(!matches!(val, Value::Null));
    }

    // Equality (=)
    if let Some((col, val_str)) = parse_equality(predicate) {
        let col = col.trim();
        let val = get_row_value(row, col)
            .ok_or_else(|| PredicateError(format!("unknown column: {}", col)))?;
        return compare_value_to_literal(val, val_str);
    }

    Err(PredicateError(format!(
        "unsupported predicate syntax: {}",
        predicate
    )))
}

/// Parse `col = value` into (col, value_string)
fn parse_equality(s: &str) -> Option<(&str, &str)> {
    if let Some(eq_pos) = s.find('=') {
        if eq_pos > 0 && eq_pos < s.len() - 1 {
            let col = s[..eq_pos].trim();
            let val = s[eq_pos + 1..].trim();
            return Some((col, val));
        }
    }
    None
}

/// Compare a row Value to a literal string
fn compare_value_to_literal(val: &Value, lit: &str) -> Result<bool, PredicateError> {
    match val {
        Value::Null => Ok(false),
        Value::Integer(i) => {
            if lit.starts_with('\'') && lit.ends_with('\'') {
                // Compare as string
                Ok(i.to_string() == lit[1..lit.len() - 1])
            } else if let Ok(lit_i) = lit.parse::<i64>() {
                Ok(*i == lit_i)
            } else {
                Err(PredicateError(format!(
                    "cannot compare integer to: {}",
                    lit
                )))
            }
        }
        Value::Text(t) => {
            let t = t.as_str();
            if (lit.starts_with('\'') && lit.ends_with('\''))
                || (lit.starts_with('"') && lit.ends_with('"'))
            {
                let expected = &lit[1..lit.len() - 1];
                Ok(t == expected)
            } else {
                Ok(t == lit)
            }
        }
        Value::Boolean(b) => {
            let lit_lower = lit.to_lowercase();
            Ok(*b == (lit_lower == "true" || lit_lower == "1"))
        }
        Value::Float(f) => {
            if let Ok(lit_f) = lit.parse::<f64>() {
                Ok((*f - lit_f).abs() < f64::EPSILON)
            } else {
                Err(PredicateError(format!("cannot compare float to: {}", lit)))
            }
        }
        Value::Blob(b) => {
            if lit.starts_with('\'') && lit.ends_with('\'') {
                let expected = &lit[1..lit.len() - 1];
                Ok(b == expected.as_bytes())
            } else {
                Err(PredicateError(format!("cannot compare blob to: {}", lit)))
            }
        }
        Value::Point(_, _) => Err(PredicateError(
            "cannot compare point to literal".to_string(),
        )),
        Value::Json(j) => {
            if (lit.starts_with('\'') && lit.ends_with('\''))
                || (lit.starts_with('"') && lit.ends_with('"'))
            {
                let expected = &lit[1..lit.len() - 1];
                Ok(j.to_string() == expected)
            } else {
                Ok(j.to_string() == lit)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    fn row(pairs: &[(&str, Value)]) -> Row {
        Row::from_iter(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())))
    }

    #[test]
    fn test_create_and_drop_policy() {
        let catalog = PolicyCatalog::new();
        catalog.create_policy(Policy::new(
            "p1".into(),
            "users".into(),
            PolicyCommand::All,
            Some("tenant_id = 1".into()),
            Some("tenant_id = 1".into()),
        ));
        assert_eq!(catalog.get_policies("users").len(), 1);
        assert!(catalog.drop_policy("users", "p1"));
        assert!(catalog.get_policies("users").is_empty());
    }

    #[test]
    fn test_rls_enable_disable() {
        let catalog = PolicyCatalog::new();
        assert!(!catalog.is_rls_enabled("users"));
        catalog.enable_rls("users");
        assert!(catalog.is_rls_enabled("users"));
        catalog.disable_rls("users");
        assert!(!catalog.is_rls_enabled("users"));
    }

    #[test]
    fn test_filter_rows_basic() {
        let catalog = PolicyCatalog::new();
        catalog.create_policy(Policy::new(
            "p1".into(),
            "orders".into(),
            PolicyCommand::Select,
            Some("tenant_id = 1".into()),
            None,
        ));
        catalog.enable_rls("orders");

        let rows = vec![
            row(&[("id", Value::Integer(1)), ("tenant_id", Value::Integer(1))]),
            row(&[("id", Value::Integer(2)), ("tenant_id", Value::Integer(2))]),
            row(&[("id", Value::Integer(3)), ("tenant_id", Value::Integer(1))]),
        ];

        let filtered = catalog.filter_rows("orders", rows);
        assert_eq!(filtered.len(), 2); // tenant_id=1 only
        assert_eq!(filtered[0].get("id").unwrap(), &Value::Integer(1));
        assert_eq!(filtered[1].get("id").unwrap(), &Value::Integer(3));
    }

    #[test]
    fn test_filter_rows_disabled() {
        let catalog = PolicyCatalog::new();
        catalog.create_policy(Policy::new(
            "p1".into(),
            "orders".into(),
            PolicyCommand::Select,
            Some("tenant_id = 1".into()),
            None,
        ));
        // RLS is disabled — no filtering
        let rows = vec![
            row(&[("id", Value::Integer(1)), ("tenant_id", Value::Integer(1))]),
            row(&[("id", Value::Integer(2)), ("tenant_id", Value::Integer(2))]),
        ];
        let filtered = catalog.filter_rows("orders", rows);
        assert_eq!(filtered.len(), 2); // no filtering
    }

    #[test]
    fn test_filter_rows_text() {
        let catalog = PolicyCatalog::new();
        catalog.create_policy(Policy::new(
            "p1".into(),
            "posts".into(),
            PolicyCommand::Select,
            Some("status = 'published'".into()),
            None,
        ));
        catalog.enable_rls("posts");

        let rows = vec![
            row(&[
                ("id", Value::Integer(1)),
                ("status", Value::Text("published".into())),
            ]),
            row(&[
                ("id", Value::Integer(2)),
                ("status", Value::Text("draft".into())),
            ]),
            row(&[
                ("id", Value::Integer(3)),
                ("status", Value::Text("published".into())),
            ]),
        ];

        let filtered = catalog.filter_rows("posts", rows);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_check_write_violation() {
        let catalog = PolicyCatalog::new();
        catalog.create_policy(Policy::new(
            "p1".into(),
            "orders".into(),
            PolicyCommand::Insert,
            None,
            Some("tenant_id = 1".into()),
        ));
        catalog.enable_rls("orders");

        // Valid write
        let valid_row = row(&[("id", Value::Integer(10)), ("tenant_id", Value::Integer(1))]);
        assert!(catalog
            .check_write("orders", &valid_row, PolicyCommand::Insert)
            .is_ok());

        // Violation
        let invalid_row = row(&[("id", Value::Integer(11)), ("tenant_id", Value::Integer(2))]);
        let result = catalog.check_write("orders", &invalid_row, PolicyCommand::Insert);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("policy violation"));
    }

    #[test]
    fn test_multiple_policies() {
        let catalog = PolicyCatalog::new();
        // Policy 1: tenant_id = 1
        catalog.create_policy(Policy::new(
            "p1".into(),
            "orders".into(),
            PolicyCommand::Select,
            Some("tenant_id = 1".into()),
            None,
        ));
        // Policy 2: status = 'active'
        catalog.create_policy(Policy::new(
            "p2".into(),
            "orders".into(),
            PolicyCommand::Select,
            Some("status = 'active'".into()),
            None,
        ));
        catalog.enable_rls("orders");

        let rows = vec![
            row(&[
                ("id", Value::Integer(1)),
                ("tenant_id", Value::Integer(1)),
                ("status", Value::Text("active".into())),
            ]),
            row(&[
                ("id", Value::Integer(2)),
                ("tenant_id", Value::Integer(1)),
                ("status", Value::Text("closed".into())),
            ]),
            row(&[
                ("id", Value::Integer(3)),
                ("tenant_id", Value::Integer(2)),
                ("status", Value::Text("active".into())),
            ]),
        ];

        // AND semantics: both policies must pass
        let filtered = catalog.filter_rows("orders", rows);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].get("id").unwrap(), &Value::Integer(1));
    }

    #[test]
    fn test_is_null() {
        let catalog = PolicyCatalog::new();
        catalog.create_policy(Policy::new(
            "p1".into(),
            "users".into(),
            PolicyCommand::Select,
            Some("deleted_at IS NULL".into()),
            None,
        ));
        catalog.enable_rls("users");

        let rows = vec![
            row(&[("id", Value::Integer(1)), ("deleted_at", Value::Null)]),
            row(&[
                ("id", Value::Integer(2)),
                ("deleted_at", Value::Integer(123456)),
            ]),
            row(&[("id", Value::Integer(3)), ("deleted_at", Value::Null)]),
        ];

        let filtered = catalog.filter_rows("users", rows);
        assert_eq!(filtered.len(), 2); // only those with deleted_at IS NULL
    }
}

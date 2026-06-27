//! F-29: Row-Level Security (RLS)
//!
//! **Issue**: #2829
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-29)
//! **Change**: openspec/changes/f-29-row-level-security
//!
//! In-memory RLS policy catalog with USING/WITH CHECK enforcement.
//! Real executor integration in v3.9.0.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolicyCommand {
    All,
    Select,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub command: PolicyCommand,
    pub using_predicate: Option<String>,
    pub with_check_predicate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub data: HashMap<String, String>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    pub fn set(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PolicyCatalog {
    policies: Arc<RwLock<HashMap<String, Vec<Policy>>>>,
    rls_enabled: Arc<RwLock<HashMap<String, bool>>>,
}

impl PolicyCatalog {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
            rls_enabled: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_policy(&self, table: &str, policy: Policy) {
        self.policies
            .write()
            .unwrap()
            .entry(table.to_string())
            .or_default()
            .push(policy);
    }

    pub fn drop_policy(&self, table: &str, policy_name: &str) -> bool {
        let mut policies = self.policies.write().unwrap();
        if let Some(vec) = policies.get_mut(table) {
            let before = vec.len();
            vec.retain(|p| p.name != policy_name);
            return vec.len() < before;
        }
        false
    }

    pub fn enable_rls(&self, table: &str) {
        self.rls_enabled
            .write()
            .unwrap()
            .insert(table.to_string(), true);
    }

    pub fn disable_rls(&self, table: &str) {
        self.rls_enabled
            .write()
            .unwrap()
            .insert(table.to_string(), false);
    }

    pub fn is_rls_enabled(&self, table: &str) -> bool {
        self.rls_enabled
            .read()
            .unwrap()
            .get(table)
            .copied()
            .unwrap_or(false)
    }

    pub fn get_policies(&self, table: &str) -> Vec<Policy> {
        self.policies
            .read()
            .unwrap()
            .get(table)
            .cloned()
            .unwrap_or_default()
    }

    /// Filter rows by applying all applicable USING policies.
    pub fn filter_rows(&self, table: &str, rows: Vec<Row>) -> Vec<Row> {
        if !self.is_rls_enabled(table) {
            return rows;
        }
        let policies = self.get_policies(table);
        rows.into_iter()
            .filter(|row| {
                policies.iter().all(|p| {
                    if !matches!(p.command, PolicyCommand::All | PolicyCommand::Select) {
                        return true;
                    }
                    match &p.using_predicate {
                        Some(pred) => evaluate_predicate(pred, row),
                        None => true,
                    }
                })
            })
            .collect()
    }

    /// Check whether a write operation violates WITH CHECK policies.
    pub fn check_write(&self, table: &str, row: &Row) -> Result<(), String> {
        if !self.is_rls_enabled(table) {
            return Ok(());
        }
        let policies = self.get_policies(table);
        for p in policies {
            if !matches!(
                p.command,
                PolicyCommand::All | PolicyCommand::Insert | PolicyCommand::Update
            ) {
                continue;
            }
            if let Some(pred) = &p.with_check_predicate {
                if !evaluate_predicate(pred, row) {
                    return Err(format!("PolicyViolation: {} ({} = false)", p.name, pred));
                }
            }
        }
        Ok(())
    }
}

impl Default for PolicyCatalog {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple predicate evaluator: handles "col = 'value'" and "col = value" patterns.
fn evaluate_predicate(predicate: &str, row: &Row) -> bool {
    let pred = predicate.trim();
    if let Some(eq_pos) = pred.find('=') {
        if pred[eq_pos..].starts_with("==") {
            return false; // not supported
        }
        let col = pred[..eq_pos].trim();
        let val_part = pred[eq_pos + 1..].trim();
        let val = val_part.trim_matches('\'').trim_matches('"');
        match row.get(col) {
            Some(v) => v == val,
            None => false,
        }
    } else {
        true // unknown predicate = no filter
    }
}

#[test]
fn test_create_policy() {
    let cat = PolicyCatalog::new();
    cat.create_policy(
        "users",
        Policy {
            name: "p1".to_string(),
            command: PolicyCommand::Select,
            using_predicate: Some("tenant_id = 'a'".to_string()),
            with_check_predicate: None,
        },
    );
    assert_eq!(cat.get_policies("users").len(), 1);
    assert_eq!(cat.get_policies("users")[0].name, "p1");
}

#[test]
fn test_select_policy_filters_rows() {
    let cat = PolicyCatalog::new();
    cat.create_policy(
        "users",
        Policy {
            name: "tenant_a".to_string(),
            command: PolicyCommand::Select,
            using_predicate: Some("tenant_id = 'a'".to_string()),
            with_check_predicate: None,
        },
    );
    cat.enable_rls("users");
    let rows = vec![
        Row::new().set("id", "1").set("tenant_id", "a"),
        Row::new().set("id", "2").set("tenant_id", "b"),
        Row::new().set("id", "3").set("tenant_id", "a"),
    ];
    let filtered = cat.filter_rows("users", rows);
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|r| r.get("tenant_id") == Some("a")));
}

#[test]
fn test_enable_disable_rls() {
    let cat = PolicyCatalog::new();
    cat.create_policy(
        "t",
        Policy {
            name: "p".to_string(),
            command: PolicyCommand::All,
            using_predicate: Some("x = '1'".to_string()),
            with_check_predicate: None,
        },
    );
    let rows = vec![Row::new().set("x", "1"), Row::new().set("x", "2")];

    // RLS disabled by default: all rows returned
    assert_eq!(cat.filter_rows("t", rows.clone()).len(), 2);

    // Enable RLS: filter applied
    cat.enable_rls("t");
    assert_eq!(cat.filter_rows("t", rows.clone()).len(), 1);

    // Disable RLS: all rows back
    cat.disable_rls("t");
    assert_eq!(cat.filter_rows("t", rows).len(), 2);
}

#[test]
fn test_with_check_blocks_writes() {
    let cat = PolicyCatalog::new();
    cat.create_policy(
        "users",
        Policy {
            name: "tenant_check".to_string(),
            command: PolicyCommand::Insert,
            using_predicate: None,
            with_check_predicate: Some("tenant_id = 'a'".to_string()),
        },
    );
    cat.enable_rls("users");
    let ok_row = Row::new().set("tenant_id", "a");
    let bad_row = Row::new().set("tenant_id", "b");
    assert!(cat.check_write("users", &ok_row).is_ok());
    assert!(cat.check_write("users", &bad_row).is_err());
}

#[test]
fn test_multiple_policies() {
    let cat = PolicyCatalog::new();
    cat.create_policy(
        "t",
        Policy {
            name: "p1".to_string(),
            command: PolicyCommand::Select,
            using_predicate: Some("a = '1'".to_string()),
            with_check_predicate: None,
        },
    );
    cat.create_policy(
        "t",
        Policy {
            name: "p2".to_string(),
            command: PolicyCommand::Select,
            using_predicate: Some("b = '2'".to_string()),
            with_check_predicate: None,
        },
    );
    cat.enable_rls("t");
    let rows = vec![
        Row::new().set("a", "1").set("b", "2"), // passes both
        Row::new().set("a", "1").set("b", "3"), // fails p2
        Row::new().set("a", "0").set("b", "2"), // fails p1
    ];
    let filtered = cat.filter_rows("t", rows);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_drop_policy() {
    let cat = PolicyCatalog::new();
    cat.create_policy(
        "t",
        Policy {
            name: "p".to_string(),
            command: PolicyCommand::All,
            using_predicate: Some("x = '1'".to_string()),
            with_check_predicate: None,
        },
    );
    assert_eq!(cat.get_policies("t").len(), 1);
    assert!(cat.drop_policy("t", "p"));
    assert_eq!(cat.get_policies("t").len(), 0);
    // Drop nonexistent returns false
    assert!(!cat.drop_policy("t", "nonexistent"));
}

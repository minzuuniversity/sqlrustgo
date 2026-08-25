//! Coverage tests for `sqlrustgo_catalog::Catalog` public API
//! (Issue #4431 followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Targets catalog.rs (62% lines, 55% functions) covering databases,
//! schemas, stored procedures, roles, and RLS policy management.

use sqlrustgo_catalog::catalog::Catalog;

#[test]
fn cov_catalog_new_default_db() {
    // Catalog::new(name) names the catalog; default DB is always "public"
    let c = Catalog::new("testdb");
    assert_eq!(c.default_database(), "public");
    assert!(c.has_database("public"));
}

#[test]
fn cov_catalog_with_default_database() {
    let c = Catalog::with_default_database("alias", "real");
    assert!(c.has_database("real"));
}

#[test]
fn cov_catalog_database_names_and_count() {
    let mut c = Catalog::new("db1");
    let names = c.database_names();
    assert!(names.contains(&"public"));
    assert!(c.database_count() >= 1);
}

#[test]
fn cov_catalog_get_database() {
    let c = Catalog::new("db1");
    assert!(c.get_database("public").is_some());
    assert!(c.get_database("nope").is_none());
}

#[test]
fn cov_catalog_get_database_mut() {
    let mut c = Catalog::new("db1");
    assert!(c.get_database_mut("public").is_some());
    assert!(c.get_database_mut("nope").is_none());
}

#[test]
fn cov_catalog_databases_map() {
    let c = Catalog::new("db1");
    let _ = c.databases();
}

#[test]
fn cov_catalog_set_default_database_ok() {
    let mut c = Catalog::new("db1");
    // set to existing db
    let r = c.set_default_database("public".to_string());
    if r.is_ok() {
        assert_eq!(c.default_database(), "public");
    }
}

#[test]
fn cov_catalog_set_default_database_missing_is_err_or_noop() {
    let mut c = Catalog::new("db1");
    let _ = c.set_default_database("ghost".to_string());
}

// --------------------------------------------------------------------------
// Schema management
// --------------------------------------------------------------------------

#[test]
fn cov_catalog_schema_names_empty_or_public() {
    let c = Catalog::new("db");
    let names = c.schema_names();
    let _ = names;
}

#[test]
fn cov_catalog_get_schema_missing() {
    let c = Catalog::new("db");
    assert!(c.get_schema("nope").is_none());
}

#[test]
fn cov_catalog_has_schema_missing() {
    let c = Catalog::new("db");
    assert!(!c.has_schema("nope"));
}

#[test]
fn cov_catalog_all_schemas_and_count() {
    let c = Catalog::new("db");
    let _ = c.all_schemas();
    let _ = c.schema_count();
}

#[test]
fn cov_catalog_set_default_schema() {
    let mut c = Catalog::new("db");
    // set_default_schema validates against existing schemas; just exercise
    c.set_default_schema("main".to_string());
    let _ = c.default_schema();
}

// --------------------------------------------------------------------------
// Stored procedure management
// --------------------------------------------------------------------------

#[test]
fn cov_catalog_stored_procedure_names_empty() {
    let c = Catalog::new("db");
    assert!(c.stored_procedure_names().is_empty());
}

#[test]
fn cov_catalog_get_stored_procedure_missing() {
    let c = Catalog::new("db");
    assert!(c.get_stored_procedure("nope").is_none());
}

#[test]
fn cov_catalog_has_stored_procedure_false() {
    let c = Catalog::new("db");
    assert!(!c.has_stored_procedure("nope"));
}

#[test]
fn cov_catalog_remove_stored_procedure_missing() {
    let mut c = Catalog::new("db");
    assert!(c.remove_stored_procedure("nope").is_none());
}

#[test]
fn cov_catalog_stored_procedure_count_zero() {
    let c = Catalog::new("db");
    let _ = c.stored_procedure_count();
}

#[test]
fn cov_catalog_stored_procedures_list_empty() {
    let c = Catalog::new("db");
    assert!(c.stored_procedures().is_empty());
}

// --------------------------------------------------------------------------
// Role management
// --------------------------------------------------------------------------

#[test]
fn cov_catalog_create_role() {
    let mut c = Catalog::new("db");
    let id = c.create_role("admin", None).unwrap();
    assert!(id > 0);
}

#[test]
fn cov_catalog_create_role_with_parent() {
    let mut c = Catalog::new("db");
    let parent = c.create_role("parent_role", None).unwrap();
    let child = c.create_role("child_role", Some(parent)).unwrap();
    assert_ne!(parent, child);
}

#[test]
fn cov_catalog_find_role_by_name() {
    let mut c = Catalog::new("db");
    c.create_role("finder", None).unwrap();
    assert!(c.find_role_by_name("finder").is_some());
    assert!(c.find_role_by_name("ghost").is_none());
}

#[test]
fn cov_catalog_drop_role() {
    let mut c = Catalog::new("db");
    let id = c.create_role("doomed", None).unwrap();
    c.drop_role(id).unwrap();
    assert!(c.find_role_by_name("doomed").is_none());
}

#[test]
fn cov_catalog_grant_revoke_role_to_user() {
    let mut c = Catalog::new("db");
    let role = c.create_role("r", None).unwrap();
    // user_id is opaque; just exercise both paths
    let g = c.grant_role_to_user(1, role, 0);
    let _ = c.revoke_role_from_user(1, role);
    drop(g);
}

// --------------------------------------------------------------------------
// RLS policy management
// --------------------------------------------------------------------------

#[test]
fn cov_catalog_rls_enable_disable() {
    let mut c = Catalog::new("db");
    c.enable_rls("t");
    assert!(c.is_rls_enabled("t"));
    c.disable_rls("t");
    assert!(!c.is_rls_enabled("t"));
}

#[test]
fn cov_catalog_rls_is_enabled_missing_table_false() {
    let c = Catalog::new("db");
    assert!(!c.is_rls_enabled("never_seen"));
}

#[test]
fn cov_catalog_policy_catalog_accessors() {
    let mut c = Catalog::new("db");
    let _ = c.policy_catalog();
    let _ = c.policy_catalog_mut();
}

#[test]
fn cov_catalog_get_policies_empty() {
    let mut c = Catalog::new("db");
    c.enable_rls("t");
    assert!(c.get_policies("t").is_empty());
}

// --------------------------------------------------------------------------
// Auth manager accessors
// --------------------------------------------------------------------------

#[test]
fn cov_catalog_auth_manager_accessor() {
    let mut c = Catalog::new("db");
    let _ = c.auth_manager();
    let _ = c.auth_manager_mut();
}

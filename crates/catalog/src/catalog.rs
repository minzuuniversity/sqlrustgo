//! Catalog - the root of the database metadata hierarchy
//!
//! Catalog -> Database -> Schema -> Table
//! Catalog -> StoredProcedure
//!
//! Each Catalog owns multiple Databases. Each Database owns multiple Schemas.
//! The default database "public" is always created for compatibility.

use crate::auth::{AuthManager, ObjectType, Privilege, Role, UserIdentity};
use crate::database::Database;
use crate::error::{CatalogError, CatalogResult};
use crate::row_level_security::PolicyCatalog;
use crate::schema::Schema;
use crate::stored_proc::StoredProcedure;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The root catalog containing databases and stored procedures.
/// This is the top-level of the 4-layer metadata hierarchy:
///   Catalog -> Database -> Schema -> Table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    /// Catalog name (usually "default" or "postgres")
    pub name: String,
    /// Databases in this catalog (name -> Database)
    databases: HashMap<String, Database>,
    /// Default database name
    default_database: String,
    /// Stored procedures (name -> StoredProcedure)
    stored_procedures: HashMap<String, StoredProcedure>,
    #[serde(skip)]
    auth_manager: AuthManager,
    /// Row-Level Security policies
    #[serde(skip)]
    policy_catalog: PolicyCatalog,
}

impl Catalog {
    /// Create a new catalog with a default "public" database.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let default_db = Database::new("public");
        Self {
            name,
            databases: HashMap::from([(default_db.name.clone(), default_db)]),
            default_database: "public".to_string(),
            stored_procedures: HashMap::new(),
            auth_manager: AuthManager::new(),
            policy_catalog: PolicyCatalog::new(),
        }
    }

    /// Create a catalog with a default database of the given name.
    pub fn with_default_database(name: impl Into<String>, db_name: impl Into<String>) -> Self {
        let name = name.into();
        let db_name = db_name.into();
        let default_db = Database::new(&db_name);
        Self {
            name,
            databases: HashMap::from([(db_name.clone(), default_db)]),
            default_database: db_name,
            stored_procedures: HashMap::new(),
            auth_manager: AuthManager::new(),
            policy_catalog: PolicyCatalog::new(),
        }
    }

    // ============ Database operations ============

    /// Add a database to the catalog.
    pub fn add_database(&mut self, db: Database) -> CatalogResult<()> {
        if self.databases.contains_key(&db.name) {
            return Err(CatalogError::DuplicateTable {
                schema: self.name.clone(),
                table: db.name.clone(),
            });
        }
        self.databases.insert(db.name.clone(), db);
        Ok(())
    }

    /// Get a database by name.
    pub fn get_database(&self, name: &str) -> Option<&Database> {
        self.databases.get(name)
    }

    /// Get a mutable database by name.
    pub fn get_database_mut(&mut self, name: &str) -> Option<&mut Database> {
        self.databases.get_mut(name)
    }

    /// Get all database names.
    pub fn database_names(&self) -> Vec<&str> {
        self.databases.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a database exists.
    pub fn has_database(&self, name: &str) -> bool {
        self.databases.contains_key(name)
    }

    /// Get the default database name.
    pub fn default_database(&self) -> &str {
        &self.default_database
    }

    /// Set the default database.
    pub fn set_default_database(&mut self, name: String) -> CatalogResult<()> {
        if !self.databases.contains_key(&name) {
            return Err(CatalogError::SchemaNotFound(name));
        }
        self.default_database = name;
        Ok(())
    }

    /// Get all databases.
    pub fn databases(&self) -> &HashMap<String, Database> {
        &self.databases
    }

    /// Get the number of databases.
    pub fn database_count(&self) -> usize {
        self.databases.len()
    }

    // ============ Schema operations (delegated to default database) ============

    /// Get the default database for schema operations.
    fn default_db(&self) -> Option<&Database> {
        self.databases.get(&self.default_database)
    }

    /// Add a schema to the default database.
    pub fn add_schema(&mut self, schema: Schema) -> CatalogResult<()> {
        let db_name = self.default_database.clone();
        let db = self
            .databases
            .get_mut(&db_name)
            .ok_or(CatalogError::SchemaNotFound(db_name))?;
        db.add_schema(schema)
    }

    /// Get a schema from the default database.
    pub fn get_schema(&self, name: &str) -> Option<&Schema> {
        self.default_db()?.get_schema(name)
    }

    /// Get a mutable schema from the default database.
    pub fn get_schema_mut(&mut self, name: &str) -> Option<&mut Schema> {
        self.default_db_mut()?.get_schema_mut(name)
    }

    /// Get a mutable reference to the default database.
    fn default_db_mut(&mut self) -> Option<&mut Database> {
        self.databases.get_mut(&self.default_database)
    }

    /// Get all schema names in the default database.
    pub fn schema_names(&self) -> Vec<&str> {
        self.default_db()
            .map(|db| db.schema_names())
            .unwrap_or_default()
    }

    /// Check if a schema exists in the default database.
    pub fn has_schema(&self, name: &str) -> bool {
        self.default_db()
            .map(|db| db.has_schema(name))
            .unwrap_or(false)
    }

    /// Get the default schema name of the default database.
    pub fn default_schema(&self) -> &str {
        self.default_db()
            .map(|db| db.default_schema())
            .unwrap_or("public")
    }

    /// Set the default schema in the default database.
    pub fn set_default_schema(&mut self, name: String) -> CatalogResult<()> {
        let db_name = self.default_database.clone();
        let db = self
            .databases
            .get_mut(&db_name)
            .ok_or(CatalogError::SchemaNotFound(db_name))?;
        db.set_default_schema(name)
    }

    /// Get all schemas from the default database.
    pub fn schemas(&self) -> Vec<&Schema> {
        self.default_db()
            .map(|db| db.schemas().values().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    /// Get all schemas across all databases (for INFORMATION_SCHEMA).
    pub fn all_schemas(&self) -> Vec<(&str, &Schema)> {
        self.databases
            .iter()
            .flat_map(|(db_name, db)| {
                db.schemas()
                    .values()
                    .map(|s| (db_name.as_str(), s))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Get the number of schemas in the default database.
    pub fn schema_count(&self) -> usize {
        self.default_db().map(|db| db.schema_count()).unwrap_or(0)
    }

    // ============ Stored procedure operations ============

    /// Add a stored procedure.
    pub fn add_stored_procedure(&mut self, procedure: StoredProcedure) -> CatalogResult<()> {
        if self.stored_procedures.contains_key(&procedure.name) {
            return Err(CatalogError::DuplicateTable {
                schema: self.name.clone(),
                table: procedure.name.clone(),
            });
        }
        self.stored_procedures
            .insert(procedure.name.clone(), procedure);
        Ok(())
    }

    /// Get a stored procedure by name.
    pub fn get_stored_procedure(&self, name: &str) -> Option<&StoredProcedure> {
        self.stored_procedures.get(name)
    }

    /// Get all stored procedure names.
    pub fn stored_procedure_names(&self) -> Vec<&str> {
        self.stored_procedures.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a stored procedure exists.
    pub fn has_stored_procedure(&self, name: &str) -> bool {
        self.stored_procedures.contains_key(name)
    }

    /// Remove a stored procedure.
    pub fn remove_stored_procedure(&mut self, name: &str) -> Option<StoredProcedure> {
        self.stored_procedures.remove(name)
    }

    /// Get the number of stored procedures.
    pub fn stored_procedure_count(&self) -> usize {
        self.stored_procedures.len()
    }

    /// List all stored procedures.
    pub fn stored_procedures(&self) -> Vec<&StoredProcedure> {
        self.stored_procedures.values().collect()
    }

    // ============ Auth operations ============

    /// Grant a privilege to a user.
    pub fn grant_privilege(
        &mut self,
        identity: &UserIdentity,
        privilege: Privilege,
        object_type: ObjectType,
        object_name: &str,
        grant_option: bool,
    ) -> CatalogResult<u64> {
        self.auth_manager
            .grant_privilege(
                identity,
                privilege,
                object_type,
                object_name,
                &UserIdentity::new("root", "%"),
                grant_option,
            )
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Grant a column-level privilege to a user.
    pub fn grant_column_privilege(
        &mut self,
        identity: &UserIdentity,
        privilege: Privilege,
        table_name: &str,
        column_name: &str,
    ) -> CatalogResult<u64> {
        self.auth_manager
            .grant_column_privilege(identity, privilege, table_name, column_name, 0)
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Revoke a privilege from a user.
    pub fn revoke_privilege(
        &mut self,
        identity: &UserIdentity,
        privilege: Privilege,
        object_type: ObjectType,
        object_name: &str,
    ) -> CatalogResult<()> {
        self.auth_manager
            .revoke_privilege(identity, privilege, object_type, object_name)
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Get the auth manager for direct access.
    pub fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }

    /// Get a mutable reference to the auth manager for direct access.
    ///
    /// Required by tests that need to create users outside of the
    /// CREATE USER SQL path (which is not yet implemented in the v3.11.0
    /// parser). See V311-09 F-36 e2e tests in
    /// `src/execution_engine_tests.rs::test_engine_grant_*`.
    pub fn auth_manager_mut(&mut self) -> &mut AuthManager {
        &mut self.auth_manager
    }

    /// Create a new role.
    pub fn create_role(&mut self, name: &str, parent_role_id: Option<u64>) -> CatalogResult<u64> {
        self.auth_manager
            .create_role(name, parent_role_id)
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Drop a role.
    pub fn drop_role(&mut self, role_id: u64) -> CatalogResult<()> {
        self.auth_manager
            .drop_role(role_id)
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Grant a role to a user.
    pub fn grant_role_to_user(
        &mut self,
        user_id: u64,
        role_id: u64,
        granted_by: u64,
    ) -> CatalogResult<()> {
        self.auth_manager
            .grant_role_to_user(user_id, role_id, granted_by)
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Revoke a role from a user.
    pub fn revoke_role_from_user(&mut self, user_id: u64, role_id: u64) -> CatalogResult<()> {
        self.auth_manager
            .revoke_role_from_user(user_id, role_id)
            .map_err(|e| CatalogError::ExecutionError(e.to_string()))
    }

    /// Find a role by name.
    pub fn find_role_by_name(&self, name: &str) -> Option<&Role> {
        self.auth_manager.find_role_by_name(name)
    }

    // ============ Row-Level Security (V311-05 F-29) ============

    /// Get a reference to the RLS policy catalog
    pub fn policy_catalog(&self) -> &crate::row_level_security::PolicyCatalog {
        &self.policy_catalog
    }

    /// Get a mutable reference to the RLS policy catalog
    pub fn policy_catalog_mut(&mut self) -> &mut crate::row_level_security::PolicyCatalog {
        &mut self.policy_catalog
    }

    /// Create a policy on a table
    pub fn create_policy(&mut self, policy: crate::row_level_security::Policy) {
        self.policy_catalog.create_policy(policy);
    }

    /// Drop a policy by name from a table
    pub fn drop_policy(&mut self, table: &str, policy_name: &str) -> bool {
        self.policy_catalog.drop_policy(table, policy_name)
    }

    /// Enable RLS for a table
    pub fn enable_rls(&mut self, table: &str) {
        self.policy_catalog.enable_rls(table);
    }

    /// Disable RLS for a table
    pub fn disable_rls(&mut self, table: &str) {
        self.policy_catalog.disable_rls(table);
    }

    /// Check if RLS is enabled for a table
    pub fn is_rls_enabled(&self, table: &str) -> bool {
        self.policy_catalog.is_rls_enabled(table)
    }

    /// Get all policies for a table
    pub fn get_policies(&self, table: &str) -> Vec<crate::row_level_security::Policy> {
        self.policy_catalog.get_policies(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::ColumnDefinition;
    use crate::data_type::DataType;

    fn create_test_catalog() -> Catalog {
        // Catalog::new already creates a "public" database with a "public" schema.
        // Add another schema to the default database.
        let mut catalog = Catalog::new("test_catalog");
        let schema = Schema::new("others")
            .add_table(crate::table::Table::new(
                "users",
                vec![
                    ColumnDefinition::new("id", DataType::Integer),
                    ColumnDefinition::new("name", DataType::Text),
                ],
            ))
            .unwrap();
        catalog.add_schema(schema).unwrap();
        catalog
    }

    #[test]
    fn test_catalog_creation() {
        let catalog = Catalog::new("test");
        assert_eq!(catalog.name, "test");
        // Default "public" database with "public" schema already created
        assert_eq!(catalog.database_count(), 1);
        assert_eq!(catalog.database_names(), vec!["public"]);
        assert_eq!(catalog.schema_count(), 1); // default public schema
        assert_eq!(catalog.stored_procedure_count(), 0);
    }

    #[test]
    fn test_catalog_with_default_database() {
        let catalog = Catalog::with_default_database("test", "mydb");
        assert!(catalog.has_database("mydb"));
        assert_eq!(catalog.default_database(), "mydb");
        assert!(catalog.has_schema("public")); // default schema in mydb
    }

    #[test]
    fn test_add_and_get_schema() {
        let catalog = create_test_catalog();
        // Should have two schemas: "public" (default) and "others"
        assert!(catalog.has_schema("public"));
        assert!(catalog.has_schema("others"));
        let schema = catalog.get_schema("others").unwrap();
        assert!(schema.has_table("users"));
    }

    #[test]
    fn test_duplicate_schema() {
        let mut catalog = create_test_catalog();
        let result = catalog.add_schema(Schema::new("public"));
        assert!(matches!(result, Err(CatalogError::DuplicateSchema(_))));
    }

    #[test]
    fn test_add_and_get_stored_procedure() {
        let mut catalog = create_test_catalog();
        let proc = StoredProcedure::new(
            "test_proc".to_string(),
            vec![],
            vec![crate::stored_proc::StoredProcStatement::RawSql(
                "SELECT 1".to_string(),
            )],
        );
        catalog.add_stored_procedure(proc).unwrap();
        assert!(catalog.has_stored_procedure("test_proc"));
        let retrieved = catalog.get_stored_procedure("test_proc").unwrap();
        assert_eq!(retrieved.name, "test_proc");
    }

    #[test]
    fn test_duplicate_stored_procedure() {
        let mut catalog = create_test_catalog();
        let proc = StoredProcedure::new("test_proc".to_string(), vec![], vec![]);
        catalog.add_stored_procedure(proc.clone()).unwrap();
        let result = catalog.add_stored_procedure(proc);
        assert!(matches!(result, Err(CatalogError::DuplicateTable { .. })));
    }

    #[test]
    fn test_remove_stored_procedure() {
        let mut catalog = create_test_catalog();
        let proc = StoredProcedure::new("test_proc".to_string(), vec![], vec![]);
        catalog.add_stored_procedure(proc).unwrap();
        let removed = catalog.remove_stored_procedure("test_proc");
        assert!(removed.is_some());
        assert!(!catalog.has_stored_procedure("test_proc"));
    }

    #[test]
    fn test_set_default_schema() {
        let mut catalog = Catalog::new("test");
        catalog.add_schema(Schema::new("s1")).unwrap();
        catalog.add_schema(Schema::new("s2")).unwrap();
        catalog.set_default_schema("s2".to_string()).unwrap();
        assert_eq!(catalog.default_schema(), "s2");
    }

    #[test]
    fn test_set_default_schema_not_found() {
        let mut catalog = Catalog::new("test");
        let result = catalog.set_default_schema("nonexistent".to_string());
        assert!(matches!(result, Err(CatalogError::SchemaNotFound(_))));
    }

    #[test]
    fn test_set_default_database() {
        let mut catalog = Catalog::new("test");
        catalog.add_database(Database::new("db1")).unwrap();
        catalog.set_default_database("db1".to_string()).unwrap();
        assert_eq!(catalog.default_database(), "db1");
    }

    #[test]
    fn test_set_default_database_not_found() {
        let mut catalog = Catalog::new("test");
        let result = catalog.set_default_database("nonexistent".to_string());
        assert!(matches!(result, Err(CatalogError::SchemaNotFound(_))));
    }

    #[test]
    fn test_all_schemas() {
        let mut catalog = create_test_catalog();
        let schemas: Vec<_> = catalog.all_schemas();
    }

    #[test]
    fn test_has_database() {
        let mut catalog = Catalog::new("test");
        catalog.add_database(Database::new("db1")).unwrap();
        assert!(catalog.has_database("db1"));
        assert!(!catalog.has_database("nonexistent"));
    }

    #[test]
    fn test_has_schema() {
        let mut catalog = create_test_catalog();
        assert!(catalog.has_schema("public"));
        assert!(catalog.has_schema("others"));
        assert!(!catalog.has_schema("nonexistent"));
    }

    #[test]
    fn test_remove_stored_procedure_not_found() {
        let mut catalog = create_test_catalog();
        let removed = catalog.remove_stored_procedure("nonexistent");
        assert!(removed.is_none());
    }
}

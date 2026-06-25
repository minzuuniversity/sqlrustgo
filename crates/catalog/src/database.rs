//! Database definition for catalog
//!
//! Database -> Schema -> Table
//! A Database is a collection of schemas (default: just "public" schema).

use crate::error::{CatalogError, CatalogResult};
use crate::schema::Schema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A database containing multiple schemas.
///
/// In v3.10 multi-database mode, each database maps to a directory
/// under `data/` (e.g. `data/mydb/table1.tbl`). The single-database
/// v3.9.0 engine has one implicit "default" database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    /// Database name
    pub name: String,
    /// Schemas in this database (name -> Schema)
    schemas: HashMap<String, Schema>,
    /// Default schema name
    default_schema: String,
}

impl Database {
    /// Create a new empty database with a "public" schema.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let mut db = Self {
            name,
            schemas: HashMap::new(),
            default_schema: "public".to_string(),
        };
        // Always create the default public schema
        db.schemas
            .insert("public".to_string(), Schema::new("public"));
        db
    }

    /// Create a database with an explicit default schema name.
    pub fn with_default_schema(name: impl Into<String>, schema_name: impl Into<String>) -> Self {
        let name = name.into();
        let schema_name = schema_name.into();
        let mut db = Self {
            name,
            schemas: HashMap::new(),
            default_schema: schema_name.clone(),
        };
        db.schemas
            .insert(schema_name, Schema::new(&db.default_schema));
        db
    }

    // ============ Schema operations ============

    /// Add a schema to this database.
    pub fn add_schema(&mut self, schema: Schema) -> CatalogResult<()> {
        if self.schemas.contains_key(&schema.name) {
            return Err(CatalogError::DuplicateSchema(schema.name.clone()));
        }
        self.schemas.insert(schema.name.clone(), schema);
        Ok(())
    }

    /// Get a schema by name.
    pub fn get_schema(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    /// Get a mutable schema by name.
    pub fn get_schema_mut(&mut self, name: &str) -> Option<&mut Schema> {
        self.schemas.get_mut(name)
    }

    /// Check if a schema exists.
    pub fn has_schema(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// Get all schema names.
    pub fn schema_names(&self) -> Vec<&str> {
        self.schemas.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a schema (returns None if trying to remove the default schema).
    pub fn remove_schema(&mut self, name: &str) -> Option<Schema> {
        if name == self.default_schema {
            return None; // Cannot remove the default schema
        }
        self.schemas.remove(name)
    }

    /// Get the default schema name.
    pub fn default_schema(&self) -> &str {
        &self.default_schema
    }

    /// Set the default schema.
    pub fn set_default_schema(&mut self, name: String) -> CatalogResult<()> {
        if !self.schemas.contains_key(&name) {
            return Err(CatalogError::SchemaNotFound(name));
        }
        self.default_schema = name;
        Ok(())
    }

    /// Get all schemas.
    pub fn schemas(&self) -> &HashMap<String, Schema> {
        &self.schemas
    }

    /// Get the number of schemas.
    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::column::ColumnDefinition;
    use crate::data_type::DataType;
    use crate::Table;

    #[test]
    fn test_database_creation() {
        let db = Database::new("testdb");
        assert_eq!(db.name, "testdb");
        assert_eq!(db.schema_names(), vec!["public"]);
        assert_eq!(db.default_schema, "public");
    }

    #[test]
    fn test_database_with_default_schema() {
        let db = Database::with_default_schema("testdb", "myschema");
        assert_eq!(db.name, "testdb");
        assert_eq!(db.default_schema, "myschema");
        assert!(db.has_schema("myschema"));
    }

    #[test]
    fn test_add_and_get_schema() {
        let mut db = Database::new("testdb");
        let mut schema = Schema::new("users");
        let table = Table::new(
            "users_table",
            vec![ColumnDefinition::new("id", DataType::Integer)],
        );
        schema = schema.add_table(table).unwrap();

        db.add_schema(schema).unwrap();
        assert!(db.has_schema("users"));
        // HashMap iteration order is non-deterministic; check membership + count
        let names = db.schema_names();
        assert!(names.contains(&"public"), "should have public schema");
        assert!(names.contains(&"users"), "should have users schema");
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_duplicate_schema() {
        let mut db = Database::new("testdb");
        let schema = Schema::new("public");
        let result = db.add_schema(schema);
        assert!(matches!(result, Err(CatalogError::DuplicateSchema(_))));
    }

    #[test]
    fn test_remove_schema() {
        let mut db = Database::new("testdb");
        let schema = Schema::new("users");
        db.add_schema(schema).unwrap();

        let removed = db.remove_schema("users");
        assert!(removed.is_some());
        assert!(!db.has_schema("users"));
    }

    #[test]
    fn test_cannot_remove_default_schema() {
        let mut db = Database::new("testdb");
        let removed = db.remove_schema("public");
        assert!(removed.is_none());
        assert!(db.has_schema("public"));
    }

    #[test]
    fn test_set_default_schema() {
        let mut db = Database::new("testdb");
        let schema = Schema::new("others");
        db.add_schema(schema).unwrap();

        db.set_default_schema("others".to_string()).unwrap();
        assert_eq!(db.default_schema, "others");
    }

    #[test]
    fn test_set_default_schema_not_found() {
        let mut db = Database::new("testdb");
        let result = db.set_default_schema("nonexistent".to_string());
        assert!(matches!(result, Err(CatalogError::SchemaNotFound(_))));
    }
}

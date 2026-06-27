use sqlrustgo_agentsql::{
    catalog::{Catalog, CatalogColumn, CatalogIndex, CatalogTableSchema, ForeignKeyRef},
    schema::{SchemaService, ColumnSchema, TableSchema},
};

#[test]
fn test_schema_service_with_real_catalog() {
    // Create a real catalog with sample data
    let mut catalog = Catalog::new();

    // Add a sample table
    let columns = vec![
        CatalogColumn::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
            true,
            true,
            None,
            None,
            None,
            None,
            None,
        ),
        CatalogColumn::new(
            "name".to_string(),
            "VARCHAR".to_string(),
            false,
            false,
            false,
            None,
            Some(255),
            None,
            None,
            None,
        ),
    ];

    let indexes = vec![
        CatalogIndex::new(
            "PRIMARY".to_string(),
            vec!["id".to_string()],
            true,
            "BTREE".to_string(),
        ),
    ];

    let table = CatalogTableSchema::new(
        "users".to_string(),
        columns,
        indexes,
        Some("User accounts table".to_string()),
    );

    catalog.add_table("users".to_string(), table);

    // Create SchemaService with the catalog
    let schema_service = SchemaService::new(catalog);

    // Test get_schema
    let schema = schema_service.get_schema();
    assert!(schema.is_some());
    let schema_json = schema.unwrap();
    assert_eq!(schema_json["database"], "sqlrustgo");
    assert_eq!(schema_json["version"], "1.6.1");
    assert_eq!(schema_json["tables"].as_array().unwrap().len(), 1);
}

#[test]
fn test_schema_service_get_table() {
    let catalog = Catalog::new();
    let schema_service = SchemaService::new(catalog);

    // Add a table
    let columns = vec![
        CatalogColumn::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
            true,
            true,
            None,
            None,
            None,
            None,
            None,
        ),
    ];

    let indexes = vec![];
    let table = CatalogTableSchema::new(
        "test_table".to_string(),
        columns,
        indexes,
        None,
    );

    catalog.add_table("test_table".to_string(), table);

    // Test get_table_schema
    let table_schema = schema_service.get_table_schema("test_table");
    assert!(table_schema.is_some());
    let table_json = table_schema.unwrap();
    assert_eq!(table_json["name"], "test_table");
    assert_eq!(table_json["columns"].as_array().unwrap().len(), 1);
}

#[test]
fn test_schema_service_list_tables() {
    let catalog = Catalog::new();
    let schema_service = SchemaService::new(catalog);

    // Add a table
    let columns = vec![
        CatalogColumn::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
            true,
            true,
            None,
            None,
            None,
            None,
            None,
        ),
    ];

    let indexes = vec![];
    let table = CatalogTableSchema::new(
        "test_table".to_string(),
        columns,
        indexes,
        None,
    );

    catalog.add_table("test_table".to_string(), table);

    // Test list_tables
    let tables = schema_service.list_tables();
    assert_eq!(tables, vec!["test_table".to_string()]);
}

#[test]
fn test_schema_service_cache() {
    let catalog = Catalog::new();
    let schema_service = SchemaService::new(catalog);

    // First call - should read from catalog
    let schema1 = schema_service.get_schema();
    assert!(schema1.is_some());

    // Second call - should use cache
    let schema2 = schema_service.get_schema();
    assert!(schema2.is_some());
    assert_eq!(schema1, schema2);

    // Clear cache
    schema_service.clear_cache();

    // Third call - should read from catalog again
    let schema3 = schema_service.get_schema();
    assert!(schema3.is_some());
}

#[test]
fn test_schema_service_debug() {
    let catalog = Catalog::new();
    let schema_service = SchemaService::new(catalog);

    // Test Debug implementation
    let _debug = format!("{:?}", schema_service);
}

#[test]
fn test_catalog_operations() {
    let mut catalog = Catalog::new();

    // Test add and get table
    let columns = vec![
        CatalogColumn::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
            true,
            true,
            None,
            None,
            None,
            None,
            None,
        ),
    ];

    let indexes = vec![];
    let table = CatalogTableSchema::new(
        "test".to_string(),
        columns,
        indexes,
        None,
    );

    catalog.add_table("test".to_string(), table);

    let retrieved = catalog.get_table("test");
    assert!(retrieved.is_some());

    // Test remove table
    let removed = catalog.remove_table("test");
    assert!(removed.is_some());
    assert!(catalog.get_table("test").is_none());

    // Test clear
    catalog.clear();
    assert_eq!(catalog.tables().count(), 0);
}

#[test]
fn test_foreign_key_reference() {
    let fk = ForeignKeyRef::new(
        "users".to_string(),
        "id".to_string(),
        Some("CASCADE".to_string()),
        Some("NO ACTION".to_string()),
    );

    assert_eq!(fk.table(), "users");
    assert_eq!(fk.column(), "id");
    assert_eq!(fk.on_delete(), Some("CASCADE"));
    assert_eq!(fk.on_update(), Some("NO ACTION"));
}

#[test]
fn test_column_metadata() {
    let col = CatalogColumn::new(
        "name".to_string(),
        "VARCHAR".to_string(),
        true,
        false,
        false,
        Some("NULL".to_string()),
        Some(255),
        None,
        None,
        None,
    );

    assert_eq!(col.name(), "name");
    assert_eq!(col.data_type(), "VARCHAR");
    assert!(col.nullable());
    assert!(!col.is_primary_key());
    assert_eq!(col.default_value(), Some("NULL"));
    assert_eq!(col.max_length(), Some(255));
}

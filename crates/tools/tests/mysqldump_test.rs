//! mysqldump crate unit tests

use sqlrustgo_tools::mysqldump::{
    ColumnDef, DumpImporter, ForeignKeyRef, ImportMode, ImportStats, SqlStatement,
};

#[test]
fn test_import_stats_default() {
    let stats = ImportStats::default();
    assert_eq!(stats.tables_created, 0);
    assert_eq!(stats.rows_inserted, 0);
    assert_eq!(stats.errors, 0);
    assert!(stats.warnings.is_empty());
}

#[test]
fn test_import_stats_increment() {
    let mut stats = ImportStats::default();
    stats.tables_created += 2;
    stats.rows_inserted += 100;
    stats.errors += 1;
    stats.warnings.push("truncated".to_string());
    assert_eq!(stats.tables_created, 2);
    assert_eq!(stats.errors, 1);
    assert_eq!(stats.warnings.len(), 1);
}

#[test]
fn test_sql_statement_variants() {
    let stmts: Vec<SqlStatement> = vec![
        SqlStatement::DropTable {
            name: "t1".to_string(),
            if_exists: false,
        },
        SqlStatement::CreateTable {
            name: "t2".to_string(),
            columns: vec![],
        },
        SqlStatement::Insert {
            table: "t3".to_string(),
            columns: vec![],
            values: vec![],
        },
        SqlStatement::Use {
            database: "testdb".to_string(),
        },
        SqlStatement::Unknown("SET foreign_key_checks=0".to_string()),
    ];
    assert_eq!(stmts.len(), 5);
    match &stmts[1] {
        SqlStatement::CreateTable { name, columns } => {
            assert_eq!(name, "t2");
            assert!(columns.is_empty());
        }
        _ => panic!("expected CreateTable"),
    }
}

#[test]
fn test_column_def() {
    let col = ColumnDef {
        name: "id".to_string(),
        data_type: "INT".to_string(),
        nullable: false,
        primary_key: true,
        auto_increment: true,
        unique: false,
        default: None,
        references: None,
    };
    assert_eq!(col.name, "id");
    assert!(!col.nullable);
    assert!(col.primary_key);
}

#[test]
fn test_column_def_with_default() {
    let col = ColumnDef {
        name: "name".to_string(),
        data_type: "VARCHAR(255)".to_string(),
        nullable: true,
        primary_key: false,
        auto_increment: false,
        unique: false,
        default: Some("'unnamed'".to_string()),
        references: None,
    };
    assert_eq!(col.default.as_deref(), Some("'unnamed'"));
}

#[test]
fn test_foreign_key_ref() {
    let fk = ForeignKeyRef {
        table: "orders".to_string(),
        column: "customer_id".to_string(),
    };
    assert_eq!(fk.table, "orders");
    assert_eq!(fk.column, "customer_id");
}

#[test]
fn test_import_mode_variants() {
    use sqlrustgo_tools::mysqldump::ImportMode::*;
    assert!(matches!(ImportMode::Full, ImportMode::Full));
    assert!(matches!(ImportMode::SchemaOnly, ImportMode::SchemaOnly));
    assert!(matches!(ImportMode::DataOnly, ImportMode::DataOnly));
}

#[test]
fn test_dump_importer_new() {
    let importer = DumpImporter::new(ImportMode::Full, false);
    assert_eq!(importer.stats().tables_created, 0);
    assert!(importer.statements().is_empty());
}

#[test]
fn test_dump_importer_verbose() {
    let importer = DumpImporter::new(ImportMode::SchemaOnly, true);
    assert_eq!(importer.stats().errors, 0);
}

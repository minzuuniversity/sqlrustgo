//! mysqldump crate unit tests

use sqlrustgo_tools::mysqldump::{
    ColumnDef, DumpImporter, ForeignKeyRef, ImportMode, ImportStats, SqlStatement,
};
use std::io::Cursor;

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
        SqlStatement::DropTable { name: "t1".to_string(), if_exists: false },
        SqlStatement::CreateTable { name: "t2".to_string(), columns: vec![] },
        SqlStatement::Insert { table: "t3".to_string(), columns: vec![], values: vec![] },
        SqlStatement::Use { database: "testdb".to_string() },
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
    let fk = ForeignKeyRef { table: "orders".to_string(), column: "customer_id".to_string() };
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

// ---------------------------------------------------------------------------
// DumpImporter::import_reader tests
// ---------------------------------------------------------------------------

#[test]
fn test_import_reader_create_table() {
    let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(255));\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.stats().tables_created, 1);
    assert_eq!(importer.stats().queries_executed, 1);
    assert_eq!(importer.statements().len(), 1);
}

#[test]
fn test_import_reader_insert() {
    let sql = "INSERT INTO users (id, name) VALUES (1, 'alice');\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.stats().queries_executed, 1);
    assert_eq!(importer.statements().len(), 1);
    match &importer.statements()[0] {
        SqlStatement::Insert { table, columns, values } => {
            assert_eq!(table, "users");
            assert_eq!(columns, &["id", "name"]);
            assert_eq!(values.len(), 1);
        }
        _ => panic!("expected Insert"),
    }
}

#[test]
fn test_import_reader_drop_table() {
    let sql = "DROP TABLE users;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.statements().len(), 1);
    match &importer.statements()[0] {
        SqlStatement::DropTable { name, if_exists } => {
            assert_eq!(name, "users");
            assert!(!*if_exists);
        }
        _ => panic!("expected DropTable"),
    }
}

#[test]
fn test_import_reader_drop_table_if_exists() {
    let sql = "DROP TABLE IF EXISTS users;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    match &importer.statements()[0] {
        SqlStatement::DropTable { name, if_exists } => {
            assert_eq!(name, "users");
            assert!(*if_exists);
        }
        _ => panic!("expected DropTable"),
    }
}

#[test]
fn test_import_reader_use_database() {
    let sql = "USE mydb;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.current_database(), &Some("mydb".to_string()));
    assert_eq!(importer.statements().len(), 1);
    match &importer.statements()[0] {
        SqlStatement::Use { database } => assert_eq!(database, "mydb"),
        _ => panic!("expected Use"),
    }
}

#[test]
fn test_import_reader_empty_input() {
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new("")).unwrap();
    assert_eq!(importer.stats().queries_executed, 0);
    assert!(importer.statements().is_empty());
}

#[test]
fn test_import_reader_lock_tables() {
    let sql = "LOCK TABLES users READ;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.statements().len(), 1);
    match &importer.statements()[0] {
        SqlStatement::LockTables { tables } => assert_eq!(tables, &["users"]),
        _ => panic!("expected LockTables"),
    }
}

#[test]
fn test_import_reader_unlock_tables() {
    let sql = "UNLOCK TABLES;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.statements().len(), 1);
    assert!(matches!(&importer.statements()[0], SqlStatement::UnlockTables));
}

#[test]
fn test_import_reader_transaction_statements() {
    let sql = "BEGIN;\nINSERT INTO t VALUES(1);\nCOMMIT;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.statements().len(), 3);
    assert!(matches!(&importer.statements()[0], SqlStatement::Begin));
    assert!(matches!(&importer.statements()[1], SqlStatement::Insert { .. }));
    assert!(matches!(&importer.statements()[2], SqlStatement::Commit));
}

#[test]
fn test_import_reader_unknown_statement() {
    let sql = "SELECT * FROM users;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.statements().len(), 1);
    assert!(matches!(&importer.statements()[0], SqlStatement::Unknown(_)));
}

#[test]
fn test_import_reader_multiple_statements() {
    let sql = "CREATE TABLE t1 (id INT);\nCREATE TABLE t2 (id INT);\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.stats().tables_created, 2);
    assert_eq!(importer.statements().len(), 2);
}

#[test]
fn test_import_reader_multiline_insert() {
    let sql = "INSERT INTO t VALUES\n(1, 'alice'),\n(2, 'bob');\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.stats().queries_executed, 1);
    assert_eq!(importer.statements().len(), 1);
}

#[test]
fn test_import_reader_no_semicolon() {
    let sql = "USE mydb";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.current_database(), &Some("mydb".to_string()));
}

#[test]
fn test_import_reader_blank_lines() {
    let sql = "CREATE TABLE t (id INT);\n\n\nINSERT INTO t VALUES(1);\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert_eq!(importer.stats().tables_created, 1);
    assert_eq!(importer.stats().queries_executed, 2);
}

#[test]
fn test_import_reader_rollback() {
    let sql = "ROLLBACK;\n";
    let mut importer = DumpImporter::new(ImportMode::Full, false);
    importer.import_reader(Cursor::new(sql)).unwrap();
    assert!(matches!(&importer.statements()[0], SqlStatement::Rollback));
}

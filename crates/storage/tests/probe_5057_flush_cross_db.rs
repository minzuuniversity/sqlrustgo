//! #5057 probe: where does `flush` actually put the rows?
//!
//! Prints only — asserts nothing, so it does not lock in current behaviour.
//! Its job is to show which directory each dirty table's rows reach, and
//! what `dirty_tables` currently contains after writes to two databases.

use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};

fn tbl(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

fn row(n: i64) -> Vec<sqlrustgo_storage::Value> {
    vec![Value::Integer(n)]
}

fn show_dir(root: &std::path::Path, label: &str) {
    println!("--- on disk under {label} ---");
    let mut any = false;
    let mut stack = vec![root.to_path_buf()];
    let mut found: Vec<String> = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string();
                let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                let body = std::fs::read_to_string(&p).unwrap_or_default();
                let rows = body.matches("\"id\"").count();
                found.push(format!("    {rel}  {size}B  id-occurrences={rows}"));
                any = true;
            }
        }
    }
    if !any {
        println!("    (empty)");
    }
    for line in found {
        println!("{line}");
    }
}

#[test]
fn probe_two_databases_same_table_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let mut storage = FileStorage::new(root.clone()).expect("open");

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");

    // Both databases hold a table with the SAME name.
    storage
        .create_table_in_db("d1", &tbl("t"))
        .expect("create d1.t");
    storage
        .create_table_in_db("d2", &tbl("t"))
        .expect("create d2.t");

    // Rows that are unmistakably different, so a misrouted write shows up.
    storage
        .insert_in_db("d1", "t", vec![row(11), row(12), row(13)])
        .expect("insert d1.t");
    storage
        .insert_in_db("d2", "t", vec![row(21), row(22), row(23)])
        .expect("insert d2.t");

    // Select the database whose flush will run.
    storage.set_current_db("d1").expect("use d1");

    println!("current_db is now d1; flushing");

    // No assertion: show where the rows land.
    let flushed = storage.flush();
    println!("flush() -> {flushed:?}");

    // What does the storage still report, in memory?
    let d1 = storage.scan_in_db("d1", "t").expect("scan d1.t").len();
    let d2 = storage.scan_in_db("d2", "t").expect("scan d2.t").len();
    println!("in memory: d1.t={d1} rows, d2.t={d2} rows");

    show_dir(&root, "data_dir");

    // Reopen from scratch: what a restart would actually load.
    drop(storage);
    println!("=== raw d1/t.json ===");
    println!(
        "{}",
        std::fs::read_to_string(root.join("d1/t.json")).unwrap_or_default()
    );
    println!("=== raw d2/t.json ===");
    println!(
        "{}",
        std::fs::read_to_string(root.join("d2/t.json")).unwrap_or_default()
    );
    println!("=== any .delta files ===");
    for e in std::fs::read_dir(&root).unwrap().flatten() {
        println!("  top: {:?}", e.path());
    }
    let reopened = FileStorage::new(root.clone()).expect("reopen");
    let rd1 = reopened.scan("t").map(|v| v.len()).unwrap_or(usize::MAX);
    println!("after reopen, scan(\"t\") on current_db={rd1} rows");
    let rd2 = reopened
        .scan_in_db("d2", "t")
        .map(|v| v.len())
        .unwrap_or(usize::MAX);
    println!("after reopen, scan_in_db(\"d2\",\"t\") = {rd2} rows");
}

#[test]
fn probe_update_path_dirty_marker() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let mut storage = FileStorage::new(root.clone()).expect("open");

    storage.create_database("d1").expect("create d1");
    storage.create_database("d2").expect("create d2");
    storage
        .create_table_in_db("d1", &tbl("t"))
        .expect("create d1.t");
    storage
        .create_table_in_db("d2", &tbl("t"))
        .expect("create d2.t");
    storage.insert_in_db("d1", "t", vec![row(11)]).expect("i1");
    storage.insert_in_db("d2", "t", vec![row(21)]).expect("i2");

    // Delete one row from d2 only, via the db-named path.
    let _ = storage.delete_in_db("d2", "t", &[Value::Integer(21)]);

    storage.set_current_db("d1").expect("use d1");
    println!("current_db is d1; a delete touched d2.t");
    let flushed = storage.flush();
    println!("flush() -> {flushed:?}");
    show_dir(&root, "data_dir after cross-db delete + flush from d1");
}

// B2.2 reroute — the live flush path is the `StorageEngine::flush` trait
// override, and both `flush` implementations must behave identically.
//
// docs/releases/v4.1.0/PERF_B22_CONCURRENT_MEASUREMENT.md §3 established
// that B2.2 optimised `FileStorage::flush(&self)` (inherent) while the
// server reaches the `StorageEngine::flush(&mut self)` override through
// `MvccStorage` — so the optimised copy had no production caller. The
// override carried its own `self.tables.get(&name).cloned()`, a whole
// `TableData` copy per dirty table, on the live path.
//
// These tests pin the behaviour that was wrong or untested before:
//
// 1. `flush_parallel` with 1-2 dirty tables used to drain the dirty set
//    and then delegate to `flush()`, which saw an empty set and
//    persisted nothing. The rows were silently dropped.
// 2. The 3+ branch read `self.tables` from spawned threads without
//    holding `write_lock`, which guards that plain HashMap.
// 3. The trait override copied whole tables; the window is what actually
//    gets written, and both entry points must produce the same result.

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::PathBuf;

fn table_info(name: &str) -> sqlrustgo_storage::engine::TableInfo {
    sqlrustgo_storage::engine::TableInfo {
        name: name.to_string(),
        columns: vec![sqlrustgo_storage::engine::ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            primary_key: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn setup(dir: &str, tables: &[&str]) -> FileStorage {
    let dir = PathBuf::from(dir);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = FileStorage::new(dir).unwrap();
    for t in tables {
        s.create_table(&table_info(t)).unwrap();
    }
    s
}

fn row_count(dir: &str, table: &str) -> usize {
    let p = PathBuf::from(dir).join(format!("{}.json", table));
    let txt = std::fs::read_to_string(&p).expect("snapshot must exist");
    // Count row objects without pulling in serde: the file is pretty
    // printed, so rows are the only place `"k"`-style value objects
    // appear. Counting `[` at the row-array level is fragile, so parse.
    let v: serde_json::Value = serde_json::from_str(&txt).expect("valid JSON");
    v["rows"].as_array().map(|a| a.len()).unwrap_or(0)
}

#[test]
fn flush_parallel_persists_one_table() {
    // Pre-fix: dirty set drained, then `self.flush()` saw it empty.
    let dir = "/tmp/b2r_parallel_1";
    let mut s = setup(dir, &["t1"]);
    s.insert("t1", vec![vec![Value::Integer(1)], vec![Value::Integer(2)]])
        .unwrap();

    s.flush_parallel().unwrap();

    assert_eq!(
        row_count(dir, "t1"),
        2,
        "flush_parallel with 1 dirty table must persist its rows; the \\
         pre-fix path drained the dirty set and then called flush(), \\
         which saw an empty set and wrote nothing"
    );
}

#[test]
fn flush_parallel_persists_two_tables() {
    let dir = "/tmp/b2r_parallel_2";
    let mut s = setup(dir, &["t1", "t2"]);
    s.insert("t1", vec![vec![Value::Integer(1)]]).unwrap();
    s.insert("t2", vec![vec![Value::Integer(2)]]).unwrap();

    s.flush_parallel().unwrap();

    assert_eq!(row_count(dir, "t1"), 1);
    assert_eq!(row_count(dir, "t2"), 1);
}

#[test]
fn flush_parallel_persists_three_or_more_tables() {
    // The parallel branch. It used to read `self.tables` from spawned
    // threads without `write_lock`, which guards that HashMap.
    let dir = "/tmp/b2r_parallel_5";
    let names = ["t1", "t2", "t3", "t4", "t5"];
    let mut s = setup(dir, &names);
    for (i, t) in names.iter().enumerate() {
        let rows: Vec<Vec<Value>> = (0..(i + 1) as i64)
            .map(|v| vec![Value::Integer(v)])
            .collect();
        s.insert(t, rows).unwrap();
    }

    s.flush_parallel().unwrap();

    for (i, t) in names.iter().enumerate() {
        assert_eq!(
            row_count(dir, t),
            i + 1,
            "{} must have {} rows on disk",
            t,
            i + 1
        );
    }
}

#[test]
fn trait_flush_and_inherent_flush_persist_the_same_rows() {
    // The two entry points are now one implementation. This pins that
    // they agree, which is what B2.2's split silently violated.
    let dir_trait = "/tmp/b2r_agree_trait";
    let dir_inherent = "/tmp/b2r_agree_inherent";

    let mut a = setup(dir_trait, &["t"]);
    a.insert(
        "t",
        vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
            vec![Value::Integer(3)],
        ],
    )
    .unwrap();
    StorageEngine::flush(&mut a).unwrap();

    let mut b = setup(dir_inherent, &["t"]);
    b.insert(
        "t",
        vec![
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
            vec![Value::Integer(3)],
        ],
    )
    .unwrap();
    b.flush().unwrap();

    assert_eq!(row_count(dir_trait, "t"), 3);
    assert_eq!(row_count(dir_inherent, "t"), 3);
}

#[test]
fn trait_flush_appends_a_delta_rather_than_rewriting() {
    // The window is the point: a second flush of a grown table writes
    // only the new rows to the delta file, so the base snapshot is not
    // rewritten. A whole-table copy or a full rewrite would show up here
    // as a changed snapshot file.
    let dir = "/tmp/b2r_delta";
    let mut s = setup(dir, &["t"]);
    s.insert("t", vec![vec![Value::Integer(1)]]).unwrap();
    StorageEngine::flush(&mut s).unwrap();

    let base_after_first = std::fs::read_to_string(PathBuf::from(dir).join("t.json")).unwrap();
    let base_mtime = std::fs::metadata(PathBuf::from(dir).join("t.json"))
        .unwrap()
        .modified()
        .unwrap();

    s.insert("t", vec![vec![Value::Integer(2)], vec![Value::Integer(3)]])
        .unwrap();
    StorageEngine::flush(&mut s).unwrap();

    let base_after_second = std::fs::read_to_string(PathBuf::from(dir).join("t.json")).unwrap();
    let mtime_after = std::fs::metadata(PathBuf::from(dir).join("t.json"))
        .unwrap()
        .modified()
        .unwrap();

    assert_eq!(
        base_after_first, base_after_second,
        "the base snapshot must not be rewritten when only a delta is needed"
    );
    assert_eq!(
        base_mtime, mtime_after,
        "base snapshot mtime must be unchanged: a rewrite would mean the \\
         whole table was re-serialised instead of just the appended window"
    );

    // And the new rows must be in the delta.
    let delta = std::fs::read_to_string(PathBuf::from(dir).join("t.delta")).unwrap();
    assert!(
        delta.contains("2") && delta.contains("3"),
        "delta must carry the appended rows, got {:?}",
        delta
    );
}

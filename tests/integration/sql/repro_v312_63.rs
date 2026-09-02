use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, FileStorage, MemoryStorage};
use std::sync::Arc;

fn fresh_file() -> ExecutionEngine<FileStorage> {
    let dir = std::env::temp_dir().join(format!(
        "v312_63_repro_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let storage = Arc::new(RwLock::new(
        FileStorage::new_with_buffer_config(dir.clone(), 10_000, false).unwrap(),
    ));
    let catalog = sqlrustgo_catalog::Catalog::new("main");
    ExecutionEngine::with_catalog(storage, Arc::new(RwLock::new(catalog)))
}

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn repro_4624_trigger_fires_on_insert_file() {
    let mut x = fresh_file();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    x.execute("CREATE TABLE log(id INT PRIMARY KEY, msg TEXT)")
        .unwrap();
    x.execute("CREATE TRIGGER tr_before BEFORE INSERT ON t FOR EACH ROW BEGIN INSERT INTO log(msg) VALUES ('before insert'); END").unwrap();
    x.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    let r = x.execute("SELECT count(*) FROM log").unwrap();
    println!("log count = {:?}", r);
    assert_eq!(r.rows.len(), 1);
    match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => {
            assert_eq!(*n, 1, "trigger should have fired and inserted 1 log row")
        }
        other => panic!("expected Integer, got {:?}", other),
    }
}

#[test]
fn repro_4624_trigger_fires_on_insert_mem() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    x.execute("CREATE TABLE log(id INT PRIMARY KEY, msg TEXT)")
        .unwrap();
    x.execute("CREATE TRIGGER tr_before BEFORE INSERT ON t FOR EACH ROW BEGIN INSERT INTO log(msg) VALUES ('before insert'); END").unwrap();
    x.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    let r = x.execute("SELECT count(*) FROM log").unwrap();
    println!("log count (mem) = {:?}", r);
    assert_eq!(r.rows.len(), 1);
    match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => assert_eq!(*n, 1),
        other => panic!("expected Integer, got {:?}", other),
    }
}

// ============================================================================
// Issue #4637 — VARCHAR(N) / CHAR(N) length validation
// ============================================================================

#[test]
fn repro_4637_varchar_insert_overlong_rejected() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, c5 VARCHAR(5))").unwrap();
    // Overlong string must be rejected.
    let r = x.execute("INSERT INTO t VALUES (1, 'abcdef')");
    assert!(r.is_err(), "overlong INSERT should error, got: {:?}", r);
    let err = r.unwrap_err().to_string();
    assert!(
        err.contains("c5") && err.contains("length"),
        "error message should mention column name and length, got: {}",
        err
    );
}

#[test]
fn repro_4637_varchar_insert_short_accepted() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, c5 VARCHAR(5))").unwrap();
    x.execute("INSERT INTO t VALUES (1, 'abc')").unwrap();
    let r = x.execute("SELECT c5 FROM t WHERE id = 1").unwrap();
    assert_eq!(r.rows.len(), 1);
    match &r.rows[0][0] {
        sqlrustgo::Value::Text(s) => assert_eq!(s, "abc"),
        other => panic!("expected Text('abc'), got {:?}", other),
    }
}

#[test]
fn repro_4637_varchar_insert_exact_max_accepted() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, c5 VARCHAR(5))").unwrap();
    x.execute("INSERT INTO t VALUES (1, 'abcde')").unwrap(); // exactly 5 chars
    let r = x.execute("SELECT c5 FROM t WHERE id = 1").unwrap();
    assert_eq!(r.rows.len(), 1);
    match &r.rows[0][0] {
        sqlrustgo::Value::Text(s) => assert_eq!(s, "abcde"),
        other => panic!("expected Text('abcde'), got {:?}", other),
    }
}

#[test]
fn repro_4637_char_insert_overlong_rejected() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, c CHAR(3))").unwrap();
    let r = x.execute("INSERT INTO t VALUES (1, 'abcd')");
    assert!(
        r.is_err(),
        "overlong CHAR INSERT should error, got: {:?}",
        r
    );
}

#[test]
fn repro_4637_char_insert_short_padded() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, c CHAR(5))").unwrap();
    x.execute("INSERT INTO t VALUES (1, 'ab')").unwrap(); // short → padded
    let r = x.execute("SELECT c FROM t WHERE id = 1").unwrap();
    assert_eq!(r.rows.len(), 1);
    match &r.rows[0][0] {
        sqlrustgo::Value::Text(s) => assert_eq!(s, "ab   ", "CHAR(5) must right-pad with spaces"),
        other => panic!("expected Text('ab   '), got {:?}", other),
    }
}

#[test]
fn repro_4637_update_overlong_rejected() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY, c5 VARCHAR(5))")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 'abc')").unwrap();
    let r = x.execute("UPDATE t SET c5 = 'abcdef' WHERE id = 1");
    assert!(r.is_err(), "overlong UPDATE should error, got: {:?}", r);
}

#[test]
fn repro_4637_odku_overlong_rejected() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT PRIMARY KEY, c5 VARCHAR(5))")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1, 'abc')").unwrap();
    let r = x.execute("INSERT INTO t VALUES (1, 'x') ON DUPLICATE KEY UPDATE c5 = 'abcdef'");
    assert!(r.is_err(), "overlong ODKU should error, got: {:?}", r);
}

#[test]
fn repro_4637_varchar_no_cap_unaffected() {
    // VARCHAR without (N) → no length cap → 'abcdef' is fine.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT, c VARCHAR)").unwrap();
    x.execute("INSERT INTO t VALUES (1, 'abcdef')").unwrap();
    let r = x.execute("SELECT c FROM t WHERE id = 1").unwrap();
    match &r.rows[0][0] {
        sqlrustgo::Value::Text(s) => assert_eq!(s, "abcdef"),
        other => panic!("expected Text('abcdef'), got {:?}", other),
    }
}

// ============================================================================
// Issue #4638 — CREATE VIEW actually creates a queryable view
// ============================================================================

#[test]
fn repro_4638_create_view_returns_rows() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE base(id INT, val INT)").unwrap();
    x.execute("INSERT INTO base VALUES (1, 100), (2, 200), (3, 300)")
        .unwrap();
    x.execute("CREATE VIEW v AS SELECT id, val FROM base WHERE val >= 150")
        .unwrap();
    let r = x.execute("SELECT id, val FROM v ORDER BY id").unwrap();
    assert_eq!(r.rows.len(), 2, "view should yield 2 rows");
    // Row 1: id=2, val=200
    match (&r.rows[0][0], &r.rows[0][1]) {
        (sqlrustgo::Value::Integer(a), sqlrustgo::Value::Integer(b)) => {
            assert_eq!(*a, 2);
            assert_eq!(*b, 200);
        }
        other => panic!("expected (Integer(2), Integer(200)), got {:?}", other),
    }
    // Row 2: id=3, val=300
    match (&r.rows[1][0], &r.rows[1][1]) {
        (sqlrustgo::Value::Integer(a), sqlrustgo::Value::Integer(b)) => {
            assert_eq!(*a, 3);
            assert_eq!(*b, 300);
        }
        other => panic!("expected (Integer(3), Integer(300)), got {:?}", other),
    }
}

#[test]
fn repro_4638_create_view_with_star_expands() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE base(id INT, val INT)").unwrap();
    x.execute("INSERT INTO base VALUES (1, 100)").unwrap();
    x.execute("CREATE VIEW v AS SELECT * FROM base").unwrap();
    let r = x.execute("SELECT * FROM v").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0].len(), 2);
}

#[test]
fn repro_4638_create_view_file_storage() {
    // FileStorage SqliteMode path: the view is stored in-memory per
    // ExecutionEngine instance; verify it resolves there too.
    let mut x = fresh_file();
    x.execute("CREATE TABLE base(id INT, val INT)").unwrap();
    x.execute("INSERT INTO base VALUES (1, 42)").unwrap();
    x.execute("CREATE VIEW v AS SELECT val FROM base").unwrap();
    let r = x.execute("SELECT val FROM v").unwrap();
    assert_eq!(r.rows.len(), 1);
    match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => assert_eq!(*n, 42),
        other => panic!("expected Integer(42), got {:?}", other),
    }
}

#[test]
fn repro_4638_create_view_with_column_alias() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE base(id INT, val INT)").unwrap();
    x.execute("INSERT INTO base VALUES (1, 100)").unwrap();
    x.execute("CREATE VIEW v(a, b) AS SELECT id, val FROM base")
        .unwrap();
    let r = x.execute("SELECT a, b FROM v").unwrap();
    assert_eq!(r.rows.len(), 1);
    match (&r.rows[0][0], &r.rows[0][1]) {
        (sqlrustgo::Value::Integer(a), sqlrustgo::Value::Integer(b)) => {
            assert_eq!(*a, 1);
            assert_eq!(*b, 100);
        }
        other => panic!("expected (Integer(1), Integer(100)), got {:?}", other),
    }
}

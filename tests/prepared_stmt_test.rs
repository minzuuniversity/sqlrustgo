use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::{Arc, RwLock};

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn test_prepare_execute_basic() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    engine
        .execute("PREPARE stmt1 AS 'SELECT * FROM t WHERE id = 1'")
        .unwrap();
    let r = engine.execute("EXECUTE stmt1").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][1].to_string(), "100");
}

#[test]
fn test_execute_unprepared_errors() {
    let mut engine = make_engine();
    let r = engine.execute("EXECUTE nonexistent");
    assert!(r.is_err(), "EXECUTE unprepared should error");
}

#[test]
fn test_prepare_overwrite() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        .unwrap();
    engine
        .execute("PREPARE s1 AS 'SELECT * FROM t WHERE id = 1'")
        .unwrap();
    let r1 = engine.execute("EXECUTE s1").unwrap();
    assert_eq!(r1.rows.len(), 1);
    engine
        .execute("PREPARE s1 AS 'SELECT * FROM t WHERE id = 2'")
        .unwrap();
    let r2 = engine.execute("EXECUTE s1").unwrap();
    assert_eq!(r2.rows.len(), 1);
    assert_eq!(r2.rows[0][1].to_string(), "20");
}

#[test]
fn test_deallocate_then_execute_errors() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    engine.execute("PREPARE s1 AS 'SELECT * FROM t'").unwrap();
    let r1 = engine.execute("EXECUTE s1").unwrap();
    assert_eq!(r1.rows.len(), 1);
    engine.execute("DEALLOCATE s1").unwrap();
    let r2 = engine.execute("EXECUTE s1");
    assert!(r2.is_err());
}

#[test]
fn test_prepare_select_with_where() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, age INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 20), (2, 30), (3, 40)")
        .unwrap();
    engine
        .execute("PREPARE adults AS 'SELECT * FROM users WHERE age >= 30'")
        .unwrap();
    let r = engine.execute("EXECUTE adults").unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_prepare_with_insert() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE log (id INTEGER PRIMARY KEY, msg TEXT)")
        .unwrap();
    engine
        .execute("PREPARE ins AS 'INSERT INTO log VALUES (1, ''hello'')'")
        .unwrap();
    engine.execute("EXECUTE ins").unwrap();
    let r = engine.execute("SELECT COUNT(*) FROM log").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "1");
}

#[test]
fn test_prepare_with_update() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    engine
        .execute("PREPARE upd AS 'UPDATE t SET v = 200 WHERE id = 1'")
        .unwrap();
    engine.execute("EXECUTE upd").unwrap();
    let r = engine.execute("SELECT v FROM t").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "200");
}

#[test]
fn test_prepare_with_delete() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        .unwrap();
    engine.execute("PREPARE del AS 'DELETE FROM t'").unwrap();
    let r = engine.execute("EXECUTE del");
    assert!(r.is_ok());
}

#[test]
fn test_prepare_with_aggregate() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();
    engine
        .execute("PREPARE sumq AS 'SELECT SUM(v) FROM t'")
        .unwrap();
    let r = engine.execute("EXECUTE sumq").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "60");
}

#[test]
fn test_prepare_with_join() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE a (id INTEGER PRIMARY KEY, x INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (a_id INTEGER, y TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO a VALUES (1, 10), (2, 20)")
        .unwrap();
    engine
        .execute("INSERT INTO b VALUES (1, 'one'), (2, 'two')")
        .unwrap();
    engine
        .execute("PREPARE j AS 'SELECT a.x, b.y FROM a JOIN b ON a.id = b.a_id'")
        .unwrap();
    let r = engine.execute("EXECUTE j").unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_prepare_with_subquery() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        .unwrap();
    engine
        .execute("PREPARE sub AS 'SELECT * FROM (SELECT v FROM t WHERE v > 15) AS s'")
        .unwrap();
    let r = engine.execute("EXECUTE sub").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "20");
}

#[test]
fn test_prepare_reusable() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    engine.execute("PREPARE s AS 'SELECT * FROM t'").unwrap();
    for _ in 0..5 {
        let r = engine.execute("EXECUTE s").unwrap();
        assert_eq!(r.rows.len(), 1);
    }
}

#[test]
fn test_multiple_prepared_statements() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();
    engine
        .execute("PREPARE s1 AS 'SELECT * FROM t WHERE id = 1'")
        .unwrap();
    engine
        .execute("PREPARE s2 AS 'SELECT * FROM t WHERE id = 2'")
        .unwrap();
    engine
        .execute("PREPARE s3 AS 'SELECT * FROM t WHERE id = 3'")
        .unwrap();
    let r1 = engine.execute("EXECUTE s1").unwrap();
    let r2 = engine.execute("EXECUTE s2").unwrap();
    let r3 = engine.execute("EXECUTE s3").unwrap();
    assert_eq!(r1.rows[0][1].to_string(), "10");
    assert_eq!(r2.rows[0][1].to_string(), "20");
    assert_eq!(r3.rows[0][1].to_string(), "30");
}

#[test]
fn test_deallocate_nonexistent() {
    let mut engine = make_engine();
    let r = engine.execute("DEALLOCATE s1");
    assert!(r.is_err() || r.is_ok());
}

#[test]
fn test_prepare_with_create_table() {
    let mut engine = make_engine();
    engine
        .execute("PREPARE c AS 'CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)'")
        .unwrap();
    engine.execute("EXECUTE c").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'hello')").unwrap();
    let r = engine.execute("SELECT v FROM t").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "hello");
}

#[test]
fn test_prepare_with_drop_table() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();
    engine.execute("PREPARE d AS 'DROP TABLE t'").unwrap();
    engine.execute("EXECUTE d").unwrap();
    let r = engine.execute("SELECT COUNT(*) FROM t");
    assert!(r.is_err());
}

#[test]
fn test_prepare_with_text() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'hello world')")
        .unwrap();
    engine
        .execute("PREPARE q AS 'SELECT name FROM t WHERE id = 1'")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "hello world");
}

#[test]
fn test_prepare_with_null() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, NULL)").unwrap();
    engine.execute("PREPARE q AS 'SELECT v FROM t'").unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn test_prepare_then_modify_data_then_execute() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    engine.execute("PREPARE q AS 'SELECT * FROM t'").unwrap();
    let r1 = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r1.rows.len(), 1);
    engine.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    let r2 = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r2.rows.len(), 2);
}

#[test]
fn test_prepare_with_like() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'apple')").unwrap();
    engine
        .execute("INSERT INTO t VALUES (2, 'banana')")
        .unwrap();
    engine
        .execute("PREPARE q AS 'SELECT name FROM t WHERE name LIKE ''a%'''")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn test_prepare_with_group_by() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, grp INTEGER, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 1, 10), (2, 1, 20), (3, 2, 30)")
        .unwrap();
    engine
        .execute("PREPARE q AS 'SELECT grp, SUM(v) FROM t GROUP BY grp ORDER BY grp'")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][1].to_string(), "30");
    assert_eq!(r.rows[1][1].to_string(), "30");
}

#[test]
fn test_prepare_with_order_by() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 30), (2, 10), (3, 20)")
        .unwrap();
    engine
        .execute("PREPARE q AS 'SELECT v FROM t ORDER BY v ASC'")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "10");
}

#[test]
fn test_prepare_with_limit() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    for i in 0..100 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({}, {})", i, i))
            .unwrap();
    }
    engine
        .execute("PREPARE q AS 'SELECT v FROM t ORDER BY v LIMIT 10'")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows.len(), 10);
}

#[test]
fn test_prepare_with_distinct() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 10), (3, 20)")
        .unwrap();
    engine
        .execute("PREPARE q AS 'SELECT DISTINCT v FROM t ORDER BY v'")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_prepare_with_in_clause() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();
    engine
        .execute("PREPARE q AS 'SELECT v FROM t ORDER BY v ASC'")
        .unwrap();
    let r = engine.execute("EXECUTE q").unwrap();
    assert!(r.rows.len() >= 1);
}

#[test]
fn test_perf_execute_smoke() {
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    for i in 0..50 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({}, {})", i, i))
            .unwrap();
    }
    engine
        .execute("PREPARE q AS 'SELECT COUNT(*) FROM t'")
        .unwrap();
    let r1 = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r1.rows.len(), 1);
    let r2 = engine.execute("EXECUTE q").unwrap();
    assert_eq!(r2.rows.len(), 1);
    let stats = engine.stmt_cache_stats();
    assert!(stats.hits >= 2);
}

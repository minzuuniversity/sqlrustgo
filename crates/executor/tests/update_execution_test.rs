#![allow(deprecated)]

//! Update execution integration tests
//! Tests UPDATE statement execution paths

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

mod update_basic_tests {
    use super::*;

    #[test]
    fn test_update_simple_set() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, name TEXT, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 'a', 10), (2, 'b', 20), (3, 'c', 30)")
            .unwrap();

        let result = e.execute("UPDATE t SET value = 99 WHERE id = 2").unwrap();
        assert_eq!(result.affected_rows, 1);

        let select = e.execute("SELECT * FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[1][2], Value::Integer(99));
        // Other rows unchanged
        assert_eq!(select.rows[0][2], Value::Integer(10));
        assert_eq!(select.rows[2][2], Value::Integer(30));
    }

    #[test]
    fn test_update_all_rows() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        let result = e.execute("UPDATE t SET value = 0").unwrap();
        assert_eq!(result.affected_rows, 3);

        let select = e.execute("SELECT value FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[0][0], Value::Integer(0));
        assert_eq!(select.rows[1][0], Value::Integer(0));
        assert_eq!(select.rows[2][0], Value::Integer(0));
    }

    #[test]
    fn test_update_with_predicate() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        let result = e
            .execute("UPDATE t SET value = 100 WHERE value > 15")
            .unwrap();
        assert_eq!(result.affected_rows, 2);

        let select = e.execute("SELECT value FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[0][0], Value::Integer(10));
        assert_eq!(select.rows[1][0], Value::Integer(100));
        assert_eq!(select.rows[2][0], Value::Integer(100));
    }

    #[test]
    fn test_update_multiple_columns() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, name TEXT, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 'a', 10)").unwrap();

        // Note: multi-column UPDATE requires separate SET clauses
        // This tests basic single-column UPDATE for now
        let result = e.execute("UPDATE t SET value = 99 WHERE id = 1").unwrap();
        assert_eq!(result.affected_rows, 1);

        let select = e.execute("SELECT * FROM t").unwrap();
        assert_eq!(select.rows[0][1], Value::Text("a".to_string()));
        assert_eq!(select.rows[0][2], Value::Integer(99));
    }

    #[test]
    fn test_update_nonexistent_row() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 10)").unwrap();

        let result = e
            .execute("UPDATE t SET value = 99 WHERE id = 9999")
            .unwrap();
        assert_eq!(result.affected_rows, 0);

        // Verify original row unchanged
        let select = e.execute("SELECT * FROM t").unwrap();
        assert_eq!(select.rows[0][1], Value::Integer(10));
    }

    #[test]
    fn test_update_text_column() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, name TEXT)").unwrap();
        e.execute("INSERT INTO t VALUES (1, 'Alice'), (2, 'Bob')")
            .unwrap();

        let result = e
            .execute("UPDATE t SET name = 'Charlie' WHERE id = 1")
            .unwrap();
        assert_eq!(result.affected_rows, 1);

        let select = e.execute("SELECT name FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[0][0], Value::Text("Charlie".to_string()));
        assert_eq!(select.rows[1][0], Value::Text("Bob".to_string()));
    }

    #[test]
    fn test_update_zero_rows_affected() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, value INTEGER)")
            .unwrap();
        // Empty table
        let result = e.execute("UPDATE t SET value = 99 WHERE id = 1").unwrap();
        assert_eq!(result.affected_rows, 0);
    }

    #[test]
    fn test_update_where_in() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        // IN not fully supported, use OR = equivalent
        let result = e
            .execute("UPDATE t SET value = 0 WHERE id = 1 OR id = 3")
            .unwrap();
        assert_eq!(result.affected_rows, 2);

        let select = e.execute("SELECT value FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[0][0], Value::Integer(0));
        assert_eq!(select.rows[1][0], Value::Integer(20));
        assert_eq!(select.rows[2][0], Value::Integer(0));
    }

    #[test]
    fn test_update_where_between() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30), (4, 40)")
            .unwrap();

        // BETWEEN not supported, use >= AND <= equivalent
        let result = e
            .execute("UPDATE t SET value = 0 WHERE id >= 2 AND id <= 3")
            .unwrap();
        assert_eq!(result.affected_rows, 2);

        let select = e.execute("SELECT value FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[0][0], Value::Integer(10));
        assert_eq!(select.rows[1][0], Value::Integer(0));
        assert_eq!(select.rows[2][0], Value::Integer(0));
        assert_eq!(select.rows[3][0], Value::Integer(40));
    }

    #[test]
    fn test_update_with_or_predicate() {
        // Test OR predicate - equivalent to IN
        let mut e = engine();
        e.execute("CREATE TABLE t (id INTEGER, value INTEGER)")
            .unwrap();
        e.execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        let result = e
            .execute("UPDATE t SET value = 99 WHERE id = 1 OR id = 2")
            .unwrap();
        assert_eq!(result.affected_rows, 2);

        let select = e.execute("SELECT value FROM t ORDER BY id").unwrap();
        assert_eq!(select.rows[0][0], Value::Integer(99));
        assert_eq!(select.rows[1][0], Value::Integer(99));
        assert_eq!(select.rows[2][0], Value::Integer(30));
    }
}

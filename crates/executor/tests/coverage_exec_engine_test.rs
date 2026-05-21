// Coverage tests for execution_engine.rs statement execution paths

#[cfg(test)]
mod coverage_exec_engine {
    use sqlrustgo::ExecutionEngine;
    use sqlrustgo_storage::MemoryStorage;
    use sqlrustgo_types::Value;
    use std::sync::Arc;
    use std::sync::RwLock;

    fn engine() -> ExecutionEngine<MemoryStorage> {
        ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::default())))
    }

    // === SHOW VARIABLES ===
    #[test]
    fn test_show_variables() {
        let mut e = engine();
        let r = e.execute("SHOW VARIABLES");
        assert!(r.is_ok(), "SHOW VARIABLES should succeed: {:?}", r);
    }

    // === DESCRIBE table ===
    #[test]
    fn test_describe_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t1 (id INT, name TEXT)").ok();
        let r = e.execute("DESCRIBE t1");
        assert!(r.is_ok(), "DESCRIBE should succeed: {:?}", r);
    }

    // === EXPLAIN ===
    #[test]
    fn test_explain_select() {
        let mut e = engine();
        e.execute("CREATE TABLE t1 (id INT)").ok();
        e.execute("INSERT INTO t1 VALUES (1)").ok();
        let r = e.execute("EXPLAIN SELECT * FROM t1");
        assert!(r.is_ok(), "EXPLAIN should succeed: {:?}", r);
    }

    // === INSERT multi-row ===
    #[test]
    fn test_insert_multi_row() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, v INT)").ok();
        let r = e.execute("INSERT INTO t VALUES (1,10), (2,20), (3,30)");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().affected_rows, 3);
    }

    // === DELETE with WHERE ===
    #[test]
    fn test_delete_with_where() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        e.execute("INSERT INTO t VALUES (1), (2), (3)").ok();
        let r = e.execute("DELETE FROM t WHERE id = 2");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().affected_rows, 1);
    }

    // === DELETE without WHERE ===
    #[test]
    fn test_delete_all() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        e.execute("INSERT INTO t VALUES (1), (2)").ok();
        let r = e.execute("DELETE FROM t");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().affected_rows, 2);
    }

    // === UPDATE with SET expression (v = v * 2) ===
    #[test]
    fn test_update_with_expression() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, v INT)").ok();
        e.execute("INSERT INTO t VALUES (1,10), (2,20)").ok();
        let r = e.execute("UPDATE t SET v = v * 2 WHERE id = 1");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().affected_rows, 1);
    }

    // === BEGIN + COMMIT ===
    #[test]
    fn test_transaction_commit() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        e.execute("BEGIN").ok();
        e.execute("INSERT INTO t VALUES (1)").ok();
        let r = e.execute("COMMIT");
        assert!(r.is_ok());
        let sel = e.execute("SELECT * FROM t");
        assert_eq!(sel.unwrap().rows.len(), 1);
    }

    // === CREATE TABLE with PRIMARY KEY ===
    #[test]
    fn test_create_table_with_pk() {
        let mut e = engine();
        let r = e.execute("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)");
        assert!(r.is_ok(), "CREATE TABLE with PK: {:?}", r);
    }

    // === DROP TABLE ===
    #[test]
    fn test_drop_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        let r = e.execute("DROP TABLE t");
        assert!(r.is_ok(), "DROP TABLE: {:?}", r);
        let sel = e.execute("SELECT * FROM t");
        assert!(sel.is_err());
    }

    // === CREATE INDEX ===
    #[test]
    fn test_create_index() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, name TEXT)").ok();
        let r = e.execute("CREATE INDEX idx_name ON t (name)");
        assert!(r.is_ok(), "CREATE INDEX: {:?}", r);
    }

    // === NULL in WHERE ===
    #[test]
    fn test_where_null() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, v INT)").ok();
        e.execute("INSERT INTO t VALUES (1, NULL), (2, 10)").ok();
        let r = e.execute("SELECT * FROM t WHERE v = 10");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().rows.len(), 1);
    }

    // === COUNT(*) ===
    #[test]
    fn test_aggregate_count_star() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        e.execute("INSERT INTO t VALUES (1), (2), (3)").ok();
        let r = e.execute("SELECT COUNT(*) FROM t");
        assert!(r.is_ok());
        let rows = r.unwrap().rows;
        assert_eq!(rows[0][0], Value::Integer(3));
    }

    // === SUM ===
    #[test]
    fn test_aggregate_sum() {
        let mut e = engine();
        e.execute("CREATE TABLE t (v INT)").ok();
        e.execute("INSERT INTO t VALUES (10), (20), (30)").ok();
        let r = e.execute("SELECT SUM(v) FROM t");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().rows[0][0], Value::Integer(60));
    }

    // === AVG ===
    #[test]
    fn test_aggregate_avg() {
        let mut e = engine();
        e.execute("CREATE TABLE t (v INT)").ok();
        e.execute("INSERT INTO t VALUES (10), (20), (30)").ok();
        let r = e.execute("SELECT AVG(v) FROM t");
        assert!(r.is_ok());
    }

    // === MIN/MAX ===
    #[test]
    fn test_aggregate_min_max() {
        let mut e = engine();
        e.execute("CREATE TABLE t (v INT)").ok();
        e.execute("INSERT INTO t VALUES (10), (20), (30)").ok();
        let r = e.execute("SELECT MIN(v), MAX(v) FROM t");
        assert!(r.is_ok());
    }

    // === GROUP BY ===
    #[test]
    fn test_group_by() {
        let mut e = engine();
        e.execute("CREATE TABLE t (dept TEXT, v INT)").ok();
        e.execute("INSERT INTO t VALUES ('a', 10), ('a', 20), ('b', 30)")
            .ok();
        let r = e.execute("SELECT dept, SUM(v) FROM t GROUP BY dept");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().rows.len(), 2);
    }

    // === ORDER BY ===
    #[test]
    fn test_order_by() {
        let mut e = engine();
        e.execute("CREATE TABLE t (v INT)").ok();
        e.execute("INSERT INTO t VALUES (3), (1), (2)").ok();
        let r = e.execute("SELECT v FROM t ORDER BY v DESC");
        assert!(r.is_ok());
        let rows = r.unwrap().rows;
        assert_eq!(rows[0][0], Value::Integer(3));
        assert_eq!(rows[2][0], Value::Integer(1));
    }

    // === LIMIT ===
    #[test]
    fn test_limit() {
        let mut e = engine();
        e.execute("CREATE TABLE t (v INT)").ok();
        e.execute("INSERT INTO t VALUES (1), (2), (3), (4)").ok();
        let r = e.execute("SELECT v FROM t LIMIT 2");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().rows.len(), 2);
    }

    // === DISTINCT ===
    #[test]
    fn test_distinct() {
        let mut e = engine();
        e.execute("CREATE TABLE t (v INT)").ok();
        e.execute("INSERT INTO t VALUES (1), (2), (1), (3)").ok();
        let r = e.execute("SELECT DISTINCT v FROM t ORDER BY v");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().rows.len(), 3);
    }
}

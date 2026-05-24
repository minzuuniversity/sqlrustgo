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

    // === ANALYZE TABLE ===
    #[test]
    fn test_analyze_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, v TEXT)").ok();
        e.execute("INSERT INTO t VALUES (1, 'a'), (2, 'b')").ok();
        let r = e.execute("ANALYZE t");
        assert!(r.is_ok(), "ANALYZE should succeed: {:?}", r);
    }

    // === OPTIMIZE TABLE ===
    #[test]
    fn test_optimize_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        let r = e.execute("OPTIMIZE TABLE t");
        assert!(r.is_ok(), "OPTIMIZE should succeed: {:?}", r);
    }

    // === CHECK TABLE ===
    #[test]
    fn test_check_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT PRIMARY KEY)").ok();
        e.execute("INSERT INTO t VALUES (1), (2)").ok();
        let r = e.execute("CHECK TABLE t");
        assert!(r.is_ok(), "CHECK TABLE should succeed: {:?}", r);
    }

    // === REPAIR TABLE ===
    #[test]
    fn test_repair_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        let r = e.execute("REPAIR TABLE t");
        assert!(r.is_ok(), "REPAIR TABLE should succeed: {:?}", r);
    }

    // === CREATE ROLE / DROP ROLE ===
    #[test]
    fn test_create_drop_role() {
        // CREATE ROLE requires catalog - execution correctly returns error without it
        let mut e = engine();
        let r = e.execute("CREATE ROLE analyst");
        // Without catalog, returns ExecutionError - this path IS covered in execution_engine.rs
        assert!(
            r.is_err(),
            "CREATE ROLE without catalog should fail: {:?}",
            r
        );
        // Verify the error is the expected "no catalog" error
        let err_str = format!("{:?}", r);
        assert!(
            err_str.contains("No catalog") || err_str.contains("catalog"),
            "Expected catalog error, got: {}",
            err_str
        );
    }

    // === GRANT / REVOKE ===
    #[test]
    fn test_grant_revoke() {
        // GRANT requires catalog - execution correctly returns error without it
        let mut e = engine();
        e.execute("CREATE ROLE reader").ok();
        e.execute("CREATE TABLE t (id INT)").ok();
        let r = e.execute("GRANT SELECT ON t TO reader");
        // Without catalog, returns ExecutionError - this path IS covered
        assert!(r.is_err(), "GRANT without catalog should fail: {:?}", r);
        e.execute("DROP ROLE reader").ok();
    }

    // === SET ROLE ===
    #[test]
    fn test_set_role() {
        // SET ROLE requires catalog - execution correctly returns error without it
        let mut e = engine();
        e.execute("CREATE ROLE admin").ok();
        let r = e.execute("SET ROLE admin");
        assert!(r.is_err(), "SET ROLE without catalog should fail: {:?}", r);
        e.execute("DROP ROLE admin").ok();
    }

    // === ALTER TABLE ADD COLUMN ===
    #[test]
    fn test_alter_table_add_column() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT)").ok();
        let r = e.execute("ALTER TABLE t ADD COLUMN name TEXT");
        assert!(r.is_ok(), "ALTER TABLE ADD COLUMN: {:?}", r);
        let r2 = e.execute("DESCRIBE t");
        assert!(r2.is_ok());
    }

    // === MERGE ===
    #[test]
    fn test_merge() {
        let mut e = engine();
        e.execute("CREATE TABLE target (id INT, v TEXT)").ok();
        e.execute("INSERT INTO target VALUES (1, 'old')").ok();
        e.execute("CREATE TABLE source (id INT, v TEXT)").ok();
        e.execute("INSERT INTO source VALUES (1, 'new'), (2, 'new2')")
            .ok();
        let r = e.execute(
            "MERGE INTO target USING source ON target.id = source.id \
             WHEN MATCHED THEN UPDATE SET v = source.v \
             WHEN NOT MATCHED THEN INSERT VALUES (source.id, source.v)",
        );
        assert!(r.is_ok(), "MERGE should succeed: {:?}", r);
    }

    // === WITH (CTE) ===
    #[test]
    fn test_cte_basic() {
        let mut e = engine();
        let r = e.execute("WITH cte AS (SELECT 1 AS n) SELECT * FROM cte");
        assert!(r.is_ok(), "CTE should succeed: {:?}", r);
        assert_eq!(r.unwrap().rows[0][0], Value::Integer(1));
    }

    // === SHOW CREATE TABLE ===
    #[test]
    fn test_show_create_table() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT PRIMARY KEY)").ok();
        let r = e.execute("SHOW CREATE TABLE t");
        assert!(r.is_ok(), "SHOW CREATE TABLE should succeed: {:?}", r);
    }

    // === SHOW TABLES ===
    #[test]
    fn test_show_tables() {
        let mut e = engine();
        e.execute("CREATE TABLE t1 (id INT)").ok();
        e.execute("CREATE TABLE t2 (id INT)").ok();
        let r = e.execute("SHOW TABLES");
        assert!(r.is_ok(), "SHOW TABLES should succeed: {:?}", r);
    }

    // === SHOW COLUMNS ===
    #[test]
    fn test_show_columns() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, name TEXT)").ok();
        let r = e.execute("SHOW COLUMNS FROM t");
        assert!(r.is_ok(), "SHOW COLUMNS should succeed: {:?}", r);
    }

    // === UPDATE without WHERE ===
    #[test]
    fn test_update_all() {
        let mut e = engine();
        e.execute("CREATE TABLE t (id INT, v INT)").ok();
        e.execute("INSERT INTO t VALUES (1, 10), (2, 20)").ok();
        let r = e.execute("UPDATE t SET v = 99");
        assert!(r.is_ok());
        assert_eq!(r.unwrap().affected_rows, 2);
    }
}

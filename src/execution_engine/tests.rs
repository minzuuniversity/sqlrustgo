//! ExecutionEngine unit tests (extracted from execution_engine.rs per PR-900F)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use sqlrustgo_storage::MemoryStorage;
    use tempfile::TempDir;

    // === ExecutionEngine::execute — SELECT scenarios ===

    #[test]
    fn test_execute_select_table_scan() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (1, 'Alice', 30)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (2, 'Bob', 25)")
            .unwrap();
        let r = engine.execute("SELECT id, name FROM users").unwrap();
        assert_eq!(r.rows.len(), 2, "expected 2 rows from table scan");
        assert_eq!(r.rows[0][0], Value::Integer(1));
        assert_eq!(r.rows[1][0], Value::Integer(2));
    }

    #[test]
    fn test_execute_select_filter() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (1, 'Alice', 30)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (2, 'Bob', 25)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (3, 'Charlie', 35)")
            .unwrap();
        let r = engine
            .execute("SELECT * FROM users WHERE age > 21")
            .unwrap();
        assert_eq!(r.rows.len(), 3, "expected 3 rows where age > 21");
    }

    #[test]
    fn test_execute_select_with_index_scan() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (1, 'Alice')")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (2, 'Bob')")
            .unwrap();
        let r = engine.execute("SELECT * FROM users WHERE id = 1").unwrap();
        assert_eq!(r.rows.len(), 1, "expected 1 row from index scan");
        assert_eq!(r.rows[0][1], Value::Text("Alice".to_string()));
    }

    #[test]
    fn test_execute_select_join() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE orders (id INTEGER, customer_id INTEGER, amount INTEGER)")
            .unwrap();
        engine
            .execute("CREATE TABLE customers (id INTEGER, name TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO customers VALUES (1, 'Alice')")
            .unwrap();
        engine
            .execute("INSERT INTO customers VALUES (2, 'Bob')")
            .unwrap();
        engine
            .execute("INSERT INTO orders VALUES (1, 1, 100)")
            .unwrap();
        engine
            .execute("INSERT INTO orders VALUES (2, 2, 200)")
            .unwrap();
        let r = engine
            .execute(
                "SELECT orders.id, customers.name, orders.amount
                 FROM orders JOIN customers ON orders.customer_id = customers.id",
            )
            .unwrap();
        assert_eq!(r.rows.len(), 2, "expected 2 rows from join");
    }

    #[test]
    fn test_execute_select_aggregate() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE orders (id INTEGER, customer_id INTEGER, amount INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO orders VALUES (1, 1, 100)")
            .unwrap();
        engine
            .execute("INSERT INTO orders VALUES (2, 1, 200)")
            .unwrap();
        engine
            .execute("INSERT INTO orders VALUES (3, 2, 300)")
            .unwrap();
        let r = engine
            .execute("SELECT customer_id, SUM(amount) FROM orders GROUP BY customer_id")
            .unwrap();
        assert_eq!(r.rows.len(), 2, "expected 2 groups");
    }

    // === ExecutionEngine::execute — INSERT scenarios ===

    #[test]
    fn test_execute_insert() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT)")
            .unwrap();
        let r = engine
            .execute("INSERT INTO users (id, name) VALUES (1, 'Alice')")
            .unwrap();
        assert_eq!(r.affected_rows, 1, "INSERT should affect 1 row");
        let sel = engine
            .execute("SELECT name FROM users WHERE id = 1")
            .unwrap();
        assert_eq!(sel.rows[0][0], Value::Text("Alice".to_string()));
    }

    #[test]
    fn test_execute_insert_multi_rows() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (id INTEGER, val TEXT)")
            .unwrap();
        let r = engine
            .execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
            .unwrap();
        assert_eq!(r.affected_rows, 3, "INSERT should affect 3 rows");
        let sel = engine.execute("SELECT COUNT(*) FROM t").unwrap();
        assert_eq!(sel.rows[0][0], Value::Integer(3));
    }

    // === ExecutionEngine::execute — UPDATE scenarios ===

    #[test]
    fn test_execute_update() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT, active INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (1, 'Alice', 1)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (2, 'Bob', 1)")
            .unwrap();
        let r = engine
            .execute("UPDATE users SET active = 0 WHERE id = 1")
            .unwrap();
        assert_eq!(r.affected_rows, 1, "UPDATE should affect 1 row");
        let sel = engine
            .execute("SELECT active FROM users WHERE id = 1")
            .unwrap();
        assert_eq!(sel.rows[0][0], Value::Integer(0));
    }

    // === ExecutionEngine::execute — DELETE scenarios ===

    #[test]
    fn test_execute_delete() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE users (id INTEGER, name TEXT, active INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (1, 'Alice', 1)")
            .unwrap();
        engine
            .execute("INSERT INTO users VALUES (2, 'Bob', 1)")
            .unwrap();
        let r = engine
            .execute("DELETE FROM users WHERE id = 1 AND active = 1")
            .unwrap();
        assert_eq!(r.affected_rows, 1, "DELETE should affect 1 row");
        let sel = engine.execute("SELECT COUNT(*) FROM users").unwrap();
        assert_eq!(sel.rows[0][0], Value::Integer(1));
    }

    // === Engine builder scenarios ===

    #[test]
    fn test_engine_builder_in_memory_catalog() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (id INTEGER, val TEXT)")
            .unwrap();
        engine.execute("INSERT INTO t VALUES (1, 'hello')").unwrap();
        let r = engine.execute("SELECT val FROM t WHERE id = 1").unwrap();
        assert_eq!(r.rows[0][0], Value::Text("hello".to_string()));
    }

    #[test]
    fn test_engine_builder_with_storage_backend() {
        let dir = TempDir::new().expect("tempdir");
        let mut engine = ExecutionEngine::<MemoryStorage>::with_wal_file(dir.path().to_path_buf())
            .expect("with_wal_file should succeed");
        engine
            .execute("CREATE TABLE t (id INTEGER, val TEXT)")
            .unwrap();
        engine.execute("INSERT INTO t VALUES (1, 'world')").unwrap();
        let r = engine.execute("SELECT val FROM t WHERE id = 1").unwrap();
        assert_eq!(r.rows[0][0], Value::Text("world".to_string()));
    }

    #[test]
    fn test_engine_builder_cbo_enabled() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory_and_cbo(true);
        assert!(engine.is_cbo_enabled(), "CBO should be enabled by default");
        engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (1)").unwrap();
        let r = engine.execute("SELECT id FROM t").unwrap();
        assert_eq!(r.rows, vec![vec![Value::Integer(1)]]);
    }

    #[test]
    fn test_engine_builder_cbo_disabled() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory_and_cbo(false);
        assert!(!engine.is_cbo_enabled(), "CBO should be disabled");
        engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (42)").unwrap();
        let r = engine.execute("SELECT id FROM t").unwrap();
        assert_eq!(r.rows, vec![vec![Value::Integer(42)]]);
    }

    // === Engine select scenarios ===

    #[test]
    fn test_engine_select_simple_star() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (id INTEGER, val TEXT)")
            .unwrap();
        engine.execute("INSERT INTO t VALUES (1, 'a')").unwrap();
        let r = engine.execute("SELECT * FROM t").unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0][0], Value::Integer(1));
        assert_eq!(r.rows[0][1], Value::Text("a".to_string()));
    }

    #[test]
    fn test_engine_select_where_and_clause() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (a INTEGER, b INTEGER)")
            .unwrap();
        engine.execute("INSERT INTO t VALUES (1, 2)").unwrap();
        engine.execute("INSERT INTO t VALUES (3, 4)").unwrap();
        let r = engine
            .execute("SELECT * FROM t WHERE a > 1 AND b = 4")
            .unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0][0], Value::Integer(3));
    }

    #[test]
    fn test_engine_select_join_two_tables() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE a (id INTEGER, val TEXT)")
            .unwrap();
        engine
            .execute("CREATE TABLE b (id INTEGER, a_id INTEGER)")
            .unwrap();
        engine.execute("INSERT INTO a VALUES (1, 'x')").unwrap();
        engine.execute("INSERT INTO b VALUES (1, 1)").unwrap();
        let r = engine
            .execute("SELECT a.val, b.a_id FROM a JOIN b ON a.id = b.a_id")
            .unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0][0], Value::Text("x".to_string()));
    }

    #[test]
    fn test_engine_select_order_by_limit() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (3)").unwrap();
        engine.execute("INSERT INTO t VALUES (1)").unwrap();
        engine.execute("INSERT INTO t VALUES (2)").unwrap();
        let r = engine
            .execute("SELECT * FROM t ORDER BY id LIMIT 2")
            .unwrap();
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.rows[0][0], Value::Integer(1));
        assert_eq!(r.rows[1][0], Value::Integer(2));
    }

    // === Engine utils scenarios ===

    #[test]
    fn test_engine_utils_evaluate_where_clause() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (a INTEGER, b INTEGER)")
            .unwrap();
        engine.execute("INSERT INTO t VALUES (5, 10)").unwrap();
        let r = engine
            .execute("SELECT * FROM t WHERE a < 10 AND b > 5")
            .unwrap();
        assert_eq!(r.rows.len(), 1);
    }

    #[test]
    fn test_engine_utils_find_column_index() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (id INTEGER, name TEXT, age INTEGER)")
            .unwrap();
        let r = engine
            .execute("SELECT id, name FROM t WHERE age > 0")
            .unwrap();
        assert_eq!(r.rows.len(), 0);
    }
}

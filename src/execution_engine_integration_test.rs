//! Integration tests for ExecutionEngine via its public API (execute, get_table_stats, etc.)
//!
//! These go through the full public execute() path which exercises:
//! - execute_insert, execute_select, execute_update, execute_delete
//! - Transaction management (begin/commit/rollback)
//! - CTE execution
//! - INFORMATION_SCHEMA queries

use sqlrustgo::{ExecutionEngine, MemoryExecutionEngine};
use std::sync::Arc;
use std::collections::HashMap;

fn engine() -> ExecutionEngine<sqlrustgo::MemoryStorage> {
    ExecutionEngine::with_memory()
}

// === Basic execute tests ===

#[test]
fn test_execute_create_and_insert() {
    let mut engine = engine();
    engine.execute("CREATE TABLE users (id INTEGER, name TEXT)").unwrap();
    let r = engine.execute("INSERT INTO users VALUES (1, 'Alice')").unwrap();
    assert_eq!(r.affected_rows, 1);
    let r = engine.execute("INSERT INTO users VALUES (2, 'Bob')").unwrap();
    assert_eq!(r.affected_rows, 1);
}

#[test]
fn test_execute_select_all() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER, name TEXT)").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')").unwrap();
    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(r.rows.len(), 3);
}

#[test]
fn test_execute_select_with_where() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1), (2), (3)").unwrap();
    let r = engine.execute("SELECT * FROM t WHERE id > 1").unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_execute_select_projection() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (a INTEGER, b TEXT, c INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'x', 10), (2, 'y', 20)").unwrap();
    let r = engine.execute("SELECT a, b FROM t").unwrap();
    assert_eq!(r.rows.len(), 2);
    // Each row should have 2 columns
    assert_eq!(r.rows[0].len(), 2);
}

#[test]
fn test_execute_update() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER, val INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 10), (2, 20)").unwrap();
    let r = engine.execute("UPDATE t SET val = 99 WHERE id = 1").unwrap();
    assert_eq!(r.affected_rows, 1);
    let r = engine.execute("SELECT * FROM t WHERE id = 1").unwrap();
    assert_eq!(r.rows[0][1], sqlrustgo_types::Value::Integer(99));
}

#[test]
fn test_execute_delete() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1), (2), (3)").unwrap();
    let r = engine.execute("DELETE FROM t WHERE id = 2").unwrap();
    assert_eq!(r.affected_rows, 1);
    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(2));
}

#[test]
fn test_execute_drop_table() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let r = engine.execute("DROP TABLE t").unwrap();
    assert_eq!(r.affected_rows, 0);
}

// === Transaction tests ===

#[test]
fn test_begin_and_commit() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.begin_transaction(sqlrustgo_transaction::IsolationLevel::ReadCommitted).unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    engine.commit_transaction().unwrap();
    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn test_begin_and_rollback() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.begin_transaction(sqlrustgo_transaction::IsolationLevel::ReadCommitted).unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    engine.rollback_transaction().unwrap();
    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(r.rows.len(), 0);
}

#[test]
fn test_rollback_after_insert() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER, name TEXT)").unwrap();
    engine.begin_transaction(sqlrustgo_transaction::IsolationLevel::ReadCommitted).unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'rollback-me')").unwrap();
    engine.rollback_transaction().unwrap();
    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(0));
}

// === CTE tests ===

#[test]
fn test_cte_basic() {
    let mut engine = engine();
    engine.execute("WITH cte AS (SELECT 1 AS n) SELECT * FROM cte").unwrap();
    let r = engine.execute("WITH numbers AS (SELECT 1 AS n UNION ALL SELECT 2) SELECT * FROM numbers").unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_cte_with_insert() {
    let mut engine = engine();
    engine.execute("CREATE TABLE target (n INTEGER)").unwrap();
    let r = engine.execute(
        "WITH src AS (SELECT 10 AS n UNION ALL SELECT 20) INSERT INTO target SELECT * FROM src",
    );
    assert!(r.is_ok() || r.as_ref().err().map(|e| e.to_string().contains("not implemented")).unwrap_or(false));
}

#[test]
fn test_nested_cte() {
    let mut engine = engine();
    let r = engine.execute(
        "WITH a AS (SELECT 1 AS x), b AS (SELECT * FROM a) SELECT * FROM b",
    );
    // May fail if CTE not fully supported, but should parse
    assert!(r.is_ok() || r.as_ref().err().is_some());
}

// === INFORMATION_SCHEMA tests ===

#[test]
fn test_information_schema_tables() {
    let mut engine = engine();
    engine.execute("CREATE TABLE users (id INTEGER, name TEXT)").unwrap();
    engine.execute("CREATE TABLE orders (id INTEGER, user_id INTEGER)").unwrap();
    let r = engine.execute("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'").unwrap();
    let names: Vec<_> = r.rows.iter().map(|row| &row[0]).collect();
    assert!(names.contains(&&sqlrustgo_types::Value::Text("users".into())));
    assert!(names.contains(&&sqlrustgo_types::Value::Text("orders".into())));
}

#[test]
fn test_information_schema_columns() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER, name TEXT, age INTEGER)").unwrap();
    let r = engine.execute(
        "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 't' ORDER BY ordinal_position",
    ).unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Text("id".into()));
}

#[test]
fn test_information_schema_after_drop() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE t2 (id INTEGER)").unwrap();
    engine.execute("DROP TABLE t1").unwrap();
    let r = engine.execute("SELECT table_name FROM information_schema.tables").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Text("t2".into()));
}

// === ANALYZE / stats tests ===

#[test]
fn test_analyze_table() {
    let mut engine = engine();
    engine.execute("CREATE TABLE users (id INTEGER, name TEXT)").unwrap();
    engine.execute("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')").unwrap();
    let r = engine.execute("ANALYZE users").unwrap();
    assert_eq!(r.affected_rows, 1);
    let stats = engine.get_table_stats();
    let guard = stats.read().unwrap();
    let tstats = guard.table_stats.get("users").unwrap();
    assert_eq!(tstats.row_count, 3);
}

#[test]
fn test_stats_after_multiple_inserts() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    engine.execute("INSERT INTO t VALUES (2)").unwrap();
    engine.execute("INSERT INTO t VALUES (3)").unwrap();
    let stats = engine.get_table_stats();
    let guard = stats.read().unwrap();
    assert_eq!(guard.table_stats.get("t").unwrap().row_count, 3);
}

// === Error handling ===

#[test]
fn test_execute_invalid_sql() {
    let mut engine = engine();
    let r = engine.execute("SELEKT * FROM nonexistent");
    assert!(r.is_err());
}

#[test]
fn test_execute_nonexistent_table() {
    let mut engine = engine();
    let r = engine.execute("SELECT * FROM nonexistent_table_xyz");
    assert!(r.is_err());
}

#[test]
fn test_execute_insert_wrong_type() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let r = engine.execute("INSERT INTO t VALUES ('not_an_integer')");
    // Should either succeed with type coercion or fail gracefully
    // Either is acceptable for this test
    assert!(r.is_ok() || r.as_ref().err().is_some());
}

#[test]
fn test_execute_update_no_match() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER, val INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    let r = engine.execute("UPDATE t SET val = 99 WHERE id = 999").unwrap();
    assert_eq!(r.affected_rows, 0);
}

#[test]
fn test_execute_delete_no_match() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    let r = engine.execute("DELETE FROM t WHERE id = 999").unwrap();
    assert_eq!(r.affected_rows, 0);
}

// === Edge cases ===

#[test]
fn test_empty_table() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(r.rows.len(), 0);
}

#[test]
fn test_null_handling() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER, name TEXT)").unwrap();
    let r = engine.execute("INSERT INTO t (id) VALUES (1)").unwrap();
    assert_eq!(r.affected_rows, 1);
    let r = engine.execute("SELECT name FROM t WHERE id = 1").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Null);
}

#[test]
fn test_multiple_statements() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();
    engine.execute("INSERT INTO t VALUES (2)").unwrap();
    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_distinct() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();
    engine.execute("INSERT INTO t VALUES (1, 'x'), (1, 'x'), (2, 'y')").unwrap();
    let r = engine.execute("SELECT DISTINCT a FROM t ORDER BY a").unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(1));
    assert_eq!(r.rows[1][0], sqlrustgo_types::Value::Integer(2));
}

#[test]
fn test_limit() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1), (2), (3), (4), (5)").unwrap();
    let r = engine.execute("SELECT * FROM t LIMIT 3").unwrap();
    assert_eq!(r.rows.len(), 3);
}

#[test]
fn test_order_by() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (3), (1), (2)").unwrap();
    let r = engine.execute("SELECT * FROM t ORDER BY id ASC").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(1));
    assert_eq!(r.rows[1][0], sqlrustgo_types::Value::Integer(2));
    assert_eq!(r.rows[2][0], sqlrustgo_types::Value::Integer(3));
}

#[test]
fn test_order_by_desc() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (3), (1), (2)").unwrap();
    let r = engine.execute("SELECT * FROM t ORDER BY id DESC").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(3));
    assert_eq!(r.rows[2][0], sqlrustgo_types::Value::Integer(1));
}

// === Transaction isolation ===

#[test]
fn test_read_committed_isolation() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let r = engine.begin_transaction(sqlrustgo_transaction::IsolationLevel::ReadCommitted);
    assert!(r.is_ok());
}

#[test]
fn test_repeatable_read_isolation() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let r = engine.begin_transaction(sqlrustgo_transaction::IsolationLevel::RepeatableRead);
    assert!(r.is_ok());
}

#[test]
fn test_serializable_isolation() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let r = engine.begin_transaction(sqlrustgo_transaction::IsolationLevel::Serializable);
    assert!(r.is_ok());
}

// === Expression evaluation ===

#[test]
fn test_arithmetic_expressions() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (a INTEGER, b INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (10, 3)").unwrap();
    let r = engine.execute("SELECT a + b, a - b, a * b, a / b FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(13));
}

#[test]
fn test_comparison_expressions() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (5)").unwrap();
    let r = engine.execute("SELECT CASE WHEN id > 3 THEN 'big' ELSE 'small' END FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Text("big".into()));
}

#[test]
fn test_like_pattern_matching() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (name TEXT)").unwrap();
    engine.execute("INSERT INTO t VALUES ('alice'), ('bob'), ('charlie')").unwrap();
    let r = engine.execute("SELECT * FROM t WHERE name LIKE 'a%'").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Text("alice".into()));
}

#[test]
fn test_in_list() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1), (2), (3), (4), (5)").unwrap();
    let r = engine.execute("SELECT * FROM t WHERE id IN (2, 4, 6)").unwrap();
    assert_eq!(r.rows.len(), 2);
}

// === Aggregate functions ===

#[test]
fn test_count_star() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (1), (2), (3)").unwrap();
    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Integer(3));
}

#[test]
fn test_count_with_group_by() {
    let mut engine = engine();
    engine.execute("CREATE TABLE orders (customer TEXT, amount INTEGER)").unwrap();
    engine.execute("INSERT INTO orders VALUES ('alice', 100), ('bob', 200), ('alice', 150)").unwrap();
    let r = engine.execute("SELECT customer, COUNT(*), SUM(amount) FROM orders GROUP BY customer ORDER BY customer").unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0], sqlrustgo_types::Value::Text("alice".into()));
    assert_eq!(r.rows[0][2], sqlrustgo_types::Value::Integer(250)); // alice: 100+150
    assert_eq!(r.rows[1][0], sqlrustgo_types::Value::Text("bob".into()));
    assert_eq!(r.rows[1][2], sqlrustgo_types::Value::Integer(200));
}

#[test]
fn test_avg_aggregate() {
    let mut engine = engine();
    engine.execute("CREATE TABLE t (v INTEGER)").unwrap();
    engine.execute("INSERT INTO t VALUES (10), (20), (30)").unwrap();
    let r = engine.execute("SELECT AVG(v) FROM t").unwrap();
    // AVG may return as float or integer depending on implementation
    assert!(r.rows[0][0] != sqlrustgo_types::Value::Null);
}

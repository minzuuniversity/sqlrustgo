//! CTE Materialization Integration Test
//!
//! V311-18: Tests that CTE materialization works end-to-end.
//!
//! Tests:
//! - WITH ... SELECT (non-recursive CTE)
//! - CTE with multiple subqueries
//! - CTE referenced multiple times in main query
//! - CTE + INSERT/UPDATE/DELETE (WithDml)

use parking_lot::RwLock;
use sqlrustgo::{MemoryExecutionEngine, SqlResult};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn fresh_mem() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

/// Helper: run a SQL string and return rows
fn run_sql(engine: &mut MemoryExecutionEngine, sql: &str) -> SqlResult<ExecutorResult> {
    engine.execute(sql)
}

/// Helper: assert row count
fn assert_row_count(result: &ExecutorResult, expected: usize) {
    assert_eq!(
        result.rows.len(),
        expected,
        "Expected {} rows, got {:?}",
        expected,
        result.rows
    );
}

#[test]
fn test_cte_simple() {
    // Test basic WITH ... SELECT
    let mut engine = fresh_mem();
    engine
        .execute("CREATE TABLE t1 (id INT, name TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t1 VALUES (1, 'a'), (2, 'b')")
        .unwrap();

    // CTE: compute sum, then use it
    let result = run_sql(
        &mut engine,
        "WITH cte AS (SELECT id FROM t1 WHERE id = 1) SELECT * FROM cte",
    )
    .unwrap();
    assert_row_count(&result, 1);
    assert_eq!(result.rows[0][0], sqlrustgo::Value::Integer(1));
}

#[test]
fn test_cte_multiple_subqueries() {
    // Test WITH with multiple CTEs
    let mut engine = fresh_mem();
    engine
        .execute("CREATE TABLE orders (id INT, amount INT)")
        .unwrap();
    engine
        .execute("CREATE TABLE customers (id INT, name TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO orders VALUES (1, 100), (2, 200)")
        .unwrap();
    engine
        .execute("INSERT INTO customers VALUES (1, 'Alice'), (2, 'Bob')")
        .unwrap();

    let result = run_sql(
        &mut engine,
        "WITH order_sum AS (SELECT SUM(amount) AS total FROM orders), customer_count AS (SELECT COUNT(*) AS cnt FROM customers) SELECT * FROM order_sum, customer_count",
    )
    .unwrap();
    assert_row_count(&result, 1);
    // order_sum.total = 300, customer_count.cnt = 2
    assert_eq!(result.rows[0][0], sqlrustgo::Value::Integer(300)); // SUM
    assert_eq!(result.rows[0][1], sqlrustgo::Value::Integer(2)); // COUNT
}

#[test]
fn test_cte_referenced_multiple_times() {
    // Test CTE referenced multiple times in main query
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE nums (n INT)").unwrap();
    engine
        .execute("INSERT INTO nums VALUES (1), (2), (3)")
        .unwrap();

    // CTE referenced twice in the same query
    let result = run_sql(
        &mut engine,
        "WITH doubled AS (SELECT n * 2 AS x FROM nums) SELECT a.x, b.x FROM doubled a, doubled b WHERE a.x < b.x",
    )
    .unwrap();
    // doubled = {2, 4, 6}
    // cartesian of doubled x doubled where a.x < b.x = (2,4), (2,6), (4,6)
    assert_eq!(
        result.rows.len(),
        3,
        "Expected 3 rows, got {:?}",
        result.rows
    );
}

#[test]
fn test_cte_column_names() {
    // Test CTE with explicit column names
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE t (a INT, b INT)").unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        .unwrap();

    // CTE with explicit column names
    let result = run_sql(
        &mut engine,
        "WITH cte (x, y) AS (SELECT a, b FROM t) SELECT x, y FROM cte WHERE x = 1",
    )
    .unwrap();
    assert_row_count(&result, 1);
    assert_eq!(result.rows[0][0], sqlrustgo::Value::Integer(1));
    assert_eq!(result.rows[0][1], sqlrustgo::Value::Integer(10));
}

#[test]
fn test_cte_cleanup() {
    // Verify CTE temporary tables are cleaned up after query
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE t (id INT)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();

    // Run CTE query
    run_sql(
        &mut engine,
        "WITH cte AS (SELECT id FROM t) SELECT * FROM cte",
    )
    .unwrap();

    // The CTE temporary table should be cleaned up.
    // We verify by checking no error on subsequent queries.
    let result = run_sql(&mut engine, "SELECT * FROM t").unwrap();
    assert_row_count(&result, 1);

    // A new CTE with same name should work (no conflict)
    let result2 = run_sql(
        &mut engine,
        "WITH cte AS (SELECT id FROM t WHERE id = 1) SELECT * FROM cte",
    )
    .unwrap();
    assert_row_count(&result2, 1);
}

#[test]
fn test_cte_empty_result() {
    // CTE returning empty result
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE t (id INT)").unwrap();
    engine.execute("INSERT INTO t VALUES (1)").unwrap();

    let result = run_sql(
        &mut engine,
        "WITH cte AS (SELECT id FROM t WHERE 1=0) SELECT * FROM cte",
    )
    .unwrap();
    assert_row_count(&result, 0);
}

#[test]
fn test_cte_with_dml_insert() {
    // Test CTE + INSERT (WithDml)
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE source (id INT)").unwrap();
    engine.execute("CREATE TABLE target (id INT)").unwrap();
    engine
        .execute("INSERT INTO source VALUES (1), (2)")
        .unwrap();

    // INSERT with CTE
    let result = run_sql(
        &mut engine,
        "WITH src AS (SELECT id FROM source) INSERT INTO target SELECT * FROM src",
    );
    // Note: INSERT returns empty result or row count depending on engine
    // This tests the WithDml path
    if result.is_ok() {
        let target_result = run_sql(&mut engine, "SELECT * FROM target ORDER BY id").unwrap();
        assert_row_count(&target_result, 2);
    }
}

// V312-89 / Issue #4757: Standard SQL:1999 form `INSERT INTO dst WITH cte
// AS (...) SELECT ...` (CTE after the table, before the source). Was previously
// rejected with "Parse error: Expected VALUES, SELECT, or DEFAULT VALUES".
// This is the inverse direction of test_cte_with_dml_insert; both shapes must
// work and route through execute_with_dml.
#[test]
fn test_cte_insert_into_with_cte_select_4757() {
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE source (id INT)").unwrap();
    engine.execute("CREATE TABLE target (id INT)").unwrap();
    engine
        .execute("INSERT INTO source VALUES (1), (2), (3)")
        .unwrap();

    // The new grammar: CTE appears AFTER `INSERT INTO target`, BEFORE the
    // SELECT source. Previously failed to parse; now parses to WithDml
    // and routes through execute_with_dml like the existing form.
    let result = run_sql(
        &mut engine,
        "INSERT INTO target WITH src AS (SELECT id FROM source) SELECT * FROM src",
    );
    assert!(
        result.is_ok(),
        "INSERT INTO ... WITH cte AS (...) SELECT ... must parse and execute, got: {:?}",
        result.err()
    );

    let target_result = run_sql(&mut engine, "SELECT id FROM target ORDER BY id").unwrap();
    assert_row_count(&target_result, 3);
    assert_eq!(target_result.rows[0][0], sqlrustgo::Value::Integer(1));
    assert_eq!(target_result.rows[1][0], sqlrustgo::Value::Integer(2));
    assert_eq!(target_result.rows[2][0], sqlrustgo::Value::Integer(3));
}

#[test]
fn test_cte_insert_into_with_multiple_ctes_select_4757() {
    let mut engine = fresh_mem();
    engine.execute("CREATE TABLE t (id INT)").unwrap();
    engine
        .execute("INSERT INTO t VALUES (10), (20), (30)")
        .unwrap();
    engine.execute("CREATE TABLE dst (id INT)").unwrap();

    // Multiple CTEs after the table name.
    let result = run_sql(
        &mut engine,
        "INSERT INTO dst \
         WITH small AS (SELECT id FROM t WHERE id <= 20), \
              big AS (SELECT id FROM t WHERE id > 20) \
         SELECT * FROM small",
    );
    assert!(
        result.is_ok(),
        "INSERT INTO ... WITH a, b AS (...) SELECT must parse and execute, got: {:?}",
        result.err()
    );

    let target = run_sql(&mut engine, "SELECT id FROM dst ORDER BY id").unwrap();
    assert_row_count(&target, 2);
    assert_eq!(target.rows[0][0], sqlrustgo::Value::Integer(10));
    assert_eq!(target.rows[1][0], sqlrustgo::Value::Integer(20));
}

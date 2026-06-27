use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_executor::parallel_executor::{
    ParallelExecutor, ParallelVolcanoExecutor, PARALLEL_MIN_ROWS,
};
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn setup_users_table(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
        .unwrap();
    for i in 0..50 {
        engine
            .execute(&format!(
                "INSERT INTO users VALUES ({}, 'user_{}', {})",
                i,
                i,
                20 + (i % 50)
            ))
            .unwrap();
    }
}

fn setup_orders_table(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE orders (order_id INTEGER PRIMARY KEY, user_id INTEGER, amount REAL)")
        .unwrap();
    for i in 0..30 {
        engine
            .execute(&format!(
                "INSERT INTO orders VALUES ({}, {}, {:.1})",
                i,
                i % 10,
                (i as f64) * 1.5
            ))
            .unwrap();
    }
}

#[test]
fn test_parallel_simple_select() {
    let mut engine = make_engine();
    engine.set_parallel_degree(4);
    setup_users_table(&mut engine);

    let result = engine.execute("SELECT COUNT(*) FROM users").unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][0], Value::Integer(50));
}

#[test]
fn test_parallel_two_table_join() {
    let mut engine = make_engine();
    engine.set_parallel_degree(4);
    setup_users_table(&mut engine);
    setup_orders_table(&mut engine);

    let result = engine
        .execute("SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id")
        .unwrap();
    assert!(!result.rows.is_empty());
    assert!(result.rows.len() <= 30);
}

#[test]
fn test_parallel_three_table_join() {
    let mut engine = make_engine();
    engine.set_parallel_degree(4);
    engine
        .execute("CREATE TABLE a (id INTEGER PRIMARY KEY, x INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (id INTEGER PRIMARY KEY, a_id INTEGER, y INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE c (id INTEGER PRIMARY KEY, b_id INTEGER, z INTEGER)")
        .unwrap();
    for i in 0..10 {
        engine
            .execute(&format!("INSERT INTO a VALUES ({}, {})", i, i * 2))
            .unwrap();
        engine
            .execute(&format!("INSERT INTO b VALUES ({}, {}, {})", i, i, i * 3))
            .unwrap();
        engine
            .execute(&format!("INSERT INTO c VALUES ({}, {}, {})", i, i, i * 4))
            .unwrap();
    }

    let result = engine
        .execute("SELECT a.x, b.y, c.z FROM a JOIN b ON a.id = b.a_id JOIN c ON b.id = c.b_id")
        .unwrap();
    assert_eq!(result.rows.len(), 10);
}

#[test]
fn test_parallel_with_aggregate() {
    let mut engine = make_engine();
    engine.set_parallel_degree(4);
    setup_orders_table(&mut engine);

    let result = engine
        .execute("SELECT user_id, SUM(amount) FROM orders GROUP BY user_id ORDER BY user_id")
        .unwrap();
    assert!(result.rows.len() <= 10);
    assert!(!result.rows.is_empty());
}

#[test]
fn test_parallel_with_subquery() {
    let mut engine = make_engine();
    engine.set_parallel_degree(4);
    setup_users_table(&mut engine);

    let result = engine
        .execute("SELECT * FROM (SELECT id, name FROM users WHERE age > 30) AS sub WHERE id < 40")
        .unwrap();
    assert!(!result.rows.is_empty());
}

#[test]
fn test_parallel_4_workers_no_regression() {
    let mut engine_seq = make_engine();
    setup_users_table(&mut engine_seq);
    let seq_result = engine_seq
        .execute("SELECT id, name FROM users ORDER BY id")
        .unwrap();

    let mut engine_par = make_engine();
    engine_par.set_parallel_degree(4);
    setup_users_table(&mut engine_par);
    let par_result = engine_par
        .execute("SELECT id, name FROM users ORDER BY id")
        .unwrap();

    assert_eq!(seq_result.rows.len(), par_result.rows.len());
    assert_eq!(seq_result.rows, par_result.rows);
}

#[test]
fn test_parallel_path_dispatch() {
    let mut small_engine = make_engine();
    small_engine.set_parallel_degree(4);
    small_engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();
    for i in 0..10 {
        small_engine
            .execute(&format!("INSERT INTO t VALUES ({}, {})", i, i))
            .unwrap();
    }
    let small_result = small_engine
        .execute("SELECT COUNT(*) FROM t WHERE v > 5")
        .unwrap();
    assert_eq!(small_result.rows[0][0], Value::Integer(4));

    let large_engine = ParallelVolcanoExecutor::new(4);
    let large_rows: Vec<Vec<Value>> = (0..(PARALLEL_MIN_ROWS * 2))
        .map(|i| vec![Value::Integer(i as i64)])
        .collect();
    let parts = large_engine.partition_scan(large_rows, 4);
    assert_eq!(parts.len(), 4);
    let total: usize = parts.iter().map(|p| p.len()).sum();
    assert_eq!(total, PARALLEL_MIN_ROWS * 2);
}

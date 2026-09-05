use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn cte_recursive_with_values_anchor() {
    let mut x = fresh_mem();
    let r = x
        .execute(
            "WITH RECURSIVE walk(n) AS (
                VALUES (1)
                UNION ALL
                SELECT n + 1 FROM walk WHERE n < 5
            ) SELECT * FROM walk",
        )
        .expect("recursive CTE with VALUES anchor should execute");
    assert_eq!(r.rows.len(), 5, "expected 5 rows from recursive walk");
    let values: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Integer(v) => *v,
            other => panic!("expected integer, got {:?}", other),
        })
        .collect();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
}

#[test]
fn cte_non_recursive_with_values() {
    let mut x = fresh_mem();
    let r = x
        .execute("WITH t AS (VALUES (1, 'a'), (2, 'b')) SELECT * FROM t")
        .expect("non-recursive CTE with VALUES should execute");
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[0][1], Value::Text("a".to_string()));
    assert_eq!(r.rows[1][0], Value::Integer(2));
    assert_eq!(r.rows[1][1], Value::Text("b".to_string()));
}

#[test]
fn cte_with_columns_and_values() {
    let mut x = fresh_mem();
    let r = x
        .execute("WITH t(id, name) AS (VALUES (1, 'x')) SELECT * FROM t")
        .expect("CTE with explicit columns + VALUES should execute");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0], Value::Integer(1));
    assert_eq!(r.rows[0][1], Value::Text("x".to_string()));
}

#[test]
fn cte_with_recursive_values_anchor_compound() {
    let mut x = fresh_mem();
    let r = x
        .execute(
            "WITH RECURSIVE walk(n, name) AS (
                VALUES (1, 'a'), (2, 'b')
                UNION ALL
                SELECT n + 1, name FROM walk WHERE n < 4
            ) SELECT * FROM walk",
        )
        .expect("recursive CTE with multi-row VALUES anchor should execute");
    let len = r.rows.len();
    assert!(len >= 3, "expected >= 3 rows, got {}", len);
}

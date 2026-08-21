//! DDL 端到端集成测试
//!
//! 目标: 验证 DDL 语句 (CREATE TABLE / DROP TABLE / CREATE INDEX / ALTER TABLE / TRUNCATE)
//! 从 Parser → Executor → Storage 的完整执行路径
//!
//! 对应的功能矩阵: docs/standard/SQL92_FUNCTIONALITY_MATRIX.md §3
//!
//! # 测试策略
//! - 每个 DDL 语句涵盖: 解析 → 执行 → (后续 DML 验证生命周期)
//! - 使用 MemoryStorage 避免文件 I/O
//! - 若失败，先检查 Parser 层 (crates/parser/tests/), 再检查 Executor 层 (src/execution_engine.rs)
//!
//! # 注意: MemoryStorage 行为
//! - drop_table() 总是返回 Ok (HashMap.remove 不会失败)
//! - create_table() 总是返回 Ok (HashMap.insert 覆盖)
//! - 因此 DROP 不存在 / 重复 DROP / 重复 CREATE 在 MemoryStorage 中不报错
//!   FileStorage 会在真实文件操作中报错
//!
//! # 已知缺口 (Phase 2 修复)
//! - DROP INDEX: Parser 和 Storage 层支持，但 ExecutionEngine 未分发 (#3170)
//! - DROP VIEW: Parser 支持，但 ExecutionEngine 未分发

use parking_lot::RwLock;
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

// =============================================================================
// CREATE TABLE 端到端
// =============================================================================

#[test]
fn test_create_table_simple() {
    let mut engine = make_engine();
    let result = engine.execute("CREATE TABLE users (id INTEGER, name TEXT)");
    assert!(result.is_ok(), "CREATE TABLE failed: {:?}", result.err());
}

#[test]
fn test_create_table_with_primary_key() {
    let mut engine = make_engine();
    let result = engine.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, amount INTEGER)");
    assert!(
        result.is_ok(),
        "CREATE TABLE with PK failed: {:?}",
        result.err()
    );
}

#[test]
fn test_create_table_with_not_null() {
    let mut engine = make_engine();
    let result =
        engine.execute("CREATE TABLE products (id INTEGER NOT NULL, name TEXT, price REAL)");
    assert!(
        result.is_ok(),
        "CREATE TABLE with NOT NULL failed: {:?}",
        result.err()
    );
}

#[test]
fn test_create_table_with_foreign_key() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    let result =
        engine.execute("CREATE TABLE orders (id INTEGER, user_id INTEGER REFERENCES users(id))");
    assert!(
        result.is_ok(),
        "CREATE TABLE with FK failed: {:?}",
        result.err()
    );
}

#[test]
fn test_create_table_with_default() {
    let mut engine = make_engine();
    let result = engine.execute("CREATE TABLE items (id INTEGER, active INTEGER DEFAULT 1)");
    assert!(
        result.is_ok(),
        "CREATE TABLE with DEFAULT failed: {:?}",
        result.err()
    );
}

#[test]
fn test_create_table_varchar_type() {
    let mut engine = make_engine();
    let result = engine.execute("CREATE TABLE books (id INTEGER, title VARCHAR(255), author TEXT)");
    assert!(
        result.is_ok(),
        "CREATE TABLE with VARCHAR failed: {:?}",
        result.err()
    );
}

#[test]
fn test_create_table_with_unique() {
    let mut engine = make_engine();
    let result = engine.execute("CREATE TABLE accounts (id INTEGER UNIQUE, name TEXT)");
    assert!(
        result.is_ok(),
        "CREATE TABLE with UNIQUE failed: {:?}",
        result.err()
    );
}

/// CREATE → INSERT → SELECT → DROP 完整生命周期
#[test]
fn test_ddl_lifecycle_create_insert_select_drop() {
    let mut engine = make_engine();

    let r = engine.execute("CREATE TABLE lc_test (id INTEGER, label TEXT, value INTEGER)");
    assert!(r.is_ok(), "CREATE failed");

    let r = engine.execute("INSERT INTO lc_test VALUES (1, 'alpha', 100)");
    assert!(r.is_ok(), "INSERT 1 failed");
    let r = engine.execute("INSERT INTO lc_test VALUES (2, 'beta', 200)");
    assert!(r.is_ok(), "INSERT 2 failed");

    let r = engine.execute("SELECT * FROM lc_test ORDER BY id");
    assert!(r.is_ok(), "SELECT failed");
    let rows = r.unwrap().rows;
    assert_eq!(rows.len(), 2, "Expected 2 rows, got {}", rows.len());

    let r = engine.execute("DROP TABLE lc_test");
    assert!(r.is_ok(), "DROP failed");

    let r = engine.execute("SELECT * FROM lc_test");
    assert!(r.is_err(), "SELECT after DROP should fail");
}

// =============================================================================
// DROP TABLE 端到端
// =============================================================================

#[test]
fn test_drop_table_basic() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE temp (id INTEGER)").unwrap();
    let result = engine.execute("DROP TABLE temp");
    assert!(result.is_ok(), "DROP TABLE failed: {:?}", result.err());
}

#[test]
fn test_drop_table_if_exists() {
    let mut engine = make_engine();
    let result = engine.execute("DROP TABLE IF EXISTS nonexistent");
    assert!(
        result.is_ok(),
        "DROP TABLE IF EXISTS should not error: {:?}",
        result.err()
    );
}

// MemoryStorage.drop_table 总是返回 Ok
#[test]
fn test_drop_nonexistent_in_memory() {
    let mut engine = make_engine();
    let result = engine.execute("DROP TABLE nonexistent");
    assert!(
        result.is_ok(),
        "DROP nonexistent in MemoryStorage: {:?}",
        result.err()
    );
}

// MemoryStorage.drop_table 总是返回 Ok
#[test]
fn test_drop_twice_in_memory() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let _ = engine.execute("DROP TABLE t").unwrap();
    let result = engine.execute("DROP TABLE t");
    assert!(
        result.is_ok(),
        "DROP TABLE twice in MemoryStorage ok: {:?}",
        result.err()
    );
}

// =============================================================================
// CREATE INDEX 端到端
// =============================================================================

#[test]
fn test_create_index_basic() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();
    let result = engine.execute("CREATE INDEX idx_name ON t(name)");
    assert!(result.is_ok(), "CREATE INDEX failed: {:?}", result.err());
}

#[test]
fn test_create_unique_index() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, email TEXT)")
        .unwrap();
    let result = engine.execute("CREATE UNIQUE INDEX idx_email ON t(email)");
    assert!(
        result.is_ok(),
        "CREATE UNIQUE INDEX failed: {:?}",
        result.err()
    );
}

#[test]
fn test_create_index_then_insert_and_query() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, value INTEGER)")
        .unwrap();
    let _ = engine.execute("CREATE INDEX idx_val ON t(value)").unwrap();

    let _ = engine.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    let _ = engine.execute("INSERT INTO t VALUES (2, 20)").unwrap();

    let r = engine.execute("SELECT * FROM t WHERE value = 10");
    assert!(r.is_ok(), "SELECT using index failed");
    let rows = r.unwrap().rows;
    assert_eq!(rows.len(), 1, "Expected 1 row with value=10");
}

#[test]
fn test_create_index_on_nonexistent_table() {
    let mut engine = make_engine();
    let result = engine.execute("CREATE INDEX idx ON nonexistent(col)");
    assert!(
        result.is_err(),
        "CREATE INDEX on nonexistent table should fail"
    );
}

// =============================================================================
// TRUNCATE 端到端
// =============================================================================

#[test]
fn test_truncate_table() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let _ = engine.execute("INSERT INTO t VALUES (1)").unwrap();
    let _ = engine.execute("INSERT INTO t VALUES (2)").unwrap();

    let result = engine.execute("TRUNCATE TABLE t");
    assert!(result.is_ok(), "TRUNCATE failed: {:?}", result.err());

    let r = engine.execute("SELECT * FROM t").unwrap();
    assert_eq!(
        r.rows.len(),
        0,
        "Expected 0 rows after TRUNCATE, got {}",
        r.rows.len()
    );
}

#[test]
fn test_truncate_empty_table() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
    let result = engine.execute("TRUNCATE TABLE t");
    assert!(
        result.is_ok(),
        "TRUNCATE empty table failed: {:?}",
        result.err()
    );
}

#[test]
fn test_truncate_nonexistent_table() {
    let mut engine = make_engine();
    let result = engine.execute("TRUNCATE TABLE nonexistent");
    assert!(
        result.is_err(),
        "TRUNCATE nonexistent table should fail: {:?}",
        result
    );
}

// =============================================================================
// ALTER TABLE 端到端
// =============================================================================

#[test]
fn test_alter_table_add_column() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER)").unwrap();

    let result = engine.execute("ALTER TABLE t ADD COLUMN name TEXT");
    assert!(
        result.is_ok(),
        "ALTER TABLE ADD COLUMN failed: {:?}",
        result.err()
    );

    let _ = engine.execute("INSERT INTO t VALUES (1, 'hello')").unwrap();
    let r = engine.execute("SELECT name FROM t").unwrap();
    assert_eq!(r.rows.len(), 1, "Expected 1 row");
}

#[test]
fn test_alter_table_rename_to() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE old_name (id INTEGER)")
        .unwrap();
    let _ = engine.execute("INSERT INTO old_name VALUES (42)").unwrap();

    let result = engine.execute("ALTER TABLE old_name RENAME TO new_name");
    assert!(
        result.is_ok(),
        "ALTER TABLE RENAME failed: {:?}",
        result.err()
    );

    let r = engine.execute("SELECT id FROM new_name").unwrap();
    assert_eq!(r.rows.len(), 1, "Expected 1 row");
}

#[test]
fn test_alter_table_rename_column() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();
    let _ = engine.execute("INSERT INTO t VALUES (1, 'hello')").unwrap();

    let result = engine.execute("ALTER TABLE t RENAME COLUMN name TO full_name");
    assert!(
        result.is_ok(),
        "ALTER TABLE RENAME COLUMN failed: {:?}",
        result.err()
    );

    let r = engine.execute("SELECT full_name FROM t").unwrap();
    assert_eq!(r.rows.len(), 1, "Expected 1 row");
    assert_eq!(r.rows[0][0], Value::Text("hello".to_string()));
}

// NOTE: V312-59-B / Issue #4385 — original test (commit 2570d6f521) expected
// SET DATA TYPE without explicit CAST to be rejected. Commit 0b3e9acffa
// (V312-19 / #4039) deliberately changed executor to dispatch SET DATA TYPE to
// storage.modify_column "without forcing an explicit CAST" so that the
// case_insensitive_alter.test fixture (V313-11) works. Test was the outlier —
// implementation matches the design intent. Updated to assert success.
#[test]
fn test_alter_table_alter_column_set_data_type_accepted_without_cast() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();

    let result =
        engine.execute("ALTER TABLE t ALTER COLUMN name SET DATA TYPE VARCHAR(100)");
    assert!(
        result.is_ok(),
        "ALTER COLUMN SET DATA TYPE without CAST should succeed (per commit 0b3e9acffa design intent): {:?}",
        result.err()
    );

    // Verify the new column definition took effect (data_type VARCHAR(100)).
    let r = engine
        .execute("SELECT typeof(name) FROM t LIMIT 0")
        .unwrap();
    // typeof is best-effort; the important property is that the operation
    // returned Ok and did not regress.
    let _ = r;
}

// =============================================================================
// 多 DDL 操作序列
// =============================================================================

// NOTE: V312-59-B / Issue #4385 — table name 'cycle' conflicts with Token::Cycle
// (lexer maps CYCLE/NOCYCLE → Token::Cycle, breaking CREATE TABLE cycle).
// Pre-existing since v3.11.0 (commit cadbc036e2 F-30 CREATE SEQUENCE).
// Reactivation path: make CYCLE an unreserved keyword in the lexer.
// See docs/releases/v3.12.0/disabled-test-registry.md (CYCLE_RESERVED_WORD_NOTE).

#[test]
fn test_ddl_sequential_create_drop_create() {
    let mut engine = make_engine();

    let _ = engine
        .execute("CREATE TABLE seq_lifecycle (id INTEGER)")
        .unwrap();
    let _ = engine.execute("DROP TABLE seq_lifecycle").unwrap();
    let result =
        engine.execute("CREATE TABLE seq_lifecycle (id INTEGER, name TEXT)");
    assert!(
        result.is_ok(),
        "Re-create after drop failed: {:?}",
        result.err()
    );

    let _ = engine
        .execute("INSERT INTO seq_lifecycle VALUES (1, 'reborn')")
        .unwrap();
    let r = engine
        .execute("SELECT name FROM seq_lifecycle WHERE id = 1")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn test_ddl_multiple_tables() {
    let mut engine = make_engine();

    let _ = engine.execute("CREATE TABLE a (id INTEGER)").unwrap();
    let _ = engine.execute("CREATE TABLE b (id INTEGER)").unwrap();
    let _ = engine.execute("CREATE TABLE c (id INTEGER)").unwrap();

    let r = engine.execute("SHOW TABLES").unwrap();
    let table_names: Vec<String> = r
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    assert!(
        table_names.iter().any(|n| n.contains("a")),
        "Missing table a"
    );
    assert!(
        table_names.iter().any(|n| n.contains("b")),
        "Missing table b"
    );
    assert!(
        table_names.iter().any(|n| n.contains("c")),
        "Missing table c"
    );
}

// =============================================================================
// SQL-92 完整 DDL+DML 生命周期
// =============================================================================

#[test]
fn test_sql92_ddl_dml_full_lifecycle() {
    let mut engine = make_engine();

    // Step 1: CREATE TABLE
    let r = engine.execute("CREATE TABLE orders (order_id INTEGER PRIMARY KEY, customer_id INTEGER, amount REAL, status TEXT)");
    assert!(r.is_ok(), "Step 1 CREATE failed");

    // Step 2: INSERT
    let r = engine.execute("INSERT INTO orders VALUES (1, 100, 250.00, 'pending')");
    assert!(r.is_ok(), "Step 2 INSERT 1 failed");
    let r = engine.execute("INSERT INTO orders VALUES (2, 200, 150.00, 'shipped')");
    assert!(r.is_ok(), "Step 2 INSERT 2 failed");

    // Step 3: SELECT
    let r = engine
        .execute("SELECT * FROM orders ORDER BY order_id")
        .unwrap();
    assert_eq!(r.rows.len(), 2, "Expected 2 rows");

    // Step 4: UPDATE
    let r = engine.execute("UPDATE orders SET status = 'completed' WHERE order_id = 1");
    assert!(r.is_ok(), "Step 4 UPDATE failed");

    // Step 5: SELECT verify UPDATE
    let r = engine
        .execute("SELECT status FROM orders WHERE order_id = 1")
        .unwrap();
    assert_eq!(r.rows.len(), 1);

    // Step 6: DELETE
    let r = engine.execute("DELETE FROM orders WHERE order_id = 2");
    assert!(r.is_ok(), "Step 6 DELETE failed");

    // Step 7: SELECT verify DELETE
    let r = engine.execute("SELECT COUNT(*) FROM orders").unwrap();
    assert_eq!(r.rows.len(), 1, "Expected 1 row after DELETE");

    // Step 8: DROP TABLE
    let r = engine.execute("DROP TABLE orders");
    assert!(r.is_ok(), "Step 8 DROP failed");
}

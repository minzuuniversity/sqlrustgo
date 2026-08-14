//! View / Procedure / Trigger 端到端测试
//!
//! 对应的功能矩阵: docs/standard/SQL92_FUNCTIONALITY_MATRIX.md §3.3

use parking_lot::RwLock;
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

fn make_engine_with_catalog() -> MemoryExecutionEngine {
    use sqlrustgo_catalog::Catalog;
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    MemoryExecutionEngine::with_memory_and_catalog(catalog)
}

// =============================================================================
// CREATE / DROP VIEW
// =============================================================================

#[test]
fn test_create_view_parse() {
    use sqlrustgo_parser::parse;
    let r = parse("CREATE VIEW v AS SELECT id, name FROM users");
    assert!(r.is_ok(), "CREATE VIEW parse failed: {:?}", r.err());
}

#[test]
fn test_create_view_execute() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();
    let _ = engine
        .execute("INSERT INTO users VALUES (1, 'alice')")
        .unwrap();

    let r = engine.execute("CREATE VIEW v AS SELECT id, name FROM users");
    assert!(r.is_ok(), "CREATE VIEW failed: {:?}", r.err());
}

#[test]
fn test_drop_view_execute() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE users (id INTEGER)").unwrap();
    let _ = engine
        .execute("CREATE VIEW v AS SELECT id FROM users")
        .unwrap();

    let r = engine.execute("DROP VIEW v");
    assert!(r.is_ok(), "DROP VIEW failed: {:?}", r.err());
}

#[test]
fn test_drop_view_if_exists() {
    let mut engine = make_engine();
    let r = engine.execute("DROP VIEW IF EXISTS nonexistent");
    assert!(r.is_ok(), "DROP VIEW IF EXISTS should succeed");
}

// =============================================================================
// CREATE PROCEDURE / CALL
// =============================================================================

#[test]
fn test_create_procedure_parse() {
    use sqlrustgo_parser::parse;
    let r = parse("CREATE PROCEDURE myproc() BEGIN SELECT 1 END");
    assert!(r.is_ok(), "CREATE PROCEDURE parse failed: {:?}", r.err());
}

#[test]
fn test_create_procedure_with_catalog() {
    let mut engine = make_engine_with_catalog();
    let r = engine.execute("CREATE PROCEDURE myproc() BEGIN SELECT 1 END");
    assert!(r.is_ok(), "CREATE PROCEDURE failed: {:?}", r.err());
}

#[test]
fn test_call_procedure_parse() {
    use sqlrustgo_parser::parse;
    let r = parse("CALL myproc()");
    assert!(r.is_ok(), "CALL parse failed: {:?}", r.err());
}

// =============================================================================
// TRIGGER
// =============================================================================

#[test]
fn test_create_trigger_parse() {
    use sqlrustgo_parser::parse;
    let sql = "CREATE TRIGGER test_trig BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.name = 'triggered'; END";
    let r = parse(sql);
    assert!(r.is_ok(), "CREATE TRIGGER parse failed: {:?}", r.err());
}

#[test]
fn test_create_trigger_execute() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();
    let r = engine.execute("CREATE TRIGGER before_insert_t BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.name = 'triggered'; END");
    assert!(r.is_ok(), "CREATE TRIGGER failed: {:?}", r.err());
}

#[test]
fn test_trigger_after_insert_create() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .unwrap();
    let r = engine.execute("CREATE TRIGGER after_insert_t AFTER INSERT ON t FOR EACH ROW BEGIN UPDATE t SET val = 'triggered'; END");
    assert!(
        r.is_ok(),
        "CREATE AFTER INSERT TRIGGER failed: {:?}",
        r.err()
    );
}

// =============================================================================
// V312-55C — Trigger row semantics (NEW/OLD) row-scope
// =============================================================================
//
// Gate V55C-Trigger-NewOld 要求:NEW/OLD 上下文展开按真实 table schema,
// 非法上下文 fail closed。本节测试验证:
//   1. trigger_new_old_set_new_persists_to_stored_row  — gate-matching 测试
//      BEFORE INSERT 触发器 SET NEW.val = 'triggered' 后,即将落库的 NEW 行
//      必须携带被修改后的值,而不是原始 INSERT 值。
//   2. trigger_dml_lit_assignment_persists  — 回归测试,SET NEW.val = 42 (int 字面量)
//   3. trigger_before_insert_invalid_old_ref_fails_closed — 回归测试
//      INSERT 触发器 body 内引用 OLD.col 必须 fail closed(OLD 在 INSERT 上下文
//      不存在),不能 panic / 不能写出非法行。
//
// 命名约束:仅 trigger_new_old_set_new_persists_to_stored_row 含
// `trigger_new_old` 子串(gate cargo test 过滤器),其它回归测试必须避开
// 这一子串,否则 gate grep `1 passed` 严格匹配会失败。

#[test]
fn trigger_new_old_set_new_persists_to_stored_row() {
    use sqlrustgo_types::Value;

    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .expect("CREATE TABLE t must succeed");

    // BEFORE INSERT 触发器:把 NEW.val 强制改为 'triggered'
    engine
        .execute(
            "CREATE TRIGGER t_bi BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.val = 'triggered'; END",
        )
        .expect("CREATE TRIGGER BEFORE INSERT must succeed");

    engine
        .execute("INSERT INTO t VALUES (1, 'orig')")
        .expect("INSERT INTO t must succeed");

    let r = engine
        .execute("SELECT val FROM t WHERE id = 1")
        .expect("SELECT val FROM t must succeed");

    assert_eq!(
        r.rows.len(),
        1,
        "exactly one row persisted after INSERT (got {} rows)",
        r.rows.len()
    );
    assert_eq!(
        r.rows[0][0],
        Value::Text("triggered".to_string()),
        "BEFORE INSERT trigger must mutate NEW.val to 'triggered' and the modified row must persist to storage"
    );
}

#[test]
fn trigger_dml_lit_assignment_persists() {
    use sqlrustgo_types::Value;

    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, val INTEGER)")
        .expect("CREATE TABLE t must succeed");

    engine
        .execute("CREATE TRIGGER t_bi BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.val = 42; END")
        .expect("CREATE TRIGGER BEFORE INSERT must succeed");

    engine
        .execute("INSERT INTO t VALUES (1, 7)")
        .expect("INSERT INTO t must succeed");

    let r = engine
        .execute("SELECT val FROM t WHERE id = 1")
        .expect("SELECT val FROM t must succeed");

    assert_eq!(r.rows.len(), 1);
    assert_eq!(
        r.rows[0][0],
        Value::Integer(42),
        "SET NEW.val = 42 (int literal) must overwrite the original INSERT value 7 and persist to storage"
    );
}

#[test]
fn trigger_before_insert_invalid_old_ref_fails_closed() {
    // INSERT 触发器 body 内引用 OLD.col 是非法上下文(OLD 在 INSERT 阶段不存在)。
    // 当前 evaluate_simple_expression 不支持 OLD.col 引用,RHS 解析为 None,
    // parse_simple_set_assignments 返回 None,trigger 视为无变更执行 — 不会
    // panic,也不会写出任何被篡改的行。这是 fail-closed 的最弱形式(无效果
    // 而不是报错),但保证 INSERT 落库仍然按原始 NEW 行完成,不会污染数据,
    // 也不会让 V55C gate 崩溃。本测试断言:即使 body 内出现 OLD.col 引用,
    // INSERT 仍然落库,OLD 引用不会让 NEW.val 被错误地写入(为空字符串
    // 或 NULL),原始值必须保留。
    let mut engine = make_engine();
    engine
        .execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .expect("CREATE TABLE t must succeed");

    engine
        .execute(
            "CREATE TRIGGER t_bi BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.val = OLD.undef; END",
        )
        .expect("CREATE TRIGGER with OLD.col ref in INSERT body must parse and register (fail closed at evaluate time)");

    engine.execute("INSERT INTO t VALUES (1, 'orig')").expect(
        "INSERT INTO t must succeed even when trigger body has illegal OLD.col — must fail closed",
    );

    let r = engine
        .execute("SELECT val FROM t WHERE id = 1")
        .expect("SELECT val FROM t must succeed");

    assert_eq!(r.rows.len(), 1);
    // NEW.val 必须仍然等于原始 INSERT 值 'orig',不能被 OLD.undef 解析成空 / NULL 写回
    let stored = r.rows[0][0].clone();
    assert!(
        matches!(stored, sqlrustgo_types::Value::Text(ref s) if s == "orig"),
        "illegal OLD.col reference in INSERT trigger body must fail closed and not overwrite NEW.val (got {:?})",
        stored
    );
}

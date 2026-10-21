//! V312-35 #4218 — StorageEngine trait API surface integration test.
//!
//! Exercises the new trait methods (`create_view`, `list_views`,
//! `get_view`, `set_cancel_flag`, `check_cancelled`, `kill_connection`,
//! `list_processes`) via the public ExecutionEngine API.
//! Round-trips parser → executor → storage so we catch any
//! AST→executor dispatch regressions too.
//!
//! #5449：这个文件此前**从未被编译过** —— 它既不在 `tests/` 顶层（cargo 不会
//! 自动发现），也没有 `[[test]]` 声明 ⇒ 它的 5 个 `#[test]` 从未执行。
//!
//! 把它变成可执行目标后，第一次编译就暴露了它写的是**另一套 API 名字**：
//!
//! | 本文件原来写的 | 实际存在的 |
//! |---------------|-----------|
//! | `sqlrustgo::parser::parse_statements` | `sqlrustgo_parser::parse_statements` |
//! | `Statement::KillConnection { conn_id, is_query }` | `Statement::Kill { connection_id, kill_query }` |
//! | `Statement::ShowProcesslist { full }` | `Statement::Show(ShowStatement::Processlist { full })` |
//!
//! 实现侧（`crates/parser/src/parser.rs`）的形状是**已发布**的 —— parser 自己的
//! 单元测试（parser.rs:15462 起）就断言 `Statement::Show(ShowStatement::Processlist)`
//! 与 `Statement::Kill { connection_id, kill_query }`。所以过期的是这个测试，
//! 不是被测代码：把测试对齐到**真实**的 API，并补上它声称要覆盖、却从未真正
//! 断言过的 view / cancel-flag 方法。

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_parser::parser::ShowStatement;
use sqlrustgo_parser::{parse_statements, Statement};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(
        MemoryStorage::new(),
    )))
}

// =============================================================================
// 解析层 —— AST 形状
// =============================================================================

#[test]
fn parse_kill_connection_and_show_processlist() {
    let stmts = parse_statements("KILL CONNECTION 42").expect("parse KILL");
    match &stmts[0] {
        Statement::Kill {
            connection_id,
            kill_query,
        } => {
            assert_eq!(*connection_id, 42);
            assert!(!*kill_query);
        }
        other => panic!("expected Statement::Kill, got {:?}", other),
    }

    let stmts = parse_statements("KILL QUERY 7").expect("parse KILL QUERY");
    match &stmts[0] {
        Statement::Kill {
            connection_id,
            kill_query,
        } => {
            assert_eq!(*connection_id, 7);
            assert!(*kill_query);
        }
        other => panic!(
            "expected Statement::Kill with kill_query=true, got {:?}",
            other
        ),
    }

    let stmts = parse_statements("SHOW PROCESSLIST").expect("parse SHOW PROCESSLIST");
    match &stmts[0] {
        Statement::Show(ShowStatement::Processlist { full }) => {
            assert!(!*full);
        }
        other => panic!("expected Show(Processlist), got {:?}", other),
    }

    let stmts = parse_statements("SHOW FULL PROCESSLIST").expect("parse SHOW FULL PROCESSLIST");
    match &stmts[0] {
        Statement::Show(ShowStatement::Processlist { full }) => {
            assert!(*full);
        }
        other => panic!("expected Show(Processlist) full=true, got {:?}", other),
    }
}

#[test]
fn parse_invalid_kill_returns_error() {
    // `KILL` without a numeric id must surface a parse error.
    let res = parse_statements("KILL CONNECTION");
    assert!(
        res.is_err(),
        "expected parse error for KILL with no id, got {:?}",
        res
    );
}

// =============================================================================
// 执行层 —— executor → storage 的分发
// =============================================================================

#[test]
fn kill_returns_graceful_not_implemented_result() {
    // MemoryStorage does not override kill_connection, and there is no live
    // process registry yet, so `execute_kill` returns a **graceful** result
    // rather than an error: the SQL path must not fail, and the wire
    // protocol still emits its OK packet.
    //
    // This is the behaviour the executor's own unit tests lock in
    // (`src/execution_engine_tests.rs::test_executor_kill_v312_35`):
    // Ok, 0 affected rows, one row whose text names the target. The dead
    // version of this test asserted `expect_err` — an API shape that was
    // designed but never implemented — so it had to be corrected to the
    // behaviour that actually ships.
    let mut eng = engine();
    let res = eng
        .execute("KILL CONNECTION 99")
        .expect("KILL must not fail");

    assert_eq!(res.affected_rows, 0);
    assert_eq!(res.rows.len(), 1, "expected exactly one result row");
    let cell = format!("{:?}", res.rows[0][0]);
    assert!(
        cell.contains("KILL") && cell.contains("99"),
        "KILL result row must name the target, got: {}",
        cell
    );
}

#[test]
fn kill_query_distinguishes_query_from_connection() {
    let mut eng = engine();
    let res = eng
        .execute("KILL QUERY 7")
        .expect("KILL QUERY must not fail");
    assert_eq!(res.rows.len(), 1);
    let cell = format!("{:?}", res.rows[0][0]);
    assert!(
        cell.contains("QUERY") && cell.contains("7"),
        "KILL QUERY result row must mark the QUERY form, got: {}",
        cell
    );
}

#[test]
fn show_processlist_returns_empty_rows_for_memory_storage() {
    let mut eng = engine();
    let res = eng
        .execute("SHOW PROCESSLIST")
        .expect("execute SHOW PROCESSLIST");
    // No active connections on a fresh in-memory engine.
    assert_eq!(res.rows.len(), 0, "expected 0 rows, got {}", res.rows.len());
}

#[test]
fn show_full_processlist_returns_empty_rows_for_memory_storage() {
    let mut eng = engine();
    let res = eng
        .execute("SHOW FULL PROCESSLIST")
        .expect("execute SHOW FULL PROCESSLIST");
    assert_eq!(res.rows.len(), 0, "expected 0 rows, got {}", res.rows.len());
}

// =============================================================================
// trait 层 —— StorageEngine 上新增的方法
// =============================================================================

#[test]
fn memory_storage_view_lifecycle() {
    use sqlrustgo_storage::ViewInfo;

    let mut storage = MemoryStorage::new();
    let info = ViewInfo {
        name: "v_orders".to_string(),
        columns: vec!["id".to_string(), "amount".to_string()],
        query_sql: "SELECT id, amount FROM orders".to_string(),
        original_sql: "CREATE VIEW v_orders AS SELECT id, amount FROM orders".to_string(),
    };

    assert!(!storage.has_view("main", "v_orders"));
    storage
        .create_view("main", info.clone())
        .expect("create_view");
    assert!(storage.has_view("main", "v_orders"));

    let got = storage.get_view("main", "v_orders").expect("get_view");
    assert_eq!(got.name, "v_orders");
    assert_eq!(got.query_sql, "SELECT id, amount FROM orders");

    assert_eq!(storage.list_views("main"), vec!["v_orders".to_string()]);

    // A duplicate CREATE VIEW must be rejected, not silently accepted.
    let dup = storage.create_view("main", info);
    assert!(
        dup.is_err(),
        "duplicate create_view must fail, got {:?}",
        dup
    );

    storage.drop_view("main", "v_orders").expect("drop_view");
    assert!(!storage.has_view("main", "v_orders"));
    assert!(storage.list_views("main").is_empty());
}

#[test]
fn memory_storage_cancel_and_process_defaults() {
    let mut storage = MemoryStorage::new();

    // MemoryStorage does not override the cancel/kill/process methods, so the
    // trait defaults apply: unsupported error, no cancel flag, no processes.
    let err = storage
        .set_cancel_flag(7)
        .expect_err("set_cancel_flag must be unsupported");
    let msg = format!("{}", err);
    assert!(
        msg.contains("set_cancel_flag") && msg.contains("not supported"),
        "unexpected set_cancel_flag error: {}",
        msg
    );

    assert!(
        !storage.check_cancelled(7),
        "check_cancelled must default to false"
    );

    let err = storage
        .kill_connection(7)
        .expect_err("kill_connection must be unsupported");
    let msg = format!("{}", err);
    assert!(
        msg.contains("KILL") && msg.contains("not supported"),
        "unexpected kill_connection error: {}",
        msg
    );

    assert!(
        storage.list_processes().is_empty(),
        "list_processes must default to an empty list"
    );
}

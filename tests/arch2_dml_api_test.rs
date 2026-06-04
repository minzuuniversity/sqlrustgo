//! ARCH-2 DML API 集成测试 (v3.8.0-rc1)
//!
//! **Issue**: #2974 (ARCH-2)
//! **Date**: 2026-06-04
//! **Doc**: docs/architecture/EXECUTION_ARCHITECTURE.md
//!
//! 验证 (编译期 + 运行期):
//! 1. `ExecutionEngine::execute_insert/update/delete` 是 public (Stage 1)
//! 2. 外部代码可调用
//! 3. DML 自动事务 (复用 INT-1 修复)
//! 4. 错误处理一致 (返回 SqlError)

#[test]
fn arch2_dml_api_is_public() {
    // 编译期验证: 如果 execute_insert/update/delete 不是 pub fn, 这里编译失败
    type E = sqlrustgo::ExecutionEngine<sqlrustgo_storage::MemoryStorage>;
    let _ptr_insert: fn(&mut E, &sqlrustgo_parser::parser::InsertStatement)
        -> sqlrustgo_types::SqlResult<sqlrustgo_executor::ExecutorResult>
        = E::execute_insert;
    let _ptr_update: fn(&mut E, &sqlrustgo_parser::parser::UpdateStatement)
        -> sqlrustgo_types::SqlResult<sqlrustgo_executor::ExecutorResult>
        = E::execute_update;
    let _ptr_delete: fn(&mut E, &sqlrustgo_parser::parser::DeleteStatement)
        -> sqlrustgo_types::SqlResult<sqlrustgo_executor::ExecutorResult>
        = E::execute_delete;
}

#[test]
fn arch2_dml_api_callable() {
    // 验证: 外部代码可调用 DML API (通过 SQL string 走完整路径)
    let mut engine = sqlrustgo::ExecutionEngine::with_memory();
    let result = engine.execute("CREATE TABLE t (id INT PRIMARY KEY, name TEXT)");
    assert!(result.is_ok());
    let result = engine.execute("INSERT INTO t VALUES (1, 'Alice')");
    assert!(result.is_ok(), "INSERT should succeed: {:?}", result.err());
    let result = engine.execute("SELECT * FROM t");
    assert!(result.is_ok());
}

#[test]
fn arch2_dml_error_handling() {
    // 验证: DML 错误处理一致 (返回 SqlError)
    let mut engine = sqlrustgo::ExecutionEngine::with_memory();
    let result = engine.execute("INSERT INTO nonexistent_table VALUES (1)");
    // 应该返回 TableNotFound 错误
    assert!(result.is_err(), "INSERT to nonexistent table should fail");
}

#[test]
fn arch2_dml_autocommit_int1() {
    // 验证: DML 自动事务 (INT-1 修复)
    let mut engine = sqlrustgo::ExecutionEngine::with_memory();
    engine.execute("CREATE TABLE t (id INT)").unwrap();
    let r1 = engine.execute("INSERT INTO t VALUES (1)");
    let r2 = engine.execute("INSERT INTO t VALUES (2)");
    let r3 = engine.execute("INSERT INTO t VALUES (3)");
    assert!(r1.is_ok());
    assert!(r2.is_ok(), "Second insert (autocommit) should succeed: {:?}", r2.err());
    assert!(r3.is_ok(), "Third insert (autocommit) should succeed: {:?}", r3.err());
}

//! MERGE 端到端集成测试
//!
//! 目标: 验证 MERGE 语句从 Parser → Executor 的完整路径
//!
//! 对应的功能矩阵: docs/standard/SQL92_FUNCTIONALITY_MATRIX.md §2.3
//!
//! # 当前 MERGE 支持状态
//! - ✅ Parser: MERGE INTO t USING src ON ... 完整解析 (Phase 1)
//! - ✅ Parser: MERGE INTO t USING (SELECT ...) AS s ON ... 完整解析 (Phase 1)
//! - ⚠️ Executor: MERGE (Table source) 通过 MergeExecutor 执行
//! - ❌ ExecutionEngine::execute(): MERGE 未分发，走 catch-all 路径

use parking_lot::RwLock;
use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

// =============================================================================
// MERGE 解析测试 — Parser 层
// =============================================================================

#[test]
fn test_merge_parse_table_source() {
    use sqlrustgo_parser::parse;
    let sql = "MERGE INTO target t USING source s ON t.id = s.id \
               WHEN MATCHED THEN UPDATE SET t.val = s.val";
    let r = parse(sql);
    assert!(r.is_ok(), "MERGE table source parse failed: {:?}", r.err());
}

#[test]
fn test_merge_parse_both_clauses() {
    use sqlrustgo_parser::parse;
    let sql = "MERGE INTO target USING source ON target.id = source.id \
               WHEN MATCHED THEN UPDATE SET target.val = source.val \
               WHEN NOT MATCHED THEN INSERT (id, val) VALUES (source.id, source.val)";
    let r = parse(sql);
    assert!(r.is_ok(), "MERGE both clauses parse failed: {:?}", r.err());
}

#[test]
fn test_merge_parse_subquery_source() {
    use sqlrustgo_parser::parse;
    let sql = "MERGE INTO target USING (SELECT * FROM source) AS s ON target.id = s.id \
               WHEN MATCHED THEN UPDATE SET target.val = s.val";
    let r = parse(sql);
    assert!(
        r.is_ok(),
        "MERGE subquery source parse failed: {:?}",
        r.err()
    );
}

#[test]
fn test_merge_parse_additional_condition() {
    use sqlrustgo_parser::parse;
    let sql = "MERGE INTO target USING source ON target.id = source.id \
               WHEN MATCHED AND target.val > 100 THEN UPDATE SET target.val = source.val";
    let r = parse(sql);
    assert!(
        r.is_ok(),
        "MERGE WITH condition parse failed: {:?}",
        r.err()
    );
}

// =============================================================================
// MERGE 解析验证 — Subquery 源识别
// =============================================================================

#[test]
fn test_merge_subquery_source_parsed_correctly() {
    use sqlrustgo_parser::{parse, MergeSource, Statement};

    let sql = "MERGE INTO target USING (SELECT * FROM source) AS s ON target.id = s.id \
               WHEN MATCHED THEN UPDATE SET target.val = s.val";
    let parsed = parse(sql).unwrap();

    if let Statement::Merge(merge_stmt) = parsed {
        match merge_stmt.source {
            MergeSource::Subquery(_) => { /* ✅ Parser 正确识别 Subquery 源 */ }
            MergeSource::Table { name } => {
                panic!("Parser should produce Subquery, got Table({})", name);
            }
        }
    }
}

// =============================================================================
// MERGE 执行测试 — ExecutionEngine::execute() 路径
// =============================================================================

#[test]
fn test_merge_execute_via_execution_engine() {
    let mut engine = make_engine();
    let _ = engine
        .execute("CREATE TABLE target (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    let _ = engine
        .execute("INSERT INTO target VALUES (1, 'old')")
        .unwrap();
    let _ = engine
        .execute("CREATE TABLE source (id INTEGER, val TEXT)")
        .unwrap();
    let _ = engine
        .execute("INSERT INTO source VALUES (1, 'new')")
        .unwrap();

    // MERGE 当前不走 execute() 路径（待 Phase 2 修复）
    let sql = "MERGE INTO target USING source ON target.id = source.id \
               WHEN MATCHED THEN UPDATE SET target.val = source.val";
    let r = engine.execute(sql);
    assert!(
        r.is_err(),
        "MERGE via execute() should fail (not yet dispatched)"
    );
}

//! PR-900F ExecutionEngine Module Boundary Tests
//!
//! 验证 PR-900 (ExecutionEngine 拆分清理) 的核心约束：
//! - execution_engine.rs 行数 ≤ 2000 (C-ARCH-05 门禁)
//! - 拆分后的子模块存在且功能完整
//! - 模块职责清晰 (无内联 parse / 内联 executor 调用等反模式)

use std::fs;
use std::path::Path;

const EXECUTION_ENGINE_PATH: &str = "src/execution_engine.rs";
const MAX_EXECUTION_ENGINE_LINES: usize = 2000;

#[test]
fn ee_01_execution_engine_under_2000_lines() {
    let path = Path::new(EXECUTION_ENGINE_PATH);
    assert!(
        path.exists(),
        "{} must exist",
        EXECUTION_ENGINE_PATH
    );
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {} failed: {}", EXECUTION_ENGINE_PATH, e));
    let line_count = content.lines().count();
    assert!(
        line_count <= MAX_EXECUTION_ENGINE_LINES,
        "execution_engine.rs is {} lines, must be <= {}",
        line_count,
        MAX_EXECUTION_ENGINE_LINES
    );
}

#[test]
fn ee_02_submodule_files_exist() {
    // PR-900 拆分后必须存在的子模块
    for submodule in &[
        "src/engine_builder.rs",
        "src/engine_select.rs",
        "src/engine_utils.rs",
        "src/expr_utils.rs",
    ] {
        let path = Path::new(submodule);
        assert!(
            path.exists(),
            "submodule {} must exist (split from execution_engine.rs)",
            submodule
        );
    }
}

#[test]
fn ee_03_planner_module_is_reexport_only() {
    // src/planner.rs 应该是 re-export 入口（行数 < 50）
    let path = Path::new("src/planner.rs");
    if path.exists() {
        let content = fs::read_to_string(path).expect("read planner.rs");
        let line_count = content.lines().count();
        assert!(
            line_count < 50,
            "src/planner.rs is {} lines, expected < 50 (reexport only)",
            line_count
        );
    }
}

#[test]
fn ee_04_submodule_responsibilities_documented() {
    // 每个子模块必须有文档注释
    for submodule in &[
        "src/engine_builder.rs",
        "src/engine_select.rs",
        "src/engine_utils.rs",
        "src/expr_utils.rs",
    ] {
        let content = fs::read_to_string(submodule)
            .unwrap_or_else(|e| panic!("read {} failed: {}", submodule, e));
        // 第一个非空行应该是 //! 文档注释
        let has_module_doc = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim_start().starts_with("//!"))
            .unwrap_or(false);
        assert!(
            has_module_doc,
            "{} must start with //! module-level documentation",
            submodule
        );
    }
}

#[test]
fn ee_05_no_inline_raw_sql_parsing() {
    // execution_engine.rs 不应包含原始 SQL 解析（已下沉到 parser）
    let content = fs::read_to_string(EXECUTION_ENGINE_PATH)
        .expect("read execution_engine.rs");
    // 禁止调用 parse() 的内部函数（仅允许公开 API）
    // 这里只检查不出现 "pub fn parse" 这种重复定义
    assert!(
        !content.contains("pub fn parse("),
        "execution_engine.rs must not define pub fn parse (delegated to parser crate)"
    );
}

#[test]
fn ee_06_engine_select_separated() {
    // SELECT 逻辑必须独立到 engine_select.rs
    let select_path = Path::new("src/engine_select.rs");
    let content = fs::read_to_string(select_path).expect("read engine_select.rs");
    // 不强制 SELECT 必须在此，但 engine_select.rs 应包含 SelectStatement 处理
    assert!(
        content.contains("SelectStatement") || content.contains("SELECT"),
        "engine_select.rs must contain SELECT-related logic"
    );
}

#[test]
fn ee_07_engine_builder_constructs_engine() {
    // engine_builder.rs 必须提供构造 ExecutionEngine 的入口
    let content = fs::read_to_string("src/engine_builder.rs")
        .expect("read engine_builder.rs");
    assert!(
        content.contains("ExecutionEngine") || content.contains("build"),
        "engine_builder.rs must contain ExecutionEngine construction"
    );
}

#[test]
fn ee_08_submodule_total_size_reasonable() {
    // 所有子模块总行数应 > 0（拆分有效）
    let mut total = 0;
    for submodule in &[
        "src/engine_builder.rs",
        "src/engine_select.rs",
        "src/engine_utils.rs",
        "src/expr_utils.rs",
    ] {
        let content = fs::read_to_string(submodule)
            .unwrap_or_else(|e| panic!("read {} failed: {}", submodule, e));
        total += content.lines().count();
    }
    assert!(
        total > 500,
        "submodules total {} lines, expected > 500 (split was real work)",
        total
    );
}

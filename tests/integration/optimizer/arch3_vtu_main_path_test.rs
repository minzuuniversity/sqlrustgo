//! ARCH-3 (#3169) VtuGuard main-path enforcement test
//!
//! Verifies that every DML entry point invokes
//! `VtuGuard::assert_path_for_dml`. Companion to the G4 gate
//! `scripts/gate/check_arch3_no_bypass.sh` (which is a static check).
//!
//! Refs: docs/openspec/3169-arch3-vtu-main-path.md
//!       V390_TEST_PLAN.md §G4

use sqlrustgo_storage::vtu_guard::VtuGuard;
use std::fs;

fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn test_vtu_guard_assert_path_for_dml_noop() {
    VtuGuard::<()>::assert_path_for_dml("test_op", "test_table");
    VtuGuard::<()>::assert_path_for_dml("insert", "users");
    VtuGuard::<()>::assert_path_for_dml("update", "users");
    VtuGuard::<()>::assert_path_for_dml("delete", "users");
    VtuGuard::<()>::assert_path_for_dml(
        "openclaw_endpoints::handle_delete",
        "very_long_table_name_with_underscores_and_digits_123",
    );
}

#[test]
fn test_vtu_guard_assert_path_for_dml_called_in_execution_engine() {
    // The ExecutionEngine impl block was extracted out of src/execution_engine.rs
    // into src/execution_engine_methods.rs, so the DML entry points now live in
    // the latter. Search both, matching the G4 gate
    // (scripts/gate/check_arch3_no_bypass.sh), which greps `src/execution_engine*.rs`.
    // Searching only execution_engine.rs silently found zero DML functions and
    // failed on `Function execute_insert not found`.
    let mut files = Vec::new();
    for name in ["src/execution_engine.rs", "src/execution_engine_methods.rs"] {
        let path = project_root().join(name);
        files.push((
            name,
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", name, e)),
        ));
    }

    for fn_name in &["execute_insert", "execute_update", "execute_delete"] {
        // Scope the check to the function's OWN body. A fixed-character window
        // is not enough: execute_insert/execute_update/execute_delete are only
        // ~12 lines apart, so a 2000-char window from execute_insert also covers
        // execute_update's guard and masks a deleted execute_insert guard.
        let pattern = format!("pub fn {}", fn_name);
        let found = files.iter().find_map(|(name, content)| {
            let start = content.find(&pattern)?;
            let body = fn_body(content, start);
            assert!(
                body.contains("assert_path_for_dml"),
                "{} in {} must call VtuGuard::assert_path_for_dml (ARCH-3 #3169)",
                fn_name,
                name
            );
            Some(())
        });
        assert!(
            found.is_some(),
            "Function {} not found in src/execution_engine*.rs",
            fn_name
        );
    }
}

/// Return the source of the function whose signature starts at `start`,
/// delimited by balanced braces. Stops early at a nested `}` so the body
/// covers only this function.
fn fn_body(content: &str, start: usize) -> &str {
    let open = content[start..]
        .find('{')
        .map(|i| start + i)
        .unwrap_or_else(|| panic!("no opening brace after byte {}", start));
    let mut depth = 0usize;
    for (i, b) in content[open..].bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &content[open..open + i + 1];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces from byte {}", open);
}

#[test]
fn test_vtu_guard_assert_path_for_dml_called_in_openclaw_endpoints() {
    let path = project_root().join("crates/server/src/openclaw_endpoints.rs");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read openclaw_endpoints.rs: {}", e));

    let count = content.matches("assert_path_for_dml").count();
    assert!(
        count >= 2,
        "openclaw_endpoints.rs has only {} VtuGuard marker calls (expected >= 2)",
        count
    );
}

#[test]
fn test_vtu_guard_storage_path_no_bypass_keywords() {
    for path_str in &[
        "src/execution_engine.rs",
        "crates/server/src/openclaw_endpoints.rs",
    ] {
        let path = project_root().join(path_str);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path_str, e));

        // Strip line comments (//) and block comments (/* */).
        let mut in_block_comment = false;
        let mut stripped = String::new();
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if !in_block_comment && i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                in_block_comment = true;
                i += 2;
                continue;
            }
            if in_block_comment {
                if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                    in_block_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            stripped.push(chars[i]);
            i += 1;
        }
        let code_bypass = stripped.matches("bypass").count();
        assert_eq!(
            code_bypass, 0,
            "{} contains {} 'bypass' references in code (only allowed in comments)",
            path_str, code_bypass
        );
    }
}

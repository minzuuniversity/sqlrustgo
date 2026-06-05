//! INT-3 (#3170) Single Expression Engine test
//!
//! Validates that the parser is single-source: parse_expression is the
//! sole entry point for general expressions; OR/AND/... chain is
//! reachable; no dead methods remain on the canonical paths.
//!
//! Refs: docs/openspec/3170-int3-single-expression-engine.md
//!       V390_TEST_PLAN.md §G3

use std::fs;

fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read_source(rel: &str) -> String {
    let p = project_root().join(rel);
    fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rel, e))
}

#[test]
fn test_int3_parse_expression_chains_through_or() {
    // parse_expression MUST call parse_or_expression so the canonical
    // expression chain (OR > AND > add > mul > primary) is reachable.
    let parser = read_source("crates/parser/src/parser.rs");

    // Locate the parse_expression function body.
    let start = parser
        .find("fn parse_expression(&mut self)")
        .expect("parse_expression function not found");
    let body: String = parser.chars().skip(start).take(1200).collect();
    assert!(
        body.contains("parse_or_expression()"),
        "parse_expression must call parse_or_expression (INT-3 #3170)"
    );
}

#[test]
fn test_int3_parse_json_path_helper_reserved() {
    // parse_json_path_expression is reserved (#[allow(dead_code)]) for
    // future dedicated JSON-path-only callers. It must not be removed
    // without updating the gate.
    let parser = read_source("crates/parser/src/parser.rs");
    let marker_idx = parser
        .find("fn parse_json_path_expression")
        .expect("parse_json_path_expression function not found");
    // Look backwards for the #[allow(dead_code)] attribute.
    let mut pre_start = marker_idx.saturating_sub(200);
    if pre_start > marker_idx {
        pre_start = 0;
    }
    let pre: String = parser[pre_start..marker_idx].to_string();
    assert!(
        pre.contains("allow(dead_code)"),
        "parse_json_path_expression must have #[allow(dead_code)] marker"
    );
}

#[test]
fn test_int3_single_paren_walker_uses_is_some() {
    // The paren-walker loop in parse_derived_table_inner_or_primary
    // uses `is_some()` per clippy::needless_bool, not the verbose
    // `!is_none()` form.
    let parser = read_source("crates/parser/src/parser.rs");
    let count_is_some = parser.matches(".is_some()").count();
    let count_is_none_neg = parser.matches("! .is_none()").count() + parser.matches("!.is_none()").count();
    assert!(
        count_is_some > 0,
        "expected at least one is_some() call (per clippy::needless_bool guidance)"
    );
    // We expect `!is_none()` to be rare; the post-fix target is zero.
    assert!(
        count_is_none_neg == 0,
        "found {} !is_none() calls; should be is_some() (clippy::needless_bool)",
        count_is_none_neg
    );
}

#[test]
fn test_int3_until_close_methods_removed() {
    // The 5 _until_close variants (or/and/additive/multiplicative/primary)
    // were deleted by #3170 because they were dead code. This test
    // documents the cleanup: future PRs that re-introduce them must
    // provide a real consumer (e.g. parse_json_path_expression calling
    // them) and update this test.
    let parser = read_source("crates/parser/src/parser.rs");
    for fn_name in &[
        "parse_or_expression_until_close(",
        "parse_and_expression_until_close(",
        "parse_additive_expression_until_close(",
        "parse_multiplicative_expression_until_close(",
        "parse_primary_expression_until_close(",
    ] {
        let count = parser.matches(fn_name).count();
        assert_eq!(
            count, 0,
            "{}: expected 0 occurrences (count={}). If re-introducing as part of a real consumer, update this test.",
            fn_name, count
        );
    }
}

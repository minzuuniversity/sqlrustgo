//! P0-02 / #4708: Block comment `/* ... */` parser regression witness.
//!
//! The P0-02 entry in `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` claims
//! that `SELECT 1 /* comment */` fails with "Expected expression"
//! because the lexer lacks block-comment support. The opposite is
//! now true: the lexer gained a `/* ... */` branch (see
//! `crates/parser/src/lexer.rs:77-100`) and the test that was
//! `#[ignore]` (crates/parser/tests/wp_a_legacy.rs:68) was re-enabled.
//! This file pins the regression witness so a future lexer change
//! cannot silently break the support claim.

use sqlrustgo::parse;

fn run_parse(sql: &str) -> Result<sqlrustgo::Statement, String> {
    parse(sql).map_err(|e| format!("{e:?}"))
}

#[test]
fn block_comment_inline() {
    // SELECT 1 /* comment */
    let r = run_parse("SELECT 1 /* comment */");
    assert!(r.is_ok(), "block comment should parse: {:?}", r.as_ref().err());
}

#[test]
fn block_comment_prefix() {
    // SELECT /* c1 */ 1
    let r = run_parse("SELECT /* c1 */ 1");
    assert!(r.is_ok(), "block comment prefix should parse: {:?}", r.as_ref().err());
}

#[test]
fn block_comment_leading() {
    // /* header */ SELECT 1
    let r = run_parse("/* header */ SELECT 1");
    assert!(r.is_ok(), "block comment leading should parse: {:?}", r.as_ref().err());
}

#[test]
fn block_comment_unterminated() {
    // MySQL accepts /* unterminated to EOF.
    let r = run_parse("SELECT 1 /* never closed");
    assert!(r.is_ok(), "unterminated block comment accepted by MySQL: {:?}", r.as_ref().err());
}

#[test]
fn block_comment_multiline() {
    // Multi-line block comment.
    let r = run_parse("SELECT /* line1\nline2\nline3 */ 1");
    assert!(r.is_ok(), "multi-line block comment should parse: {:?}", r.as_ref().err());
}

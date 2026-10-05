//! Integration test: INSERT ... ON DUPLICATE KEY UPDATE
//!
//! Uses the wired `sqlrustgo-mysql-server repl` over stdin (true e2e).
//! `exec` only accepts a single statement per process and starts a
//! fresh MemoryStorage each invocation, so multi-statement ODKU
//! testing requires the REPL where state persists across statements.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn bin_path() -> String {
    std::env::var("CARGO_BIN_EXE_sqlrustgo-mysql-server")
        .ok()
        .or_else(|| std::env::var("SQLRUSTGO_BIN").ok())
        .unwrap_or_else(|| {
            for candidate in [
                "target/release/sqlrustgo-mysql-server",
                "target/debug/sqlrustgo-mysql-server",
                "../target/release/sqlrustgo-mysql-server",
                "../target/debug/sqlrustgo-mysql-server",
            ] {
                if std::path::Path::new(candidate).exists() {
                    return candidate.to_string();
                }
            }
            "sqlrustgo-mysql-server".to_string()
        })
}

fn run_repl(script: &str) -> (String, String, i32) {
    let mut child = Command::new(bin_path())
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn REPL");
    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut h = stdout;
        let _ = h.read_to_string(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut h = stderr;
        let _ = h.read_to_string(&mut buf);
        buf
    });
    stdin
        .write_all(script.as_bytes())
        .expect("Failed to write to stdin");
    std::thread::spawn(move || {
        drop(stdin);
    });
    let status = child.wait().expect("wait failed");
    let stdout = stdout_thread.join().expect("stdout thread");
    let stderr = stderr_thread.join().expect("stderr thread");
    let code = status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn test_insert_odku_new_row() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku1 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\n\
         INSERT INTO odku1 VALUES (1, 'apple', 100);\n\
         SELECT * FROM odku1 WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("apple"),
        "should have apple; got:\n{}",
        combined
    );
    assert!(
        combined.contains("100") || combined.contains("Integer(100)"),
        "should have value 100; got:\n{}",
        combined
    );
}

#[test]
fn test_insert_odku_duplicate_key_update() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku2 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\n\
         INSERT INTO odku2 VALUES (1, 'apple', 100);\n\
         INSERT INTO odku2 VALUES (1, 'apple', 100) ON DUPLICATE KEY UPDATE value = 200;\n\
         SELECT * FROM odku2 WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("200") || combined.contains("Integer(200)"),
        "value should be 200 after ODKU update; got:\n{}",
        combined
    );
    // Old value 100 should NOT appear (replaced by 200).
    assert!(
        !combined.contains("Integer(100)"),
        "old value 100 must be replaced; got:\n{}",
        combined
    );
}

#[test]
fn test_insert_odku_multiple_rows() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku3 (id INT PRIMARY KEY, name VARCHAR(50), value INT);\n\
         INSERT INTO odku3 VALUES (1, 'one', 1), (2, 'two', 2), (3, 'three', 3);\n\
         INSERT INTO odku3 VALUES (2, 'two', 999) ON DUPLICATE KEY UPDATE value = 999;\n\
         SELECT name, value FROM odku3 ORDER BY id;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    for name in &["one", "two", "three"] {
        assert!(
            combined.contains(name),
            "should have {}; got:\n{}",
            name,
            combined
        );
    }
    assert!(
        combined.contains("999") || combined.contains("Integer(999)"),
        "should have 999 for two; got:\n{}",
        combined
    );
}

#[test]
fn test_insert_odku_affects_rows() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku4 (id INT PRIMARY KEY, value INT);\n\
         INSERT INTO odku4 VALUES (1, 100), (2, 200);\n\
         SELECT COUNT(*) FROM odku4;\n\
         INSERT INTO odku4 VALUES (1, 999) ON DUPLICATE KEY UPDATE value = 999;\n\
         SELECT COUNT(*) FROM odku4;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    // Both COUNT must show 2 (ODKU must not insert new row on duplicate).
    assert!(
        combined.matches("Integer(2)").count() >= 2,
        "Both COUNT must show Integer(2); got:\n{}",
        combined
    );
}

// ── #5009: `VALUES(col)` row-value references ──────────────────────────
//
// MySQL's `VALUES(col)` inside `ON DUPLICATE KEY UPDATE` denotes the value
// *this INSERT would have written* — the same thing PostgreSQL/SQLite spell
// `EXCLUDED.col`. It is deprecated in MySQL 8.0.20 in favour of row aliases
// (`AS new`), but it is still the most common form in existing production
// code, and the parser used to reject it outright with
// `1064 Parse error: Expected expression`.
//
// The load-bearing detail in every test below: the incoming value differs
// from the stored one, so a `VALUES(col)` that silently resolved to the
// *existing* row would still "pass" a test that only checked the statement
// succeeded. Asserting on the new value is the whole point.

// `VALUES(col)` must resolve to the incoming row, not the stored row.
#[test]
fn test_odku_values_ref_takes_incoming_value() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku_v (id INT PRIMARY KEY, n INT);\n\
         INSERT INTO odku_v VALUES (1, 10);\n\
         INSERT INTO odku_v VALUES (1, 99) ON DUPLICATE KEY UPDATE n = VALUES(n);\n\
         SELECT n FROM odku_v WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("1064") && !combined.contains("Parse error"),
        "#5009: `VALUES(n)` must parse, not 1064; got:\n{}",
        combined
    );
    assert!(
        combined.contains("99") || combined.contains("Integer(99)"),
        "#5009: `VALUES(n)` must be the value this INSERT would write (99), \
         not the stored value (10); got:\n{}",
        combined
    );
    assert!(
        !combined.contains("Integer(10)\n"),
        "#5009: stored value 10 must have been overwritten; got:\n{}",
        combined
    );
}

// Mixing both spellings in one statement: they must agree, because the
// parser lowers `VALUES(col)` to the very same `excluded.col` shape.
#[test]
fn test_odku_values_ref_agrees_with_excluded() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku_b (id INT PRIMARY KEY, a INT, b INT);\n\
         INSERT INTO odku_b VALUES (1, 1, 1);\n\
         INSERT INTO odku_b VALUES (1, 42, 43) ON DUPLICATE KEY UPDATE \
         a = VALUES(a), b = EXCLUDED.b;\n\
         SELECT a, b FROM odku_b WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("1064") && !combined.contains("Parse error"),
        "#5009: both spellings must parse; got:\n{}",
        combined
    );
    assert!(
        combined.contains("42") && combined.contains("43"),
        "#5009: `VALUES(a)` and `EXCLUDED.b` must both take the incoming \
         values 42/43; got:\n{}",
        combined
    );
}

// The lowering must not swallow an ordinary expression on the RHS.
#[test]
fn test_odku_literal_and_arithmetic_still_work() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku_l (id INT PRIMARY KEY, v INT);\n\
         INSERT INTO odku_l VALUES (1, 5);\n\
         INSERT INTO odku_l VALUES (1, 0) ON DUPLICATE KEY UPDATE v = v + 100;\n\
         INSERT INTO odku_l VALUES (1, 0) ON DUPLICATE KEY UPDATE v = 7;\n\
         SELECT v FROM odku_l WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        !combined.contains("Parse error"),
        "literal / arithmetic ODKU must keep working; got:\n{}",
        combined
    );
    assert!(
        combined.contains("7") || combined.contains("Integer(7)"),
        "`v = 7` must win as the last assignment; got:\n{}",
        combined
    );
}

// A bare `VALUES` is NOT a row reference. The lowering must only fire on
// the exact `VALUES ( ident )` shape; anything else has to fall through to
// the general expression parser and be rejected there — and, critically,
// a rejected statement must leave the stored row untouched.
#[test]
fn test_odku_bare_values_not_reinterpreted() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku_nv (id INT PRIMARY KEY, v INT);\n\
         INSERT INTO odku_nv VALUES (1, 5);\n\
         INSERT INTO odku_nv VALUES (1, 6) ON DUPLICATE KEY UPDATE v = VALUES;\n\
         SELECT v FROM odku_nv WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("Parse error"),
        "#5009: a bare `VALUES` is not a row reference and must be rejected \
         by the general expression parser; got:\n{}",
        combined
    );
    assert!(
        combined.contains("Integer(5)"),
        "#5009: the rejected statement must leave the stored row at 5, \
         not write the incoming 6; got:\n{}",
        combined
    );
}

// Over-lowering guard: `VALUES ( a , b )` is not a row reference either
// (MySQL takes exactly one column). A greedy lowering that accepted the
// comma list would silently assign the wrong thing instead of erroring.
#[test]
fn test_odku_values_ref_requires_exactly_one_column() {
    let (out, err, _code) = run_repl(
        "CREATE TABLE odku_mc (id INT PRIMARY KEY, a INT, b INT);\n\
         INSERT INTO odku_mc VALUES (1, 5, 5);\n\
         INSERT INTO odku_mc VALUES (1, 6, 7) ON DUPLICATE KEY UPDATE \
         a = VALUES(a, b);\n\
         SELECT a, b FROM odku_mc WHERE id = 1;\n\
         .exit\n",
    );
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("Parse error"),
        "#5009: `VALUES(a, b)` takes no row-reference meaning; the lowering \
         must not accept a comma list; got:\n{}",
        combined
    );
    assert!(
        combined.contains("Integer(5)"),
        "#5009: the rejected multi-arg statement must not have written \
         anything; got:\n{}",
        combined
    );
}

use std::process::Command;

fn sqlrustgo_bin() -> String {
    std::env::var("SQLRUSTGO_BIN")
        .unwrap_or_else(|_| "/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo".to_string())
}

fn sqlite_bin() -> String {
    std::env::var("SQLITE_BIN")
        .unwrap_or_else(|_| "/usr/bin/sqlite3".to_string())
}

fn run_sql(sqlrustgo: bool, sql: &str) -> String {
    let bin = if sqlrustgo {
        sqlrustgo_bin()
    } else {
        sqlite_bin()
    };

    let output = if sqlrustgo {
        Command::new(&bin)
            .args(["sqlite", "--batch", "--mode", "csv", "--headers", "true", ":memory:"])
            .arg(sql)
            .output()
            .expect("Failed to execute sqlrustgo")
    } else {
        Command::new(&bin)
            .args(["-csv", "-header", ":memory:"])
            .arg(sql)
            .output()
            .expect("Failed to execute sqlite3")
    };

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn normalize_csv(csv: &str) -> String {
    csv.lines()
        .filter(|l| !l.trim().is_empty())
        .skip(1) // skip header
        .map(|l| l.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_match(test_name: &str, sql: &str) {
    let ours = normalize_csv(&run_sql(true, sql));
    let theirs = normalize_csv(&run_sql(false, sql));

    if ours == theirs {
        println!("  PASS: {}", test_name);
    } else {
        println!("  FAIL: {}", test_name);
        println!("    SQLRustGo: {}", ours.lines().next().unwrap_or("(empty)"));
        println!("    SQLite:    {}", theirs.lines().next().unwrap_or("(empty)"));
        panic!("Differential test failed: {}", test_name);
    }
}

#[test]
fn test_p3_math_001_mod_integer() {
    assert_match(
        "P3-MATH-001: MOD returns Integer",
        "CREATE TABLE t(a INT, b INT); INSERT INTO t VALUES (10, 3); SELECT MOD(a, b) FROM t;",
    );
}

#[test]
fn test_p3_agg_002_count_sum() {
    assert_match(
        "P3-AGG-002: COUNT vs SUM",
        "CREATE TABLE t(val INT); INSERT INTO t VALUES (1), (2), (3), (NULL); SELECT COUNT(val), SUM(val) FROM t;",
    );
}

#[test]
fn test_p3_set_001_union_all() {
    assert_match(
        "P3-SET-001: UNION ALL",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT * FROM t1 UNION ALL SELECT * FROM t2 ORDER BY id;",
    );
}

#[test]
fn test_p3_set_002_union_distinct() {
    assert_match(
        "P3-SET-002: UNION DISTINCT",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT * FROM t1 UNION SELECT * FROM t2 ORDER BY id;",
    );
}

#[test]
fn test_p3_null_001_null_comparison() {
    assert_match(
        "P3-NULL-001: NULL in WHERE",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, NULL), (2, 10); SELECT * FROM t WHERE val IS NULL;",
    );
}

#[test]
fn test_p3_char_001_char_length() {
    assert_match(
        "P3-CHAR-001: CHAR_LENGTH",
        "SELECT CHAR_LENGTH('hello');",
    );
}

#[test]
fn test_p3_agg_001_sum_empty() {
    assert_match(
        "P3-AGG-001: SUM empty",
        "CREATE TABLE t(val INT); SELECT SUM(val) FROM t;",
    );
}

#[test]
fn test_p3_tx_001_rollback() {
    assert_match(
        "P3-TX-001: Transaction rollback",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10); BEGIN; UPDATE t SET val = 20 WHERE id = 1; ROLLBACK; SELECT val FROM t WHERE id = 1;",
    );
}

#[test]
fn test_p3_update_001_expression() {
    assert_match(
        "P3-UPDATE-001: UPDATE expression",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10); UPDATE t SET val = val * 2 WHERE id = 1; SELECT val FROM t WHERE id = 1;",
    );
}

#[test]
fn test_p3_schema_001_sqlite_master() {
    assert_match(
        "P3-SCHEMA-001: sqlite_master",
        "CREATE TABLE t(id INT); SELECT type, name FROM sqlite_master WHERE name = 't';",
    );
}

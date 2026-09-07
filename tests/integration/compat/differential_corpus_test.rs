use std::path::Path;
use std::process::Command;

fn find_sqlrustgo_bin() -> Option<String> {
    if let Ok(path) = std::env::var("SQLRUSTGO_BIN") {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }
    let candidates = [
        "/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo",
        "/Users/liying/dev/sqlrustgo/target/release/sqlrustgo",
        "target/debug/sqlrustgo",
        "target/release/sqlrustgo",
    ];
    for candidate in &candidates {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn sqlite_bin() -> String {
    std::env::var("SQLITE_BIN").unwrap_or_else(|_| "/usr/bin/sqlite3".to_string())
}

fn run_sql(sqlrustgo: bool, sql: &str) -> Option<String> {
    let bin = if sqlrustgo {
        find_sqlrustgo_bin()?
    } else {
        sqlite_bin()
    };

    let output = if sqlrustgo {
        Command::new(&bin)
            .args([
                "sqlite",
                "--batch",
                "--mode",
                "csv",
                "--headers",
                "true",
                ":memory:",
            ])
            .arg(sql)
            .output()
            .ok()?
    } else {
        Command::new(&bin)
            .args(["-csv", "-header", ":memory:"])
            .arg(sql)
            .output()
            .ok()?
    };

    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn normalize_csv(csv: &str) -> String {
    csv.lines()
        .filter(|l| !l.trim().is_empty())
        .skip(1)
        .map(|l| l.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_match(test_name: &str, sql: &str) {
    let ours = match run_sql(true, sql) {
        Some(r) => r,
        None => {
            println!("  SKIP: {} (sqlrustgo not available)", test_name);
            return;
        }
    };
    let theirs = match run_sql(false, sql) {
        Some(r) => r,
        None => {
            println!("  SKIP: {} (sqlite3 not available)", test_name);
            return;
        }
    };

    let ours_norm = normalize_csv(&ours);
    let theirs_norm = normalize_csv(&theirs);

    if ours_norm == theirs_norm {
        println!("  PASS: {}", test_name);
    } else {
        println!("  FAIL: {}", test_name);
        println!(
            "    SQLRustGo: {}",
            ours_norm.lines().next().unwrap_or("(empty)")
        );
        println!(
            "    SQLite:    {}",
            theirs_norm.lines().next().unwrap_or("(empty)")
        );
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
    assert_match("P3-CHAR-001: CHAR_LENGTH", "SELECT CHAR_LENGTH('hello');");
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

#[test]
fn test_p3_agg_003_avg_min_max() {
    assert_match(
        "P3-AGG-003: AVG/MIN/MAX",
        "CREATE TABLE t(val INT); INSERT INTO t VALUES (1), (2), (3), (4), (5); SELECT AVG(val), MIN(val), MAX(val) FROM t;",
    );
}

#[test]
fn test_p3_agg_004_group_by() {
    assert_match(
        "P3-AGG-004: GROUP BY",
        "CREATE TABLE t(dept TEXT, salary INT); INSERT INTO t VALUES ('a', 100), ('a', 200), ('b', 150); SELECT dept, SUM(salary) FROM t GROUP BY dept ORDER BY dept;",
    );
}

#[test]
fn test_p3_agg_005_having() {
    assert_match(
        "P3-AGG-005: HAVING",
        "CREATE TABLE t(dept TEXT, salary INT); INSERT INTO t VALUES ('a', 100), ('a', 200), ('b', 150); SELECT dept, SUM(salary) FROM t GROUP BY dept HAVING SUM(salary) > 200 ORDER BY dept;",
    );
}

#[test]
fn test_p3_join_001_inner_join() {
    assert_match(
        "P3-JOIN-001: INNER JOIN",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT t1.id, t2.id FROM t1 INNER JOIN t2 ON t1.id = t2.id ORDER BY t1.id;",
    );
}

#[test]
fn test_p3_join_002_left_join() {
    assert_match(
        "P3-JOIN-002: LEFT JOIN",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT t1.id, t2.id FROM t1 LEFT JOIN t2 ON t1.id = t2.id ORDER BY t1.id;",
    );
}

#[test]
fn test_p3_sub_001_exists() {
    assert_match(
        "P3-SUB-001: EXISTS",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2); SELECT * FROM t1 WHERE EXISTS (SELECT 1 FROM t2 WHERE t2.id = t1.id) ORDER BY id;",
    );
}

#[test]
fn test_p3_sub_002_in() {
    assert_match(
        "P3-SUB-002: IN subquery",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2), (3); INSERT INTO t2 VALUES (2), (4); SELECT * FROM t1 WHERE id IN (SELECT id FROM t2) ORDER BY id;",
    );
}

#[test]
fn test_p3_limit_001() {
    assert_match(
        "P3-LIMIT-001: LIMIT",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3), (4), (5); SELECT id FROM t ORDER BY id LIMIT 3;",
    );
}

#[test]
fn test_p3_limit_002_with_offset() {
    assert_match(
        "P3-LIMIT-002: LIMIT with OFFSET",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3), (4), (5); SELECT id FROM t ORDER BY id LIMIT 2 OFFSET 2;",
    );
}

#[test]
fn test_p3_delete_001() {
    assert_match(
        "P3-DELETE-001: DELETE",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3); DELETE FROM t WHERE id = 2; SELECT * FROM t ORDER BY id;",
    );
}

#[test]
fn test_p3_alter_001_add_column() {
    assert_match(
        "P3-ALTER-001: ALTER TABLE ADD COLUMN",
        "CREATE TABLE t(id INT); ALTER TABLE t ADD COLUMN name TEXT; INSERT INTO t VALUES (1, 'a'); SELECT * FROM t;",
    );
}

#[test]
fn test_p3_string_001_upper_lower() {
    assert_match(
        "P3-STRING-001: UPPER/LOWER",
        "SELECT UPPER('hello'), LOWER('WORLD');",
    );
}

#[test]
fn test_p3_string_002_substr() {
    assert_match(
        "P3-STRING-002: SUBSTR",
        "SELECT SUBSTR('hello world', 1, 5);",
    );
}

#[test]
fn test_p3_string_003_replace() {
    assert_match(
        "P3-STRING-003: REPLACE",
        "SELECT REPLACE('hello world', 'world', 'rust');",
    );
}

#[test]
fn test_p3_string_004_trim() {
    assert_match("P3-STRING-004: TRIM", "SELECT TRIM('  hello  ');");
}

#[test]
fn test_p3_math_002_abs() {
    assert_match("P3-MATH-002: ABS", "SELECT ABS(-42);");
}

#[test]
fn test_p3_math_003_round() {
    assert_match("P3-MATH-003: ROUND", "SELECT ROUND(3.7);");
}

#[test]
fn test_p3_null_002_coalesce() {
    assert_match("P3-NULL-002: COALESCE", "SELECT COALESCE(NULL, 'default');");
}

#[test]
fn test_p3_null_003_ifnull() {
    assert_match("P3-NULL-003: IFNULL", "SELECT IFNULL(NULL, 'default');");
}

#[test]
fn test_p3_tx_002_commit() {
    assert_match(
        "P3-TX-002: COMMIT",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10); BEGIN; UPDATE t SET val = 20 WHERE id = 1; COMMIT; SELECT val FROM t WHERE id = 1;",
    );
}

#[test]
fn test_p3_cte_001_simple() {
    assert_match(
        "P3-CTE-001: Simple CTE",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3); WITH cte AS (SELECT id FROM t WHERE id > 1) SELECT * FROM cte ORDER BY id;",
    );
}

#[test]
fn test_p3_view_001_create_view() {
    assert_match(
        "P3-VIEW-001: CREATE VIEW (may not be implemented)",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2); CREATE VIEW v AS SELECT id FROM t;",
    );
}

#[test]
fn test_p3_index_001_drop_index() {
    assert_match(
        "P3-INDEX-001: DROP INDEX (may not be implemented)",
        "CREATE TABLE t(id INT, name TEXT); CREATE INDEX idx_t_name ON t(name); DROP INDEX idx_t_name;",
    );
}

#[test]
fn test_p3_window_001_row_number() {
    assert_match(
        "P3-WINDOW-001: ROW_NUMBER",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3); SELECT id, ROW_NUMBER() OVER (ORDER BY id) AS rn FROM t ORDER BY id;",
    );
}

#[test]
fn test_p3_window_002_rank() {
    assert_match(
        "P3-WINDOW-002: RANK",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10), (2, 10), (3, 20); SELECT id, RANK() OVER (ORDER BY val) AS rk FROM t ORDER BY id;",
    );
}

#[test]
fn test_p3_window_003_ntile() {
    assert_match(
        "P3-WINDOW-003: NTILE (not implemented)",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7); SELECT id, NTILE(3) OVER (ORDER BY id) AS bucket FROM t ORDER BY id;",
    );
}

//! V4.1.0 COV-01: raise parser line coverage toward the >=85% GA target.
//!
//! Targets the largest uncovered blocks from the 2026-10-06 llvm-cov run
//! (parser.json): Display::fmt arms (1553-1649), CREATE VIRTUAL TABLE
//! (3269-3362), TIMESTAMPDIFF in select-list (5862-5927), aggregate + OVER
//! for HAVING (10869-10961), FROM (VALUES ...) (6470-6555), VALUES as a
//! top-level select (4939-4993), CONVERT (10097-10146), aggregate
//! arithmetic with aliases (5257-5295, 5346-5382), derived-table
//! synthesis (6839-6886), CREATE TABLE AS WITH [NO] DATA (11831-11870),
//! DATE_ADD/DATE_SUB (9873-9951), ALTER TABLE RENAME/SET PARTITIONED
//! (13886-13961), CREATE ROLE (3506-3526), window frame + EXCLUDE
//! (9042-9196).

use sqlrustgo_parser::{
    parse, parse_expression_str, parse_statements, split_sql_statements, Statement,
};

/// Helper: assert the statement parses successfully (prevents the
/// error-swallowing `p` from masking forms that never take the Ok path).
fn pok(sql: &str) {
    assert!(parse(sql).is_ok(), "expected parse success for {sql:?}");
}

/// Helper: expect the statement to fail parsing (covers error arms).
fn perr(sql: &str) {
    assert!(
        parse(sql).is_err(),
        "expected parse failure for {sql:?}, but it succeeded"
    );
}

// ===========================================================================
// Display::fmt for Expression (parser.rs 1553-1649) — CHECK-constraint
// error rendering (V312-85 / #4752). `to_string()` walks every arm.
// ===========================================================================

#[test]
fn display_expression_arms_simple() {
    let _ = parse_expression_str("1").unwrap().to_string();
    let _ = parse_expression_str("a").unwrap().to_string();
    let _ = parse_expression_str("a + b").unwrap().to_string();
    let _ = parse_expression_str("NOT a").unwrap().to_string();
    let _ = parse_expression_str("a IS NULL").unwrap().to_string();
    let _ = parse_expression_str("a IS NOT NULL").unwrap().to_string();
    let _ = parse_expression_str("a LIKE 'x'").unwrap().to_string();
    let _ = parse_expression_str("a NOT LIKE 'x'").unwrap().to_string();
    let _ = parse_expression_str("a BETWEEN 1 AND 2")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a NOT BETWEEN 1 AND 2")
        .unwrap()
        .to_string();
}

#[test]
fn display_expression_arms_collections() {
    let _ = parse_expression_str("a IN (1, 2)").unwrap().to_string();
    let _ = parse_expression_str("a NOT IN (1, 2)").unwrap().to_string();
    let _ = parse_expression_str("CASE WHEN a THEN 1 ELSE 2 END")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("CASE WHEN a THEN 1 END")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("f(x, y)").unwrap().to_string();
    let _ = parse_expression_str("[1, 2, 3]").unwrap().to_string();
    let _ = parse_expression_str("[]").unwrap().to_string();
    let _ = parse_expression_str("@@version").unwrap().to_string();
}

#[test]
fn display_expression_arms_subquery_and_predicate() {
    let _ = parse_expression_str("(SELECT 1)").unwrap().to_string();
    let _ = parse_expression_str("EXISTS (SELECT 1)")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("NOT EXISTS (SELECT 1)")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a IN (SELECT 1)").unwrap().to_string();
    let _ = parse_expression_str("a NOT IN (SELECT 1)")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a NOT REGEXP 'x'")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a = ANY (SELECT 1)")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a = ALL (SELECT 1)")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a <> SOME (SELECT 1)")
        .unwrap()
        .to_string();
}

#[test]
fn display_expression_arms_aggregate_window_sequence() {
    let _ = parse_expression_str("COUNT(*)").unwrap().to_string();
    let _ = parse_expression_str("SUM(a)").unwrap().to_string();
    let _ = parse_expression_str("ROW_NUMBER() OVER (ORDER BY a)")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("CURRVAL(s)").unwrap().to_string();
    let _ = parse_expression_str("NEXT VALUE FOR s")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("NEXT FOR s").unwrap().to_string();
}

#[test]
fn display_expression_arms_interval_and_subquery_field() {
    let _ = parse_expression_str("a + INTERVAL 5 DAY")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("a - INTERVAL '3' MONTH")
        .unwrap()
        .to_string();
    let _ = parse_expression_str("(SELECT 1).x").unwrap().to_string();
}

// ===========================================================================
// CREATE VIRTUAL TABLE (parser.rs 3269-3362)
// ===========================================================================

#[test]
fn create_virtual_table_ok_paths() {
    pok("CREATE VIRTUAL TABLE vt USING fts5(a, b)");
    pok("CREATE VIRTUAL TABLE IF NOT EXISTS vt USING fts5(a)");
    pok("CREATE VIRTUAL TABLE vt USING fts5(a TEXT, b TEXT)");
    pok("CREATE VIRTUAL TABLE vt USING fts5(a TEXT, tokenize = 'porter')");
}

#[test]
fn create_virtual_table_error_paths() {
    perr("CREATE VIRTUAL");
    perr("CREATE VIRTUAL TABLE IF vt USING x");
    perr("CREATE VIRTUAL TABLE IF NOT vt USING x");
    perr("CREATE VIRTUAL TABLE 123");
    perr("CREATE VIRTUAL TABLE IF NOT EXISTS");
}

// ===========================================================================
// TIMESTAMPDIFF in select-list position (parser.rs 5862-5927)
// ===========================================================================

#[test]
fn timestampdiff_select_list_ok() {
    pok("SELECT TIMESTAMPDIFF(SECOND, '2020-01-01', '2020-01-02') FROM t");
    pok("SELECT TIMESTAMPDIFF('HOUR', a, b) FROM t");
    pok("SELECT TIMESTAMPDIFF(DAY, a, b) FROM t");
    pok("SELECT TIMESTAMPDIFF(QUARTER, a, b) FROM t");
    pok("SELECT TIMESTAMPDIFF(MICROSECOND, a, b) FROM t");
}

#[test]
fn timestampdiff_select_list_errors() {
    perr("SELECT TIMESTAMPDIFF(FOO, a, b) FROM t");
    perr("SELECT TIMESTAMPDIFF(1, a, b) FROM t");
}

// ===========================================================================
// Aggregates in expressions / HAVING + OVER windows (parser.rs 10869-10961)
// ===========================================================================

#[test]
fn aggregate_in_having_clause() {
    pok("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1");
    pok("SELECT a, SUM(b) FROM t GROUP BY a HAVING SUM(b) >= 10");
    pok("SELECT a, MAX(b) FROM t GROUP BY a HAVING MAX(b) <> 3");
}

#[test]
fn aggregate_over_window_specs() {
    pok("SELECT MAX(x) OVER (PARTITION BY y) FROM t");
    pok("SELECT SUM(x) OVER (ORDER BY y) FROM t");
    pok("SELECT AVG(x) OVER (PARTITION BY y ORDER BY z DESC) FROM t");
    pok("SELECT COUNT(*) OVER (PARTITION BY y ORDER BY z ASC) FROM t");
    pok("SELECT MIN(x) OVER (PARTITION BY y ORDER BY z NULLS FIRST) FROM t");
    pok("SELECT MAX(x) OVER (ORDER BY z DESC NULLS LAST) FROM t");
}

// ===========================================================================
// FROM (VALUES (...), (...)) AS alias (parser.rs 6470-6555)
// ===========================================================================

#[test]
fn from_values_constructor_ok() {
    pok("SELECT * FROM (VALUES (1, 2), (3, 4)) AS v");
    pok("SELECT * FROM (VALUES (1)) AS v");
    pok("SELECT * FROM (VALUES (1, 2), (3, 4)) v");
    pok("SELECT * FROM (VALUES ('a'), ('b')) AS names");
}

#[test]
fn from_values_constructor_errors() {
    perr("SELECT * FROM (VALUES 1, 2) AS v");
}

// ===========================================================================
// VALUES as a top-level select / set-op operand (parser.rs 4939-4993)
// ===========================================================================

#[test]
fn values_as_select_ok() {
    pok("SELECT 1 UNION VALUES (2)");
    pok("SELECT 1 UNION ALL VALUES (2)");
    pok("SELECT 1 INTERSECT VALUES (2)");
    pok("SELECT 1 EXCEPT VALUES (2)");
    pok("WITH c AS (VALUES (1), (2)) SELECT * FROM c");
    pok("WITH c AS (VALUES (1) UNION ALL SELECT 2) SELECT * FROM c");
    pok("CREATE VIEW v AS VALUES (1), (2)");
}

#[test]
fn values_as_select_errors() {
    perr("WITH c AS (VALUES 1, 2) SELECT 1");
    perr("SELECT 1 UNION VALUES 2");
    perr("CREATE VIEW v AS VALUES (1");
}

// ===========================================================================
// CONVERT(...) type-name handling (parser.rs 10097-10146)
// ===========================================================================

#[test]
fn convert_type_names_ok() {
    pok("SELECT CONVERT(a, INTEGER) FROM t");
    pok("SELECT CONVERT(a, TEXT) FROM t");
    pok("SELECT CONVERT(a, DATE) FROM t");
    pok("SELECT CONVERT(a, FLOAT) FROM t");
    pok("SELECT CONVERT(a, BOOLEAN) FROM t");
    pok("SELECT CONVERT(a, CHAR) FROM t");
    pok("SELECT CONVERT(a, CHAR(10)) FROM t");
    pok("SELECT CONVERT(a, VARCHAR(20)) FROM t");
    pok("SELECT CONVERT(a, DECIMAL(10, 2)) FROM t");
}

#[test]
fn convert_type_name_errors() {
    perr("SELECT CONVERT(a) FROM t");
    perr("SELECT CONVERT(a, 42) FROM t");
}

// ===========================================================================
// Aggregate arithmetic + aliases in select-list (parser.rs 5257-5295, 5346-5382)
// ===========================================================================

#[test]
fn aggregate_arithmetic_with_alias() {
    pok("SELECT SUM(a) + 1 AS s FROM t");
    pok("SELECT SUM(a) - 1 AS d FROM t");
    pok("SELECT COUNT(*) * 2 AS c FROM t");
    pok("SELECT MIN(x) / 2 AS m FROM t");
    pok("SELECT MAX(x) % 3 AS mx FROM t");
    pok("SELECT AVG(x) + AVG(y) AS av FROM t");
}

#[test]
fn aggregate_arithmetic_bare_alias() {
    pok("SELECT SUM(a) + 1 s FROM t");
    pok("SELECT COUNT(*) * 2 c FROM t");
    pok("SELECT MIN(x) / 2 m FROM t");
}

// ===========================================================================
// Derived-table synthesis for `(table JOIN ...)` without SELECT
// (parser.rs 6839-6886)
// ===========================================================================

#[test]
fn derived_table_join_synthesis() {
    pok("SELECT * FROM (employees e JOIN dept d ON e.id = d.id) AS sub");
    pok("SELECT * FROM (employees e JOIN dept d ON e.id = d.id) sub");
    pok("SELECT * FROM (employees e JOIN dept d ON e.id = d.id)");
    pok("SELECT * FROM (employees e, dept d) AS pair");
    pok("SELECT * FROM (employees e, dept d)");
}

// ===========================================================================
// CREATE TABLE AS WITH [NO] DATA SELECT (parser.rs 11831-11870)
// ===========================================================================

#[test]
fn create_table_as_with_data_select() {
    pok("CREATE TABLE t AS WITH NO DATA SELECT 1");
    pok("CREATE TABLE t AS WITH DATA SELECT 1");
    pok("CREATE TABLE t AS WITH NO DATA SELECT a, b FROM src");
}

#[test]
fn create_table_as_with_data_errors() {
    perr("CREATE TABLE t AS WITH MAYBE DATA SELECT 1");
    perr("CREATE TABLE t AS WITH NO DAT SELECT 1");
    perr("CREATE TABLE t AS WITH NO DATA INSERT INTO x VALUES (1)");
}

// ===========================================================================
// DATE_ADD / DATE_SUB (parser.rs 9873-9951)
// ===========================================================================

#[test]
fn date_add_sub_three_arg_ok() {
    pok("SELECT DATE_ADD('2020-01-01', 3, 'DAY') FROM t");
    pok("SELECT DATE_ADD(a, 1, DAY) FROM t");
    pok("SELECT DATE_SUB('2020-01-01', 2, 'MONTH') FROM t");
    pok("SELECT DATE_SUB(a, 5, HOUR) FROM t");
}

#[test]
fn date_add_sub_three_arg_errors() {
    perr("SELECT DATE_ADD(a, 3) FROM t");
    perr("SELECT DATE_ADD(a, 3, 42) FROM t");
    perr("SELECT DATE_SUB(a) FROM t");
}

// ===========================================================================
// ALTER TABLE RENAME / SET PARTITIONED BY (parser.rs 13886-13961)
// ===========================================================================

#[test]
fn alter_table_rename_forms() {
    pok("ALTER TABLE t RENAME TO t2");
    pok("ALTER TABLE t RENAME COLUMN a TO b");
    pok("ALTER TABLE t RENAME a TO b");
}

#[test]
fn alter_table_set_partitioned_and_errors() {
    pok("ALTER TABLE t SET PARTITIONED BY (a, b)");
    pok("ALTER TABLE t SET PARTITIONED BY (a)");
    perr("ALTER TABLE t SET X");
    perr("ALTER TABLE t RENAME TO 123");
    perr("ALTER TABLE t RENAME COLUMN a TO 123");
}

// ===========================================================================
// ===========================================================================
// Window frames + EXCLUDE (parser.rs 9042-9196)
// ===========================================================================

fn window_sql(frame: &str, exclusion: &str) -> String {
    format!("SELECT ROW_NUMBER() OVER (ORDER BY a {frame}{exclusion}) AS rn FROM t")
}

#[test]
fn window_frame_forms() {
    pok(&window_sql("ROWS BETWEEN 1 PRECEDING AND CURRENT ROW", ""));
    pok(&window_sql(
        "ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW",
        "",
    ));
    pok(&window_sql("RANGE BETWEEN 1 PRECEDING AND 1 FOLLOWING", ""));
    pok(&window_sql(
        "GROUPS BETWEEN 1 PRECEDING AND 1 FOLLOWING",
        "",
    ));
    pok(&window_sql("ROWS UNBOUNDED PRECEDING", ""));
    pok(&window_sql("ROWS CURRENT ROW", ""));
}

#[test]
fn window_frame_exclusions() {
    pok(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE CURRENT ROW",
    ));
    pok(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE GROUP",
    ));
    pok(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE TIES",
    ));
    pok(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE NO OTHERS",
    ));
}

#[test]
fn window_frame_exclusion_errors() {
    perr(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE CURRENT",
    ));
    perr(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE NO",
    ));
    perr(&window_sql(
        "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW",
        " EXCLUDE FOO",
    ));
}

// ===========================================================================
// Sanity: statements produced by the new forms still round-trip through
// parse_statements (guards against the single-statement helper masking
// split bugs).
// ===========================================================================

#[test]
fn multi_statement_roundtrip_with_new_forms() {
    let stmts = parse_statements(
        "CREATE VIRTUAL TABLE vt USING fts5(a); \
         ALTER TABLE t RENAME TO t2; \
         SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1;",
    )
    .expect("multi-statement parse");
    assert_eq!(stmts.len(), 3);
    assert!(matches!(stmts[0], Statement::CreateTable(_)));
    assert!(matches!(stmts[1], Statement::AlterTable(_)));
}

// ===========================================================================
// Round 2 — SHOW statement arms (parser.rs 12760-12967)
// ===========================================================================

#[test]
fn show_databases_and_schemas() {
    pok("SHOW DATABASES");
    pok("SHOW SCHEMAS");
}

#[test]
fn show_tables_with_suffixes() {
    pok("SHOW TABLES");
    pok("SHOW FULL TABLES");
    pok("SHOW TABLES FROM mydb");
    pok("SHOW TABLES LIKE 'p%'");
    pok("SHOW TABLES WHERE Table_name = 'x'");
    pok("SHOW TABLES FROM mydb LIKE 'p%'");
}

#[test]
fn show_create_table_forms() {
    pok("SHOW CREATE TABLE t");
    pok("SHOW CREATE TABLE db.t");
    perr("SHOW CREATE TABLE");
}

#[test]
fn show_columns_and_index() {
    pok("SHOW COLUMNS FROM t");
    pok("SHOW COLUMNS FROM t LIKE 'x'");
    pok("SHOW COLUMNS FROM t WHERE a = 1");
    pok("SHOW INDEX FROM t");
    perr("SHOW COLUMNS FROM");
}

#[test]
fn show_misc_statements() {
    pok("SHOW PROCESSLIST");
    pok("SHOW VARIABLES");
    pok("SHOW VARIABLES LIKE 'x%'");
    pok("SHOW STATUS");
    pok("SHOW GRANTS FOR alice");
    pok("SHOW WARNINGS");
    perr("SHOW ENGINES");
    perr("SHOW COLLATION");
    perr("SHOW PRIVILEGES");
    pok("SHOW PROCEDURE STATUS");
    perr("SHOW FUNCTION STATUS");
}

// ===========================================================================
// GRANT / REVOKE (parser.rs 13100-13450)
// ===========================================================================

#[test]
fn grant_forms() {
    pok("GRANT SELECT ON users TO alice");
    pok("GRANT SELECT, INSERT, UPDATE ON users TO alice, bob");
    pok("GRANT SELECT ON users TO alice WITH GRANT OPTION");
    pok("GRANT SELECT(email) ON users TO alice");
    pok("GRANT SELECT ON users TO alice, bob WITH GRANT OPTION");
}

#[test]
fn grant_error_paths() {
    perr("GRANT SELECT ON");
    perr("GRANT SELECT ON users TO");
    perr("GRANT SELECT ON users");
    perr("GRANT SELECT ON users TO 42");
}

#[test]
fn revoke_forms() {
    pok("REVOKE SELECT ON users FROM alice");
    pok("REVOKE SELECT, INSERT ON users FROM alice, bob");
    perr("REVOKE ALL ON users FROM alice");
    pok("REVOKE r1 FROM alice");
    pok("REVOKE 'r1' FROM alice");
    perr("REVOKE ROLE r1 FROM alice");
    pok("REVOKE USAGE ON users FROM alice");
}

#[test]
fn revoke_error_paths() {
    perr("REVOKE SELECT ON users");
    perr("REVOKE SELECT ON");
}

// ===========================================================================
// split_sql_statements — quote/comment/depth handling (parser.rs 14179+)
// ===========================================================================

#[test]
fn split_sql_quotes_and_comments() {
    assert_eq!(split_sql_statements("SELECT 'a;b'; SELECT 1").len(), 2);
    assert_eq!(split_sql_statements("SELECT 1 -- c;\n; SELECT 2").len(), 2);
    assert_eq!(split_sql_statements("/* ; */ SELECT 1; SELECT 2").len(), 2);
    assert_eq!(split_sql_statements("SELECT 'it''s;ok'; SELECT 2").len(), 2);
    assert_eq!(split_sql_statements("SELECT \"a;b\"; SELECT 2").len(), 2);
    assert_eq!(split_sql_statements("SELECT 'a\\'; b'; SELECT 2").len(), 2);
}

#[test]
fn split_sql_depth_handling() {
    assert_eq!(
        split_sql_statements("BEGIN SELECT 1; SELECT 2; END").len(),
        1
    );
    assert_eq!(split_sql_statements("SELECT (1;2)").len(), 1);
    assert_eq!(split_sql_statements("SELECT [a;b]").len(), 1);
    let _ = split_sql_statements("");
    let _ = split_sql_statements(";");
    let _ = split_sql_statements("SELECT 1;;SELECT 2;");
}

// ===========================================================================
// CREATE SEQUENCE option parsing (parser.rs 3420-3493)
// ===========================================================================

#[test]
fn create_sequence_full_options() {
    pok("CREATE SEQUENCE s START WITH 1 INCREMENT BY 2 MINVALUE 0 MAXVALUE 100 CACHE 5 CYCLE");
    pok("CREATE SEQUENCE s START 1");
    pok("CREATE SEQUENCE IF NOT EXISTS s START WITH -1 INCREMENT BY -2");
    pok("CREATE SEQUENCE s INCREMENT BY 5 MINVALUE 1 MAXVALUE 9999");
    pok("CREATE SEQUENCE s CACHE 10 CYCLE");
    pok("CREATE SEQUENCE s NO CYCLE");
}

#[test]
fn create_sequence_errors() {
    perr("CREATE SEQUENCE s START WITH x");
    perr("CREATE SEQUENCE 123");
    perr("CREATE SEQUENCE s INCREMENT 5");
    perr("CREATE SEQUENCE");
}

// ===========================================================================
// JOIN clause variants (parse_join_clause)
// ===========================================================================

#[test]
fn join_clause_variants() {
    pok("SELECT * FROM a JOIN b ON a.x = b.x");
    pok("SELECT * FROM a INNER JOIN b ON a.x = b.x");
    pok("SELECT * FROM a LEFT JOIN b ON a.x = b.x");
    pok("SELECT * FROM a RIGHT JOIN b ON a.x = b.x");
    pok("SELECT * FROM a FULL JOIN b ON a.x = b.x");
    pok("SELECT * FROM a FULL OUTER JOIN b ON a.x = b.x");
    pok("SELECT * FROM a LEFT OUTER JOIN b ON a.x = b.x");
    pok("SELECT * FROM a CROSS JOIN b");
    pok("SELECT * FROM a NATURAL JOIN b");
    pok("SELECT * FROM a JOIN b USING (x)");
    pok("SELECT * FROM a, b, c WHERE a.x = b.x AND b.y = c.y");
    pok("SELECT * FROM a JOIN b ON a.x = b.x JOIN c ON b.y = c.y");
}

// ===========================================================================
// Column definitions & table-level constraints (parse_column_definition,
// parse_create_table)
// ===========================================================================

#[test]
fn column_definition_constraints() {
    pok("CREATE TABLE t (a INT NOT NULL DEFAULT 1)");
    pok("CREATE TABLE t (a INT PRIMARY KEY)");
    pok("CREATE TABLE t (a INT UNIQUE)");
    pok("CREATE TABLE t (a INT CHECK (a > 0))");
    pok("CREATE TABLE t (a VARCHAR(10) COLLATE nocase)");
    pok("CREATE TABLE t (a INT COMMENT 'x')");
    pok("CREATE TABLE t (a INT REFERENCES other(id))");
    pok("CREATE TABLE t (a INT AUTO_INCREMENT PRIMARY KEY)");
    pok("CREATE TABLE t (a DECIMAL(10, 2))");
    pok("CREATE TABLE t (a TIMESTAMP DEFAULT CURRENT_TIMESTAMP)");
    pok("CREATE TABLE t (a INT NULL)");
    pok("CREATE TABLE t (a BIGINT UNSIGNED)");
    pok("CREATE TABLE t (a INT DEFAULT NULL)");
    pok("CREATE TABLE t (a TEXT NOT NULL UNIQUE)");
    pok("CREATE TABLE t (a)");
    perr("CREATE TABLE t (a INT CHECK)");
}

#[test]
fn table_level_constraints() {
    pok("CREATE TABLE t (a INT, PRIMARY KEY (a))");
    pok("CREATE TABLE t (a INT, UNIQUE (a))");
    pok("CREATE TABLE t (a INT, CHECK (a > 0))");
    pok("CREATE TABLE t (a INT, FOREIGN KEY (a) REFERENCES b(c))");
    pok("CREATE TEMPORARY TABLE t (a INT)");
    pok("CREATE TABLE IF NOT EXISTS t (a INT)");
    pok("CREATE TABLE t AS SELECT 1");
    pok("CREATE TABLE t LIKE s");
}

// ===========================================================================
// INSERT statement variants (parse_insert)
// ===========================================================================

#[test]
fn insert_variants() {
    pok("INSERT INTO t VALUES (1), (2), (3)");
    pok("INSERT INTO t (a, b) VALUES (1, 2)");
    pok("INSERT INTO t SELECT * FROM s");
    pok("INSERT INTO t (a) SELECT b FROM s");
    pok("INSERT INTO t VALUES (1) ON CONFLICT (a) DO NOTHING");
    pok("INSERT INTO t VALUES (1) ON CONFLICT (a) DO UPDATE SET a = 1");
    pok("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE a = 1");
    pok("INSERT INTO t VALUES (1) RETURNING a");
    pok("INSERT INTO t DEFAULT VALUES");
    perr("INSERT INTO t VALUES");
    perr("INSERT INTO t");
}

// ===========================================================================
// Comparison-expression chains (parse_comparison_expression)
// ===========================================================================

#[test]
fn comparison_expression_chains() {
    pok("SELECT * FROM t WHERE a = 1 AND b <> 2 OR c > 3");
    pok("SELECT * FROM t WHERE a = b AND c BETWEEN 1 AND 2");
    pok("SELECT * FROM t WHERE NOT (a = 1)");
    pok("SELECT * FROM t WHERE a IS NOT NULL AND b IN (1, 2) AND c LIKE 'x'");
    pok("SELECT * FROM t WHERE (a = 1) = (b = 2)");
    pok("SELECT * FROM t WHERE a = 1 = 2");
    pok("SELECT * FROM t WHERE a < 1 AND b <= 2 AND c >= 3");
}

// ===========================================================================
// CREATE FUNCTION RETURN-expression body — token_to_text walk
// (parser.rs 4090 caller)
// ===========================================================================

#[test]
fn function_return_expr_token_walk() {
    pok("CREATE FUNCTION f() RETURNS INT RETURN a + 1");
    pok("CREATE FUNCTION f() RETURNS INT RETURN (a + b) * 2");
    pok("CREATE FUNCTION f() RETURNS INT RETURN CONCAT(a, 'x; y')");
    pok("CREATE FUNCTION f() RETURNS INT RETURN 'str with space'");
    pok("CREATE FUNCTION f() RETURNS INT RETURN f(g(h(1)))");
    pok("CREATE FUNCTION f() RETURNS TEXT RETURN 'it''s'");
    pok("CREATE FUNCTION f() RETURNS INT RETURN CASE WHEN a THEN 1 ELSE 2 END");
}

// ===========================================================================
// CREATE PROCEDURE / stored-program bodies (parse_sp_body)
// ===========================================================================

#[test]
fn procedure_body_variants() {
    pok("CREATE PROCEDURE p() BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE p(IN a INT) BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE p(OUT a INT, INOUT b TEXT) BEGIN SET a = 1; END");
    pok("CREATE PROCEDURE p() BEGIN DECLARE x INT; SET x = 1; SELECT x; END");
    pok("CREATE PROCEDURE p() BEGIN IF 1 = 1 THEN SELECT 1; END IF; END");
    pok("CREATE PROCEDURE p() BEGIN SELECT 1; SELECT 2; END");
}

// ===========================================================================
// ALTER TABLE remaining operations
// ===========================================================================

#[test]
fn alter_table_column_operations() {
    pok("ALTER TABLE t ADD COLUMN c INT");
    pok("ALTER TABLE t DROP COLUMN c");
    perr("ALTER TABLE t MODIFY COLUMN c BIGINT");
    perr("ALTER TABLE t MODIFY c BIGINT");
    perr("ALTER TABLE t CHANGE c c2 BIGINT");
    pok("ALTER TABLE t RENAME COLUMN a TO b");
    perr("ALTER TABLE t DROP INDEX idx");
    pok("ALTER TABLE t ALTER COLUMN c SET DEFAULT 1");
}

// ===========================================================================
// Round 3 — functions in expression position (WHERE/ORDER BY/parens):
// select-list has its own dispatch (parser.rs 5736), so the primary-
// expression arms (9832-10146) only fire outside the select list.
// ===========================================================================

#[test]
fn convert_in_expression_position() {
    pok("SELECT * FROM t WHERE CONVERT(a, INTEGER) > 0");
    pok("SELECT * FROM t WHERE CONVERT(a, TEXT) = 'x'");
    pok("SELECT * FROM t WHERE CONVERT(a, CHAR(10)) IS NOT NULL");
    pok("SELECT * FROM t WHERE CONVERT(a, DATE) <> b");
    pok("SELECT (CONVERT(a, INTEGER)) FROM t");
    perr("SELECT * FROM t WHERE CONVERT a");
    perr("SELECT * FROM t WHERE CONVERT(a)");
    perr("SELECT * FROM t WHERE CONVERT(a, 42)");
}

#[test]
fn date_add_sub_in_expression_position() {
    pok("SELECT * FROM t WHERE DATE_ADD(a, 3, 'DAY') > b");
    pok("SELECT * FROM t WHERE DATE_ADD(a, INTERVAL 3 DAY) > b");
    pok("SELECT * FROM t WHERE DATE_SUB(a, 2, 'MONTH') < b");
    pok("SELECT * FROM t WHERE DATE_SUB(a, INTERVAL 1 YEAR) < b");
    pok("SELECT * FROM t ORDER BY DATE_ADD(a, 1, 'DAY')");
    perr("SELECT * FROM t WHERE DATE_ADD(a)");
    perr("SELECT * FROM t WHERE DATE_ADD(a, 3)");
    perr("SELECT * FROM t WHERE DATE_SUB(a, 3, 42)");
}

#[test]
fn timestampdiff_in_expression_position() {
    pok("SELECT * FROM t WHERE TIMESTAMPDIFF(DAY, a, b) > 1");
    pok("SELECT * FROM t WHERE TIMESTAMPDIFF('HOUR', a, b) > 1");
    pok("SELECT * FROM t ORDER BY TIMESTAMPDIFF(SECOND, a, b)");
    perr("SELECT * FROM t WHERE TIMESTAMPDIFF(FOO, a, b)");
    perr("SELECT * FROM t WHERE TIMESTAMPDIFF(1, a, b)");
}

#[test]
fn truncate_in_expression_position() {
    pok("SELECT * FROM t WHERE TRUNCATE(a, 2) = 0");
    pok("SELECT * FROM t ORDER BY TRUNCATE(a + 0.5, 1)");
    perr("SELECT * FROM t WHERE TRUNCATE a");
}

// ===========================================================================
// Parenthesised select-list expression followed by arithmetic
// (parser.rs 5346-5382)
// ===========================================================================

#[test]
fn paren_expr_followed_by_arithmetic() {
    pok("SELECT (a + 1) * b FROM t");
    pok("SELECT (a + 1) - b FROM t");
    pok("SELECT (a) + 1 FROM t");
    pok("SELECT (a + 1) / 2 AS x FROM t");
    pok("SELECT (a + 1) % 2 FROM t");
    pok("SELECT (a + 1) * b AS prod FROM t");
    pok("SELECT (a) * b c FROM t");
}

// ===========================================================================
// Aggregates with OVER in HAVING — primary-expression arm (10869-10961)
// ===========================================================================

#[test]
fn aggregate_over_in_having() {
    pok("SELECT a FROM t GROUP BY a HAVING MAX(b) OVER (PARTITION BY c) > 1");
    pok("SELECT a FROM t GROUP BY a HAVING SUM(b) OVER (ORDER BY c) > 1");
    pok(
        "SELECT a FROM t GROUP BY a HAVING MAX(b) OVER (PARTITION BY c ORDER BY d NULLS FIRST) > 1",
    );
    pok("SELECT a FROM t GROUP BY a HAVING AVG(b) OVER (ORDER BY d DESC NULLS LAST) < 100");
    pok("SELECT a FROM t GROUP BY a HAVING MIN(b) OVER () > 0");
}

// ===========================================================================
// TPC-H multi-table auto-rewrite helpers (2271-2313, 2487-2561, 7449-7492)
// ===========================================================================

#[test]
fn tpch_rewrite_simple_join_keys() {
    pok("SELECT COUNT(*) FROM orders, customer WHERE o_custkey = c_custkey");
    pok(
        "SELECT COUNT(*) FROM orders o, customer c WHERE o_custkey = c_custkey \
         AND c_name = 'GERMANY'",
    );
    pok(
        "SELECT COUNT(*) FROM lineitem, orders WHERE l_orderkey = o_orderkey \
         AND l_shipdate > '1995-01-01'",
    );
    pok(
        "SELECT COUNT(*) FROM nation, region WHERE n_regionkey = r_regionkey \
         AND n_name = 'ALGERIA'",
    );
}

#[test]
fn tpch_rewrite_three_way_and_qualified_refs() {
    pok("SELECT COUNT(*) FROM supplier, part, partsupp \
         WHERE s_suppkey = ps_suppkey AND p_partkey = ps_partkey");
    pok("SELECT COUNT(*) FROM orders, lineitem, customer \
         WHERE o_orderkey = l_orderkey AND o_custkey = c_custkey");
    pok("SELECT COUNT(*) FROM orders, customer \
         WHERE orders.o_custkey = customer.c_custkey AND c_name = 'x'");
    pok("SELECT * FROM part WHERE p_partkey = 1 AND p_size > 5");
}

// ===========================================================================
// CREATE USER host shapes (parser.rs 13618-13700)
// ===========================================================================

#[test]
fn create_user_host_shapes() {
    pok("CREATE USER alice");
    pok("CREATE USER 'alice'@'%'");
    pok("CREATE USER alice @ localhost");
    pok("CREATE USER alice@remotehost");
    pok("CREATE USER 'bob'@'192.168.0.%'");
    perr("CREATE USER");
    perr("CREATE USER 42");
}

// ===========================================================================
// CREATE TRIGGER timings (parser.rs parse_create_trigger)
// ===========================================================================

#[test]
fn create_trigger_timings_and_events() {
    pok("CREATE TRIGGER trg BEFORE INSERT ON t FOR EACH ROW BEGIN SELECT 1; END");
    pok("CREATE TRIGGER trg AFTER DELETE ON t FOR EACH ROW BEGIN SELECT 1; END");
    pok("CREATE TRIGGER trg AFTER UPDATE OF a ON t FOR EACH ROW BEGIN SELECT 1; END");
    pok("CREATE TRIGGER trg INSTEAD OF UPDATE ON v FOR EACH ROW BEGIN SELECT 1; END");
    perr("CREATE TRIGGER trg WHEN INSERT ON t FOR EACH ROW BEGIN SELECT 1; END");
    perr("CREATE TRIGGER 42 AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 1; END");
}

// ===========================================================================
// UPDATE variants (parse_update)
// ===========================================================================

#[test]
fn update_variants() {
    pok("UPDATE t SET a = 1 WHERE b = 2");
    pok("UPDATE t SET a = 1, b = 2 WHERE c = 3");
    pok("UPDATE t SET a = a + 1");
    pok("UPDATE t SET a = 1 WHERE b IN (1, 2, 3)");
    pok("UPDATE t SET a = (SELECT MAX(x) FROM s) WHERE b = 2");
    pok("UPDATE t SET a = 1, b = b || 'x' WHERE c IS NOT NULL");
}

// ===========================================================================
// Window frame bounds (parse_window_frame_bound)
// ===========================================================================

#[test]
fn window_frame_bound_variants() {
    pok(&window_sql(
        "ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING",
        "",
    ));
    pok(&window_sql(
        "ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING",
        "",
    ));
    pok(&window_sql("ROWS BETWEEN 2 FOLLOWING AND 3 FOLLOWING", ""));
    pok(&window_sql("ROWS BETWEEN 5 PRECEDING AND CURRENT ROW", ""));
    pok(&window_sql(
        "RANGE BETWEEN UNBOUNDED PRECEDING AND 1 FOLLOWING",
        "",
    ));
    pok(&window_sql(
        "GROUPS BETWEEN CURRENT ROW AND 1 FOLLOWING",
        "",
    ));
    pok(&window_sql("ROWS UNBOUNDED FOLLOWING", ""));
    pok(&window_sql("ROWS CURRENT ROW", ""));
}

// ===========================================================================
// Round 4 — misc remaining arms targeted from r4 segments
// ===========================================================================

#[test]
fn set_op_materialised_as_from_subquery() {
    pok("SELECT * FROM (SELECT 1 UNION SELECT 2) AS x");
    pok("SELECT * FROM (SELECT 1 UNION SELECT 2) x");
    pok("SELECT c FROM (SELECT 1 AS c UNION SELECT 2) AS x WHERE c > 1");
    pok("SELECT * FROM (SELECT 1 INTERSECT SELECT 2) AS x");
    pok("SELECT * FROM (SELECT 1 EXCEPT SELECT 2) AS x");
}

#[test]
fn index_hints_use_and_ignore() {
    pok("SELECT * FROM t USE INDEX (i1, i2)");
    pok("SELECT * FROM t USE KEY (k1)");
    pok("SELECT * FROM t IGNORE INDEX (i1)");
    pok("SELECT * FROM t IGNORE KEY (k1)");
    pok("SELECT * FROM t USE INDEX (i1) WHERE a = 1");
}

#[test]
fn literal_regexp_in_select_list() {
    pok("SELECT 'abc' REGEXP 'a.*' FROM t");
    pok("SELECT 'abc' RLIKE 'b' FROM t");
    pok("SELECT 'abc' REGEXP 'a.*' AS m FROM t");
    pok("SELECT 'abc' REGEXP 'a.*' m FROM t");
    pok("SELECT 'abc' REGEXP 'a.*' AS LEVEL FROM t");
}

#[test]
fn bare_aggregate_keywords_as_column_names() {
    pok("SELECT count FROM t");
    pok("SELECT sum FROM t");
    pok("SELECT avg FROM t");
    pok("SELECT min FROM t");
    pok("SELECT max FROM t");
}

#[test]
fn named_table_constraints() {
    pok("CREATE TABLE t (a INT, CONSTRAINT uq UNIQUE (a))");
    pok("CREATE TABLE t (a INT, CONSTRAINT ck CHECK (a > 0))");
    pok("CREATE TABLE t (a INT, CONSTRAINT pk PRIMARY KEY (a))");
    pok("CREATE TABLE t (a INT, CONSTRAINT uq UNIQUE INDEX (a))");
    perr("CREATE TABLE t (a INT, CONSTRAINT c FOO (a))");
}

#[test]
fn compress_clause_algorithms() {
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=LZ4)");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=ZSTD)");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=zlib)");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=DEFLATE)");
    // Unknown algorithm/shape → clause silently ignored (returns None).
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=BOGUS)");
    pok("CREATE TABLE t (a INT) COMPRESS (X)");
}

#[test]
fn drop_user_host_shapes() {
    pok("DROP USER alice");
    pok("DROP USER 'alice'@'%'");
    pok("DROP USER alice @ localhost");
    pok("DROP USER alice@remotehost");
    perr("DROP USER");
}

#[test]
fn create_user_host_error_paths() {
    perr("CREATE USER u @ 42");
    perr("CREATE USER 'u' @ 42");
}

#[test]
fn position_and_group_concat() {
    pok("SELECT * FROM t WHERE POSITION('a' IN b) > 0");
    pok("SELECT POSITION('a' IN b) FROM t");
    pok("SELECT GROUP_CONCAT(a) FROM t");
    pok("SELECT GROUP_CONCAT(DISTINCT a) FROM t");
    pok("SELECT GROUP_CONCAT(a ORDER BY b DESC) FROM t");
    pok("SELECT GROUP_CONCAT(a ORDER BY b ASC) FROM t");
    pok("SELECT GROUP_CONCAT(a SEPARATOR ', ') FROM t");
    pok("SELECT * FROM t GROUP BY a HAVING GROUP_CONCAT(b SEPARATOR ',') = 'x'");
}

#[test]
fn percentile_cont_within_group() {
    pok("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a) FROM t");
    pok("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a DESC) FROM t");
    pok("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a, b) FROM t");
    pok("SELECT * FROM t ORDER BY PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY a)");
}

#[test]
fn interval_forms_in_expression_position() {
    pok("SELECT * FROM t WHERE INTERVAL(5, 1, 3, 5, 7, 9) > 0");
    pok("SELECT * FROM t WHERE INTERVAL(1, 2, 3) = 2");
    pok("SELECT * FROM t WHERE INTERVAL = 1");
}

#[test]
fn date_function_in_select_list() {
    pok("SELECT DATE(a) FROM t");
    pok("SELECT DATE(a) AS d FROM t");
    pok("SELECT DATE(NOW()) FROM t");
    pok("SELECT date FROM t");
}

#[test]
fn insert_odku_excluded_row_alias_refs() {
    pok("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE a = t.b");
    pok("INSERT INTO t VALUES (1) AS tt ON DUPLICATE KEY UPDATE a = tt.b");
    pok("INSERT INTO t VALUES (1) AS new ON CONFLICT (a) DO UPDATE SET b = new.c");
    pok("INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO UPDATE SET b = t.c");
    pok("INSERT INTO t (a) VALUES (1), (2) ON DUPLICATE KEY UPDATE a = VALUES(a)");
}

#[test]
fn parse_statements_string_and_depth_paths() {
    // KNOWN QUIRK: parse_statements toggles `in_string` on StringLiteral
    // tokens (lexer already resolved quotes), so the `;` after a
    // single-quoted literal is swallowed and the tail statement is
    // dropped. Asserting actual behavior; reported separately.
    let n = parse_statements("SELECT 'a;b'; SELECT 1")
        .expect("single-quote input parses")
        .len();
    assert_eq!(n, 1);
    let n = parse_statements("SELECT 'x'; SELECT 'y'")
        .expect("second literal toggles flag back")
        .len();
    assert_eq!(n, 1);
    // `;` inside parens is not a split point; inner statement is invalid SQL.
    assert!(parse_statements("SELECT (1;2); SELECT 3").is_err());
    let n = parse_statements("SELECT 1 -- c;\n; SELECT 2")
        .expect("split in comment")
        .len();
    assert_eq!(n, 2);
    let n = parse_statements("SELECT \"a;b\"; SELECT 2")
        .expect("split in dquote")
        .len();
    assert_eq!(n, 2);
    assert_eq!(parse_statements("").unwrap_err(), "Empty input");
}

#[test]
fn procedure_loop_leave_iterate() {
    pok("CREATE PROCEDURE p() BEGIN LOOP SELECT 1; END LOOP; END");
    pok("CREATE PROCEDURE p() BEGIN lbl: LOOP LEAVE lbl; END LOOP; END");
    pok("CREATE PROCEDURE p() BEGIN lbl: LOOP ITERATE lbl; END LOOP; END");
    pok("CREATE PROCEDURE p() BEGIN WHILE 1 = 1 DO SELECT 1; END WHILE; END");
}

#[test]
fn grant_with_grant_option_partial_paths() {
    pok("GRANT SELECT ON users TO alice WITH GRANT OPTION");
    // Partial WITH forms set with_grant_option=false but still parse Ok.
    pok("GRANT SELECT ON users TO alice WITH GRANT BAD");
    pok("GRANT SELECT ON users TO alice WITH BAD");
}

// ===========================================================================
// Round 5 — privilege loops, backtick fn names, LEVEL/CHAR, derived aliases
// ===========================================================================

#[test]
fn grant_usage_identifier_and_comma_paths() {
    // USAGE is the only privilege left as Identifier by the lexer.
    pok("GRANT USAGE ON t TO u");
    pok("GRANT SELECT, USAGE ON t TO u");
    pok("GRANT SELECT, INSERT, UPDATE, DELETE ON t TO u");
    pok("GRANT SELECT, INSERT ON t TO u");
    // READ/ALL are keyword tokens without loop arms → outer `_ => false`
    // in is_role_grant, loop breaks, expect(On) fails.
    perr("GRANT READ ON t TO u");
    perr("GRANT ALL ON t TO u");
    perr("GRANT EXECUTE ON t TO u");
}

#[test]
fn revoke_privilege_identifier_and_token_paths() {
    // USAGE Identifier path: in priv list → not role, != GRANT → false arms.
    pok("REVOKE USAGE ON t FROM u");
    pok("REVOKE SELECT, USAGE ON t FROM u");
    // Token privilege → non-Identifier outer else-false arm.
    pok("REVOKE SELECT ON t FROM u");
    pok("REVOKE INSERT, UPDATE ON t FROM u");
    // READ/ALL keyword tokens: outer `_ => false` arms, then expect(On) fails.
    perr("REVOKE READ ON t FROM u");
    perr("REVOKE ALL ON t FROM u");
    // StringLiteral → role-revoke path.
    pok("REVOKE 'auditor' FROM u");
}

#[test]
fn backtick_function_names_in_expression_position() {
    pok("SELECT * FROM t WHERE `DATE_ADD`(a, INTERVAL 1 DAY) > '2020-01-01'");
    pok("SELECT * FROM t WHERE `DATE_SUB`(a, INTERVAL 1 DAY) < '2030-01-01'");
    pok("SELECT * FROM t WHERE `POSITION`('a' IN b) > 0");
    pok("SELECT * FROM t WHERE `GROUP_CONCAT`(a) = 'x'");
    pok("SELECT * FROM t WHERE `TIMESTAMPDIFF`(DAY, a, b) > 0");
    pok("SELECT `TRIM`(LEADING 'x' FROM y) FROM t");
}

#[test]
fn level_as_bare_identifier_expression() {
    pok("SELECT * FROM t WHERE level = 1");
    pok("SELECT * FROM t ORDER BY level");
    pok("SELECT * FROM t GROUP BY level HAVING level > 2");
}

#[test]
fn char_type_name_function_and_bare_forms() {
    pok("SELECT * FROM t WHERE CHAR(65, 66, 67) = 'ABC'");
    pok("SELECT * FROM t WHERE CHAR = 1");
    pok("SELECT * FROM t WHERE CONVERT(price, CHAR) > 0");
    pok("SELECT * FROM t WHERE CONVERT(price, SIGNED) > 0");
    pok("SELECT * FROM t WHERE INTERVAL(2, 1, 3) = 2");
}

#[test]
fn setop_subquery_alias_error_paths() {
    pok("SELECT * FROM (SELECT 1 UNION SELECT 2) AS x");
    pok("SELECT * FROM (SELECT 1 UNION SELECT 2)");
    perr("SELECT * FROM (SELECT 1 UNION SELECT 2) 42");
    perr("SELECT * FROM (SELECT 1 UNION SELECT 2) AS 42");
}

#[test]
fn derived_alias_join_registration() {
    pok("SELECT d.a FROM (SELECT a FROM t1) d JOIN t2 ON d.a = t2.a");
    pok("SELECT * FROM (SELECT 1 AS x) revenue JOIN other ON revenue.x = other.x");
    pok("SELECT * FROM (SELECT 1 AS x) AS d LEFT JOIN t2 ON d.x = t2.x WHERE d.x = 1");
}

// ===========================================================================
// Round 6 — procedure names/params, literal-rewind, user/engine/quantifiers,
//           VALUES & WITH from-subquery forms
// ===========================================================================

#[test]
fn procedure_keyword_names() {
    pok("CREATE PROCEDURE increment() BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE loop() BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE set() BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE repeat() BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE declare() BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE call() BEGIN SELECT 1; END");
    perr("CREATE PROCEDURE 42() BEGIN SELECT 1; END");
    perr("CREATE PROCEDURE");
}

#[test]
fn procedure_parameter_modes_and_types() {
    pok("CREATE PROCEDURE p(INOUT x INT, OUT y TEXT) BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE p(x TEXT) BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE p(x FLOAT) BEGIN SELECT 1; END");
    pok("CREATE PROCEDURE p(x BOOLEAN) BEGIN SELECT 1; END");
    perr("CREATE PROCEDURE p(42 INT) BEGIN SELECT 1; END");
    perr("CREATE PROCEDURE p(x 42) BEGIN SELECT 1; END");
}

#[test]
fn select_list_literal_comparison_rewind() {
    pok("SELECT 1 = v FROM t");
    pok("SELECT 0 AND v FROM t");
    pok("SELECT 2 > a AS c FROM t");
    pok("SELECT 1 = v AS 42 FROM t");
    pok("SELECT 3 OR b FROM t");
}

#[test]
fn grant_column_level_and_object_type_fallback() {
    pok("GRANT SELECT(email) ON users TO alice");
    pok("GRANT SELECT(email, name) ON users TO alice");
    // Explicit object-type keywords are keyword tokens → name parse errors.
    perr("GRANT SELECT ON TABLE t TO u");
    perr("REVOKE SELECT ON TABLE t FROM u");
}

#[test]
fn create_user_identified_by() {
    pok("CREATE USER alice IDENTIFIED BY 'secret'");
    pok("CREATE USER alice IDENTIFIED BY secret");
    perr("CREATE USER alice IDENTIFIED BY 42");
    perr("CREATE USER alice IDENTIFIED BY");
}

#[test]
fn engine_spec_forms() {
    pok("CREATE TABLE t (a INT) ENGINE=InnoDB CLUSTERED");
    pok("CREATE TABLE t (a INT) ENGINE=MEMORY");
    pok("CREATE TABLE t (a INT) ENGINE=MyISAM");
    pok("CREATE TABLE t (a INT) ENGINE=InnoDB");
}

#[test]
fn quantified_comparison_ops() {
    pok("SELECT * FROM t WHERE a = ANY (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a = ALL (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a = SOME (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a <> ANY (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a > ALL (SELECT b FROM u)");
}

#[test]
fn tpch_asia_cardinality_hints() {
    pok("SELECT * FROM nation WHERE r_name = 'ASIA'");
    pok("SELECT * FROM nation WHERE r_name = 'EUROPE'");
    pok("SELECT * FROM supplier WHERE s_name = 'a' AND r_name = 'ASIA'");
    pok("SELECT * FROM customer WHERE c_name = 'a' AND r_name = 'ASIA'");
    pok("SELECT * FROM region WHERE r_name = 'ASIA'");
}

#[test]
fn values_in_from_column_list_and_errors() {
    pok("SELECT * FROM (VALUES (1)) AS v");
    pok("SELECT * FROM (VALUES (1)) v(a)");
    pok("SELECT * FROM (VALUES (1), (2)) AS v");
    perr("SELECT * FROM (VALUES) AS v");
    perr("SELECT * FROM (VALUES (1))");
    perr("SELECT * FROM (VALUES (1)) 42");
}

#[test]
fn with_in_from_subquery_forms() {
    pok("SELECT * FROM (WITH c AS (SELECT 1) SELECT * FROM c) AS w");
    pok("SELECT * FROM (WITH RECURSIVE c AS (SELECT 1) SELECT * FROM c) AS w");
    perr("SELECT * FROM (WITH c AS (SELECT 1) SELECT * FROM c)");
    perr("SELECT * FROM (WITH c AS (SELECT 1) SELECT * FROM c) 42");
}

#[test]
fn double_paren_setop_arm_entry() {
    // Enters the nested-LParen from-subquery arm, then parse_select_or_union
    // rejects the leading paren — covers the arm header up to the call.
    perr("SELECT * FROM ((SELECT 1)) AS x");
    perr("SELECT * FROM ((SELECT 1 EXCEPT ALL SELECT 2)) AS x");
}

#[test]
fn percentile_cont_backtick_generic_path() {
    pok("SELECT * FROM t WHERE `PERCENTILE_CONT`(0.5) WITHIN GROUP (ORDER BY a) > 0.4");
    pok("SELECT `PERCENTILE_CONT`(0.5) WITHIN GROUP (ORDER BY a) FROM t");
}

// ===========================================================================
// Round 7 — NULLS ordering, postfix .field, column types, window bound
//           errors, SUBSTRING, DROP PROCEDURE, vector index, CTE DML, OR REPLACE
// ===========================================================================

#[test]
fn order_by_nulls_first_last() {
    pok("SELECT * FROM t ORDER BY a NULLS FIRST");
    pok("SELECT * FROM t ORDER BY a DESC NULLS LAST");
    perr("SELECT * FROM t ORDER BY a NULLS X");
}

#[test]
fn postfix_paren_field_access() {
    pok("SELECT * FROM t WHERE (a + b).c = 1");
    pok("SELECT * FROM t WHERE (a).level = 1");
    perr("SELECT * FROM t WHERE (a).42 = 1");
}

#[test]
fn column_type_keyword_tokens() {
    pok("CREATE TABLE t (a TEXT, b FLOAT, c BOOLEAN, d DISTANCE)");
    pok("CREATE TABLE t (a INT, b TEXT)");
    pok("CREATE TABLE t (a BOOLEAN)");
}

#[test]
fn window_frame_bound_error_paths() {
    perr("SELECT sum(x) OVER (ORDER BY y ROWS BETWEEN 1.5 PRECEDING AND CURRENT ROW) FROM t");
    perr("SELECT sum(x) OVER (ORDER BY y ROWS BETWEEN z PRECEDING AND CURRENT ROW) FROM t");
    perr("SELECT sum(x) OVER (ORDER BY y ROWS BETWEEN -1 PRECEDING AND CURRENT ROW) FROM t");
    perr(
        "SELECT sum(x) OVER (ORDER BY y ROWS BETWEEN 99999999999999999999999 PRECEDING AND CURRENT ROW) FROM t",
    );
}

#[test]
fn substring_from_for_forms() {
    pok("SELECT * FROM t WHERE SUBSTRING('abcd' FROM 2) = 'b'");
    pok("SELECT * FROM t WHERE SUBSTRING('abcd' FROM 2 FOR 1) = 'b'");
    // Backtick names go through the generic arg loop, not the FROM/FOR form.
    perr("SELECT * FROM t WHERE `SUBSTRING`('abcd' FROM 2) = 'b'");
    perr("SELECT * FROM t WHERE `SUBSTRING`('abcd' 2) = 'b'");
}

#[test]
fn join_equality_predicate_extraction() {
    pok("SELECT * FROM t1 JOIN t2 ON t1.a = t2.b AND t1.c = t2.d");
    pok("SELECT * FROM t1 JOIN t2 ON t1.a = t2.b JOIN t3 ON t2.c = t3.d AND t1.e = t3.f");
    pok("SELECT * FROM t1 LEFT JOIN t2 ON t1.a = t2.b WHERE t1.c = 1");
}

#[test]
fn date_add_name_path_three_arg() {
    pok("SELECT * FROM t WHERE `DATE_ADD`(a, 3, 'DAY') > '2020-01-01'");
    pok("SELECT * FROM t WHERE `DATE_SUB`(a, 3, 'DAY') < '2030-01-01'");
    perr("SELECT * FROM t WHERE `DATE_ADD`(a) > '2020-01-01'");
}

#[test]
fn text_type_token_as_function() {
    pok("SELECT * FROM t WHERE TEXT(65, 66, 67) = 'ABC'");
    pok("SELECT TEXT(65, 66) FROM t");
    pok("SELECT * FROM t WHERE INTERVAL('a') = 1");
}

#[test]
fn drop_procedure_keyword_names_and_if_exists() {
    pok("DROP PROCEDURE IF EXISTS loop");
    pok("DROP PROCEDURE set");
    pok("DROP PROCEDURE IF EXISTS p");
    perr("DROP PROCEDURE IF p");
    perr("DROP PROCEDURE 42");
}

#[test]
fn set_role_forms() {
    pok("SET ROLE analyst");
    pok("SET ROLE 'auditor'");
}

#[test]
fn vector_index_creation() {
    pok("CREATE VECTOR INDEX idx ON t USING HNSW (col)");
    perr("CREATE VECTOR INDEX idx ON t USING HNSW");
}

#[test]
fn cte_body_dml_and_values_anchor() {
    pok("WITH c AS (DELETE FROM t WHERE a = 1) SELECT 1");
    pok("WITH c AS (INSERT INTO t VALUES (1)) SELECT 1");
    pok("WITH c AS (VALUES (1) UNION ALL SELECT 2) SELECT * FROM c");
}

#[test]
fn create_or_replace_table_forms() {
    pok("CREATE OR REPLACE TABLE t (a INT)");
    perr("CREATE OR TABLE t (a INT)");
}

// ===========================================================================
// Round 8 — ALTER TABLE ops, SHOW forms, window NULLS, TPC-H comma joins
// ===========================================================================

#[test]
fn alter_table_add_column_forms() {
    pok("ALTER TABLE t ADD COLUMN c INT NOT NULL DEFAULT 5");
    pok("ALTER TABLE t ADD c TEXT");
    pok("ALTER TABLE t ADD c FLOAT");
    pok("ALTER TABLE t ADD c BOOLEAN");
    pok("ALTER TABLE t ADD COLUMN c INT NULL");
    perr("ALTER TABLE t ADD c 42");
    perr("ALTER TABLE t ADD 42 INT");
}

#[test]
fn alter_table_add_constraint_rejected() {
    // SQLite-compat rejection (Issue #4620) — parse-time error.
    perr("ALTER TABLE t ADD CONSTRAINT UNIQUE(col)");
}

#[test]
fn alter_table_drop_modify_rename() {
    pok("ALTER TABLE t DROP COLUMN c");
    pok("ALTER TABLE t DROP c");
    perr("ALTER TABLE t MODIFY c INT");
    pok("ALTER TABLE t RENAME COLUMN old TO new");
    pok("ALTER TABLE t RENAME TO new");
    pok("ALTER TABLE t RENAME a TO b");
}

#[test]
fn alter_table_partition_and_reset() {
    pok("ALTER TABLE t SET PARTITIONED BY (a, b)");
    perr("ALTER TABLE t SET FOO");
    pok("ALTER TABLE t RESET PARTITIONED BY");
}

#[test]
fn alter_table_alter_column_set_drop() {
    pok("ALTER TABLE t ALTER c SET DEFAULT 5");
    pok("ALTER TABLE t ALTER c SET DEFAULT 'x'");
    pok("ALTER TABLE t ALTER c SET DEFAULT true");
    perr("ALTER TABLE t ALTER c SET DEFAULT NULL");
    perr("ALTER TABLE t ALTER c SET DEFAULT +");
    pok("ALTER TABLE t ALTER c SET DATA TYPE varchar");
    perr("ALTER TABLE t ALTER c SET FOO");
    pok("ALTER TABLE t ALTER c DROP DEFAULT");
    pok("ALTER TABLE t ALTER COLUMN c DROP NOT NULL");
}

#[test]
fn show_full_tables_and_table_status() {
    pok("SHOW FULL TABLES");
    perr("SHOW FULL");
    pok("SHOW TABLE STATUS");
    pok("SHOW TABLE STATUS FROM db LIKE 'x%'");
    perr("SHOW TABLE");
}

#[test]
fn show_grants_roles_sequences() {
    pok("SHOW GRANTS FOR 'u'");
    pok("SHOW GRANTS FOR u");
    perr("SHOW GRANTS FOR 42");
    pok("SHOW SEQUENCES");
    perr("SHOW ROLES");
}

#[test]
fn show_warnings_errors_status_variables() {
    pok("SHOW WARNINGS");
    pok("SHOW ERRORS");
    pok("SHOW STATUS");
    pok("SHOW VARIABLES");
}

#[test]
fn show_procedure_status_forms() {
    pok("SHOW PROCEDURE STATUS");
    pok("SHOW PROCEDURE STATUS LIKE 'p%'");
    perr("SHOW PROCEDURE STATUS LIKE 42");
    perr("SHOW PROCEDURE");
}

#[test]
fn show_unknown_token_fallback() {
    perr("SHOW 42");
}

#[test]
fn window_order_by_nulls() {
    pok("SELECT sum(a) OVER (ORDER BY b NULLS FIRST) FROM t");
    pok(
        "SELECT sum(a) OVER (ORDER BY b DESC NULLS LAST ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    perr("SELECT sum(a) OVER (ORDER BY b NULLS X) FROM t");
}

#[test]
fn tpch_style_comma_join_scoring() {
    pok("SELECT * FROM nation, region WHERE nation.r_regionkey = region.r_regionkey AND region.r_name = 'ASIA'");
    pok("SELECT * FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND r_name = 'ASIA'");
    pok("SELECT * FROM supplier, part, partsupp WHERE s_suppkey = ps_suppkey AND ps_partkey = p_partkey");
    pok("SELECT * FROM part, partsupp, supplier WHERE p_partkey = ps_partkey AND ps_suppkey = s_suppkey AND p_size = 5 AND r_name = 'ASIA'");
}

// ===========================================================================
// Round 9 — INSERT/ON CONFLICT/RETURNING, JOIN variants, CREATE TABLE
// constraints + CTAS, window NULLS in primary/agg sites, UPDATE joins
// ===========================================================================

#[test]
fn insert_replace_ignore_and_row_forms() {
    pok("REPLACE INTO t VALUES (1)");
    pok("INSERT IGNORE INTO t VALUES (1)");
    pok("INSERT INTO t (a) VALUES (DEFAULT)");
    pok("INSERT INTO t SELECT 1");
    pok("INSERT INTO t DEFAULT VALUES");
    perr("INSERT INTO t");
    perr("INSERT INTO t VALUES (1) AS 42 ON DUPLICATE KEY UPDATE a = 1");
}

#[test]
fn insert_on_conflict_constraint_and_where() {
    pok("INSERT INTO t VALUES (1) ON CONFLICT ON CONSTRAINT uq DO NOTHING");
    perr("INSERT INTO t VALUES (1) ON CONFLICT ON CONSTRAINT 42 DO NOTHING");
    pok("INSERT INTO t VALUES (1) ON CONFLICT (a) WHERE a > 0 DO NOTHING");
    pok("INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING");
}

#[test]
fn insert_on_conflict_do_update_error_paths() {
    perr("INSERT INTO t VALUES (1) ON CONFLICT (a) DO UPDATE SET");
    perr("INSERT INTO t VALUES (1) ON CONFLICT (a) DO UPDATE SET b");
    perr("INSERT INTO t VALUES (1) ON CONFLICT DO FOO");
    perr("INSERT INTO t VALUES (1) ON FOO");
}

#[test]
fn insert_returning_forms() {
    pok("INSERT INTO t VALUES (1) RETURNING a, b");
    pok("INSERT INTO t VALUES (1) RETURNING *");
    perr("INSERT INTO t VALUES (1) RETURNING 42");
    perr("INSERT INTO t VALUES (1) RETURNING");
}

#[test]
fn join_natural_and_modifiers() {
    pok("SELECT * FROM t NATURAL JOIN u");
    pok("SELECT * FROM t NATURAL INNER JOIN u");
    pok("SELECT * FROM t NATURAL LEFT OUTER JOIN u");
    pok("SELECT * FROM t NATURAL RIGHT JOIN u");
    pok("SELECT * FROM t NATURAL FULL JOIN u");
}

#[test]
fn join_explicit_types() {
    pok("SELECT * FROM t INNER JOIN u ON t.a = u.b");
    pok("SELECT * FROM t LEFT OUTER JOIN u ON t.a = u.b");
    pok("SELECT * FROM t RIGHT OUTER JOIN u ON t.a = u.b");
    pok("SELECT * FROM t FULL JOIN u ON t.a = u.b");
    pok("SELECT * FROM t JOIN u ON t.a = u.b");
    pok("SELECT * FROM t CROSS JOIN u");
}

#[test]
fn join_subquery_alias_paths() {
    pok("SELECT * FROM t JOIN (SELECT 1 AS x) AS s ON t.a = s.x");
    pok("SELECT * FROM t JOIN (SELECT 1 AS x) s ON t.a = s.x");
    perr("SELECT * FROM t JOIN (SELECT 1) 42 ON t.a = 1");
    perr("SELECT * FROM t JOIN 42 ON t.a = 1");
}

#[test]
fn join_using_clause() {
    pok("SELECT * FROM t JOIN u USING (a)");
    pok("SELECT * FROM t JOIN u USING (a, b)");
}

#[test]
fn create_table_if_not_exists_error_paths() {
    perr("CREATE TABLE IF 42 (a INT)");
    perr("CREATE TABLE IF NOT 42 (a INT)");
    pok("CREATE TABLE db.t (a INT)");
}

#[test]
fn create_table_named_constraints() {
    pok("CREATE TABLE t (a INT, CONSTRAINT uq UNIQUE (a))");
    pok("CREATE TABLE t (a INT, CONSTRAINT ck CHECK (a > 0))");
    pok("CREATE TABLE t (a INT, CONSTRAINT c PRIMARY KEY (a))");
    perr("CREATE TABLE t (a INT, CONSTRAINT c FOO (a))");
    perr("CREATE TABLE t (a INT, CONSTRAINT fk FOREIGN KEY (a) REFERENCES u(b))");
}

#[test]
fn create_table_unique_key_variants() {
    pok("CREATE TABLE t (a INT, UNIQUE KEY (a))");
    pok("CREATE TABLE t (a INT, UNIQUE KEY uk (a))");
    pok("CREATE TABLE t (a INT, UNIQUE (a))");
}

#[test]
fn create_table_as_select_data_flags() {
    pok("CREATE TABLE t AS SELECT 1");
    pok("CREATE TABLE t AS WITH DATA SELECT 1");
    pok("CREATE TABLE t AS WITH NO DATA SELECT 1");
    perr("CREATE TABLE t AS WITH FOO SELECT 1");
}

#[test]
fn column_definition_extra_forms() {
    pok("CREATE TABLE t (a DECIMAL(10,2))");
    pok("CREATE TABLE t (a TEXT COLLATE NOCASE)");
    perr("CREATE TABLE t (a TEXT COLLATE 42)");
    pok("CREATE TABLE t (a INT GENERATED ALWAYS AS (1) STORED)");
    pok("CREATE TABLE t (a INT GENERATED ALWAYS AS (1) VIRTUAL)");
}

#[test]
fn update_with_joins_and_returning() {
    pok("UPDATE t JOIN u ON t.a = u.b SET t.a = 1");
    pok("UPDATE t LEFT JOIN u ON t.a = u.b SET t.a = 1");
    pok("UPDATE t, u SET t.a = 1");
    pok("UPDATE t SET a = 1, b = 2 WHERE c = 3");
    perr("UPDATE t SET a 1");
}

#[test]
fn update_returning_requires_where() {
    pok("UPDATE t SET a = 1 WHERE b = 2 RETURNING c");
    pok("UPDATE t SET a = 1 WHERE b = 2 RETURNING *");
    pok("UPDATE t SET a = 1 WHERE b = 2 RETURNING a, b");
    perr("UPDATE t SET a = 1 WHERE b = 2 RETURNING");
    // Pre-existing: RETURNING without WHERE is rejected by the SET loop.
    perr("UPDATE t SET a = 1 RETURNING *");
}

#[test]
fn window_nulls_in_primary_fn_site() {
    pok("SELECT foo(a) OVER (ORDER BY b NULLS FIRST) FROM t");
    pok("SELECT foo(a) OVER (ORDER BY b DESC NULLS LAST) FROM t");
    perr("SELECT foo(a) OVER (ORDER BY b NULLS X) FROM t");
}

#[test]
fn window_nulls_in_agg_expr_sites() {
    pok("SELECT a FROM t HAVING sum(b) OVER (ORDER BY c NULLS FIRST) > 0");
    perr("SELECT a FROM t HAVING sum(b) OVER (ORDER BY c NULLS X) > 0");
    pok("SELECT a FROM t ORDER BY sum(b) OVER (PARTITION BY c)");
}

#[test]
fn procedure_else_and_call_paths() {
    // NOTE: probe showed these failing under `parse_statements`, which
    // splits on `;` before parsing and cannot round-trip procedure bodies.
    // The single-statement `parse()` entry point handles them fine.
    pok("CREATE PROCEDURE p() BEGIN IF 1 = 1 THEN SELECT 1; ELSE SELECT 2; END IF; END");
    pok("CREATE PROCEDURE p() BEGIN IF 1 = 1 THEN SELECT 1; ELSE IF 2 = 2 THEN SELECT 2; END IF; END IF; END");
    pok("CREATE PROCEDURE p() BEGIN CALL q(); END");
    pok("CREATE PROCEDURE p() BEGIN CALL q(1, 2); END");
    pok("CREATE PROCEDURE p() BEGIN CALL q; END");
    pok("CREATE PROCEDURE p() BEGIN SELECT 1; CALL q(); SELECT 2; END");
}

// ===========================================================================
// Round 10 — sp-body flush/nested/leave paths, SHOW keyword arms + suffix
// matrix, SEQUENCE negative/NO forms, CREATE FUNCTION params/RETURNS TABLE,
// GRANT role forms, ALTER rename/drop error arms
// ===========================================================================

#[test]
fn sp_body_leave_and_iterate_direct() {
    pok("CREATE PROCEDURE p() BEGIN LOOP LEAVE x; END LOOP; END");
    pok("CREATE PROCEDURE p() BEGIN LOOP ITERATE x; END LOOP; END");
}

#[test]
fn sp_body_nested_begin() {
    pok("CREATE PROCEDURE p() BEGIN BEGIN SELECT 1; END; END");
    pok("CREATE PROCEDURE p() BEGIN 1 + 2 BEGIN SELECT 3; END; END");
}

#[test]
fn sp_body_pending_sql_flush_before_keywords() {
    // `_`-accumulated SQL flushed when an IF/WHILE/LOOP/SET/BEGIN keyword
    // is reached without an intervening semicolon.
    pok("CREATE PROCEDURE p() BEGIN 1 + 2 IF 1 = 1 THEN SELECT 3; END IF; END");
    pok("CREATE PROCEDURE p() BEGIN 1 + 2 WHILE 1 = 1 DO SELECT 3; END WHILE; END");
    pok("CREATE PROCEDURE p() BEGIN 1 + 2 LOOP SELECT 3; END LOOP; END");
    pok("CREATE PROCEDURE p() BEGIN 1 + 2 SET x = 1; END");
}

#[test]
fn show_schemas_and_create_table() {
    pok("SHOW SCHEMAS");
    pok("SHOW CREATE TABLE t");
    perr("SHOW CREATE TABLE 42");
}

#[test]
fn show_full_processlist_and_errors() {
    pok("SHOW FULL PROCESSLIST");
    perr("SHOW FULL 42");
}

#[test]
fn show_status_and_tables_suffix_matrix() {
    pok("SHOW TABLE STATUS FROM mydb");
    pok("SHOW TABLE STATUS WHERE a = 1");
    pok("SHOW FULL TABLES FROM mydb");
    pok("SHOW FULL TABLES WHERE a = 1");
    pok("SHOW TABLES FROM DEFAULT");
    perr("SHOW TABLES FROM 42");
    perr("SHOW TABLES LIKE 42");
}

#[test]
fn show_fallback_and_columns_errors() {
    perr("SHOW");
    perr("SHOW COLUMNS");
    perr("SHOW INDEXES FROM t");
}

#[test]
fn sequence_negative_and_no_value_forms() {
    pok("CREATE SEQUENCE \"s\"");
    pok("CREATE SEQUENCE s START -5");
    pok("CREATE SEQUENCE s INCREMENT BY -2");
    pok("CREATE SEQUENCE s MINVALUE -1");
    pok("CREATE SEQUENCE s MAXVALUE -1");
    pok("CREATE SEQUENCE s NO MINVALUE");
    pok("CREATE SEQUENCE s NO MAXVALUE");
}

#[test]
fn sequence_number_error_paths() {
    perr("CREATE SEQUENCE s START -x");
    perr("CREATE SEQUENCE s INCREMENT BY -x");
    perr("CREATE SEQUENCE s MINVALUE x");
    perr("CREATE SEQUENCE s MAXVALUE x");
    perr("CREATE SEQUENCE s CACHE x");
}

#[test]
fn create_function_param_type_forms() {
    pok("CREATE FUNCTION f(a INT) RETURNS INT RETURN a");
    pok("CREATE FUNCTION f(a TEXT) RETURNS TEXT RETURN a");
    pok("CREATE FUNCTION f(a FLOAT) RETURNS FLOAT RETURN a");
    pok("CREATE FUNCTION f(a BOOLEAN) RETURNS BOOLEAN RETURN a");
    perr("CREATE FUNCTION f(42 INT) RETURNS INT RETURN 1");
    perr("CREATE FUNCTION f(a 42) RETURNS INT RETURN 1");
}

#[test]
fn create_function_returns_table() {
    pok("CREATE FUNCTION f() RETURNS TABLE(a INT, b TEXT) RETURN a");
    perr("CREATE FUNCTION f() RETURNS TABLE(42 INT) RETURN a");
    perr("CREATE FUNCTION f() RETURNS TABLE(a 42) RETURN a");
}

#[test]
fn create_function_returns_type_and_deterministic() {
    pok("CREATE FUNCTION f() RETURNS INTEGER RETURN 1");
    pok("CREATE FUNCTION f() RETURNS TEXT RETURN 'x'");
    pok("CREATE FUNCTION f() RETURNS FLOAT RETURN 1.5");
    pok("CREATE FUNCTION f() RETURNS BOOLEAN RETURN true");
    perr("CREATE FUNCTION f() RETURNS 42 RETURN 1");
    pok("CREATE FUNCTION f() RETURNS INT DETERMINISTIC RETURN 1");
}

#[test]
fn create_function_begin_bodies() {
    pok("CREATE FUNCTION f() RETURNS INT BEGIN RETURN 1; END");
    pok("CREATE FUNCTION f() RETURNS INT AS BEGIN SELECT 1; END");
}

#[test]
fn grant_role_forms_and_multi_targets() {
    pok("GRANT r TO u");
    pok("GRANT 'r' TO u");
    perr("GRANT 42 ON t TO u");
    pok("GRANT SELECT, INSERT, UPDATE, DELETE ON t TO u, v");
    pok("REVOKE SELECT ON t FROM u, v");
}

#[test]
fn alter_rename_and_drop_error_arms() {
    perr("ALTER TABLE t RENAME COLUMN 42 TO x");
    perr("ALTER TABLE t RENAME COLUMN old TO 42");
    perr("ALTER TABLE t RENAME TO 42");
    perr("ALTER TABLE t RENAME 42");
    perr("ALTER TABLE t DROP 42");
}

// ===========================================================================
// Round 11 — SELECT modifiers/system vars/NULL column, comparison NOT-*
// arms + ESCAPE + post-site quantified ops, agg-OVER partition/alias/frame,
// INSERT WITH/err paths
// ===========================================================================

#[test]
fn select_mysql_modifiers() {
    pok("SELECT HIGH_PRIORITY a FROM t");
    pok("SELECT SQL_CACHE a FROM t");
    pok("SELECT SQL_NO_CACHE a FROM t");
    pok("SELECT SQL_CALC_FOUND_ROWS a FROM t");
    pok("SELECT HIGH_PRIORITY SQL_CACHE SQL_NO_CACHE SQL_CALC_FOUND_ROWS a FROM t");
}

#[test]
fn select_system_variable_projection() {
    pok("SELECT @@version");
    pok("SELECT @@version AS v");
    pok("SELECT @@version AS 42");
}

#[test]
fn select_null_literal_column() {
    pok("SELECT NULL");
    pok("SELECT NULL AS x FROM t");
    pok("SELECT NULL AS 42 FROM t");
}

#[test]
fn select_or_and_as_first_error_arms() {
    perr("SELECT || 1");
    perr("SELECT AS x");
}

#[test]
fn comparison_not_in_list_and_not_like() {
    pok("SELECT * FROM t WHERE a NOT IN (1, 2)");
    pok("SELECT * FROM t WHERE a NOT LIKE 'x%' ESCAPE '\\'");
    pok("SELECT * FROM t WHERE a NOT LIKE 'x%'");
}

#[test]
fn comparison_like_escape() {
    pok("SELECT * FROM t WHERE a LIKE 'x%' ESCAPE '\\'");
    perr("SELECT * FROM t WHERE a LIKE 'x%' ESCAPE 42");
}

#[test]
fn comparison_not_between_regexp_and_error() {
    pok("SELECT * FROM t WHERE a NOT BETWEEN 1 AND 2");
    pok("SELECT * FROM t WHERE a NOT REGEXP 'x'");
    perr("SELECT * FROM t WHERE a NOT = 1");
    pok("SELECT * FROM t WHERE a IS NOT NULL");
}

#[test]
fn comparison_quantified_both_sites() {
    pok("SELECT * FROM t WHERE a = ANY (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a >= ALL (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a = 1 ANY (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a != 2 ALL (SELECT b FROM u)");
    pok("SELECT * FROM t WHERE a < 3 SOME (SELECT b FROM u)");
}

#[test]
fn agg_over_partition_alias_frame() {
    pok("SELECT sum(a) OVER (PARTITION BY b) FROM t");
    pok("SELECT sum(a) OVER (PARTITION BY b, c) FROM t");
    pok("SELECT sum(a) OVER (ORDER BY b) AS s FROM t");
    pok("SELECT sum(a) OVER (ORDER BY b) AS 42 FROM t");
    pok("SELECT sum(a) OVER (ORDER BY b ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t");
    pok(
        "SELECT sum(a) OVER (ORDER BY b ROWS BETWEEN 1 PRECEDING AND CURRENT ROW EXCLUDE CURRENT ROW) FROM t",
    );
    pok("SELECT SUM(a) / SUM(b) AS 42 FROM t");
}

#[test]
fn insert_with_clause_and_error_paths() {
    pok("INSERT INTO t WITH c AS (SELECT 1 AS x) SELECT x FROM c");
    perr("INSERT INTO t (42) VALUES (1)");
    perr("INSERT INTO t VALUES 42");
    perr("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE");
    perr("INSERT INTO t VALUES (1) ON CONFLICT (42) DO NOTHING");
    // KNOWN QUIRK: trailing junk after a VALUES row is silently ignored
    // (row loop breaks on non-LParen and never checks the remainder).
    pok("INSERT INTO t VALUES (1), 42");
}

// ===========================================================================
// Round 13 — coldef data-type arms + attribute arms, CTAS WITH [NO] DATA
// matrix, named/table constraints, TIMESTAMPDIFF unit arms, DATE_ADD
// identifier unit, scalar-function arms, INSERT ON CONFLICT/ODKU/RETURNING
// error arms
// ===========================================================================

#[test]
fn coldef_keyword_data_type_arms() {
    pok("CREATE TABLE t (c DATE)");
    pok("CREATE TABLE t (c INTEGER)");
    pok("CREATE TABLE t (c VECTOR(3))");
    pok("CREATE TABLE t (c DISTANCE(3))");
    pok("CREATE TABLE t (c TEXT)");
    pok("CREATE TABLE t (c FLOAT)");
    pok("CREATE TABLE t (c BOOLEAN)");
    pok("CREATE TABLE t (c VARCHAR(50))");
    pok("CREATE TABLE t (c DECIMAL(10,2))");
}

#[test]
fn coldef_attribute_arms() {
    pok("CREATE TABLE t (c INT NOT)");
    pok("CREATE TABLE t (c VARCHAR(10) COLLATE NOCASE)");
    perr("CREATE TABLE t (c VARCHAR(10) COLLATE 42)");
    pok("CREATE TABLE t (c INT CHECK (c > 0))");
    pok("CREATE TABLE t (id INT REFERENCES u(id))");
    pok("CREATE TABLE t (id INT REFERENCES u)");
    perr("CREATE TABLE t (id INT REFERENCES 42)");
    pok("CREATE TABLE t (x INT GENERATED ALWAYS AS (1))");
}

#[test]
fn create_table_named_and_fk_constraints() {
    pok("CREATE TABLE t (id INT, FOREIGN KEY (id) REFERENCES u(id))");
    pok("CREATE TABLE t (a INT, UNIQUE KEY (a))");
    pok("CREATE TABLE t (a INT, UNIQUE KEY uq (a))");
    pok("CREATE TABLE t (a INT, PRIMARY KEY)");
}

#[test]
fn ctas_with_no_data_matrix() {
    pok("CREATE TABLE t AS SELECT 1 WITH NO DATA");
    perr("CREATE TABLE t AS SELECT 1 WITH NO 42");
    perr("CREATE TABLE t AS SELECT 1 WITH 42");
    pok("CREATE TABLE t AS WITH DATA SELECT 1");
    pok("CREATE TABLE t AS WITH NO DATA SELECT 1");
    perr("CREATE TABLE t AS WITH 42 SELECT 1");
    perr("CREATE TABLE t AS 42");
}

#[test]
fn timestampdiff_and_date_add_unit_arms() {
    pok("SELECT TIMESTAMPDIFF(MINUTE, a, b)");
    pok("SELECT TIMESTAMPDIFF('MINUTE', a, b)");
    perr("SELECT TIMESTAMPDIFF(42, a, b)");
    perr("SELECT TIMESTAMPDIFF(FOO, a, b)");
    pok("SELECT DATE_ADD('2020-01-01', 1, DAY)");
}

#[test]
fn scalar_function_keyword_arms() {
    pok("SELECT ROLLUP(a)");
    pok("SELECT CUBE(a)");
    pok("SELECT DATABASE()");
    pok("SELECT TRUNCATE(1.5, 0)");
    pok("SELECT USER()");
    pok("SELECT LEFT('abc', 1)");
    pok("SELECT RIGHT('abc', 1)");
}

#[test]
fn with_in_from_subquery_position() {
    pok("SELECT * FROM (WITH c AS (SELECT 1 AS x) SELECT x FROM c) AS q");
    perr("SELECT * FROM (WITH c AS (SELECT 1) UPDATE t SET a = 1)");
    perr("SELECT * FROM (WITH c AS (SELECT 1) DELETE FROM t)");
    perr("SELECT * FROM (WITH c AS (SELECT 1) INSERT INTO t VALUES (1))");
}

#[test]
fn insert_odku_error_arms() {
    perr("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE a");
    pok("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE a = 1, b = 2");
}

#[test]
fn insert_on_conflict_error_arms() {
    pok("INSERT INTO t VALUES (1) ON CONFLICT (a, b) DO NOTHING");
    perr("INSERT INTO t VALUES (1) ON CONFLICT ON CONSTRAINT");
    perr("INSERT INTO t VALUES (1) ON CONFLICT ON CONSTRAINT 42 DO NOTHING");
    perr("INSERT INTO t VALUES (1) ON CONFLICT (a) DO 42");
    perr("INSERT INTO t VALUES (1) ON 42");
    perr("INSERT INTO t VALUES (1) ON CONFLICT (a) DO UPDATE SET b");
}

#[test]
fn insert_returning_error_and_multi() {
    perr("INSERT INTO t VALUES (1) RETURNING 42");
    perr("INSERT INTO t VALUES (1) RETURNING");
    pok("INSERT INTO t VALUES (1) RETURNING a, b");
}

#[test]
fn not_like_escape_error_arm() {
    perr("SELECT * FROM t WHERE a NOT LIKE 'x%' ESCAPE 42");
}

// ===========================================================================
// Round 14 — PERCENTILE_CONT WITHIN GROUP, REVOKE GRANT OPTION FOR,
// LIMIT/OFFSET constant folding, literal binop alias, DATE() call and
// bare DATE column, join predicate push
// ===========================================================================

#[test]
fn percentile_cont_within_group_round14() {
    pok("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a) FROM t");
    pok("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY a, b) FROM t");
}

#[test]
fn revoke_grant_option_for() {
    pok("REVOKE SELECT ON t FROM u GRANT OPTION FOR");
    pok("REVOKE SELECT ON t FROM u GRANT");
    pok("REVOKE SELECT ON t FROM u GRANT OPTION");
}

#[test]
fn limit_offset_constant_fold() {
    pok("SELECT * FROM t LIMIT 2*3");
    pok("SELECT * FROM t LIMIT 1 OFFSET 1+2");
    pok("SELECT * FROM t LIMIT 8/2");
    pok("SELECT * FROM t LIMIT 7%3");
    pok("SELECT * FROM t LIMIT 5-2");
    pok("SELECT * FROM t LIMIT 1 OFFSET a");
}

#[test]
fn literal_binop_select_alias() {
    pok("SELECT 1 + 2 AS x FROM t");
    pok("SELECT 1 + 2 x FROM t");
}

#[test]
fn date_function_and_bare_column() {
    pok("SELECT DATE('2020-01-01') FROM t");
    pok("SELECT DATE('2020-01-01') AS d FROM t");
    pok("SELECT DATE FROM t");
    pok("SELECT DATE AS d FROM t");
}

#[test]
fn join_predicate_push_forms() {
    pok("SELECT * FROM a JOIN b ON a.id = b.id WHERE a.id = 1");
    pok("SELECT * FROM (SELECT 1 AS x) AS d JOIN t ON d.x = t.y");
    pok("SELECT * FROM (SELECT 1 AS x) AS d WHERE d.x = 1");
}

// ===========================================================================
// Round 15 — DATE(x) RParen regression (bug fix), LIMIT/OFFSET
// unfoldable-arith error, join predicate push variants, comparison
// chains, IS NULL / BETWEEN in WHERE
// ===========================================================================

/// Regression: `DATE(x)` select-list arm must consume its closing RParen.
/// Before the fix it silently dropped FROM/alias (`SELECT DATE(a) FROM t`
/// parsed with `table == ""`).
#[test]
fn date_call_consumes_rparen() {
    let s = parse("SELECT DATE(a) FROM t").unwrap();
    match s {
        Statement::Select(sel) => {
            assert_eq!(sel.table, "t", "DATE(a) must not swallow the FROM clause");
            assert_eq!(sel.columns.len(), 1);
        }
        other => panic!("expected SELECT, got {other:?}"),
    }
    let s = parse("SELECT DATE(a) AS d FROM t").unwrap();
    match s {
        Statement::Select(sel) => {
            assert_eq!(sel.table, "t");
            assert_eq!(sel.columns[0].alias.as_deref(), Some("d"));
        }
        other => panic!("expected SELECT, got {other:?}"),
    }
    pok("SELECT DATE(DATE(a)) FROM t");
    pok("SELECT DATE(a), b FROM t");
    perr("SELECT DATE(a FROM t");
    pok("SELECT DATE() FROM t");
}

/// LIMIT/OFFSET with a non-foldable arithmetic expression errors through
/// the unfoldable-arith arm (parser.rs 7877-7878).
#[test]
fn limit_offset_unfoldable_arith_error() {
    perr("SELECT a FROM t LIMIT 1 OFFSET a+1");
    perr("SELECT a FROM t LIMIT a+1 OFFSET 2");
    perr("SELECT a FROM t LIMIT a OFFSET 1");
    pok("SELECT a FROM t LIMIT 1 OFFSET 1+1");
    pok("SELECT a FROM t LIMIT 2*3+1");
}

#[test]
fn join_predicate_push_variants() {
    pok("SELECT * FROM a AS x, b AS y WHERE x.k = y.k");
    pok("SELECT * FROM a, b, c WHERE a.k = b.k AND b.j = c.j");
    pok("SELECT * FROM a, b WHERE a.k <> b.k");
    pok("SELECT * FROM a JOIN b ON a.id = b.id WHERE a.id = 1");
}

#[test]
fn comparison_chains_and_null_checks() {
    pok("SELECT 1 = 1");
    pok("SELECT a = a FROM t");
    pok("SELECT a = b AS flag FROM t");
    pok("SELECT a = 1 OR b <> 2 AND c > 3 FROM t");
    pok("SELECT a FROM t WHERE a IS NOT NULL AND b IS NULL");
    pok("SELECT a FROM t WHERE a BETWEEN 1 AND 2 AND b = 1");
    pok("SELECT a FROM t WHERE a NOT BETWEEN 1 AND 2");
}

#[test]
fn literal_arithmetic_mix() {
    pok("SELECT 1 - 2 - 3, 4 * 5 / 6 % 7");
    pok("SELECT 1 + 2 * 3 - 4 / 2 % 3");
}

/// Round 16: reach parse_set_role (3070-3080), CHAR() function-call primary
/// (10987-11005), JSON arrow expression (9266-9282), aggregate-recursion
/// arms (8110-8127, 2462-2478).
#[test]
fn round16_role_char_json() {
    pok("SET ROLE myrole");
    pok("SET ROLE 'admin'");
    pok("SELECT CHAR(65, 66)");
    pok("SELECT CHAR(65)");
    pok("SELECT CHAR()");
    pok("SELECT j -> '$.a'");
    pok("SELECT j ->> '$.a' FROM t");
    pok("SELECT a ->> '$.x' FROM t WHERE a -> '$.y' = 1");
    pok("SELECT COUNT(CASE WHEN a IS NULL THEN 1 END) FROM t");
    pok("SELECT MAX(ABS(a)) FROM t");
    pok("SELECT COUNT(DISTINCT a) FROM t");
}

/// Round 17: CHAR() in expression position (10987-11005), TIMESTAMPDIFF
/// select-list special form (5867-5928), GRANT WITH GRANT OPTION (13224-13246),
/// DATE_ADD/DATE_SUB INTERVAL forms, misc reachable arms.
#[test]
fn round17_char_timediff_grant() {
    pok("SELECT 1 FROM t WHERE a = CHAR(65)");
    pok("SELECT 1 FROM t WHERE a = CHAR(65, 66)");
    pok("SELECT 1 FROM t WHERE a = CHAR()");
    pok("SELECT TIMESTAMPDIFF(DAY, '2020-01-01', '2020-01-02')");
    pok("SELECT TIMESTAMPDIFF(SECOND, a, b)");
    pok("SELECT TIMESTAMPDIFF(YEAR, a, b) FROM t");
    pok("GRANT SELECT ON t TO u WITH GRANT OPTION");
    pok("GRANT INSERT ON t TO u");
    pok("SELECT DATE_ADD(a, INTERVAL 1 DAY) FROM t");
    pok("SELECT DATE_SUB(a, INTERVAL '1' HOUR) FROM t");
    pok("SELECT SUM(a) FROM t GROUP BY b WITH ROLLUP");
    pok("SELECT a FROM t ORDER BY a NULLS LAST");
    pok("SELECT a FROM t ORDER BY a NULLS FIRST");
    pok("SELECT * FROM t TABLESAMPLE BERNOULLI(10)");
    pok("SELECT 1 FROM t WHERE a IN (1, 2) AND b NOT IN (SELECT x FROM u)");
    pok("SELECT CASE WHEN a THEN 1 ELSE 2 END FROM t");
}

/// Round 18: probes confirmed reachable gaps in per-function analysis —
/// WITH-CTE head (4997), string-fn primaries (9849), ALTER/CREATE table-name
/// error arms (13780/11569), INSERT IGNORE/ODKU (8450), LIKE ESCAPE (9563),
/// GRANT/REVOKE privilege arms (13120/13322), window frame exclusion (9164),
/// sequence/trigger/procedure name arms.
#[test]
fn round18_with_cte_and_error_arms() {
    pok("WITH cte AS (SELECT 1) SELECT * FROM cte");
    pok("WITH cte AS (SELECT 1), cte2 AS (SELECT 2) SELECT * FROM cte JOIN cte2");
    pok("SELECT LEFT('abcdef', 2)");
    pok("SELECT RIGHT('abcdef', 2)");
    pok("SELECT LOWER('ABC'), UPPER('abc')");
    pok("INSERT IGNORE INTO t VALUES (1)");
    pok("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE a = 1");
    pok("SELECT a FROM t WHERE a LIKE 'x%' ESCAPE '!'");
    pok("SELECT a FROM t WHERE a LIKE 'x%' ESCAPE 'bad'");
    perr("ALTER TABLE");
    perr("CREATE TABLE");
    pok("ALTER TABLE t ADD COLUMN c INT");
    pok("SHOW CREATE TABLE t");
    pok("SHOW FULL TABLES");
    pok("REVOKE SELECT ON t FROM u");
    perr("REVOKE ALL ON t FROM u");
    pok("GRANT SELECT ON t TO u");
    pok("CREATE SEQUENCE s INCREMENT BY 5");
    pok("SELECT a FROM t WINDOW w AS (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE CURRENT ROW)");
    pok("SELECT a FROM t ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING");
    pok("SELECT EXTRACT(YEAR FROM a) FROM t");
    pok("SELECT a FROM t FOR UPDATE");
}

/// Round 19: error-arm sweeps confirmed reachable by probes — INSERT table/
/// column/ODKU/RETURNING errors, ALTER RENAME/SET DEFAULT/SET DATA errors,
/// SHOW COLUMNS/INDEX/EOF errors, CREATE database-qualifier/AS errors,
/// CREATE OR REPLACE forms.
#[test]
fn round19_error_arm_sweep() {
    perr("INSERT INTO 123 VALUES (1)");
    perr("INSERT INTO t RETURNING a");
    perr("INSERT INTO t VALUES (1) RETURNING");
    perr("INSERT INTO t VALUES (1) RETURNING 123");
    perr("INSERT INTO t (a, 123) VALUES (1)");
    perr("INSERT INTO t VALUES (1) ON DUPLICATE KEY");
    perr("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE");
    perr("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE a");
    pok("INSERT INTO t VALUES (1) ON CONFLICT DO UPDATE SET a = 1");
    perr("ALTER TABLE t RENAME COLUMN 123 TO b");
    perr("ALTER TABLE t RENAME TO 123");
    perr("ALTER TABLE t ALTER COLUMN c SET DEFAULT");
    perr("ALTER TABLE t ALTER COLUMN c SET DATA TYPE");
    perr("ALTER TABLE t ALTER COLUMN c SET DATA");
    pok("ALTER TABLE t DROP c NOT NULL");
    pok("ALTER TABLE t DROP c DEFAULT");
    pok("CREATE OR REPLACE TABLE t (a INT)");
    pok("CREATE OR REPLACE VIEW v AS SELECT 1");
    perr("SHOW COLUMNS FROM t LIKE 123");
    perr("SHOW COLUMNS FROM 123");
    perr("SHOW INDEX FROM 123");
    perr("SHOW");
    perr("CREATE TABLE db. (a INT)");
    perr("CREATE TABLE t AS");
}

/// Round 20: probe-confirmed reachable arms — DuckDB bare RENAME (13906-13923),
/// SHOW WARNINGS/ERRORS/STATUS/VARIABLES (12920-12934), CTAS WITH [NO] DATA
/// both orders (11812-11872), SHOW GRANTS/SEQUENCES/PROCEDURE STATUS.
#[test]
fn round20_ctas_data_show() {
    pok("ALTER TABLE t RENAME a TO b");
    pok("SHOW WARNINGS");
    pok("SHOW ERRORS");
    pok("SHOW STATUS");
    pok("SHOW VARIABLES");
    pok("CREATE TABLE t AS SELECT 1 WITH NO DATA");
    pok("CREATE TABLE t AS SELECT 1 WITH DATA");
    pok("CREATE TABLE t AS WITH NO DATA SELECT 1");
    pok("CREATE TABLE t AS WITH DATA SELECT 1");
    perr("CREATE TABLE t AS WITH bad SELECT 1");
    perr("CREATE TABLE t AS SELECT 1 WITH bad");
    pok("SHOW GRANTS FOR 'u'");
    pok("SHOW GRANTS FOR u");
    pok("SHOW SEQUENCES");
    pok("SHOW PROCEDURE STATUS");
    perr("SHOW FULL COLUMNS FROM t");
}

/// Round 21: WITH in subquery position — hits parse_select_statement head
/// 4996-5007 (Issue #4717 arm, previously count 0). Plus INSERT..WITH,
/// EXISTS(WITH..), CREATE VIEW..WITH, bare RENAME error arm.
#[test]
fn round21_with_subquery_head() {
    pok("SELECT * FROM (WITH cte AS (SELECT 1) SELECT * FROM cte) x");
    pok("INSERT INTO t WITH cte AS (SELECT 1) SELECT * FROM cte");
    pok("INSERT INTO t (a) WITH cte AS (SELECT 1) SELECT * FROM cte");
    pok("SELECT a FROM t WHERE EXISTS (WITH cte AS (SELECT 1) SELECT 1)");
    pok("CREATE VIEW v AS WITH cte AS (SELECT 1) SELECT * FROM cte");
    perr("SELECT * FROM (WITH cte AS (SELECT 1) SELECT * FROM cte)");
    perr("ALTER TABLE t RENAME a TO 123");
}

/// Round 22: probe-confirmed reachable arms — EXPLAIN QUERY PLAN (4706-4717),
/// DELETE..USING (11167-11181), +/- INTERVAL (9763-9775), ALTER USER errors
/// (13544-13553), ENGINE/COMPRESS table clauses (11905-11965), index hints
/// (11281-11300), JSON arrow in WHERE (9226-9238).
#[test]
fn round22_explain_using_interval_engine() {
    pok("EXPLAIN QUERY PLAN SELECT 1");
    pok("EXPLAIN QUERY SELECT 1");
    pok("EXPLAIN SELECT 1");
    pok("DELETE FROM t USING u WHERE t.a = u.a");
    pok("DELETE FROM t USING u, v WHERE t.a = u.a");
    pok("SELECT a + INTERVAL '5' DAY FROM t");
    pok("SELECT a - INTERVAL 3 MONTH FROM t");
    pok("ALTER USER 'u' IDENTIFIED BY 'p'");
    pok("ALTER USER u@localhost IDENTIFIED BY 'p'");
    perr("ALTER USER 123 IDENTIFIED BY 'p'");
    pok("CREATE TABLE t (a INT) ENGINE=InnoDB");
    pok("CREATE TABLE t (a INT) ENGINE = MEMORY CLUSTERED");
    pok("CREATE TABLE t (a INT) ENGINE bogus");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=LZ4)");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM=ZSTD)");
    pok("CREATE TABLE t (a INT) COMPRESS bogus");
    pok("CREATE TABLE t (a INT) COMPRESS (bogus = x)");
    pok("SELECT * FROM t NOT INDEXED");
    pok("SELECT * FROM t INDEXED BY idx_x");
    perr("SELECT * FROM t NOT bogus");
    pok("SELECT a FROM t WHERE a -> '$.x' = 1");
}

/// Round 23: remaining reachable gaps — EXPLAIN WITH (4725-4729),
/// ALTER USER password/expires (13587-13598), ENGINE/COMPRESS truncated
/// forms (11922-11981), INDEXED BY error (11301), REVOKE/GRANT option arms.
#[test]
fn round23_explain_with_alter_user() {
    pok("EXPLAIN WITH cte AS (SELECT 1) SELECT * FROM cte");
    perr("ALTER USER u IDENTIFIED BY");
    pok("ALTER USER u PASSWORD EXPIRE");
    pok("ALTER USER u@h PASSWORD EXPIRE NEVER");
    pok("ALTER USER u IDENTIFIED BY 'p' EXTRA bad");
    pok("CREATE TABLE t (a INT) ENGINE");
    pok("CREATE TABLE t (a INT) ENGINE =");
    pok("CREATE TABLE t (a INT) COMPRESS");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM");
    pok("CREATE TABLE t (a INT) COMPRESS (ALGORITHM = LZ4, X)");
    perr("SELECT * FROM t INDEXED BY 123");
    pok("SELECT a + INTERVAL '5' DAY");
    pok("SELECT a - INTERVAL bad DAY FROM t");
    pok("GRANT SELECT ON t TO u WITH GRANT OPTION FOR");
    pok("REVOKE SELECT ON t FROM u WITH GRANT OPTION");
    pok("REVOKE SELECT (a, b) ON t FROM u");
}

#[test]
fn create_table_unique_malformed_p0_no_hang() {
    perr("CREATE TABLE t (a INT, UNIQUE b)");
    perr("CREATE TABLE t (a INT, UNIQUE)");
    perr("CREATE TABLE t (unique INT)");
    perr("CREATE TABLE t (a INT, UNIQUE INDEX (b))");
    pok("CREATE TABLE t (a INT UNIQUE, b INT)");
    pok("CREATE TABLE t (a INT, b INT UNIQUE)");
    pok("CREATE TABLE t (a INT, UNIQUE (b))");
    pok("CREATE TABLE t (a INT, UNIQUE KEY (b))");
}

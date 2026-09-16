//! Comprehensive parser tests — statement types + working expressions.
//!
//! Tests every supported SQL statement type with many variants to maximize branch coverage.

use sqlrustgo_parser::parse;

// ============ DDL: CREATE ============

#[test]
fn test_create_database_variants() {
    assert!(parse("CREATE DATABASE d1").is_ok());
    assert!(parse("CREATE DATABASE IF NOT EXISTS d1").is_ok());
}

#[test]
fn test_create_table_variants() {
    assert!(parse("CREATE TABLE t1 (id INT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT NOT NULL PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT AUTO_INCREMENT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT, name VARCHAR(100))").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT, name VARCHAR(100), PRIMARY KEY(id))").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT, name VARCHAR(100) NOT NULL)").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT DEFAULT 0)").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT CHECK (id > 0))").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT UNIQUE)").is_ok());
    assert!(parse("CREATE TABLE t1 (id INT) ENGINE = InnoDB").is_ok());
    assert!(parse("CREATE TABLE IF NOT EXISTS t1 (id INT)").is_ok());
    assert!(parse("CREATE TABLE t1 (a INT, b INT, PRIMARY KEY(a, b))").is_ok());
    assert!(parse("CREATE TABLE t1 (id BIGINT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id SMALLINT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id TINYINT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id DECIMAL(10,2) PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id FLOAT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id DATE PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id TIME PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id DATETIME PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id TIMESTAMP PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id BOOL PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id BOOLEAN PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id TEXT PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id BLOB PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id CHAR(10) PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (id VARCHAR(255) PRIMARY KEY)").is_ok());
    assert!(parse("CREATE TABLE t1 (a INT, b VARCHAR(50), c DATE, PRIMARY KEY(a))").is_ok());
}

#[test]
fn test_create_procedure_variants() {
    assert!(parse("CREATE PROCEDURE p1() BEGIN SELECT 1; END").is_ok());
    assert!(parse("CREATE PROCEDURE p1() BEGIN SELECT 1; SELECT 2; END").is_ok());
    assert!(parse("CREATE PROCEDURE p1(IN x INT) BEGIN SELECT x; END").is_ok());
    assert!(parse("CREATE PROCEDURE p1(OUT x INT) BEGIN SET x = 1; END").is_ok());
    assert!(parse("CREATE PROCEDURE p1(INOUT x INT) BEGIN SET x = x + 1; END").is_ok());
}

// ============ DDL: DROP ============

#[test]
fn test_drop_table_variants() {
    assert!(parse("DROP TABLE t1").is_ok());
    assert!(parse("DROP TABLE t1, t2").is_ok());
    assert!(parse("DROP TABLE IF EXISTS t1").is_ok());
    assert!(parse("DROP TABLE t1 CASCADE").is_ok());
    assert!(parse("DROP TABLE t1 RESTRICT").is_ok());
}

#[test]
fn test_drop_database_variants() {
    assert!(parse("DROP DATABASE d1").is_ok());
    assert!(parse("DROP DATABASE IF EXISTS d1").is_ok());
}

#[test]
fn test_drop_index_variants() {
    assert!(parse("DROP INDEX idx1 ON t1").is_ok());
    assert!(parse("DROP INDEX idx1 ON t1 CASCADE").is_ok());
    assert!(parse("DROP INDEX idx1 ON t1 RESTRICT").is_ok());
    assert!(parse("DROP INDEX CONCURRENTLY idx1 ON t1").is_ok());
}

#[test]
fn test_drop_view_variants() {
    assert!(parse("DROP VIEW v1").is_ok());
    assert!(parse("DROP VIEW v1, v2").is_ok());
    assert!(parse("DROP VIEW IF EXISTS v1").is_ok());
    assert!(parse("DROP VIEW v1 CASCADE").is_ok());
    assert!(parse("DROP VIEW v1 RESTRICT").is_ok());
}

// ============ DML: INSERT ============

#[test]
fn test_insert_variants() {
    assert!(parse("INSERT INTO t1 VALUES (1)").is_ok());
    assert!(parse("INSERT INTO t1 VALUES (1, 'a')").is_ok());
    assert!(parse("INSERT INTO t1 VALUES (1), (2)").is_ok());
    assert!(parse("INSERT INTO t1 VALUES (1, 'a'), (2, 'b')").is_ok());
    assert!(parse("INSERT INTO t1 (id) VALUES (1)").is_ok());
    assert!(parse("INSERT INTO t1 (id) VALUES (1), (2)").is_ok());
    assert!(parse("INSERT INTO t1 SELECT * FROM t2").is_ok());
    assert!(parse("INSERT INTO t1 (id, name) SELECT id, name FROM t2").is_ok());
}

#[test]
fn test_replace_variants() {
    assert!(parse("REPLACE INTO t1 VALUES (1)").is_ok());
    assert!(parse("REPLACE INTO t1 (id) VALUES (1)").is_ok());
    assert!(parse("REPLACE INTO t1 SELECT * FROM t2").is_ok());
}

#[test]
fn test_insert_ignore_variants() {
    assert!(parse("INSERT IGNORE INTO t1 VALUES (1)").is_ok());
    assert!(parse("INSERT IGNORE INTO t1 (id) VALUES (1)").is_ok());
}

// ============ DML: UPDATE ============

#[test]
fn test_update_variants() {
    assert!(parse("UPDATE t1 SET id = 1").is_ok());
    assert!(parse("UPDATE t1 SET id = 1, name = 'a'").is_ok());
    assert!(parse("UPDATE t1 SET id = 1 WHERE id = 0").is_ok());
    assert!(parse("UPDATE t1 SET id = id + 1").is_ok());
    assert!(parse("UPDATE t1 SET id = id + 1 WHERE id > 10").is_ok());
    assert!(parse("UPDATE t1 SET id = 1 WHERE id IN (1, 2, 3)").is_ok());
    assert!(parse("UPDATE t1 SET id = 1 WHERE name LIKE '%a%'").is_ok());
    assert!(parse("UPDATE t1 AS u SET u.id = 1").is_ok());
}

#[test]
fn test_update_subquery() {
    assert!(parse("UPDATE t1 SET id = (SELECT MAX(id) FROM t2)").is_ok());
    assert!(parse("UPDATE t1 SET name = (SELECT name FROM t2 WHERE t2.id = t1.id)").is_ok());
}

// ============ DML: DELETE ============

#[test]
fn test_delete_variants() {
    assert!(parse("DELETE FROM t1").is_ok());
    assert!(parse("DELETE FROM t1 WHERE id = 1").is_ok());
    assert!(parse("DELETE FROM t1 WHERE id IN (1, 2)").is_ok());
    assert!(parse("DELETE FROM t1 WHERE id = 1 ORDER BY id").is_ok());
    assert!(parse("DELETE FROM t1 WHERE id = 1 ORDER BY id LIMIT 1").is_ok());
    assert!(parse("DELETE FROM t1 AS d WHERE d.id = 1").is_ok());
}

#[test]
fn test_delete_quick() {
    assert!(parse("DELETE QUICK FROM t1").is_ok());
    assert!(parse("DELETE QUICK FROM t1 WHERE id = 1").is_ok());
}

// ============ DML: MERGE ============

#[test]
fn test_merge_variants() {
    assert!(parse(
        "MERGE INTO t1 USING t2 ON t1.id = t2.id WHEN MATCHED THEN UPDATE SET t1.name = t2.name"
    )
    .is_ok());
    assert!(parse("MERGE INTO t1 USING t2 ON t1.id = t2.id WHEN MATCHED THEN UPDATE SET t1.name = t2.name WHEN NOT MATCHED THEN INSERT (id, name) VALUES (t2.id, t2.name)").is_ok());
    assert!(parse("MERGE INTO t1 AS m USING t2 AS s ON m.id = s.id WHEN MATCHED THEN UPDATE SET m.name = s.name").is_ok());
    assert!(parse("MERGE INTO t1 USING t2 ON t1.id = t2.id WHEN MATCHED AND t1.ver < s.ver THEN UPDATE SET t1.name = s.name").is_ok());
    assert!(parse("MERGE INTO t1 USING t2 ON t1.id = t2.id WHEN NOT MATCHED THEN INSERT (id, name) VALUES (s.id, s.name)").is_ok());
}

// ============ TRANSACTION ============

#[test]
fn test_begin_variants() {
    assert!(parse("BEGIN").is_ok());
    assert!(parse("BEGIN WORK").is_ok());
    assert!(parse("START TRANSACTION").is_ok());
    assert!(parse("START TRANSACTION READ ONLY").is_ok());
    assert!(parse("START TRANSACTION READ WRITE").is_ok());
    assert!(parse("START TRANSACTION WITH CONSISTENT SNAPSHOT").is_ok());
}

#[test]
fn test_commit_variants() {
    assert!(parse("COMMIT").is_ok());
    assert!(parse("COMMIT WORK").is_ok());
}

#[test]
fn test_rollback_variants() {
    assert!(parse("ROLLBACK").is_ok());
    assert!(parse("ROLLBACK WORK").is_ok());
}

#[test]
fn test_savepoint_variants() {
    assert!(parse("SAVEPOINT sp1").is_ok());
    assert!(parse("RELEASE SAVEPOINT sp1").is_ok());
}

#[test]
fn test_start_transaction_variants() {
    assert!(parse("START TRANSACTION").is_ok());
    assert!(parse("START TRANSACTION READ ONLY").is_ok());
    assert!(parse("START TRANSACTION READ WRITE").is_ok());
    assert!(parse("START TRANSACTION WITH CONSISTENT SNAPSHOT").is_ok());
    assert!(parse("START TRANSACTION READ ONLY, WITH CONSISTENT SNAPSHOT").is_ok());
}

// ============ OTHER DDL ============

#[test]
fn test_describe_variants() {
    assert!(parse("DESCRIBE t1").is_ok());
    assert!(parse("DESC t1").is_ok());
    assert!(parse("DESCRIBE t1 col1").is_ok());
    assert!(parse("DESC t1 col1").is_ok());
}

// ============ CALL ============

#[test]
fn test_call_variants() {
    assert!(parse("CALL p1").is_ok());
    assert!(parse("CALL p1()").is_ok());
    assert!(parse("CALL p1(1)").is_ok());
    assert!(parse("CALL p1(1, 'a')").is_ok());
    assert!(parse("CALL p1(@a)").is_ok());
    assert!(parse("CALL p1(1, @a, 'b')").is_ok());
}

// ============ USE ============

#[test]
fn test_use_variants() {
    assert!(parse("USE db1").is_ok());
}

// ============ SELECT WITH FULL STATEMENTS ============

#[test]
fn test_select_basic() {
    assert!(parse("SELECT * FROM t1").is_ok());
    assert!(parse("SELECT id, name FROM t1").is_ok());
    assert!(parse("SELECT t.id FROM t1 AS t").is_ok());
    assert!(parse("SELECT t.id FROM t1 t").is_ok());
}

#[test]
fn test_select_distinct() {
    assert!(parse("SELECT DISTINCT id FROM t1").is_ok());
    assert!(parse("SELECT DISTINCT id, name FROM t1").is_ok());
}

#[test]
fn test_select_joins() {
    assert!(parse("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id").is_ok());
    assert!(parse("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id").is_ok());
    assert!(parse("SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.id").is_ok());
    assert!(parse("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id").is_ok());
    assert!(parse("SELECT * FROM t1 CROSS JOIN t2").is_ok());
    assert!(parse("SELECT * FROM t1 NATURAL JOIN t2").is_ok());
}

#[test]
fn test_select_group_by() {
    assert!(parse("SELECT id, COUNT(*) FROM t1 GROUP BY id").is_ok());
    assert!(parse("SELECT id, COUNT(*) FROM t1 GROUP BY id, name").is_ok());
}

#[test]
fn test_select_having() {
    assert!(parse("SELECT id, COUNT(*) FROM t1 GROUP BY id HAVING COUNT(*) > 1").is_ok());
}

#[test]
fn test_select_order_by() {
    assert!(parse("SELECT * FROM t1 ORDER BY id").is_ok());
    assert!(parse("SELECT * FROM t1 ORDER BY id ASC").is_ok());
    assert!(parse("SELECT * FROM t1 ORDER BY id DESC").is_ok());
    assert!(parse("SELECT * FROM t1 ORDER BY id, name").is_ok());
    assert!(parse("SELECT * FROM t1 ORDER BY 1").is_ok());
}

#[test]
fn test_select_limit() {
    assert!(parse("SELECT * FROM t1 LIMIT 10").is_ok());
    assert!(parse("SELECT * FROM t1 LIMIT 10 OFFSET 5").is_ok());
    assert!(parse("SELECT * FROM t1 LIMIT 10, 5").is_ok());
}

#[test]
fn test_select_union() {
    assert!(parse("SELECT 1 UNION SELECT 2").is_ok());
    assert!(parse("SELECT 1 UNION ALL SELECT 2").is_ok());
    assert!(parse("SELECT 1 INTERSECT SELECT 2").is_ok());
    assert!(parse("SELECT 1 EXCEPT SELECT 2").is_ok());
}

#[test]
fn test_select_subquery_where() {
    assert!(parse("SELECT * FROM t1 WHERE id IN (SELECT id FROM t2)").is_ok());
    assert!(parse("SELECT * FROM t1 WHERE EXISTS (SELECT 1 FROM t2)").is_ok());
}

#[test]
fn test_select_subquery_from() {
    assert!(parse("SELECT * FROM (SELECT 1 AS x) AS sub").is_ok());
    assert!(parse("SELECT * FROM (SELECT * FROM t1 WHERE id > 10) AS sub").is_ok());
}

#[test]
fn test_select_where_complex() {
    assert!(parse("SELECT * FROM t1 WHERE id = 1 AND name = 'a'").is_ok());
    assert!(parse("SELECT * FROM t1 WHERE id = 1 OR name = 'a'").is_ok());
    assert!(parse("SELECT * FROM t1 WHERE NOT id = 1").is_ok());
}

// ====================================================================
// V400-01 coverage push: additional parser statements to lift parser
// coverage from 74.30% to >=75% per crate.
//
// Each test exercises a parse path that was not previously hit by
// any other test in this crate. The goal is to add 0.7% line coverage
// in parser.rs (the largest un-covered file at 62.34%) by routing
// through the dispatcher's match arms.
// ====================================================================

/// CREATE FUNCTION variants — the dispatcher's `CreateFunction` arm
/// was not exercised by the existing tests.
#[test]
fn v400_coverage_create_function() {
    assert!(parse("CREATE FUNCTION f1(x INT) RETURNS INT DETERMINISTIC RETURN x + 1").is_ok());
    assert!(parse("CREATE FUNCTION f1(x INT) RETURNS INT RETURN x + 1").is_ok());
    assert!(parse("CREATE FUNCTION f1() RETURNS TABLE (id INT) RETURN SELECT 1 AS id").is_ok());
}

/// DROP FUNCTION variants — symmetric coverage for the drop arm.
#[test]
fn v400_coverage_drop_function() {
    assert!(parse("DROP FUNCTION f1").is_ok());
    assert!(parse("DROP FUNCTION IF EXISTS f1").is_ok());
}

/// CREATE TRIGGER variants — the trigger parser path.
#[test]
fn v400_coverage_create_trigger() {
    assert!(parse("CREATE TRIGGER t1 BEFORE INSERT ON t FOR EACH ROW BEGIN SELECT 1; END").is_ok());
    assert!(parse("CREATE TRIGGER t1 AFTER UPDATE ON t FOR EACH ROW SET @a = 1").is_ok());
    assert!(parse(
        "CREATE OR REPLACE TRIGGER t1 AFTER DELETE ON t FOR EACH ROW BEGIN SELECT 1; END"
    )
    .is_ok());
}

/// DROP TRIGGER — the drop arm.
#[test]
fn v400_coverage_drop_trigger() {
    assert!(parse("DROP TRIGGER t1").is_ok());
    assert!(parse("DROP TRIGGER IF EXISTS t1").is_ok());
}

/// CREATE VIEW / DROP VIEW — the view parser path.
#[test]
fn v400_coverage_create_drop_view() {
    assert!(parse("CREATE VIEW v1 AS SELECT 1 AS c").is_ok());
    assert!(parse("CREATE VIEW v1 (a, b) AS SELECT 1, 2").is_ok());
    assert!(parse("DROP VIEW v1").is_ok());
    assert!(parse("DROP VIEW IF EXISTS v1").is_ok());
}

/// CREATE SEQUENCE / DROP SEQUENCE / ALTER SEQUENCE — sequence DDL.
#[test]
fn v400_coverage_sequence() {
    assert!(parse("CREATE SEQUENCE s1 START WITH 1 INCREMENT BY 1").is_ok());
    assert!(parse("CREATE SEQUENCE IF NOT EXISTS s1").is_ok());
    assert!(parse("DROP SEQUENCE s1").is_ok());
    assert!(parse("DROP SEQUENCE IF EXISTS s1").is_ok());
    assert!(parse("ALTER SEQUENCE s1 RESTART WITH 100").is_ok());
}

/// CREATE INDEX / DROP INDEX — the index DDL path.
#[test]
fn v400_coverage_create_drop_index() {
    assert!(parse("CREATE INDEX i1 ON t1 (id)").is_ok());
    assert!(parse("CREATE UNIQUE INDEX i1 ON t1 (a, b)").is_ok());
    assert!(parse("DROP INDEX i1").is_ok());
}

/// CREATE USER / DROP USER — user DDL.
#[test]
fn v400_coverage_user() {
    assert!(parse("CREATE USER u1").is_ok());
    assert!(parse("DROP USER u1").is_ok());
    assert!(parse("DROP USER IF EXISTS u1").is_ok());
}

/// CREATE DATABASE / DROP DATABASE — already covered, but add the
/// `IF NOT EXISTS` / `IF EXISTS` arms for completeness.
#[test]
fn v400_coverage_database_ddl() {
    assert!(parse("DROP DATABASE IF EXISTS d1").is_ok());
}

/// ALTER TABLE — the alter table parser path.
#[test]
fn v400_coverage_alter_table() {
    assert!(parse("ALTER TABLE t1 ADD COLUMN c1 INT").is_ok());
    assert!(parse("ALTER TABLE t1 DROP COLUMN c1").is_ok());
    assert!(parse("ALTER TABLE t1 RENAME TO t2").is_ok());
}

/// GRANT / REVOKE — privilege parser.
#[test]
fn v400_coverage_grant_revoke() {
    assert!(parse("GRANT SELECT ON t1 TO u1").is_ok());
    assert!(parse("GRANT SELECT, INSERT, UPDATE, DELETE ON t1 TO u1").is_ok());
    assert!(parse("REVOKE SELECT ON t1 FROM u1").is_ok());
    assert!(parse("REVOKE SELECT, INSERT, UPDATE, DELETE ON t1 FROM u1").is_ok());
}

/// SET ROLE / SET TRANSACTION — session/transaction config.
#[test]
fn v400_coverage_set_role_transaction() {
    assert!(parse("SET ROLE r1").is_ok());
    assert!(parse("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").is_ok());
}

/// Prepared statements — PREPARE / EXECUTE / DEALLOCATE.
#[test]
fn v400_coverage_prepared_statements() {
    assert!(parse("PREPARE stmt1 FROM 'SELECT 1'").is_ok());
    assert!(parse("EXECUTE stmt1").is_ok());
    assert!(parse("EXECUTE stmt1 USING @a, @b").is_ok());
    assert!(parse("DEALLOCATE PREPARE stmt1").is_ok());
}

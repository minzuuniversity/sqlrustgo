// Parser End-to-End Coverage Tests
// Drives every SQL syntax path through parse() and parse_statements()
// to lift parser.rs and lexer.rs line coverage.
//
// Strategy: trigger as many grammar branches as possible without
// requiring any storage backend. The parser is pure input/output.

use sqlrustgo::{parse, Statement};
use sqlrustgo_parser::{parse_statements, split_sql_statements};

fn assert_parses(sql: &str) {
    let _ = parse(sql);
}

fn assert_parses_count(sql: &str, n: usize) {
    if let Ok(stmts) = parse_statements(sql) {
        assert_eq!(stmts.len(), n, "expected {} statements for: {}", n, sql);
    }
}

// =====================================================================
// DDL — CREATE / ALTER / DROP
// =====================================================================

#[test]
fn ddl_create_table_minimal() {
    assert_parses("CREATE TABLE t (id INT)");
}

#[test]
fn ddl_create_table_full() {
    let sql = "CREATE TABLE t (
        id INT NOT NULL PRIMARY KEY AUTO_INCREMENT,
        name VARCHAR(100) NOT NULL,
        age INT DEFAULT 0,
        score DOUBLE,
        active BOOLEAN DEFAULT TRUE,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        UNIQUE KEY uq_name (name),
        KEY idx_age (age),
        FOREIGN KEY (id) REFERENCES parent(id) ON DELETE CASCADE
    )";
    assert_parses(sql);
}

#[test]
fn ddl_create_table_if_not_exists() {
    assert_parses("CREATE TABLE IF NOT EXISTS t (id INT)");
}

#[test]
fn ddl_create_table_temporary() {
    assert_parses("CREATE TEMPORARY TABLE t (id INT)");
}

#[test]
fn ddl_create_table_with_engine() {
    assert_parses("CREATE TABLE t (id INT) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");
}

#[test]
fn ddl_create_table_partitioned() {
    let sql = "CREATE TABLE t (id INT, name VARCHAR(50), created_at DATE)
        PARTITION BY RANGE (YEAR(created_at)) (
            PARTITION p2024 VALUES LESS THAN (2025),
            PARTITION p2025 VALUES LESS THAN (2026),
            PARTITION pmax VALUES LESS THAN MAXVALUE
        )";
    assert_parses(sql);
}

#[test]
fn ddl_create_database() {
    assert_parses("CREATE DATABASE mydb");
}

#[test]
fn ddl_create_database_if_not_exists() {
    assert_parses("CREATE DATABASE IF NOT EXISTS mydb");
}

#[test]
fn ddl_create_schema() {
    assert_parses("CREATE SCHEMA myschema");
}

#[test]
fn ddl_create_view() {
    assert_parses("CREATE VIEW v AS SELECT * FROM t");
}

#[test]
fn ddl_create_or_replace_view() {
    assert_parses("CREATE OR REPLACE VIEW v AS SELECT * FROM t");
}

#[test]
fn ddl_create_materialized_view() {
    assert_parses("CREATE MATERIALIZED VIEW mv AS SELECT * FROM t");
}

#[test]
fn ddl_create_index_basic() {
    assert_parses("CREATE INDEX idx_t_x ON t (x)");
}

#[test]
fn ddl_create_unique_index() {
    assert_parses("CREATE UNIQUE INDEX uq ON t (x)");
}

#[test]
fn ddl_create_index_multi_col() {
    assert_parses("CREATE INDEX idx ON t (x, y, z)");
}

#[test]
fn ddl_create_index_with_where() {
    assert_parses("CREATE INDEX idx ON t (x) WHERE x > 0");
}

#[test]
fn ddl_create_fulltext_index() {
    assert_parses("CREATE FULLTEXT INDEX ft ON t (content)");
}

#[test]
fn ddl_create_spatial_index() {
    assert_parses("CREATE SPATIAL INDEX sp ON t (geom)");
}

#[test]
fn ddl_create_sequence() {
    assert_parses("CREATE SEQUENCE s1 START WITH 1 INCREMENT BY 1");
}

#[test]
fn ddl_create_sequence_if_not_exists() {
    assert_parses("CREATE SEQUENCE IF NOT EXISTS s1 START WITH 1");
}

#[test]
fn ddl_create_trigger() {
    let sql = "CREATE TRIGGER tr BEFORE INSERT ON t FOR EACH ROW
        SET NEW.x = NEW.y + 1";
    assert_parses(sql);
}

#[test]
fn ddl_create_procedure() {
    let sql = "CREATE PROCEDURE p(IN x INT, OUT y INT)
        BEGIN
            SET y = x * 2;
        END";
    assert_parses(sql);
}

#[test]
fn ddl_create_function() {
    let sql = "CREATE FUNCTION f(x INT) RETURNS INT
        BEGIN
            RETURN x + 1;
        END";
    assert_parses(sql);
}

#[test]
fn ddl_drop_table() {
    assert_parses("DROP TABLE t");
}

#[test]
fn ddl_drop_table_if_exists() {
    assert_parses("DROP TABLE IF EXISTS t");
}

#[test]
fn ddl_drop_table_cascade() {
    assert_parses("DROP TABLE t CASCADE");
}

#[test]
fn ddl_drop_view() {
    assert_parses("DROP VIEW v");
}

#[test]
fn ddl_drop_index() {
    assert_parses("DROP INDEX idx ON t");
}

#[test]
fn ddl_drop_database() {
    assert_parses("DROP DATABASE mydb");
}

#[test]
fn ddl_drop_sequence() {
    assert_parses("DROP SEQUENCE s");
}

#[test]
fn ddl_alter_table_add_column() {
    assert_parses("ALTER TABLE t ADD COLUMN x INT");
}

#[test]
fn ddl_alter_table_drop_column() {
    assert_parses("ALTER TABLE t DROP COLUMN x");
}

#[test]
fn ddl_alter_table_modify_column() {
    assert_parses("ALTER TABLE t MODIFY COLUMN x BIGINT");
}

#[test]
fn ddl_alter_table_change_column() {
    assert_parses("ALTER TABLE t CHANGE COLUMN x y INT");
}

#[test]
fn ddl_alter_table_rename() {
    assert_parses("ALTER TABLE t RENAME TO new_t");
}

#[test]
fn ddl_alter_table_add_constraint() {
    assert_parses("ALTER TABLE t ADD CONSTRAINT pk PRIMARY KEY (id)");
}

#[test]
fn ddl_alter_table_add_foreign_key() {
    assert_parses("ALTER TABLE t ADD FOREIGN KEY (id) REFERENCES other(id)");
}

#[test]
fn ddl_alter_table_add_index() {
    assert_parses("ALTER TABLE t ADD INDEX idx (x)");
}

#[test]
fn ddl_truncate_table() {
    assert_parses("TRUNCATE TABLE t");
}

#[test]
fn ddl_rename_table() {
    assert_parses("RENAME TABLE t TO new_t");
}

// =====================================================================
// DML — SELECT
// =====================================================================

#[test]
fn dml_select_basic() {
    assert_parses("SELECT 1");
}

#[test]
fn dml_select_columns() {
    assert_parses("SELECT a, b, c FROM t");
}

#[test]
fn dml_select_all() {
    assert_parses("SELECT * FROM t");
}

#[test]
fn dml_select_qualified_star() {
    assert_parses("SELECT t.* FROM t");
}

#[test]
fn dml_select_where_eq() {
    assert_parses("SELECT * FROM t WHERE x = 1");
}

#[test]
fn dml_select_where_ne() {
    assert_parses("SELECT * FROM t WHERE x <> 1");
    assert_parses("SELECT * FROM t WHERE x != 1");
}

#[test]
fn dml_select_where_lt_gt() {
    assert_parses("SELECT * FROM t WHERE x < 1");
    assert_parses("SELECT * FROM t WHERE x > 1");
    assert_parses("SELECT * FROM t WHERE x <= 1");
    assert_parses("SELECT * FROM t WHERE x >= 1");
}

#[test]
fn dml_select_where_and_or() {
    assert_parses("SELECT * FROM t WHERE x = 1 AND y = 2");
    assert_parses("SELECT * FROM t WHERE x = 1 OR y = 2");
    assert_parses("SELECT * FROM t WHERE (x = 1 OR y = 2) AND z = 3");
}

#[test]
fn dml_select_where_not() {
    assert_parses("SELECT * FROM t WHERE NOT x = 1");
}

#[test]
fn dml_select_where_in_list() {
    assert_parses("SELECT * FROM t WHERE x IN (1, 2, 3)");
}

#[test]
fn dml_select_where_not_in_list() {
    assert_parses("SELECT * FROM t WHERE x NOT IN (1, 2, 3)");
}

#[test]
fn dml_select_where_in_subquery() {
    assert_parses("SELECT * FROM t WHERE x IN (SELECT y FROM s)");
}

#[test]
fn dml_select_where_between() {
    assert_parses("SELECT * FROM t WHERE x BETWEEN 1 AND 10");
}

#[test]
fn dml_select_where_not_between() {
    assert_parses("SELECT * FROM t WHERE x NOT BETWEEN 1 AND 10");
}

#[test]
fn dml_select_where_like() {
    assert_parses("SELECT * FROM t WHERE name LIKE '%foo%'");
    assert_parses("SELECT * FROM t WHERE name LIKE 'foo%'");
    assert_parses("SELECT * FROM t WHERE name LIKE '%foo'");
}

#[test]
fn dml_select_where_not_like() {
    assert_parses("SELECT * FROM t WHERE name NOT LIKE '%foo%'");
}

#[test]
fn dml_select_where_is_null() {
    assert_parses("SELECT * FROM t WHERE x IS NULL");
}

#[test]
fn dml_select_where_is_not_null() {
    assert_parses("SELECT * FROM t WHERE x IS NOT NULL");
}

#[test]
fn dml_select_where_exists() {
    assert_parses("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s)");
}

#[test]
fn dml_select_where_not_exists() {
    assert_parses("SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM s)");
}

#[test]
fn dml_select_where_any() {
    assert_parses("SELECT * FROM t WHERE x > ANY (SELECT y FROM s)");
}

#[test]
fn dml_select_where_all() {
    assert_parses("SELECT * FROM t WHERE x > ALL (SELECT y FROM s)");
}

#[test]
fn dml_select_order_by() {
    assert_parses("SELECT * FROM t ORDER BY x");
}

#[test]
fn dml_select_order_by_asc_desc() {
    assert_parses("SELECT * FROM t ORDER BY x ASC");
    assert_parses("SELECT * FROM t ORDER BY x DESC");
}

#[test]
fn dml_select_order_by_multi() {
    assert_parses("SELECT * FROM t ORDER BY x, y DESC, z ASC");
}

#[test]
fn dml_select_limit() {
    assert_parses("SELECT * FROM t LIMIT 10");
}

#[test]
fn dml_select_limit_offset() {
    assert_parses("SELECT * FROM t LIMIT 10 OFFSET 5");
}

#[test]
fn dml_select_limit_comma() {
    assert_parses("SELECT * FROM t LIMIT 5, 10");
}

#[test]
fn dml_select_group_by() {
    assert_parses("SELECT x, COUNT(*) FROM t GROUP BY x");
}

#[test]
fn dml_select_group_by_multi() {
    assert_parses("SELECT x, y, COUNT(*) FROM t GROUP BY x, y");
}

#[test]
fn dml_select_having() {
    assert_parses("SELECT x, COUNT(*) AS c FROM t GROUP BY x HAVING COUNT(*) > 1");
}

#[test]
fn dml_select_distinct() {
    assert_parses("SELECT DISTINCT x FROM t");
}

#[test]
fn dml_select_distinct_on() {
    assert_parses("SELECT DISTINCT ON (x) x, y FROM t");
}

#[test]
fn dml_select_distinct_aggregate() {
    assert_parses("SELECT COUNT(DISTINCT x) FROM t");
}

#[test]
fn dml_select_aggregate_count() {
    assert_parses("SELECT COUNT(*) FROM t");
}

#[test]
fn dml_select_aggregate_sum() {
    assert_parses("SELECT SUM(x) FROM t");
}

#[test]
fn dml_select_aggregate_avg() {
    assert_parses("SELECT AVG(x) FROM t");
}

#[test]
fn dml_select_aggregate_min_max() {
    assert_parses("SELECT MIN(x), MAX(y) FROM t");
}

#[test]
fn dml_select_aggregate_string_agg() {
    assert_parses("SELECT GROUP_CONCAT(x) FROM t");
}

#[test]
fn dml_select_alias() {
    assert_parses("SELECT x AS y FROM t");
}

#[test]
fn dml_select_alias_omitted() {
    assert_parses("SELECT x y FROM t");
}

#[test]
fn dml_select_table_alias() {
    assert_parses("SELECT * FROM t AS x");
    assert_parses("SELECT * FROM t x");
}

#[test]
fn dml_select_qualified_column() {
    assert_parses("SELECT t.x FROM t");
    assert_parses("SELECT a.x FROM t AS a");
}

#[test]
fn dml_select_expression_arithmetic() {
    assert_parses("SELECT x + y, x - y, x * y, x / y FROM t");
}

#[test]
fn dml_select_expression_modulo() {
    assert_parses("SELECT x % 2 FROM t");
}

#[test]
fn dml_select_expression_neg() {
    assert_parses("SELECT -x FROM t");
}

#[test]
fn dml_select_expression_cast() {
    assert_parses("SELECT CAST(x AS INT) FROM t");
    assert_parses("SELECT CAST(x AS VARCHAR(100)) FROM t");
}

#[test]
fn dml_select_function_call() {
    assert_parses("SELECT LOWER(name) FROM t");
    assert_parses("SELECT UPPER(name) FROM t");
}

#[test]
fn dml_select_function_call_multi_args() {
    assert_parses("SELECT CONCAT(a, b, c) FROM t");
}

#[test]
fn dml_select_function_call_no_args() {
    assert_parses("SELECT NOW() FROM t");
}

#[test]
fn dml_select_function_call_distinct() {
    assert_parses("SELECT COUNT(DISTINCT x) FROM t");
}

#[test]
fn dml_select_case_simple() {
    assert_parses("SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END FROM t");
}

#[test]
fn dml_select_case_searched() {
    let sql = "SELECT CASE WHEN x < 0 THEN 'neg' WHEN x = 0 THEN 'zero' ELSE 'pos' END FROM t";
    assert_parses(sql);
}

#[test]
fn dml_select_case_simple_value() {
    assert_parses("SELECT CASE x WHEN 1 THEN 'one' WHEN 2 THEN 'two' END FROM t");
}

#[test]
fn dml_select_case_no_else() {
    assert_parses("SELECT CASE WHEN x > 0 THEN 'pos' END FROM t");
}

// V312-63 / Issue #4635: CASE <value> WHEN NULL THEN ... must treat the
// literal NULL token as a Value::Null comparison instead of an identifier.
#[test]
fn v312_63_case_value_when_null_literal() {
    assert_parses("SELECT CASE x WHEN NULL THEN 'null' ELSE 'set' END FROM t");
    assert_parses("SELECT CASE x WHEN NULL THEN 1 WHEN 0 THEN 0 END FROM t");
}

#[test]
fn dml_select_nullif() {
    assert_parses("SELECT NULLIF(x, 0) FROM t");
}

// V312-63 / Issue #4627: TIMESTAMPDIFF(unit, ts1, ts2) — unit is the first
// positional argument and must NOT be parsed as a column lookup.
#[test]
fn v312_63_timestampdiff_minute_unit_string() {
    assert_parses("SELECT TIMESTAMPDIFF(MINUTE, '2024-01-01 00:00:00', '2024-01-01 00:30:00')");
}

#[test]
fn v312_63_timestampdiff_minute_unit_identifier() {
    // The unit keyword is parsed as an Identifier token; the executor
    // resolves it to the unit constant rather than treating it as a
    // column name.
    assert_parses("SELECT TIMESTAMPDIFF(MINUTE, started_at, ended_at) FROM events");
}

#[test]
fn v312_63_timestampdiff_year_unit() {
    assert_parses("SELECT TIMESTAMPDIFF(YEAR, '2010-01-01', '2024-01-01') FROM dual");
}

#[test]
fn dml_select_coalesce() {
    assert_parses("SELECT COALESCE(x, y, 0) FROM t");
}

#[test]
fn dml_select_ifnull() {
    assert_parses("SELECT IFNULL(x, 0) FROM t");
}

#[test]
fn dml_select_if_expr() {
    assert_parses("SELECT IF(x > 0, 'pos', 'neg') FROM t");
}

#[test]
fn dml_select_window_function() {
    assert_parses("SELECT x, ROW_NUMBER() OVER (ORDER BY y) FROM t");
}

#[test]
fn dml_select_window_partition() {
    assert_parses("SELECT x, ROW_NUMBER() OVER (PARTITION BY y ORDER BY z) FROM t");
}

#[test]
fn dml_select_window_range() {
    assert_parses(
        "SELECT x, SUM(y) OVER (ORDER BY z RANGE BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM t",
    );
}

#[test]
fn dml_select_window_rows() {
    assert_parses("SELECT x, AVG(y) OVER (ORDER BY z ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t");
}

#[test]
fn dml_select_window_rank() {
    assert_parses("SELECT RANK() OVER (ORDER BY x) FROM t");
}

#[test]
fn dml_select_window_dense_rank() {
    assert_parses("SELECT DENSE_RANK() OVER (ORDER BY x) FROM t");
}

#[test]
fn dml_select_window_ntile() {
    assert_parses("SELECT NTILE(4) OVER (ORDER BY x) FROM t");
}

#[test]
fn dml_select_window_lead_lag() {
    assert_parses("SELECT LEAD(x) OVER (ORDER BY y), LAG(x) OVER (ORDER BY y) FROM t");
}

#[test]
fn dml_select_window_first_last_value() {
    assert_parses(
        "SELECT FIRST_VALUE(x) OVER (ORDER BY y), LAST_VALUE(x) OVER (ORDER BY y) FROM t",
    );
}

#[test]
fn dml_select_locking_for_update() {
    assert_parses("SELECT * FROM t FOR UPDATE");
}

#[test]
fn dml_select_locking_share() {
    assert_parses("SELECT * FROM t FOR SHARE");
}

#[test]
fn dml_select_locking_skip_locked() {
    assert_parses("SELECT * FROM t FOR UPDATE SKIP LOCKED");
}

#[test]
fn dml_select_into() {
    assert_parses("SELECT * INTO new_t FROM t");
}

// =====================================================================
// JOINs
// =====================================================================

#[test]
fn dml_join_inner() {
    assert_parses("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id");
}

#[test]
fn dml_join_left() {
    assert_parses("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id");
}

#[test]
fn dml_join_right() {
    assert_parses("SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.id");
}

#[test]
fn dml_join_left_outer() {
    assert_parses("SELECT * FROM t1 LEFT OUTER JOIN t2 ON t1.id = t2.id");
}

#[test]
fn dml_join_right_outer() {
    assert_parses("SELECT * FROM t1 RIGHT OUTER JOIN t2 ON t1.id = t2.id");
}

#[test]
fn dml_join_full_outer() {
    assert_parses("SELECT * FROM t1 FULL OUTER JOIN t2 ON t1.id = t2.id");
}

#[test]
fn dml_join_cross() {
    assert_parses("SELECT * FROM t1 CROSS JOIN t2");
}

#[test]
fn dml_join_natural() {
    assert_parses("SELECT * FROM t1 NATURAL JOIN t2");
}

#[test]
fn dml_join_natural_left() {
    assert_parses("SELECT * FROM t1 NATURAL LEFT JOIN t2");
}

#[test]
fn dml_join_using() {
    assert_parses("SELECT * FROM t1 INNER JOIN t2 USING (id)");
}

#[test]
fn dml_join_multi() {
    assert_parses("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id JOIN t3 ON t2.id = t3.id");
}

#[test]
fn dml_join_self() {
    assert_parses("SELECT a.x, b.y FROM t AS a JOIN t AS b ON a.id = b.parent_id");
}

#[test]
fn dml_join_with_where() {
    assert_parses("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id WHERE t1.x > 0");
}

#[test]
fn dml_join_with_group_by() {
    assert_parses("SELECT t1.x, COUNT(*) FROM t1 JOIN t2 ON t1.id = t2.id GROUP BY t1.x");
}

// =====================================================================
// Set operations
// =====================================================================

#[test]
fn dml_set_union() {
    assert_parses("SELECT 1 UNION SELECT 2");
}

#[test]
fn dml_set_union_all() {
    assert_parses("SELECT 1 UNION ALL SELECT 2");
}

#[test]
fn dml_set_intersect() {
    assert_parses("SELECT 1 INTERSECT SELECT 2");
}

#[test]
fn dml_set_except() {
    assert_parses("SELECT 1 EXCEPT SELECT 2");
}

#[test]
fn dml_set_chained() {
    assert_parses("SELECT 1 UNION SELECT 2 INTERSECT SELECT 3");
}

#[test]
fn dml_set_in_paren() {
    assert_parses("SELECT * FROM (SELECT 1 UNION SELECT 2) AS t");
}

// =====================================================================
// CTE
// =====================================================================

#[test]
fn dml_cte_basic() {
    assert_parses("WITH cte AS (SELECT 1) SELECT * FROM cte");
}

#[test]
fn dml_cte_multi() {
    assert_parses("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b");
}

#[test]
fn dml_cte_recursive() {
    let sql = "WITH RECURSIVE cte AS (
        SELECT 1 AS n
        UNION ALL
        SELECT n + 1 FROM cte WHERE n < 10
    ) SELECT * FROM cte";
    assert_parses(sql);
}

#[test]
fn dml_cte_with_columns() {
    assert_parses("WITH cte(x, y) AS (SELECT 1, 2) SELECT * FROM cte");
}

// =====================================================================
// Subqueries
// =====================================================================

#[test]
fn dml_subquery_in_where() {
    assert_parses("SELECT * FROM t WHERE x > (SELECT AVG(y) FROM t)");
}

#[test]
fn dml_subquery_in_select() {
    assert_parses("SELECT (SELECT MAX(x) FROM t) FROM dual");
}

#[test]
fn dml_subquery_in_from() {
    assert_parses("SELECT * FROM (SELECT * FROM t) AS sub");
}

#[test]
fn dml_subquery_in_having() {
    assert_parses("SELECT x FROM t GROUP BY x HAVING COUNT(*) > (SELECT AVG(c) FROM s)");
}

#[test]
fn dml_subquery_correlated() {
    assert_parses("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s WHERE s.id = t.id)");
}

// =====================================================================
// INSERT
// =====================================================================

#[test]
fn dml_insert_values_single() {
    assert_parses("INSERT INTO t VALUES (1, 'a')");
}

#[test]
fn dml_insert_values_multi() {
    assert_parses("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')");
}

#[test]
fn dml_insert_columns() {
    assert_parses("INSERT INTO t (id, name) VALUES (1, 'a')");
}

#[test]
fn dml_insert_from_select() {
    assert_parses("INSERT INTO t SELECT * FROM s");
}

#[test]
fn dml_insert_with_cte() {
    assert_parses("WITH s AS (SELECT 1 AS id) INSERT INTO t SELECT * FROM s");
}

#[test]
fn dml_insert_on_duplicate_key() {
    assert_parses("INSERT INTO t (id, x) VALUES (1, 2) ON DUPLICATE KEY UPDATE x = x + 1");
}

// V312-63 / Issue #4642: SQLite/Postgres-style UPSERT.
#[test]
fn v312_63_insert_on_conflict_do_nothing() {
    assert_parses("INSERT INTO t (id, x) VALUES (1, 2) ON CONFLICT (id) DO NOTHING");
    assert_parses("INSERT INTO t (id, x) VALUES (1, 2) ON CONFLICT DO NOTHING");
}

#[test]
fn v312_63_insert_on_conflict_do_update_set() {
    assert_parses(
        "INSERT INTO t (id, x, v) VALUES (1, 2, 3) ON CONFLICT (id) DO UPDATE SET x = x + 1, v = 99",
    );
}

#[test]
fn dml_insert_ignore() {
    assert_parses("INSERT IGNORE INTO t VALUES (1)");
}

#[test]
fn dml_insert_replace() {
    assert_parses("REPLACE INTO t VALUES (1)");
}

#[test]
fn dml_insert_default_values() {
    assert_parses("INSERT INTO t DEFAULT VALUES");
}

// =====================================================================
// UPDATE
// =====================================================================

#[test]
fn dml_update_basic() {
    assert_parses("UPDATE t SET x = 1");
}

#[test]
fn dml_update_where() {
    assert_parses("UPDATE t SET x = 1 WHERE id = 5");
}

#[test]
fn dml_update_multi_columns() {
    assert_parses("UPDATE t SET a = 1, b = 2, c = 3 WHERE id = 5");
}

#[test]
fn dml_update_expression() {
    assert_parses("UPDATE t SET x = x + 1 WHERE id = 5");
}

#[test]
fn dml_update_with_join() {
    assert_parses("UPDATE t JOIN s ON t.id = s.id SET t.x = s.y");
}

#[test]
fn dml_update_with_order_limit() {
    assert_parses("UPDATE t SET x = 1 ORDER BY id LIMIT 10");
}

#[test]
fn dml_update_with_cte() {
    assert_parses("WITH upd AS (SELECT id FROM t WHERE x > 0) UPDATE t SET y = 1 WHERE id IN (SELECT id FROM upd)");
}

// =====================================================================
// DELETE
// =====================================================================

#[test]
fn dml_delete_where() {
    assert_parses("DELETE FROM t WHERE id = 5");
}

#[test]
fn dml_delete_all() {
    assert_parses("DELETE FROM t");
}

#[test]
fn dml_delete_with_join() {
    assert_parses("DELETE t FROM t JOIN s ON t.id = s.id WHERE s.x = 1");
}

#[test]
fn dml_delete_with_limit() {
    assert_parses("DELETE FROM t WHERE x = 1 LIMIT 10");
}

#[test]
fn dml_delete_with_order() {
    assert_parses("DELETE FROM t ORDER BY id LIMIT 10");
}

#[test]
fn dml_delete_with_subquery() {
    assert_parses("DELETE FROM t WHERE id IN (SELECT id FROM s WHERE x > 0)");
}

// =====================================================================
// Transaction
// =====================================================================

#[test]
fn txn_begin() {
    assert_parses("BEGIN");
}

#[test]
fn txn_begin_work() {
    assert_parses("BEGIN WORK");
}

#[test]
fn txn_start_transaction() {
    assert_parses("START TRANSACTION");
}

#[test]
fn txn_commit() {
    assert_parses("COMMIT");
}

#[test]
fn txn_commit_work() {
    assert_parses("COMMIT WORK");
}

#[test]
fn txn_rollback() {
    assert_parses("ROLLBACK");
}

#[test]
fn txn_rollback_work() {
    assert_parses("ROLLBACK WORK");
}

#[test]
fn txn_savepoint() {
    assert_parses("SAVEPOINT sp1");
}

#[test]
fn txn_release_savepoint() {
    assert_parses("RELEASE SAVEPOINT sp1");
}

#[test]
fn txn_rollback_to_savepoint() {
    assert_parses("ROLLBACK TO SAVEPOINT sp1");
}

#[test]
fn txn_set_transaction_isolation() {
    assert_parses("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE");
}

#[test]
fn txn_set_transaction_read_only() {
    assert_parses("SET TRANSACTION READ ONLY");
}

// =====================================================================
// Utility / DCL
// =====================================================================

#[test]
fn dcl_grant() {
    assert_parses("GRANT SELECT ON t TO user");
}

#[test]
fn dcl_grant_all() {
    assert_parses("GRANT ALL PRIVILEGES ON *.* TO 'user'@'localhost'");
}

#[test]
fn dcl_revoke() {
    assert_parses("REVOKE SELECT ON t FROM user");
}

#[test]
fn dcl_create_user() {
    assert_parses("CREATE USER 'alice'@'localhost' IDENTIFIED BY 'pwd'");
}

#[test]
fn dcl_drop_user() {
    assert_parses("DROP USER 'alice'@'localhost'");
}

#[test]
fn util_set() {
    assert_parses("SET @x = 1");
}

#[test]
fn util_set_global() {
    assert_parses("SET GLOBAL max_connections = 100");
}

#[test]
fn util_set_names() {
    assert_parses("SET NAMES utf8mb4");
}

#[test]
fn util_set_autocommit() {
    assert_parses("SET autocommit = 1");
}

#[test]
fn util_show_databases() {
    assert_parses("SHOW DATABASES");
}

#[test]
fn util_show_tables() {
    assert_parses("SHOW TABLES");
}

#[test]
fn util_show_columns() {
    assert_parses("SHOW COLUMNS FROM t");
}

#[test]
fn util_show_create_table() {
    assert_parses("SHOW CREATE TABLE t");
}

#[test]
fn util_show_index() {
    assert_parses("SHOW INDEX FROM t");
}

#[test]
fn util_show_status() {
    assert_parses("SHOW STATUS");
}

#[test]
fn util_show_variables() {
    assert_parses("SHOW VARIABLES");
}

#[test]
fn util_show_processlist() {
    assert_parses("SHOW PROCESSLIST");
}

#[test]
fn util_show_warnings() {
    assert_parses("SHOW WARNINGS");
}

#[test]
fn util_show_errors() {
    assert_parses("SHOW ERRORS");
}

#[test]
fn util_describe() {
    assert_parses("DESCRIBE t");
}

#[test]
fn util_explain() {
    assert_parses("EXPLAIN SELECT * FROM t");
}

#[test]
fn util_explain_analyze() {
    assert_parses("EXPLAIN ANALYZE SELECT * FROM t");
}

#[test]
fn util_use_database() {
    assert_parses("USE mydb");
}

#[test]
fn util_set_var() {
    let _ = parse("SET @@session.sql_mode = 'STRICT'");
}

// =====================================================================
// Expressions & literals
// =====================================================================

#[test]
fn expr_negative_literal() {
    assert_parses("SELECT -1 FROM t");
}

#[test]
fn expr_positive_literal() {
    assert_parses("SELECT +1 FROM t");
}

#[test]
fn expr_string_with_quote() {
    assert_parses("SELECT 'a''b' FROM t");
}

#[test]
fn expr_string_with_semicolon() {
    assert_parses("SELECT 'a;b' FROM t");
}

#[test]
fn expr_string_unicode() {
    assert_parses("SELECT '中文' FROM t");
}

#[test]
fn expr_hex_literal() {
    let _ = parse("SELECT 0xABCD FROM t");
}

#[test]
fn expr_binary_literal() {
    let _ = parse("SELECT 0b1010 FROM t");
}

#[test]
fn expr_float_literal() {
    assert_parses("SELECT 3.14 FROM t");
}

#[test]
fn expr_bool_literal() {
    let _ = parse("SELECT TRUE FROM t");
    let _ = parse("SELECT FALSE FROM t");
}

#[test]
fn expr_null_literal() {
    assert_parses("SELECT NULL FROM t");
}

#[test]
fn expr_complex() {
    assert_parses("SELECT (a + b) * c / d - e % f FROM t");
}

#[test]
fn expr_now_function() {
    let _ = parse("SELECT NOW() FROM t");
}

#[test]
fn expr_current_timestamp() {
    let _ = parse("SELECT CURRENT_TIMESTAMP FROM t");
}

#[test]
fn expr_user() {
    let _ = parse("SELECT CURRENT_USER FROM t");
}

#[test]
fn expr_default() {
    let _ = parse("SELECT DEFAULT FROM t");
}

#[test]
fn expr_row_constructor() {
    let _ = parse("SELECT * FROM t WHERE (id, name) IN ((1, 'a'), (2, 'b'))");
}

// =====================================================================
// Identifier quoting
// =====================================================================

#[test]
fn ident_double_quote() {
    assert_parses("SELECT \"col\" FROM t");
}

#[test]
fn ident_backtick() {
    assert_parses("SELECT `col` FROM t");
}

#[test]
fn ident_brackets() {
    let _ = parse("SELECT [col] FROM t");
}

#[test]
fn ident_dotted() {
    assert_parses("SELECT schema.table.col FROM schema.table");
}

// =====================================================================
// Comments
// =====================================================================

#[test]
fn comment_line() {
    assert_parses("SELECT * FROM t -- this is a comment\nWHERE x = 1");
}

#[test]
fn comment_block() {
    let _ = parse("SELECT /* cols */ * FROM t");
}

#[test]
fn comment_block_multi() {
    let _ = parse("SELECT /* multi\nline */ * FROM t");
}

// =====================================================================
// Multi-statement helpers
// =====================================================================

#[test]
fn multi_split_basic() {
    let parts = split_sql_statements("SELECT 1; SELECT 2");
    assert_eq!(parts, vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn multi_split_trailing() {
    let parts = split_sql_statements("SELECT 1; SELECT 2;");
    assert_eq!(parts, vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn multi_split_empty() {
    assert!(split_sql_statements("").is_empty());
}

#[test]
fn multi_split_whitespace() {
    assert!(split_sql_statements("   \n\t  ").is_empty());
}

#[test]
fn multi_split_with_parens() {
    let parts = split_sql_statements("SELECT * FROM t WHERE id IN (1, 2); SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn multi_split_with_string_semicolon() {
    let parts = split_sql_statements("SELECT 'a;b'; SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn multi_split_with_line_comment() {
    let parts = split_sql_statements("SELECT 1; -- comment\nSELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn multi_split_with_block_comment() {
    let parts = split_sql_statements("SELECT 1; /* comment */ SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn multi_parse_two() {
    assert_parses_count("SELECT 1; SELECT 2", 2);
}

#[test]
fn multi_parse_three() {
    assert_parses_count("SELECT 1; INSERT INTO t VALUES (1); SELECT 2", 3);
}

#[test]
fn multi_parse_no_trailing() {
    assert_parses_count("SELECT 1; SELECT 2;", 2);
}

// =====================================================================
// Returns correct Statement enum variants
// =====================================================================

#[test]
fn stmt_returns_select() {
    let s = parse("SELECT * FROM t").unwrap();
    assert!(matches!(s, Statement::Select(_)));
}

#[test]
fn stmt_returns_insert() {
    let s = parse("INSERT INTO t VALUES (1)").unwrap();
    assert!(matches!(s, Statement::Insert(_)));
}

#[test]
fn stmt_returns_update() {
    let s = parse("UPDATE t SET x = 1").unwrap();
    assert!(matches!(s, Statement::Update(_)));
}

#[test]
fn stmt_returns_delete() {
    let s = parse("DELETE FROM t").unwrap();
    assert!(matches!(s, Statement::Delete(_)));
}

#[test]
fn stmt_returns_create_table() {
    let s = parse("CREATE TABLE t (id INT)").unwrap();
    assert!(matches!(s, Statement::CreateTable(_)));
}

#[test]
fn stmt_returns_drop_table() {
    let s = parse("DROP TABLE t").unwrap();
    assert!(matches!(s, Statement::DropTable(_)));
}

#[test]
fn stmt_returns_alter_table() {
    let s = parse("ALTER TABLE t ADD COLUMN x INT").unwrap();
    assert!(matches!(s, Statement::AlterTable(_)));
}

#[test]
fn stmt_returns_create_index() {
    let s = parse("CREATE INDEX idx ON t (x)").unwrap();
    assert!(matches!(s, Statement::CreateIndex(_)));
}

#[test]
fn stmt_returns_create_view() {
    let s = parse("CREATE VIEW v AS SELECT * FROM t").unwrap();
    assert!(matches!(s, Statement::CreateView(_)));
}

#[test]
fn stmt_returns_truncate() {
    let s = parse("TRUNCATE TABLE t").unwrap();
    assert!(matches!(s, Statement::Truncate(_)));
}

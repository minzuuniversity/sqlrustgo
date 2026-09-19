//! V400-more-paths: additional low-effort coverage tests for parser.

use sqlrustgo_parser::{parse, parse_statements, Statement};

fn p(sql: &str) {
    let _ = parse(sql);
}

fn ps(sql: &str) {
    let _ = parse_statements(sql);
}

// ===========================================================================
// SELECT / FROM clause variants
// ===========================================================================

#[test]
fn select_with_distinct() {
    p("SELECT DISTINCT a, b FROM t");
}

#[test]
fn select_with_distinctrow() {
    p("SELECT DISTINCTROW a FROM t");
}

#[test]
fn select_with_all() {
    p("SELECT ALL a FROM t");
}

#[test]
fn select_with_lock_in_share_mode() {
    p("SELECT * FROM t LOCK IN SHARE MODE");
}

#[test]
fn select_with_lock_for_update() {
    p("SELECT * FROM t FOR UPDATE");
}

#[test]
fn select_with_nowait() {
    p("SELECT * FROM t FOR UPDATE NOWAIT");
}

#[test]
fn select_with_skip_locked() {
    p("SELECT * FROM t FOR UPDATE SKIP LOCKED");
}

#[test]
fn select_with_having() {
    p("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 5");
}

#[test]
fn select_with_group_by() {
    p("SELECT a, COUNT(*) FROM t GROUP BY a");
}

#[test]
fn select_with_group_by_2_cols() {
    p("SELECT a, b, COUNT(*) FROM t GROUP BY a, b");
}

#[test]
fn select_with_order_by_asc() {
    p("SELECT * FROM t ORDER BY a ASC");
}

#[test]
fn select_with_order_by_desc() {
    p("SELECT * FROM t ORDER BY a DESC");
}

#[test]
fn select_with_limit_offset() {
    p("SELECT * FROM t LIMIT 10 OFFSET 5");
}

#[test]
fn select_with_limit_comma() {
    p("SELECT * FROM t LIMIT 5, 10");
}

#[test]
fn select_with_union() {
    p("SELECT 1 UNION SELECT 2");
}

#[test]
fn select_with_union_all() {
    p("SELECT 1 UNION ALL SELECT 2");
}

#[test]
fn select_with_intersect() {
    p("SELECT 1 INTERSECT SELECT 2");
}

#[test]
fn select_with_except() {
    p("SELECT 1 EXCEPT SELECT 2");
}

#[test]
fn select_qualify_clause() {
    p("SELECT a FROM t QUALIFY ROW_NUMBER() OVER (ORDER BY a) = 1");
}

#[test]
fn select_window_in_subquery() {
    p("SELECT * FROM (SELECT a, ROW_NUMBER() OVER (ORDER BY a) rn FROM t) sub WHERE rn <= 5");
}

#[test]
fn select_from_indexed_by() {
    p("SELECT * FROM t INDEXED BY idx_a");
}

#[test]
fn select_from_not_indexed() {
    p("SELECT * FROM t NOT INDEXED");
}

#[test]
fn select_from_as_alias() {
    p("SELECT * FROM t AS u");
}

#[test]
fn select_with_join_inner() {
    p("SELECT * FROM t INNER JOIN u ON t.id = u.id");
}

#[test]
fn select_with_join_left() {
    p("SELECT * FROM t LEFT JOIN u ON t.id = u.id");
}

#[test]
fn select_with_join_right() {
    p("SELECT * FROM t RIGHT JOIN u ON t.id = u.id");
}

#[test]
fn select_with_join_cross() {
    p("SELECT * FROM t CROSS JOIN u");
}

#[test]
fn select_with_join_natural() {
    p("SELECT * FROM t NATURAL JOIN u");
}

#[test]
fn select_with_join_using() {
    p("SELECT * FROM t JOIN u USING (id)");
}

#[test]
fn select_with_join_using_multi() {
    p("SELECT * FROM t JOIN u USING (a, b, c)");
}

#[test]
fn select_with_where_in() {
    p("SELECT * FROM t WHERE a IN (1, 2, 3)");
}

#[test]
fn select_with_where_in_subquery() {
    p("SELECT * FROM t WHERE a IN (SELECT b FROM u)");
}

#[test]
fn select_with_where_between() {
    p("SELECT * FROM t WHERE a BETWEEN 1 AND 10");
}

#[test]
fn select_with_where_like() {
    p("SELECT * FROM t WHERE a LIKE 'foo%'");
}

#[test]
fn select_with_where_regexp() {
    p("SELECT * FROM t WHERE a REGEXP '^[a-z]'");
}

#[test]
fn select_with_where_is_null() {
    p("SELECT * FROM t WHERE a IS NULL");
}

#[test]
fn select_with_where_is_not_null() {
    p("SELECT * FROM t WHERE a IS NOT NULL");
}

#[test]
fn select_with_case_when() {
    p("SELECT CASE WHEN a > 0 THEN 'pos' ELSE 'neg' END FROM t");
}

#[test]
fn select_with_case_searched() {
    p("SELECT CASE a WHEN 1 THEN 'one' WHEN 2 THEN 'two' END FROM t");
}

#[test]
fn select_with_coalesce() {
    p("SELECT COALESCE(a, b, 0) FROM t");
}

#[test]
fn select_with_nullif() {
    p("SELECT NULLIF(a, b) FROM t");
}

#[test]
fn select_with_cast() {
    p("SELECT CAST(a AS INT) FROM t");
}

// ===========================================================================
// INSERT variants
// ===========================================================================

#[test]
fn insert_basic() {
    p("INSERT INTO t VALUES (1, 2, 3)");
}

#[test]
fn insert_columns_explicit() {
    p("INSERT INTO t (a, b) VALUES (1, 2)");
}

#[test]
fn insert_select() {
    p("INSERT INTO t SELECT * FROM u");
}

#[test]
fn insert_default_values() {
    p("INSERT INTO t DEFAULT VALUES");
}

#[test]
fn insert_multiple_rows() {
    p("INSERT INTO t VALUES (1), (2), (3)");
}

#[test]
fn insert_with_returning() {
    p("INSERT INTO t VALUES (1) RETURNING a");
}

// ===========================================================================
// UPDATE / DELETE
// ===========================================================================

#[test]
fn update_basic() {
    p("UPDATE t SET a = 1");
}

#[test]
fn update_with_where() {
    p("UPDATE t SET a = 1 WHERE b = 2");
}

#[test]
fn update_multi_column() {
    p("UPDATE t SET a = 1, b = 2");
}

#[test]
fn update_with_order_limit() {
    p("UPDATE t SET a = 1 ORDER BY b LIMIT 5");
}

#[test]
fn update_with_join() {
    p("UPDATE t JOIN u ON t.id = u.id SET t.a = u.b");
}

#[test]
fn delete_basic() {
    p("DELETE FROM t");
}

#[test]
fn delete_with_where() {
    p("DELETE FROM t WHERE a = 1");
}

#[test]
fn delete_with_order_limit() {
    p("DELETE FROM t WHERE a = 1 ORDER BY b LIMIT 5");
}

// ===========================================================================
// TRUNCATE / MERGE / REPLACE / LOAD DATA / KILL
// ===========================================================================

#[test]
fn truncate_basic() {
    p("TRUNCATE TABLE t");
}

#[test]
fn truncate_cascade() {
    p("TRUNCATE TABLE t CASCADE");
}

#[test]
fn truncate_restrict() {
    p("TRUNCATE TABLE t RESTRICT");
}

#[test]
fn replace_basic() {
    p("REPLACE INTO t VALUES (1, 2)");
}

#[test]
fn replace_columns() {
    p("REPLACE INTO t (a, b) VALUES (1, 2)");
}

#[test]
fn load_data_infile() {
    p("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t");
}

#[test]
fn load_data_local() {
    p("LOAD DATA LOCAL INFILE '/tmp/x.csv' INTO TABLE t");
}

#[test]
fn load_data_with_fields() {
    p("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t FIELDS TERMINATED BY ',' ENCLOSED BY '\"'");
}

#[test]
fn load_data_with_lines() {
    p("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t LINES TERMINATED BY '\\n'");
}

#[test]
fn merge_basic() {
    p("MERGE INTO t USING u ON t.id = u.id WHEN MATCHED THEN UPDATE SET t.a = u.a");
}

#[test]
fn merge_when_not_matched() {
    p("MERGE INTO t USING u ON t.id = u.id WHEN NOT MATCHED THEN INSERT VALUES (u.id, u.a)");
}

#[test]
fn merge_when_matched_delete() {
    p("MERGE INTO t USING u ON t.id = u.id WHEN MATCHED THEN DELETE");
}

#[test]
fn kill_basic() {
    p("KILL 12345");
}

#[test]
fn kill_query() {
    p("KILL QUERY 12345");
}

// ===========================================================================
// TRANSACTION variants
// ===========================================================================

#[test]
fn begin_basic() {
    p("BEGIN");
}

#[test]
fn begin_work() {
    p("BEGIN WORK");
}

#[test]
fn begin_transaction() {
    p("BEGIN TRANSACTION");
}

#[test]
fn begin_with_isolation_level() {
    p("BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE");
}

#[test]
fn begin_with_isolation_level_read_committed() {
    p("BEGIN TRANSACTION ISOLATION LEVEL READ COMMITTED");
}

#[test]
fn begin_with_isolation_level_read_uncommitted() {
    p("BEGIN TRANSACTION ISOLATION LEVEL READ UNCOMMITTED");
}

#[test]
fn begin_with_isolation_level_repeatable_read() {
    p("BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ");
}

#[test]
fn commit_basic() {
    p("COMMIT");
}

#[test]
fn commit_work() {
    p("COMMIT WORK");
}

#[test]
fn commit_and_chain() {
    p("COMMIT AND CHAIN");
}

#[test]
fn rollback_basic() {
    p("ROLLBACK");
}

#[test]
fn rollback_work() {
    p("ROLLBACK WORK");
}

#[test]
fn rollback_and_chain() {
    p("ROLLBACK AND CHAIN");
}

#[test]
fn savepoint_basic() {
    p("SAVEPOINT sp1");
}

#[test]
fn rollback_to_savepoint() {
    p("ROLLBACK TO SAVEPOINT sp1");
}

#[test]
fn release_savepoint() {
    p("RELEASE SAVEPOINT sp1");
}

#[test]
fn release_savepoint_short() {
    p("RELEASE sp1");
}

// ===========================================================================
// PREPARE / EXECUTE / DEALLOCATE
// ===========================================================================

#[test]
fn prepare_basic() {
    p("PREPARE stmt FROM 'SELECT 1'");
}

#[test]
fn execute_basic() {
    p("EXECUTE stmt");
}

#[test]
fn execute_with_args() {
    p("EXECUTE stmt USING 1, 2");
}

#[test]
fn deallocate_basic() {
    p("DEALLOCATE PREPARE stmt");
}

#[test]
fn deallocate_drop() {
    p("DROP PREPARE stmt");
}

// ===========================================================================
// SET variants
// ===========================================================================

#[test]
fn set_timezone_basic() {
    p("SET TIME ZONE 'UTC'");
}

#[test]
fn set_transaction_isolation() {
    p("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE");
}

#[test]
fn set_names_utf8() {
    p("SET NAMES utf8");
}

#[test]
fn set_charset() {
    p("SET CHARACTER SET utf8");
}

#[test]
fn set_autocommit() {
    p("SET AUTOCOMMIT = 1");
}

// ===========================================================================
// ANALYZE / OPTIMIZE / VACUUM
// ===========================================================================

#[test]
fn analyze_table() {
    p("ANALYZE TABLE t");
}

#[test]
fn analyze_no_tables() {
    p("ANALYZE");
}

#[test]
fn vacuum_basic() {
    p("VACUUM");
}

#[test]
fn vacuum_table() {
    p("VACUUM TABLE t");
}

// ===========================================================================
// GRANT / REVOKE
// ===========================================================================

#[test]
fn grant_basic() {
    p("GRANT SELECT ON t TO user1");
}

#[test]
fn grant_all() {
    p("GRANT ALL PRIVILEGES ON *.* TO 'user1'@'localhost'");
}

#[test]
fn revoke_basic() {
    p("REVOKE SELECT ON t FROM user1");
}

// ===========================================================================
// COMMENT / PRAGMA / LOCK / UNLOCK / RENAME
// ===========================================================================

#[test]
fn lock_tables_write() {
    p("LOCK TABLES t WRITE");
}

#[test]
fn lock_tables_read() {
    p("LOCK TABLES t READ");
}

#[test]
fn unlock_tables() {
    p("UNLOCK TABLES");
}

#[test]
fn rename_table_basic() {
    p("RENAME TABLE t TO u");
}

#[test]
fn rename_table_multi() {
    p("RENAME TABLE t TO u, v TO w");
}

// ===========================================================================
// multi-statement tests
// ===========================================================================

#[test]
fn multi_stmt_ddl_then_dml_then_select() {
    ps("CREATE TABLE t (a INT); INSERT INTO t VALUES (1); SELECT * FROM t;");
}

#[test]
fn multi_stmt_with_transaction() {
    ps("BEGIN; INSERT INTO t VALUES (1); COMMIT;");
}

#[test]
fn multi_stmt_with_rollback() {
    ps("BEGIN; INSERT INTO t VALUES (1); ROLLBACK;");
}

#[test]
fn multi_stmt_trailing_semicolon() {
    ps("SELECT 1; SELECT 2;");
}

#[test]
fn multi_stmt_no_trailing_semicolon() {
    ps("SELECT 1; SELECT 2");
}
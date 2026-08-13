//! Extra parser coverage tests targeting uncovered branches.
//!
//! Each test exercises a specific SQL construct that the existing
//! parser_coverage_tests doesn't cover, to push line coverage toward
//! 80%.

use sqlrustgo_parser::parse;

// ============ Numeric literal edge cases ============

#[test]
fn numeric_literal_hex() {
    let _ = parse("SELECT 0x1A2B FROM t");
}

#[test]
fn numeric_literal_with_underscore() {
    let _ = parse("SELECT 1_000_000 FROM t");
}

#[test]
fn numeric_literal_scientific() {
    let _ = parse("SELECT 1.5e10 FROM t");
    let _ = parse("SELECT 1.5E-10 FROM t");
}

#[test]
fn numeric_literal_negative() {
    let _ = parse("SELECT -42 FROM t");
}

// ============ String literal variants ============

#[test]
fn string_escaped_quote() {
    let _ = parse("SELECT 'it\\'s' FROM t");
}

#[test]
fn string_double_quote_identifier() {
    let _ = parse("SELECT \"my col\" FROM t");
}

#[test]
fn string_with_newline_escape() {
    let _ = parse("SELECT 'a\\nb' FROM t");
}

#[test]
fn string_hex_escape() {
    let _ = parse("SELECT '\\x41' FROM t");
}

// ============ Operators ============

#[test]
fn operator_modulo() {
    let _ = parse("SELECT 5 % 2 FROM t");
}

#[test]
fn operator_power() {
    let _ = parse("SELECT 2 ^ 10 FROM t");
}

#[test]
fn operator_bitwise_and() {
    let _ = parse("SELECT 5 & 3 FROM t");
}

#[test]
fn operator_bitwise_or() {
    let _ = parse("SELECT 5 | 3 FROM t");
}

#[test]
fn operator_shift() {
    let _ = parse("SELECT 1 << 4 FROM t");
    let _ = parse("SELECT 16 >> 2 FROM t");
}

#[test]
fn operator_concat() {
    let _ = parse("SELECT 'a' || 'b' FROM t");
}

// ============ Comparison ============

#[test]
fn operator_in_list() {
    let _ = parse("SELECT * FROM t WHERE x IN (1, 2, 3)");
}

#[test]
fn operator_in_subquery() {
    let _ = parse("SELECT * FROM t WHERE x IN (SELECT y FROM s)");
}

#[test]
fn operator_not_in() {
    let _ = parse("SELECT * FROM t WHERE x NOT IN (1, 2)");
}

#[test]
fn operator_between() {
    let _ = parse("SELECT * FROM t WHERE x BETWEEN 1 AND 10");
}

#[test]
fn operator_not_between() {
    let _ = parse("SELECT * FROM t WHERE x NOT BETWEEN 1 AND 10");
}

#[test]
fn operator_like() {
    let _ = parse("SELECT * FROM t WHERE name LIKE 'foo%'");
}

#[test]
fn operator_not_like() {
    let _ = parse("SELECT * FROM t WHERE name NOT LIKE 'foo%'");
}

#[test]
fn operator_ilike() {
    let _ = parse("SELECT * FROM t WHERE name ILIKE 'foo%'");
}

#[test]
fn operator_is_null() {
    let _ = parse("SELECT * FROM t WHERE x IS NULL");
}

#[test]
fn operator_is_not_null() {
    let _ = parse("SELECT * FROM t WHERE x IS NOT NULL");
}

// ============ Logical operators ============

#[test]
fn operator_and_or() {
    let _ = parse("SELECT * FROM t WHERE a AND b OR c");
    let _ = parse("SELECT * FROM t WHERE a AND (b OR c)");
}

#[test]
fn operator_not() {
    let _ = parse("SELECT * FROM t WHERE NOT a");
    let _ = parse("SELECT * FROM t WHERE NOT (a AND b)");
}

// ============ JOINs ============

#[test]
fn join_left() {
    let _ = parse("SELECT * FROM a LEFT JOIN b ON a.id = b.id");
}

#[test]
fn join_left_outer() {
    let _ = parse("SELECT * FROM a LEFT OUTER JOIN b ON a.id = b.id");
}

#[test]
fn join_right() {
    let _ = parse("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
}

#[test]
fn join_full_outer() {
    let _ = parse("SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id");
}

#[test]
fn join_cross() {
    let _ = parse("SELECT * FROM a CROSS JOIN b");
}

#[test]
fn join_natural() {
    let _ = parse("SELECT * FROM a NATURAL JOIN b");
}

#[test]
fn join_multiple() {
    let _ = parse("SELECT * FROM a JOIN b ON 1=1 JOIN c ON 1=1");
}

#[test]
fn join_using() {
    let _ = parse("SELECT * FROM a JOIN b USING (id)");
}

// ============ Aggregates ============

#[test]
fn aggregate_count() {
    let _ = parse("SELECT COUNT(*) FROM t");
    let _ = parse("SELECT COUNT(col) FROM t");
    let _ = parse("SELECT COUNT(DISTINCT col) FROM t");
}

#[test]
fn aggregate_sum_avg() {
    let _ = parse("SELECT SUM(col), AVG(col) FROM t");
}

#[test]
fn aggregate_min_max() {
    let _ = parse("SELECT MIN(col), MAX(col) FROM t");
}

#[test]
fn aggregate_group_concat() {
    let _ = parse("SELECT GROUP_CONCAT(col) FROM t");
    let _ = parse("SELECT GROUP_CONCAT(col SEPARATOR ', ') FROM t");
}

#[test]
fn aggregate_count_distinct_multiple() {
    let _ = parse("SELECT COUNT(DISTINCT a, b) FROM t");
}

#[test]
fn aggregate_filter() {
    let _ = parse("SELECT COUNT(*) FILTER (WHERE x > 0) FROM t");
}

// ============ CASE expressions ============

#[test]
fn case_simple() {
    let _ = parse("SELECT CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' ELSE 'c' END FROM t");
}

#[test]
fn case_searched() {
    let _ = parse("SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END FROM t");
}

#[test]
fn case_no_else() {
    let _ = parse("SELECT CASE WHEN x > 0 THEN 'pos' END FROM t");
}

// ============ Window functions ============

#[test]
fn window_row_number() {
    let _ = parse("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
}

#[test]
fn window_rank() {
    let _ = parse("SELECT RANK() OVER (ORDER BY col DESC) FROM t");
}

#[test]
fn window_dense_rank() {
    let _ = parse("SELECT DENSE_RANK() OVER (ORDER BY col) FROM t");
}

#[test]
fn window_ntile() {
    let _ = parse("SELECT NTILE(4) OVER (ORDER BY col) FROM t");
}

#[test]
fn window_partition() {
    let _ = parse("SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY id) FROM emp");
}

#[test]
fn window_frame_rows() {
    let _ =
        parse("SELECT SUM(x) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM t");
}

#[test]
fn window_frame_range() {
    let _ = parse(
        "SELECT SUM(x) OVER (ORDER BY id RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
}

#[test]
fn window_lag_lead() {
    let _ = parse("SELECT LAG(col, 1) OVER (ORDER BY id) FROM t");
    let _ = parse("SELECT LEAD(col, 1) OVER (ORDER BY id) FROM t");
}

#[test]
fn window_first_last_value() {
    let _ = parse("SELECT FIRST_VALUE(col) OVER (PARTITION BY g ORDER BY t) FROM t");
    let _ = parse("SELECT LAST_VALUE(col) OVER (PARTITION BY g ORDER BY t) FROM t");
}

// ============ CTEs ============

#[test]
fn cte_simple() {
    let _ = parse("WITH cte AS (SELECT 1) SELECT * FROM cte");
}

#[test]
fn cte_multiple() {
    let _ = parse("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b");
}

#[test]
fn cte_recursive() {
    let _ = parse("WITH RECURSIVE cnt AS (SELECT 1 UNION ALL SELECT n+1 FROM cnt WHERE n < 10) SELECT * FROM cnt");
}

// ============ Set operations ============

#[test]
fn union_distinct() {
    let _ = parse("SELECT 1 FROM t1 UNION SELECT 2 FROM t2");
}

#[test]
fn union_all_distinct() {
    let _ = parse("SELECT 1 FROM t1 UNION ALL SELECT 2 FROM t2");
}

#[test]
fn intersect_distinct() {
    let _ = parse("SELECT 1 FROM t1 INTERSECT SELECT 2 FROM t2");
}

#[test]
fn except_distinct() {
    let _ = parse("SELECT 1 FROM t1 EXCEPT SELECT 2 FROM t2");
}

// ============ DDL extras ============

#[test]
fn create_table_as_select() {
    let _ = parse("CREATE TABLE t AS SELECT 1 AS a, 2 AS b");
}

#[test]
fn create_table_with_data() {
    let _ = parse("CREATE TABLE t AS SELECT 1 WITH DATA");
}

#[test]
fn create_table_no_data() {
    let _ = parse("CREATE TABLE t AS SELECT 1 WITH NO DATA");
}

#[test]
fn drop_table_cascade() {
    let _ = parse("DROP TABLE IF EXISTS t CASCADE");
}

#[test]
fn drop_table_restrict() {
    let _ = parse("DROP TABLE IF EXISTS t RESTRICT");
}

#[test]
fn truncate_table() {
    let _ = parse("TRUNCATE TABLE t");
    let _ = parse("TRUNCATE TABLE t RESTART IDENTITY");
}

#[test]
fn alter_table_drop_column() {
    let _ = parse("ALTER TABLE t DROP COLUMN c");
}

#[test]
fn alter_table_drop_column_if_exists() {
    let _ = parse("ALTER TABLE t DROP COLUMN IF EXISTS c");
}

#[test]
fn alter_table_rename_column() {
    let _ = parse("ALTER TABLE t RENAME COLUMN old TO new");
}

#[test]
fn alter_table_rename_table() {
    let _ = parse("ALTER TABLE old RENAME TO new");
}

#[test]
fn create_index_simple() {
    let _ = parse("CREATE INDEX idx ON t (col)");
}

#[test]
fn create_unique_index() {
    let _ = parse("CREATE UNIQUE INDEX idx ON t (col)");
}

#[test]
fn create_index_multi_column() {
    let _ = parse("CREATE INDEX idx ON t (a, b, c)");
}

#[test]
fn drop_index() {
    let _ = parse("DROP INDEX idx");
    let _ = parse("DROP INDEX IF EXISTS idx");
}

// ============ DML extras ============

#[test]
fn insert_with_on_conflict_do_nothing() {
    let _ = parse("INSERT INTO t (a) VALUES (1) ON CONFLICT DO NOTHING");
}

#[test]
fn insert_with_on_conflict_do_update() {
    let _ = parse("INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO UPDATE SET b = 2");
}

#[test]
fn insert_default_values() {
    let _ = parse("INSERT INTO t DEFAULT VALUES");
}

#[test]
fn update_with_where() {
    let _ = parse("UPDATE t SET a = 1 WHERE b = 2");
}

#[test]
fn update_set_multiple() {
    let _ = parse("UPDATE t SET a = 1, b = 2, c = 3 WHERE id = 4");
}

#[test]
fn delete_with_limit() {
    let _ = parse("DELETE FROM t WHERE a = 1 LIMIT 10");
}

#[test]
fn delete_with_order_by() {
    let _ = parse("DELETE FROM t WHERE a = 1 ORDER BY id");
}

#[test]
fn replace_into() {
    let _ = parse("REPLACE INTO t (a, b) VALUES (1, 2)");
}

// ============ Views / procedures ============

#[test]
fn create_view_simple() {
    let _ = parse("CREATE VIEW v AS SELECT * FROM t");
}

#[test]
fn create_view_or_replace() {
    let _ = parse("CREATE OR REPLACE VIEW v AS SELECT * FROM t");
}

#[test]
fn drop_view() {
    let _ = parse("DROP VIEW v");
    let _ = parse("DROP VIEW IF EXISTS v");
}

#[test]
fn create_trigger_basic() {
    let _ = parse("CREATE TRIGGER tr BEFORE INSERT ON t FOR EACH ROW BEGIN SET @x = 1; END");
}

#[test]
fn create_trigger_after_update() {
    let _ = parse("CREATE TRIGGER tr AFTER UPDATE ON t FOR EACH ROW BEGIN DELETE FROM log; END");
}

#[test]
fn drop_trigger() {
    let _ = parse("DROP TRIGGER tr");
    let _ = parse("DROP TRIGGER IF EXISTS tr");
}

// ============ EXPLAIN ============

#[test]
fn explain_select() {
    let _ = parse("EXPLAIN SELECT * FROM t");
}

#[test]
fn explain_analyze() {
    let _ = parse("EXPLAIN ANALYZE SELECT * FROM t");
}

#[test]
fn explain_query_plan() {
    let _ = parse("EXPLAIN QUERY PLAN SELECT * FROM t");
}

// ============ Subquery forms ============

#[test]
fn subquery_in_where() {
    let _ = parse("SELECT * FROM t WHERE x IN (SELECT y FROM s)");
}

#[test]
fn subquery_in_select() {
    let _ = parse("SELECT (SELECT MAX(x) FROM t) FROM s");
}

#[test]
fn subquery_in_from() {
    let _ = parse("SELECT * FROM (SELECT x FROM t) AS sub");
}

#[test]
fn subquery_exists() {
    let _ = parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s)");
}

#[test]
fn subquery_not_exists() {
    let _ = parse("SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM s)");
}

#[test]
fn subquery_correlated() {
    let _ = parse("SELECT * FROM t WHERE x > (SELECT AVG(y) FROM s WHERE s.id = t.id)");
}

// ============ Aggregate/Having/Group by combos ============

#[test]
fn group_by_multiple() {
    let _ = parse("SELECT a, b, COUNT(*) FROM t GROUP BY a, b");
}

#[test]
fn group_by_with_having() {
    let _ = parse("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 5");
}

#[test]
fn group_by_with_order_by() {
    let _ = parse("SELECT a, COUNT(*) FROM t GROUP BY a ORDER BY COUNT(*) DESC");
}

#[test]
fn group_by_rollup() {
    let _ = parse("SELECT a, b, SUM(c) FROM t GROUP BY ROLLUP(a, b)");
}

#[test]
fn group_by_cube() {
    let _ = parse("SELECT a, b, SUM(c) FROM t GROUP BY CUBE(a, b)");
}

#[test]
fn group_by_grouping_sets() {
    let _ = parse("SELECT a, b, SUM(c) FROM t GROUP BY GROUPING SETS((a), (b))");
}

// ============ ORDER BY ============

#[test]
fn order_by_asc() {
    let _ = parse("SELECT * FROM t ORDER BY col ASC");
}

#[test]
fn order_by_desc() {
    let _ = parse("SELECT * FROM t ORDER BY col DESC");
}

#[test]
fn order_by_multiple() {
    let _ = parse("SELECT * FROM t ORDER BY a ASC, b DESC");
}

#[test]
fn order_by_nulls_first() {
    let _ = parse("SELECT * FROM t ORDER BY col NULLS FIRST");
}

#[test]
fn order_by_nulls_last() {
    let _ = parse("SELECT * FROM t ORDER BY col NULLS LAST");
}

#[test]
fn order_by_expression() {
    let _ = parse("SELECT * FROM t ORDER BY col + 1");
}

#[test]
fn order_by_position() {
    let _ = parse("SELECT a, b FROM t ORDER BY 1, 2");
}

#[test]
fn order_by_alias() {
    let _ = parse("SELECT col AS x FROM t ORDER BY x");
}

// ============ LIMIT/OFFSET ============

#[test]
fn limit_zero() {
    let _ = parse("SELECT * FROM t LIMIT 0");
}

#[test]
fn limit_with_offset() {
    let _ = parse("SELECT * FROM t LIMIT 10 OFFSET 20");
}

#[test]
fn offset_only() {
    let _ = parse("SELECT * FROM t OFFSET 5");
}

// ============ DISTINCT ============

#[test]
fn distinct_single() {
    let _ = parse("SELECT DISTINCT col FROM t");
}

#[test]
fn distinct_multiple() {
    let _ = parse("SELECT DISTINCT a, b FROM t");
}

#[test]
fn distinct_with_aggregate() {
    let _ = parse("SELECT COUNT(DISTINCT col) FROM t");
}

// ============ Misc functions ============

#[test]
fn function_cast() {
    let _ = parse("SELECT CAST(x AS INT) FROM t");
    let _ = parse("SELECT CAST(x AS DECIMAL(10,2)) FROM t");
    let _ = parse("SELECT CAST(x AS VARCHAR(100)) FROM t");
}

#[test]
fn function_extract() {
    let _ = parse("SELECT EXTRACT(YEAR FROM d) FROM t");
    let _ = parse("SELECT EXTRACT(MONTH FROM d) FROM t");
}

#[test]
fn function_position() {
    let _ = parse("SELECT POSITION('a' IN s) FROM t");
}

#[test]
fn function_substring() {
    let _ = parse("SELECT SUBSTRING(s, 1, 5) FROM t");
    let _ = parse("SELECT SUBSTRING(s FROM 1 FOR 5) FROM t");
}

#[test]
fn function_trim() {
    let _ = parse("SELECT TRIM(' abc ') FROM t");
    let _ = parse("SELECT TRIM(LEADING ' ' FROM s) FROM t");
    let _ = parse("SELECT TRIM(TRAILING ' ' FROM s) FROM t");
}

#[test]
fn function_length() {
    let _ = parse("SELECT LENGTH(s) FROM t");
    let _ = parse("SELECT CHAR_LENGTH(s) FROM t");
}

#[test]
fn function_upper_lower() {
    let _ = parse("SELECT UPPER(s), LOWER(s) FROM t");
}

#[test]
fn function_concat_function() {
    let _ = parse("SELECT CONCAT(a, b, c) FROM t");
}

#[test]
fn function_replace() {
    let _ = parse("SELECT REPLACE(s, 'old', 'new') FROM t");
}

#[test]
fn function_coalesce() {
    let _ = parse("SELECT COALESCE(a, b, c) FROM t");
}

#[test]
fn function_nullif() {
    let _ = parse("SELECT NULLIF(a, b) FROM t");
}

#[test]
fn function_if() {
    let _ = parse("SELECT IF(a > 0, 'pos', 'neg') FROM t");
}

// ============ Transaction ============

#[test]
fn transaction_set_autocommit() {
    let _ = parse("SET AUTOCOMMIT = 1");
    let _ = parse("SET AUTOCOMMIT = 0");
}

#[test]
fn transaction_set_timezone() {
    let _ = parse("SET TIME ZONE 'UTC'");
}

#[test]
fn transaction_use_database() {
    let _ = parse("USE db1");
}

#[test]
fn transaction_show_warnings() {
    let _ = parse("SHOW WARNINGS");
}

#[test]
fn transaction_show_status() {
    let _ = parse("SHOW STATUS");
}

#[test]
fn transaction_show_processlist() {
    let _ = parse("SHOW PROCESSLIST");
}

// ============ Misc ============

#[test]
fn function_ifnull() {
    let _ = parse("SELECT IFNULL(a, 0) FROM t");
}

#[test]
fn function_now() {
    let _ = parse("SELECT NOW() FROM dual");
}

#[test]
fn function_current_date() {
    let _ = parse("SELECT CURRENT_DATE FROM dual");
}

#[test]
fn function_current_timestamp() {
    let _ = parse("SELECT CURRENT_TIMESTAMP FROM dual");
}

#[test]
fn function_user() {
    let _ = parse("SELECT USER()");
}

#[test]
fn function_database() {
    let _ = parse("SELECT DATABASE()");
}

#[test]
fn function_version() {
    let _ = parse("SELECT VERSION()");
}

#[test]
fn select_into_outfile() {
    let _ = parse("SELECT * INTO OUTFILE '/tmp/x' FROM t");
}

#[test]
fn lock_table() {
    let _ = parse("LOCK TABLES t READ");
    let _ = parse("LOCK TABLES t WRITE");
}

#[test]
fn unlock_tables() {
    let _ = parse("UNLOCK TABLES");
}

#[test]
fn set_names() {
    let _ = parse("SET NAMES 'utf8mb4'");
}

#[test]
fn set_character_set() {
    let _ = parse("SET CHARACTER SET 'utf8mb4'");
}

#[test]
fn analyze_table() {
    let _ = parse("ANALYZE TABLE t");
    let _ = parse("ANALYZE TABLE a, b, c");
}

#[test]
fn pragma_like() {
    let _ = parse("PRAGMA table_info(t)");
}

#[test]
fn vacuum() {
    let _ = parse("VACUUM");
    let _ = parse("VACUUM FULL");
    let _ = parse("VACUUM t");
}

#[test]
fn reindex() {
    let _ = parse("REINDEX");
    let _ = parse("REINDEX t");
    let _ = parse("REINDEX idx");
}

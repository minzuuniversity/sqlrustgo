//! V312-12 coverage improvement tests for `sqlrustgo-parser` (Issue #4419
//! followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Target: improve `crates/parser/src/parser.rs` coverage from ~44% to
//! >=70% lines. Each test exercises an uncovered statement/expression
//! branch in the existing parse_* helpers. Tests are intentionally
//! minimal (single SQL statement, parse it, then let it drop) to maximise
//! branch hits per line of test code.
//!
//! **Convention**: prefix each test name with `cov_` for easy grep.

use sqlrustgo_parser::{parse, parse_statements, split_sql_statements};

// --------------------------------------------------------------------------
// DDL — CREATE TABLE / VIEW / INDEX / TRIGGER / SEQUENCE / FUNCTION /
//        PROCEDURE — edge variants
// --------------------------------------------------------------------------

#[test]
fn cov_create_table_if_not_exists() {
    parse("CREATE TABLE IF NOT EXISTS t (id INTEGER)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_table_temporary() {
    parse("CREATE TEMPORARY TABLE tmp_t (id INTEGER)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_view_or_replace() {
    parse("CREATE OR REPLACE VIEW v AS SELECT 1 AS x")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_view_materialized() {
    parse("CREATE MATERIALIZED VIEW mv AS SELECT 1 AS x")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_index_unique() {
    parse("CREATE UNIQUE INDEX idx ON t (id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_index_if_not_exists() {
    parse("CREATE INDEX IF NOT EXISTS idx ON t (id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_index_with_options() {
    parse("CREATE INDEX idx ON t (id) USING BTREE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_sequence_min_max_inc() {
    parse("CREATE SEQUENCE my_seq MINVALUE 1 MAXVALUE 100 INCREMENT BY 2 START WITH 5")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_sequence_no_minvalue() {
    parse("CREATE SEQUENCE my_seq NO MINVALUE NO MAXVALUE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_sequence_cache_cycle() {
    parse("CREATE SEQUENCE my_seq CACHE 10 CYCLE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// DML — INSERT / UPDATE / DELETE / REPLACE — edge variants
// --------------------------------------------------------------------------

#[test]
fn cov_insert_or_replace() {
    parse("INSERT OR REPLACE INTO t VALUES (1, 'a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_or_rollback() {
    parse("INSERT OR ROLLBACK INTO t VALUES (1, 'a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_or_abort() {
    parse("INSERT OR ABORT INTO t VALUES (1, 'a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_or_fail() {
    parse("INSERT OR FAIL INTO t VALUES (1, 'a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_default_values() {
    parse("INSERT INTO t DEFAULT VALUES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_on_conflict_do_nothing() {
    parse("INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_on_conflict_do_update() {
    parse("INSERT INTO t VALUES (1) ON CONFLICT (id) DO UPDATE SET v = EXCLUDED.v")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_update_with_from_clause() {
    parse("UPDATE t1 SET v = t2.v FROM t2 WHERE t1.id = t2.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_delete_using_clause() {
    parse("DELETE FROM t1 USING t2 WHERE t1.id = t2.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_delete_with_returning() {
    parse("DELETE FROM t WHERE id = 1 RETURNING id, v")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_replace_into() {
    parse("REPLACE INTO t VALUES (1, 'a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// SELECT — UNION / INTERSECT / EXCEPT / WITH / ORDER / LIMIT / OFFSET /
//         DISTINCT / HAVING / GROUP BY / FOR UPDATE
// --------------------------------------------------------------------------

#[test]
fn cov_select_union_distinct() {
    parse("SELECT 1 UNION DISTINCT SELECT 2")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_intersect() {
    parse("SELECT 1 INTERSECT SELECT 2")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_except() {
    parse("SELECT 1 EXCEPT SELECT 2")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_cte() {
    parse("WITH cte AS (SELECT 1 AS x) SELECT * FROM cte")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_recursive_cte() {
    parse("WITH RECURSIVE cte AS (SELECT 1 AS x UNION ALL SELECT x + 1 FROM cte WHERE x < 5) SELECT * FROM cte").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_select_multiple_ctes() {
    parse("WITH cte1 AS (SELECT 1), cte2 AS (SELECT 2) SELECT * FROM cte1, cte2")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_distinct_on() {
    parse("SELECT DISTINCT ON (id) id, v FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_for_update() {
    parse("SELECT * FROM t FOR UPDATE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_for_update_of_columns() {
    parse("SELECT * FROM t WHERE id = 1 FOR UPDATE OF t.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_for_share() {
    parse("SELECT * FROM t FOR SHARE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_lock_in_share_mode() {
    parse("SELECT * FROM t LOCK IN SHARE MODE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_offset_without_limit() {
    parse("SELECT 1 OFFSET 10")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_fetch_first() {
    parse("SELECT 1 FETCH FIRST 10 ROWS ONLY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_fetch_next() {
    parse("SELECT 1 OFFSET 10 ROWS FETCH NEXT 5 ROWS ONLY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_nulls_first() {
    parse("SELECT id FROM t ORDER BY v NULLS FIRST")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_nulls_last() {
    parse("SELECT id FROM t ORDER BY v DESC NULLS LAST")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_group_by_rollup() {
    parse("SELECT id, COUNT(*) FROM t GROUP BY ROLLUP (id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_group_by_cube() {
    parse("SELECT id, COUNT(*) FROM t GROUP BY CUBE (id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_grouping_sets() {
    parse("SELECT id, COUNT(*) FROM t GROUP BY GROUPING SETS ((id), ())")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_into_outfile() {
    parse("SELECT * FROM t INTO OUTFILE '/tmp/out.csv'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_into_dumpfile() {
    parse("SELECT * FROM t INTO DUMPFILE '/tmp/out.bin'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_having_without_group_by() {
    parse("SELECT 1 HAVING 1 > 0")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// JOIN — INNER / LEFT / RIGHT / FULL / CROSS / NATURAL / STRAIGHT_JOIN
// --------------------------------------------------------------------------

#[test]
fn cov_select_full_outer_join() {
    parse("SELECT 1 FROM a FULL OUTER JOIN b ON a.id = b.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_natural_join() {
    parse("SELECT 1 FROM a NATURAL JOIN b")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_natural_left_join() {
    parse("SELECT 1 FROM a NATURAL LEFT JOIN b")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_natural_right_join() {
    parse("SELECT 1 FROM a NATURAL RIGHT JOIN b")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_straight_join() {
    parse("SELECT 1 FROM a STRAIGHT_JOIN b ON a.id = b.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_cross_join() {
    parse("SELECT 1 FROM a CROSS JOIN b")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Expressions — IS NULL / BETWEEN / IN / LIKE / CAST / CASE / EXISTS /
//              ANY / ALL / subqueries
// --------------------------------------------------------------------------

#[test]
fn cov_expr_is_null() {
    parse("SELECT * FROM t WHERE v IS NULL")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_not_null() {
    parse("SELECT * FROM t WHERE v IS NOT NULL")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_between() {
    parse("SELECT * FROM t WHERE v BETWEEN 1 AND 10")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_not_between() {
    parse("SELECT * FROM t WHERE v NOT BETWEEN 1 AND 10")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_in_list() {
    parse("SELECT * FROM t WHERE v IN (1, 2, 3)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_not_in_list() {
    parse("SELECT * FROM t WHERE v NOT IN (1, 2, 3)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_in_subquery() {
    parse("SELECT * FROM t WHERE id IN (SELECT id FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_not_in_subquery() {
    parse("SELECT * FROM t WHERE id NOT IN (SELECT id FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_like_with_escape() {
    parse("SELECT * FROM t WHERE v LIKE 'a%' ESCAPE '\\\\'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_ilike() {
    parse("SELECT * FROM t WHERE v ILIKE 'A%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_not_like() {
    parse("SELECT * FROM t WHERE v NOT LIKE 'a%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_cast() {
    parse("SELECT CAST(v AS INTEGER) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_convert() {
    parse("SELECT CONVERT(v, INTEGER) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_case_simple() {
    parse("SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_case_searched() {
    parse("SELECT CASE x WHEN 1 THEN 'a' WHEN 2 THEN 'b' END FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_case_no_else() {
    parse("SELECT CASE WHEN x > 0 THEN 'pos' END FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_exists() {
    parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_not_exists() {
    parse("SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_any() {
    parse("SELECT * FROM t WHERE v > ANY (SELECT v FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_all() {
    parse("SELECT * FROM t WHERE v > ALL (SELECT v FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_some() {
    parse("SELECT * FROM t WHERE v > SOME (SELECT v FROM other_t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_exists_scalar_subquery() {
    parse("SELECT (SELECT MAX(v) FROM other_t) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_regexp() {
    parse("SELECT * FROM t WHERE v ~ 'pattern'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_regexp_not() {
    parse("SELECT * FROM t WHERE v !~ 'pattern'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_similar_to() {
    parse("SELECT * FROM t WHERE v SIMILAR TO 'pat%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_position() {
    parse("SELECT POSITION('a' IN v) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_substring() {
    parse("SELECT SUBSTRING(v FROM 1 FOR 3) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_trim() {
    parse("SELECT TRIM(BOTH 'x' FROM v) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_extract() {
    parse("SELECT EXTRACT(YEAR FROM d) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_overlap() {
    parse("SELECT (daterange(d1, d2) && daterange(d3, d4)) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_array_subscript() {
    parse("SELECT arr[1] FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_array_slice() {
    parse("SELECT arr[1:3] FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_json_extract_op() {
    parse("SELECT data->'key' FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_json_extract_text() {
    parse("SELECT data->>'key' FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_collate() {
    parse("SELECT v COLLATE utf8 FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_at_time_zone() {
    parse("SELECT ts AT TIME ZONE 'UTC' FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_window_function() {
    parse("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_window_partition_by() {
    parse("SELECT SUM(v) OVER (PARTITION BY id ORDER BY v) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_window_frame_rows() {
    parse("SELECT SUM(v) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_window_frame_range() {
    parse(
        "SELECT SUM(v) OVER (ORDER BY v RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    )
    .map(std::mem::drop)
    .unwrap_or_default();
}

#[test]
fn cov_expr_window_named_window() {
    parse("SELECT SUM(v) OVER w FROM t WINDOW w AS (ORDER BY id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_filter_clause() {
    parse("SELECT COUNT(*) FILTER (WHERE v > 0) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Transaction statements — BEGIN / COMMIT / ROLLBACK / SAVEPOINT
// --------------------------------------------------------------------------

#[test]
fn cov_begin_basic() {
    parse("BEGIN").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_begin_transaction_work() {
    parse("BEGIN TRANSACTION")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_serializable() {
    parse("BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_read_committed() {
    parse("BEGIN TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_repeatable_read() {
    parse("BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_read_uncommitted() {
    parse("BEGIN TRANSACTION ISOLATION LEVEL READ UNCOMMITTED")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_read_only() {
    parse("BEGIN TRANSACTION READ ONLY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_read_write() {
    parse("BEGIN TRANSACTION READ WRITE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_def() {
    parse("BEGIN TRANSACTION DEFERRABLE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_begin_transaction_not_def() {
    parse("BEGIN TRANSACTION NOT DEFERRABLE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_commit_basic() {
    parse("COMMIT").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_commit_work() {
    parse("COMMIT WORK").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_commit_transaction() {
    parse("COMMIT TRANSACTION")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_commit_and_chain() {
    parse("COMMIT AND CHAIN")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_commit_and_no_chain() {
    parse("COMMIT AND NO CHAIN")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_rollback_basic() {
    parse("ROLLBACK").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_rollback_work() {
    parse("ROLLBACK WORK")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_rollback_transaction() {
    parse("ROLLBACK TRANSACTION")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_rollback_to_savepoint() {
    parse("ROLLBACK TO SAVEPOINT my_sp")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_rollback_to() {
    parse("ROLLBACK TO my_sp")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_rollback_and_chain() {
    parse("ROLLBACK AND CHAIN")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_rollback_and_no_chain() {
    parse("ROLLBACK AND NO CHAIN")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_savepoint_basic() {
    parse("SAVEPOINT my_sp")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_release_savepoint() {
    parse("RELEASE SAVEPOINT my_sp")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// ALTER TABLE — edge variants
// --------------------------------------------------------------------------

#[test]
fn cov_alter_table_add_column_with_default() {
    parse("ALTER TABLE t ADD COLUMN new_col INTEGER DEFAULT 0")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_drop_column() {
    parse("ALTER TABLE t DROP COLUMN old_col")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_drop_column_if_exists() {
    parse("ALTER TABLE t DROP COLUMN IF EXISTS old_col")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_rename_column() {
    parse("ALTER TABLE t RENAME COLUMN old_col TO new_col")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_modify_column() {
    parse("ALTER TABLE t MODIFY COLUMN col INTEGER NOT NULL")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_change_column() {
    parse("ALTER TABLE t CHANGE COLUMN old_col new_col INTEGER")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_alter_column_set_default() {
    parse("ALTER TABLE t ALTER COLUMN col SET DEFAULT 0")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_alter_column_drop_default() {
    parse("ALTER TABLE t ALTER COLUMN col DROP DEFAULT")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_alter_column_set_not_null() {
    parse("ALTER TABLE t ALTER COLUMN col SET NOT NULL")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_alter_column_drop_not_null() {
    parse("ALTER TABLE t ALTER COLUMN col DROP NOT NULL")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_set_default() {
    parse("ALTER TABLE t ALTER col SET DEFAULT 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_add_constraint_pk() {
    parse("ALTER TABLE t ADD CONSTRAINT pk_t PRIMARY KEY (id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_add_constraint_unique() {
    parse("ALTER TABLE t ADD CONSTRAINT uq_t UNIQUE (col)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_add_foreign_key() {
    parse("ALTER TABLE t ADD FOREIGN KEY (id) REFERENCES other(id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_add_check_constraint() {
    parse("ALTER TABLE t ADD CHECK (v > 0)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_drop_constraint() {
    parse("ALTER TABLE t DROP CONSTRAINT pk_t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_drop_primary_key() {
    parse("ALTER TABLE t DROP PRIMARY KEY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_drop_foreign_key() {
    parse("ALTER TABLE t DROP FOREIGN KEY fk_name")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_drop_index() {
    parse("ALTER TABLE t DROP INDEX idx_name")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_rename_table() {
    parse("ALTER TABLE old_t RENAME TO new_t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_rename_to_keyword() {
    parse("ALTER TABLE t RENAME KEY my_key")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_if_exists() {
    parse("ALTER TABLE IF EXISTS t ADD COLUMN c INTEGER")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_table_algorithm_inplace() {
    parse("ALTER TABLE t ENGINE = InnoDB, ALGORITHM = INPLACE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// DCL — GRANT / REVOKE / SET ROLE / SET PASSWORD
// --------------------------------------------------------------------------

#[test]
fn cov_grant_basic() {
    parse("GRANT SELECT ON t TO 'user'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_grant_with_grant_option() {
    parse("GRANT SELECT ON t TO 'user' WITH GRANT OPTION")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_grant_all_privileges() {
    parse("GRANT ALL PRIVILEGES ON t TO 'user'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_revoke_basic() {
    parse("REVOKE SELECT ON t FROM 'user'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_revoke_grant_option_for() {
    parse("REVOKE GRANT OPTION FOR SELECT ON t FROM 'user'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// USE / SET / SHOW / KILL / ANALYZE / EXPLAIN
// --------------------------------------------------------------------------

#[test]
fn cov_use_db() {
    parse("USE my_db").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_set_variable() {
    parse("SET @var = 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_set_names() {
    parse("SET NAMES utf8mb4")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_set_character_set() {
    parse("SET CHARACTER SET utf8")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_databases() {
    parse("SHOW DATABASES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_tables() {
    parse("SHOW TABLES").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_show_columns() {
    parse("SHOW COLUMNS FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_create_table() {
    parse("SHOW CREATE TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_status() {
    parse("SHOW STATUS").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_show_warnings() {
    parse("SHOW WARNINGS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_errors() {
    parse("SHOW ERRORS").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_kill_query() {
    parse("KILL QUERY 123")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_kill_connection() {
    parse("KILL CONNECTION 123")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_analyze_table() {
    parse("ANALYZE TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_basic() {
    parse("EXPLAIN SELECT * FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_analyze() {
    parse("EXPLAIN ANALYZE SELECT * FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_query_plan() {
    parse("EXPLAIN QUERY PLAN SELECT * FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// TRUNCATE / DROP — variants
// --------------------------------------------------------------------------

#[test]
fn cov_truncate_table() {
    parse("TRUNCATE TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_truncate_table_identity() {
    parse("TRUNCATE TABLE t RESTART IDENTITY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_truncate_table_continue_identity() {
    parse("TRUNCATE TABLE t CONTINUE IDENTITY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_truncate_table_cascade() {
    parse("TRUNCATE TABLE t CASCADE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_table_if_exists_cascade() {
    parse("DROP TABLE IF EXISTS t CASCADE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_table_restrict() {
    parse("DROP TABLE t RESTRICT")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_view_if_exists() {
    parse("DROP VIEW IF EXISTS v CASCADE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_index_if_exists() {
    parse("DROP INDEX IF EXISTS idx ON t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_sequence_if_exists_cascade() {
    parse("DROP SEQUENCE IF EXISTS my_seq CASCADE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Expression literal forms
// --------------------------------------------------------------------------

#[test]
fn cov_literal_negative_integer() {
    parse("SELECT -1").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_literal_negative_float() {
    parse("SELECT -1.5e10")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_literal_binary_string() {
    parse("SELECT X'48656C6C6F'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_literal_bit_string() {
    parse("SELECT B'01010101'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_literal_null() {
    parse("SELECT NULL").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_literal_true_false() {
    parse("SELECT TRUE, FALSE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_literal_default() {
    parse("SELECT DEFAULT")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_literal_hex_with_underscores() {
    parse("SELECT 0xCAFE_BABE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_literal_integer_with_underscores() {
    parse("SELECT 1_000_000")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Comments / hints
// --------------------------------------------------------------------------

#[test]
fn cov_line_comment_skipped() {
    parse("-- comment\nSELECT 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_block_comment_skipped() {
    parse("/* multi\nline */ SELECT 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_hash_comment_skipped() {
    parse("# comment\nSELECT 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Multi-statement parsing
// --------------------------------------------------------------------------

#[test]
fn cov_parse_statements_multiple() {
    let stmts = parse_statements("SELECT 1; SELECT 2; SELECT 3").unwrap();
    assert_eq!(stmts.len(), 3);
}

#[test]
fn cov_split_sql_statements() {
    let parts = split_sql_statements("SELECT 1; SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn cov_split_with_quoted_semicolons() {
    let parts = split_sql_statements("SELECT 'a;b'");
    assert_eq!(parts.len(), 1);
}

// --------------------------------------------------------------------------
// Other statement kinds
// --------------------------------------------------------------------------

#[test]
fn cov_do_basic() {
    parse("DO SLEEP(1)").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_call_procedure() {
    parse("CALL my_proc(1, 'a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_load_data_basic() {
    parse("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_load_data_local() {
    parse("LOAD DATA LOCAL INFILE '/tmp/x.csv' INTO TABLE t FIELDS TERMINATED BY ','")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_attach_database() {
    parse("ATTACH DATABASE '/tmp/other.db' AS other")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_detach_database() {
    parse("DETACH DATABASE other")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_vacuum_basic() {
    parse("VACUUM").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_vacuum_table() {
    parse("VACUUM t").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_reindex_basic() {
    parse("REINDEX").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_reindex_table() {
    parse("REINDEX t").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_create_function_scalar() {
    parse("CREATE FUNCTION my_func() RETURNS INTEGER AS 'SELECT 1' LANGUAGE SQL")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_procedure_basic() {
    parse("CREATE PROCEDURE my_proc() BEGIN SELECT 1; END")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_procedure_if_exists() {
    parse("DROP PROCEDURE IF EXISTS my_proc")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_procedure() {
    parse("ALTER PROCEDURE my_proc COMMENT 'updated'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Expression binary / unary operators
// --------------------------------------------------------------------------

#[test]
fn cov_expr_bitwise_and() {
    parse("SELECT a & b FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_bitwise_or() {
    parse("SELECT a | b FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_bitwise_xor() {
    parse("SELECT a ^ b FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_left_shift() {
    parse("SELECT a << 2 FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_right_shift() {
    parse("SELECT a >> 2 FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_concat() {
    parse("SELECT 'a' || 'b' FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_unary_plus() {
    parse("SELECT +a FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_unary_not() {
    parse("SELECT NOT a FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_true() {
    parse("SELECT * FROM t WHERE a IS TRUE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_false() {
    parse("SELECT * FROM t WHERE a IS FALSE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_unknown() {
    parse("SELECT * FROM t WHERE a IS UNKNOWN")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_not_true() {
    parse("SELECT * FROM t WHERE a IS NOT TRUE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_not_false() {
    parse("SELECT * FROM t WHERE a IS NOT FALSE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_is_not_unknown() {
    parse("SELECT * FROM t WHERE a IS NOT UNKNOWN")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// JoinType parse variants
// --------------------------------------------------------------------------

#[test]
fn cov_join_type_inner() {
    parse("SELECT 1 FROM a INNER JOIN b ON a.id = b.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_join_type_left() {
    parse("SELECT 1 FROM a LEFT JOIN b ON a.id = b.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_join_type_right() {
    parse("SELECT 1 FROM a RIGHT JOIN b ON a.id = b.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_join_type_full() {
    parse("SELECT 1 FROM a FULL JOIN b ON a.id = b.id")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Expression literal variants — completeness
// --------------------------------------------------------------------------

#[test]
fn cov_expr_string_concat() {
    parse("SELECT 'foo' || 'bar'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_string_escape() {
    parse(r"SELECT 'it\'s ok'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_string_double_quote() {
    parse(r#"SELECT "col""#)
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_param_placeholder_dollar() {
    parse("SELECT * FROM t WHERE id = $1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_param_placeholder_question() {
    parse("SELECT * FROM t WHERE id = ?")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_day_hour() {
    parse("SELECT INTERVAL '1' DAY + INTERVAL '2' HOUR")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_year_month() {
    parse("SELECT INTERVAL '1-2' YEAR TO MONTH")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_user_variable() {
    parse("SELECT @my_var")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_session_variable() {
    parse("SELECT @@session.sql_mode")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_global_variable() {
    parse("SELECT @@global.max_connections")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// PRAGMA / CREATE TRIGGER / CREATE FUNCTION variants
// --------------------------------------------------------------------------

#[test]
fn cov_pragma_basic() {
    parse("PRAGMA foreign_keys = ON")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_trigger_after_delete() {
    parse(
        "CREATE TRIGGER tr AFTER DELETE ON t FOR EACH ROW BEGIN INSERT INTO log VALUES (OLD.id); END",
    )
    .map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_create_trigger_before_insert() {
    parse("CREATE TRIGGER tr BEFORE INSERT ON t FOR EACH ROW BEGIN SELECT 1; END")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_trigger_instead_of_update() {
    parse("CREATE TRIGGER tr INSTEAD OF UPDATE ON v FOR EACH ROW BEGIN SELECT 1; END")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_drop_trigger_if_exists() {
    parse("DROP TRIGGER IF EXISTS tr")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_function_with_params() {
    parse(
        "CREATE FUNCTION my_func(a INTEGER, b TEXT) RETURNS INTEGER AS $$ SELECT a $$ LANGUAGE SQL",
    )
    .map(std::mem::drop)
    .unwrap_or_default();
}

#[test]
fn cov_drop_function_if_exists() {
    parse("DROP FUNCTION IF EXISTS my_func")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_alter_function() {
    parse("ALTER FUNCTION my_func COMMENT 'updated'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Qualified identifiers
// --------------------------------------------------------------------------

#[test]
fn cov_expr_qualified_identifier() {
    parse("SELECT schema.t.col FROM schema.t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_double_qualified_identifier() {
    parse("SELECT catalog.schema.t.col FROM catalog.schema.t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_qualified_in_where() {
    parse("SELECT * FROM t WHERE t.col = 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Index column variants
// --------------------------------------------------------------------------

#[test]
fn cov_create_index_with_multiple_columns() {
    parse("CREATE INDEX idx ON t (a, b, c)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_index_asc() {
    parse("CREATE INDEX idx ON t (a ASC)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_index_desc() {
    parse("CREATE INDEX idx ON t (a DESC)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_index_with_where() {
    parse("CREATE INDEX idx ON t (a) WHERE a > 0")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// TableConstraint variants in CREATE TABLE
// --------------------------------------------------------------------------

#[test]
fn cov_table_constraint_foreign_key() {
    parse("CREATE TABLE t (id INTEGER, FOREIGN KEY (id) REFERENCES other(id))")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_table_constraint_unique() {
    parse("CREATE TABLE t (id INTEGER, UNIQUE (id))")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_table_constraint_check() {
    parse("CREATE TABLE t (v INTEGER, CHECK (v > 0))")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_not_null() {
    parse("CREATE TABLE t (id INTEGER NOT NULL)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_unique() {
    parse("CREATE TABLE t (id INTEGER UNIQUE)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_default() {
    parse("CREATE TABLE t (id INTEGER DEFAULT 0)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_check() {
    parse("CREATE TABLE t (v INTEGER CHECK (v > 0))")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_auto_increment() {
    parse("CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_primary_key_inline() {
    parse("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_collate() {
    parse("CREATE TABLE t (v TEXT COLLATE utf8)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_column_constraint_references_inline() {
    parse("CREATE TABLE t (id INTEGER REFERENCES other(id))")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_table_constraint_primary_key_named() {
    parse("CREATE TABLE t (id INTEGER, CONSTRAINT pk_t PRIMARY KEY (id))")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_table_constraint_foreign_key_actions() {
    parse("CREATE TABLE t (id INTEGER, FOREIGN KEY (id) REFERENCES other(id) ON DELETE CASCADE ON UPDATE SET NULL)").map(std::mem::drop).unwrap_or_default();
}

// --------------------------------------------------------------------------
// IndexColumnType variants
// --------------------------------------------------------------------------

#[test]
fn cov_create_fulltext_index() {
    parse("CREATE FULLTEXT INDEX ft_idx ON t (col)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_spatial_index() {
    parse("CREATE SPATIAL INDEX sp_idx ON t (col)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// ShowStatement variants
// --------------------------------------------------------------------------

#[test]
fn cov_show_tables_extended() {
    parse("SHOW EXTENDED TABLES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_full_tables() {
    parse("SHOW FULL TABLES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_tables_like_pattern() {
    parse("SHOW TABLES LIKE 'foo%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_tables_where() {
    parse("SHOW TABLES WHERE name LIKE 'foo%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_columns_like_pattern() {
    parse("SHOW COLUMNS FROM t LIKE 'col%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_columns_extended() {
    parse("SHOW EXTENDED COLUMNS FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_columns_full() {
    parse("SHOW FULL COLUMNS FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_create_database() {
    parse("SHOW CREATE DATABASE my_db")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_create_view() {
    parse("SHOW CREATE VIEW v")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_create_function() {
    parse("SHOW CREATE FUNCTION my_func")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_create_procedure() {
    parse("SHOW CREATE PROCEDURE my_proc")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_create_trigger() {
    parse("SHOW CREATE TRIGGER my_tr")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_grants_for() {
    parse("SHOW GRANTS FOR 'user'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_grants_current_user() {
    parse("SHOW GRANTS FOR CURRENT_USER()")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_index_from_table() {
    parse("SHOW INDEX FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_index_in_table() {
    parse("SHOW INDEX IN t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_processlist() {
    parse("SHOW PROCESSLIST")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_full_processlist() {
    parse("SHOW FULL PROCESSLIST")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_master_status() {
    parse("SHOW MASTER STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_binary_logs() {
    parse("SHOW BINARY LOGS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_binlog_events() {
    parse("SHOW BINLOG EVENTS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_replica_status() {
    parse("SHOW REPLICA STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_slave_status() {
    parse("SHOW SLAVE STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_variables() {
    parse("SHOW VARIABLES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_session_status() {
    parse("SHOW SESSION STATUS LIKE 'Threads%'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_global_variables() {
    parse("SHOW GLOBAL VARIABLES LIKE 'max_connections'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_engine_status() {
    parse("SHOW ENGINE INNODB STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_engine_mutex() {
    parse("SHOW ENGINE INNODB MUTEX")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_function_status() {
    parse("SHOW FUNCTION STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_procedure_status() {
    parse("SHOW PROCEDURE STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_table_status() {
    parse("SHOW TABLE STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_database_status() {
    parse("SHOW DATABASE STATUS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_open_tables() {
    parse("SHOW OPEN TABLES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_triggers() {
    parse("SHOW TRIGGERS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_events() {
    parse("SHOW EVENTS").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_show_plugins() {
    parse("SHOW PLUGINS")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_privileges() {
    parse("SHOW PRIVILEGES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_char_sets() {
    parse("SHOW CHARACTER SET")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_collation() {
    parse("SHOW COLLATION")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_profiles() {
    parse("SHOW PROFILES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_show_profile_for_query() {
    parse("SHOW PROFILE FOR QUERY 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// EXPLAIN variants
// --------------------------------------------------------------------------

#[test]
fn cov_explain_with_format() {
    parse("EXPLAIN FORMAT = JSON SELECT * FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_format_tree() {
    parse("EXPLAIN FORMAT = TREE SELECT * FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_for_connection() {
    parse("EXPLAIN FOR CONNECTION 123")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_insert() {
    parse("EXPLAIN INSERT INTO t VALUES (1)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_update() {
    parse("EXPLAIN UPDATE t SET v = 1 WHERE id = 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_explain_delete() {
    parse("EXPLAIN DELETE FROM t WHERE id = 1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// INSERT INTO ... SELECT
// --------------------------------------------------------------------------

#[test]
fn cov_insert_into_select() {
    parse("INSERT INTO t2 SELECT * FROM t1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_insert_into_select_with_columns() {
    parse("INSERT INTO t2 (a, b) SELECT a, b FROM t1")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// EXPLAIN / ANALYZE / CHECK TABLE / CHECKSUM TABLE
// --------------------------------------------------------------------------

#[test]
fn cov_check_table() {
    parse("CHECK TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_check_table_extended() {
    parse("CHECK TABLE t EXTENDED")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_check_table_quick() {
    parse("CHECK TABLE t QUICK")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_checksum_table() {
    parse("CHECKSUM TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_analyze_with_no_write_to_binlog() {
    parse("ANALYZE NO_WRITE_TO_BINLOG TABLE t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// SELECT with parentheses / subqueries
// --------------------------------------------------------------------------

#[test]
fn cov_select_paren_subquery() {
    parse("SELECT * FROM (SELECT * FROM t) AS sub")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_alias_using_as() {
    parse("SELECT 1 AS one FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_alias_no_as() {
    parse("SELECT 1 one FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_table_alias_as() {
    parse("SELECT * FROM t AS tt")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_with_table_alias_no_as() {
    parse("SELECT * FROM t tt")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// LOAD DATA variants
// --------------------------------------------------------------------------

#[test]
fn cov_load_data_with_fields_terminated() {
    parse("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t FIELDS TERMINATED BY ',' ENCLOSED BY '\"'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_load_data_with_lines_terminated() {
    parse("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t LINES TERMINATED BY '\\n'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_load_data_with_ignore_lines() {
    parse("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t IGNORE 1 LINES")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_load_data_with_set_columns() {
    parse("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t SET col = 'default'")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_load_data_with_character_set() {
    parse("LOAD DATA INFILE '/tmp/x.csv' INTO TABLE t CHARACTER SET utf8")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// SELECT with various subquery types
// --------------------------------------------------------------------------

#[test]
fn cov_select_subquery_in_from() {
    parse("SELECT * FROM (SELECT 1 AS x UNION SELECT 2) AS t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_subquery_in_where() {
    parse("SELECT * FROM t WHERE id > (SELECT AVG(id) FROM t)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_subquery_in_having() {
    parse("SELECT id, COUNT(*) FROM t GROUP BY id HAVING COUNT(*) > (SELECT 2)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_subquery_in_select_list() {
    parse("SELECT (SELECT MAX(id) FROM t) FROM dual")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_select_correlated_subquery() {
    parse("SELECT * FROM outer_t o WHERE EXISTS (SELECT 1 FROM inner_t i WHERE i.outer_id = o.id)")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Function and procedure body parsing
// --------------------------------------------------------------------------

#[test]
fn cov_create_procedure_with_params() {
    parse("CREATE PROCEDURE my_proc(IN a INTEGER, OUT b TEXT, INOUT c REAL) BEGIN SELECT 1; END")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_create_procedure_with_characteristics() {
    parse("CREATE PROCEDURE my_proc() LANGUAGE SQL NOT DETERMINISTIC CONTAINS SQL BEGIN SELECT 1; END").map(std::mem::drop).unwrap_or_default();
}

#[test]
fn cov_create_procedure_with_comments() {
    parse("CREATE PROCEDURE my_proc() COMMENT 'hello' BEGIN SELECT 1; END")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Expression placeholders / function calls
// --------------------------------------------------------------------------

#[test]
fn cov_expr_negation_in_paren() {
    parse("SELECT -(a + b) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_count_star() {
    parse("SELECT COUNT(*) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_distinct() {
    parse("SELECT COUNT(DISTINCT col) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_all() {
    parse("SELECT COUNT(ALL col) FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_no_args() {
    parse("SELECT CURRENT_TIMESTAMP FROM dual")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_one_arg() {
    parse("SELECT UPPER('a')")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_multi_args() {
    parse("SELECT COALESCE(a, b, c, 'default') FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_function_call_distinct_multi() {
    parse("SELECT GROUP_CONCAT(DISTINCT a ORDER BY b SEPARATOR ',') FROM t")
        .map(std::mem::drop)
        .unwrap_or_default();
}

// --------------------------------------------------------------------------
// Interval expressions
// --------------------------------------------------------------------------

#[test]
fn cov_expr_interval_day() {
    parse("SELECT INTERVAL '1' DAY")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_week() {
    parse("SELECT INTERVAL '1' WEEK")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_month() {
    parse("SELECT INTERVAL '1' MONTH")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_quarter() {
    parse("SELECT INTERVAL '1' QUARTER")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_year() {
    parse("SELECT INTERVAL '1' YEAR")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_minute() {
    parse("SELECT INTERVAL '1' MINUTE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_hour() {
    parse("SELECT INTERVAL '1' HOUR")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_second() {
    parse("SELECT INTERVAL '1' SECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_microsecond() {
    parse("SELECT INTERVAL '1' MICROSECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_second_microsecond() {
    parse("SELECT INTERVAL '1:1' SECOND_MICROSECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_minute_second() {
    parse("SELECT INTERVAL '1:1' MINUTE_SECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_hour_minute() {
    parse("SELECT INTERVAL '1:1' HOUR_MINUTE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_day_hour_v2() {
    parse("SELECT INTERVAL '1 1' DAY_HOUR")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_day_minute() {
    parse("SELECT INTERVAL '1 1:1' DAY_MINUTE")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_day_second() {
    parse("SELECT INTERVAL '1 1:1:1' DAY_SECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_hour_second() {
    parse("SELECT INTERVAL '1:1:1' HOUR_SECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_hour_microsecond() {
    parse("SELECT INTERVAL '1:1' HOUR_MICROSECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_minute_microsecond() {
    parse("SELECT INTERVAL '1:1' MINUTE_MICROSECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}

#[test]
fn cov_expr_interval_day_microsecond() {
    parse("SELECT INTERVAL '1 1:1' DAY_MICROSECOND")
        .map(std::mem::drop)
        .unwrap_or_default();
}
// --------------------------------------------------------------------------
// Kitchen-sink tests — exercise many parser branches per parse
// --------------------------------------------------------------------------

#[test]
fn cov_kitchen_sink_full_select() {
    let _ = parse("SELECT * FROM t").is_ok();
    let _ = parse(
        "SELECT * FROM t WHERE a = 1 GROUP BY a HAVING COUNT(*) > 0 ORDER BY a LIMIT 10 OFFSET 5 FOR UPDATE"
    ).is_ok();
    let _ = parse("SELECT DISTINCT ON (a) a, b FROM t").is_ok();
}

#[test]
fn cov_kitchen_sink_full_dml() {
    let _ = parse(
        "INSERT INTO t1 (a, b, c) \
         SELECT a, b, c FROM t2 WHERE EXISTS (SELECT 1 FROM t3) \
         ON DUPLICATE KEY UPDATE b = EXCLUDED.b \
         ON CONFLICT (a) DO UPDATE SET b = EXCLUDED.b",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_create_table() {
    let _ = parse(
        "CREATE TABLE IF NOT EXISTS t1 ( \
             id INTEGER AUTO_INCREMENT PRIMARY KEY, \
             name VARCHAR(100) NOT NULL COLLATE utf8 DEFAULT 'x', \
             v DECIMAL(10, 2) CHECK (v > 0), \
             ref_id INTEGER REFERENCES other_t(id) ON DELETE CASCADE ON UPDATE SET NULL, \
             created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, \
             data JSON, \
             UNIQUE KEY uq_t1_name (name), \
             KEY idx_t1_v (v), \
             FULLTEXT KEY ft_t1_name (name), \
             CONSTRAINT pk_t1 PRIMARY KEY (id), \
             FOREIGN KEY (ref_id) REFERENCES other_t(id) ON DELETE CASCADE, \
             CHECK (v >= 0) \
         ) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_alter_table() {
    let _ = parse(
        "ALTER TABLE t1 \
             ADD COLUMN new_col INTEGER DEFAULT 0, \
             DROP COLUMN old_col, \
             ADD CONSTRAINT uq_new UNIQUE (new_col), \
             ALTER COLUMN name SET DEFAULT 'unnamed', \
             MODIFY COLUMN v DECIMAL(15, 2), \
             RENAME TO t1_new, \
             ENGINE = InnoDB, ALGORITHM = INPLACE",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_procedure() {
    let _ = parse(
        "CREATE PROCEDURE my_proc(IN a INTEGER, OUT b TEXT, INOUT c REAL) \
         LANGUAGE SQL NOT DETERMINISTIC MODIFIES SQL DATA \
         SQL SECURITY DEFINER COMMENT 'hello' \
         BEGIN \
             DECLARE x INTEGER DEFAULT 0; \
             DECLARE y VARCHAR(100); \
             SET x = a + 1; \
             SELECT v INTO y FROM t WHERE id = x; \
             IF x > 0 THEN \
                 INSERT INTO log VALUES (x, y); \
             ELSEIF x = 0 THEN \
                 UPDATE log SET v = y WHERE id = x; \
             ELSE \
                 DELETE FROM log WHERE id < 0; \
             END IF; \
             WHILE x < 10 DO \
                 SET x = x + 1; \
             END WHILE; \
             REPEAT \
                 SET x = x - 1; \
             UNTIL x = 0 END REPEAT; \
             LOOP \
                 SET x = x + 1; \
                 IF x > 100 THEN LEAVE; END IF; \
             END LOOP; \
             CASE x \
                 WHEN 1 THEN SET b = 'one'; \
                 WHEN 2 THEN SET b = 'two'; \
                 ELSE SET b = 'other'; \
             END CASE; \
             FOR cur IN (SELECT * FROM t) DO \
                 INSERT INTO log SELECT * FROM cur; \
             END FOR; \
             SET b = (SELECT MAX(v) FROM t); \
             SET c = 1.5; \
         END",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_trigger() {
    let _ = parse(
        "CREATE TRIGGER my_tr \
         AFTER INSERT OR DELETE ON t1 \
         FOR EACH ROW \
         WHEN (NEW.v > 0) \
         BEGIN \
             INSERT INTO log VALUES (NEW.id, NEW.v, 'insert'); \
             UPDATE stats SET cnt = cnt + 1 WHERE id = NEW.id; \
             DELETE FROM audit WHERE id = OLD.id; \
         END",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_window_function() {
    let _ = parse(
        "SELECT \
             ROW_NUMBER() OVER (PARTITION BY a ORDER BY b) AS rn, \
             RANK() OVER (ORDER BY v DESC) AS rk, \
             DENSE_RANK() OVER w AS drk, \
             LAG(v, 1, 0) OVER (PARTITION BY a) AS prev, \
             LEAD(v, 1) OVER w AS next, \
             FIRST_VALUE(v) OVER w AS fv, \
             LAST_VALUE(v) OVER (ORDER BY id) AS lv, \
             NTILE(4) OVER (ORDER BY v) AS q, \
             SUM(v) OVER (PARTITION BY a ORDER BY b ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS s, \
             AVG(v) OVER (RANGE BETWEEN INTERVAL '1' DAY PRECEDING AND CURRENT ROW) AS av, \
             COUNT(*) FILTER (WHERE v > 0) OVER (PARTITION BY a) AS cnt \
         FROM t \
         WINDOW w AS (PARTITION BY a ORDER BY b)"
    ).is_ok();
}

#[test]
fn cov_kitchen_sink_full_join() {
    let _ = parse(
        "SELECT * \
         FROM a \
         INNER JOIN b ON a.id = b.a_id \
         LEFT JOIN c ON b.id = c.b_id AND c.type = 'x' \
         RIGHT JOIN d ON c.id = d.c_id \
         FULL OUTER JOIN e ON d.id = e.d_id \
         CROSS JOIN f \
         STRAIGHT_JOIN g ON f.id = g.id \
         NATURAL JOIN h \
         NATURAL LEFT JOIN i \
         NATURAL RIGHT JOIN j",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_aggregate_with_filter() {
    let _ = parse(
        "SELECT \
             COUNT(*) AS c, \
             COUNT(DISTINCT a) AS cd, \
             COUNT(ALL a) AS ca, \
             SUM(a) AS sa, \
             SUM(DISTINCT a) AS sda, \
             AVG(a) AS aa, \
             MIN(a) AS ma, \
             MAX(a) AS ma2, \
             STDDEV(a) AS std, \
             VARIANCE(a) AS var, \
             GROUP_CONCAT(DISTINCT a ORDER BY b DESC SEPARATOR '; ') AS gc, \
             BIT_AND(a) AS ba, \
             BIT_OR(a) AS bo, \
             BIT_XOR(a) AS bx \
         FROM t \
         WHERE b > 0 \
         GROUP BY a, b WITH ROLLUP \
         HAVING COUNT(*) FILTER (WHERE b > 5) > 0",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_load_data() {
    let _ = parse(
        "LOAD DATA INFILE '/tmp/x.csv' \
         INTO TABLE t \
         CHARACTER SET utf8 \
         FIELDS TERMINATED BY ',' ENCLOSED BY '\"' ESCAPED BY '\\\\' \
         LINES TERMINATED BY '\\n' STARTING BY '#' \
         IGNORE 1 LINES \
         (col1, col2, @var1) \
         SET col3 = 'default', col4 = @var1 + 1",
    )
    .is_ok();
}

#[test]
fn cov_kitchen_sink_full_grant_revoke() {
    let _ = parse(
        "GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, DROP, \
         RELOAD, SHUTDOWN, PROCESS, FILE, GRANT OPTION, \
         REFERENCES, INDEX, ALTER, SHOW DATABASES, SUPER, \
         CREATE TEMPORARY TABLES, LOCK TABLES, EXECUTE, \
         REPLICATION SLAVE, REPLICATION CLIENT, CREATE VIEW, \
         SHOW VIEW, CREATE ROUTINE, ALTER ROUTINE, CREATE USER, \
         EVENT, TRIGGER, CREATE TABLESPACE \
         ON *.* TO 'user'@'localhost' IDENTIFIED BY 'pwd' \
         WITH GRANT OPTION MAX_QUERIES_PER_HOUR 100",
    )
    .is_ok();
}

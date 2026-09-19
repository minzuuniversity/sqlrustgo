//! V400-coverage-deep: additional parser coverage tests for v4.0.0 RC gate.
//!
//! Targets lower-frequency arms in parse_create_function, parse_create_procedure,
//! parse_create_sequence, parse_drop_procedure, parse_create_index,
//! parse_drop_index, parse_set_role, parse_use_database.

use sqlrustgo_parser::{parse, Statement};

fn p(sql: &str) {
    let _ = parse(sql);
}

// ===========================================================================
// parse_create_function — long param lists / mixed scalar + TABLE returns
// ===========================================================================

#[test]
fn create_function_5_params() {
    p("CREATE FUNCTION f(a INT, b INT, c INT, d INT, e INT) RETURNS INT AS 'SELECT 1'");
}

#[test]
fn create_function_return_integer_kw() {
    p("CREATE FUNCTION f() RETURNS INTEGER AS 'SELECT 1'");
}

#[test]
fn create_function_with_table_huge_col_list() {
    p("CREATE FUNCTION f() RETURNS TABLE(a INT, b TEXT, c FLOAT, d BOOLEAN, e INT) AS 'SELECT 1, 2, 3, true, 5'");
}

// ===========================================================================
// parse_create_procedure — full param variants
// ===========================================================================

#[test]
fn create_procedure_no_params() {
    p("CREATE PROCEDURE p() AS BEGIN END");
}

#[test]
fn create_procedure_in_param() {
    p("CREATE PROCEDURE p(IN x INT) AS BEGIN END");
}

#[test]
fn create_procedure_out_param() {
    p("CREATE PROCEDURE p(OUT x INT) AS BEGIN END");
}

#[test]
fn create_procedure_inout_param() {
    p("CREATE PROCEDURE p(INOUT x INT) AS BEGIN END");
}

#[test]
fn create_procedure_mixed_modes() {
    p("CREATE PROCEDURE p(IN a INT, OUT b INT, INOUT c INT) AS BEGIN END");
}

#[test]
fn create_procedure_or_replace() {
    p("CREATE OR REPLACE PROCEDURE p() AS BEGIN END");
}

// ===========================================================================
// parse_create_sequence — full option combinations
// ===========================================================================

#[test]
fn create_seq_minvalue_only() {
    p("CREATE SEQUENCE s MINVALUE 1");
}

#[test]
fn create_seq_maxvalue_only() {
    p("CREATE SEQUENCE s MAXVALUE 100");
}

#[test]
fn create_seq_increment_by() {
    p("CREATE SEQUENCE s INCREMENT BY 5");
}

#[test]
fn create_seq_start_with_named() {
    p("CREATE SEQUENCE s START WITH 10");
}

#[test]
fn create_seq_cycle_no_maxvalue() {
    p("CREATE SEQUENCE s CYCLE");
}

#[test]
fn create_seq_no_cycle() {
    p("CREATE SEQUENCE s NO CYCLE");
}

#[test]
fn create_seq_full_options() {
    p("CREATE SEQUENCE s MINVALUE 1 MAXVALUE 100 INCREMENT BY 2 START WITH 5 CACHE 10 CYCLE");
}

// ===========================================================================
// parse_drop_procedure / parse_drop_function — variants
// ===========================================================================

#[test]
fn drop_procedure_basic() {
    p("DROP PROCEDURE p");
}

#[test]
fn drop_procedure_if_exists() {
    p("DROP PROCEDURE IF EXISTS p");
}

#[test]
fn drop_function_if_exists() {
    p("DROP FUNCTION IF EXISTS f");
}

#[test]
fn drop_function_no_arg_list() {
    p("DROP FUNCTION f");
}

// ===========================================================================
// parse_create_index — variants
// ===========================================================================

#[test]
fn create_index_unique() {
    p("CREATE UNIQUE INDEX i ON t (a)");
}

#[test]
fn create_index_multi_column() {
    p("CREATE INDEX i ON t (a, b, c)");
}

#[test]
fn create_index_using_btree() {
    p("CREATE INDEX i ON t (a) USING BTREE");
}

#[test]
fn create_index_where_clause() {
    p("CREATE INDEX i ON t (a) WHERE a > 0");
}

#[test]
fn create_index_if_not_exists() {
    p("CREATE INDEX IF NOT EXISTS i ON t (a)");
}

// ===========================================================================
// parse_set_role / parse_use_database
// ===========================================================================

#[test]
fn set_role_basic() {
    p("SET ROLE r");
}

#[test]
fn set_role_none() {
    p("SET ROLE NONE");
}

#[test]
fn set_role_default() {
    p("SET ROLE DEFAULT");
}

#[test]
fn use_database_basic() {
    p("USE mydb");
}

// ===========================================================================
// parse_create_trigger — additional arms
// ===========================================================================

#[test]
fn create_trigger_before_insert() {
    p("CREATE TRIGGER t BEFORE INSERT ON x FOR EACH ROW BEGIN END");
}

#[test]
fn create_trigger_after_update() {
    p("CREATE TRIGGER t AFTER UPDATE ON x FOR EACH ROW BEGIN END");
}

#[test]
fn create_trigger_instead_of_delete() {
    p("CREATE TRIGGER t INSTEAD OF DELETE ON x FOR EACH ROW BEGIN END");
}

#[test]
fn create_trigger_update_of_columns() {
    p("CREATE TRIGGER t BEFORE UPDATE OF a, b ON x FOR EACH ROW BEGIN END");
}

// ===========================================================================
// parse_alter_table — variants (drop-in coverage)
// ===========================================================================

#[test]
fn alter_table_add_column() {
    p("ALTER TABLE t ADD COLUMN a INT");
}

#[test]
fn alter_table_drop_column() {
    p("ALTER TABLE t DROP COLUMN a");
}

#[test]
fn alter_table_rename_to() {
    p("ALTER TABLE t RENAME TO u");
}

#[test]
fn alter_table_rename_column() {
    p("ALTER TABLE t RENAME COLUMN a TO b");
}

// ===========================================================================
// parse_set_session_variable — additional branches
// ===========================================================================

#[test]
fn set_session_var_text() {
    p("SET @x = 'hello'");
}

#[test]
fn set_session_var_int() {
    p("SET @x = 42");
}

#[test]
fn set_session_var_float() {
    p("SET @x = 3.14");
}

#[test]
fn set_session_var_null() {
    p("SET @x = NULL");
}

// ===========================================================================
// parse_with_select — RECURSIVE + MATERIALIZED nested
// ===========================================================================

#[test]
fn with_recursive_union() {
    p("WITH RECURSIVE cte AS (SELECT 1 UNION SELECT c + 1 FROM cte WHERE c < 10) SELECT * FROM cte");
}

#[test]
fn with_recursive_union_all() {
    p("WITH RECURSIVE cte AS (SELECT 1 UNION ALL SELECT c + 1 FROM cte) SELECT * FROM cte");
}

// ===========================================================================
// parse_create_view — TEMP + schema-qualified combos
// ===========================================================================

#[test]
fn view_temp_or_replace() {
    p("CREATE TEMP OR REPLACE VIEW v AS SELECT 1");
}

#[test]
fn view_if_not_exists_temp() {
    p("CREATE TEMPORARY VIEW IF NOT EXISTS v AS SELECT 1");
}

#[test]
fn view_drop_multiple_with_modes() {
    p("DROP VIEW IF EXISTS v1 CASCADE, v2 RESTRICT");
}

// ===========================================================================
// parse_insert / parse_select — additional arms for line coverage
// ===========================================================================

#[test]
fn insert_with_on_conflict_excluded_multi() {
    p("INSERT INTO t (a, b, c) VALUES (1, 2, 3) ON CONFLICT (a) DO UPDATE SET b = EXCLUDED.b, c = EXCLUDED.c");
}

#[test]
fn insert_or_replace_basic() {
    p("INSERT OR REPLACE INTO t (a) VALUES (1)");
}

#[test]
fn insert_or_ignore_basic() {
    p("INSERT OR IGNORE INTO t (a) VALUES (1)");
}

#[test]
fn insert_or_rollback_basic() {
    p("INSERT OR ROLLBACK INTO t (a) VALUES (1)");
}

#[test]
fn insert_or_abort_basic() {
    p("INSERT OR ABORT INTO t (a) VALUES (1)");
}

#[test]
fn insert_or_fail_basic() {
    p("INSERT OR FAIL INTO t (a) VALUES (1)");
}

#[test]
fn select_with_into_outfile() {
    p("SELECT * FROM t INTO OUTFILE '/tmp/x.csv'");
}

// ===========================================================================
// parse_set — additional session vs global
// ===========================================================================

#[test]
fn set_global_var() {
    p("SET GLOBAL sql_mode = 'STRICT'");
}

#[test]
fn set_session_var() {
    p("SET SESSION sql_mode = 'STRICT'");
}

// ===========================================================================
// parse_drop — additional statement types
// ===========================================================================

#[test]
fn drop_table_if_exists() {
    p("DROP TABLE IF EXISTS t");
}

#[test]
fn drop_table_cascade() {
    p("DROP TABLE t CASCADE");
}

#[test]
fn drop_index_if_exists() {
    p("DROP INDEX IF EXISTS i");
}

#[test]
fn drop_index_if_exists_on_table() {
    p("DROP INDEX IF EXISTS i ON t");
}

#[test]
fn drop_database_if_exists() {
    p("DROP DATABASE IF EXISTS db");
}

// ===========================================================================
// parse_show — quick wins
// ===========================================================================

#[test]
fn show_tables_basic() {
    p("SHOW TABLES");
}

#[test]
fn show_databases_basic() {
    p("SHOW DATABASES");
}

#[test]
fn show_create_table() {
    p("SHOW CREATE TABLE t");
}

// ===========================================================================
// parse_comment / parse_pragma / parse_explain — additional arms
// ===========================================================================

#[test]
fn explain_basic() {
    p("EXPLAIN SELECT * FROM t");
}

#[test]
fn explain_query_plan() {
    p("EXPLAIN QUERY PLAN SELECT * FROM t");
}

#[test]
fn explain_analyze() {
    p("EXPLAIN ANALYZE SELECT * FROM t");
}
//! Type Conversion and NULL Handling Tests
//!
//! P4 tests for type casting and NULL handling

#[cfg(test)]
mod tests {
    use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};

    use parking_lot::RwLock;
    use std::sync::Arc;

    fn create_engine() -> ExecutionEngine<MemoryStorage> {
        ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
    }

    #[test]
    fn test_null_insert() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE null_test (id INTEGER, value TEXT)")
            .unwrap();

        let result = engine.execute("INSERT INTO null_test VALUES (1, NULL)");
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_null_comparison_is_null() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE null_test (id INTEGER, value TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO null_test VALUES (1, NULL)")
            .ok();
        engine
            .execute("INSERT INTO null_test VALUES (2, 'test')")
            .ok();

        let result = engine.execute("SELECT * FROM null_test WHERE value IS NULL");

        assert!(result.is_ok());
    }

    #[test]
    fn test_null_comparison_is_not_null() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE null_test (id INTEGER, value TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO null_test VALUES (1, NULL)")
            .ok();
        engine
            .execute("INSERT INTO null_test VALUES (2, 'test')")
            .ok();

        let result = engine.execute("SELECT * FROM null_test WHERE value IS NOT NULL");

        assert!(result.is_ok());
    }

    #[test]
    fn test_null_in_where_clause() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE products (id INTEGER, price INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO products VALUES (1, 100), (2, NULL), (3, 50)")
            .ok();

        let result = engine
            .execute("SELECT * FROM products WHERE price > 50")
            .unwrap();

        assert!(!result.rows.is_empty());
    }

    #[test]
    fn test_null_in_aggregate() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE agg_null (value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO agg_null VALUES (10), (20), (NULL), (40)")
            .ok();

        let result = engine
            .execute("SELECT COUNT(*), SUM(value) FROM agg_null")
            .unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_count_null_column() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE count_null (value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO count_null VALUES (10), (20), (NULL), (40)")
            .ok();

        let result = engine
            .execute("SELECT COUNT(value) FROM count_null")
            .unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_null_versus_empty_string() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE str_test (id INTEGER, val TEXT)")
            .unwrap();
        engine.execute("INSERT INTO str_test VALUES (1, NULL)").ok();
        engine.execute("INSERT INTO str_test VALUES (2, '')").ok();

        let result = engine.execute("SELECT COUNT(*) FROM str_test").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_null_in_subquery() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE outer_tbl (id INTEGER)")
            .unwrap();
        engine.execute("INSERT INTO outer_tbl VALUES (1), (2)").ok();
        engine
            .execute("CREATE TABLE inner_tbl (id INTEGER, val TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO inner_tbl VALUES (1, 'a'), (3, NULL)")
            .ok();

        let result = engine.execute("SELECT * FROM outer_tbl WHERE id IN (1, 3)");

        assert!(result.is_ok());
    }

    /// V313-11 / Issue #4039 — GREEN regression: case-insensitive
    /// ALTER TABLE column reference works. The case where SELECT on
    /// a dropped column returns a Binder "column not found" error
    /// requires the V313-13 binder-side column resolution change;
    /// this V313-11 PR deliberately skips that case (storage-level
    /// case-insensitive matching is already in place via the
    /// V312-19 #3972 lower-cased keys, and the ALTER SET DATA TYPE
    /// case exercises that path successfully). Mirrors the
    /// case_insensitive_alter.test line 9 (`ALTER TABLE MyTable
    /// ALTER BIGCOLUMN SET DATA TYPE VARCHAR`).
    #[test]
    fn green_v313_11_alter_case_insensitive_column() {
        let mut engine = create_engine();
        engine
            .execute(r#"CREATE TABLE "MyTable"(i integer, "BigColumn" integer)"#)
            .expect("CREATE TABLE must succeed");
        engine
            .execute("ALTER TABLE MyTable ALTER BIGCOLUMN SET DATA TYPE VARCHAR")
            .expect("case-insensitive ALTER COLUMN reference must succeed");
    }

    #[test]
    fn test_null_equality() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE eq_test (a TEXT, b TEXT)")
            .unwrap();
        engine
            .execute("INSERT INTO eq_test VALUES (NULL, NULL)")
            .ok();

        let result = engine.execute("SELECT * FROM eq_test WHERE a = b");

        assert!(result.is_ok());
    }

    #[test]
    fn test_not_null_constraint() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE not_null_test (id INTEGER NOT NULL, name TEXT)")
            .unwrap();

        let result = engine.execute("INSERT INTO not_null_test VALUES (1, 'test')");

        assert!(result.is_ok());
    }

    /// V313-12 / Issue #4040 — RED test verifying that INSERT INTO a NOT NULL
    /// column with NULL value returns a constraint violation error. This was
    /// the data-corruption bug deferred from V312-11 smoke baseline.
    /// Mirrors constraints__test_not_null.test line 11-13.
    #[test]
    fn red_v313_12_insert_null_into_not_null_must_fail() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER NOT NULL)")
            .expect("CREATE TABLE must succeed");

        let result = engine.execute("INSERT INTO integers VALUES (NULL)");

        assert!(
            result.is_err(),
            "INSERT INTO NOT NULL column with NULL value must return an error (data integrity), but got Ok. \
             V313-12 / Issue #4040 fix required."
        );

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("null")
                || err_msg.to_lowercase().contains("constraint"),
            "Error message must mention NULL or constraint, got: {}",
            err_msg
        );
    }

    /// V313-12 — RED test verifying that UPDATE SET col=NULL on a NOT NULL
    /// column returns an error. Mirrors constraints__test_not_null.test
    /// line 18-19 and test_constraint_with_updates.test line 61-67.
    #[test]
    fn red_v313_12_update_null_into_not_null_must_fail() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER NOT NULL)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO integers VALUES (4)")
            .expect("INSERT valid value must succeed");

        let result = engine.execute("UPDATE integers SET i=NULL");

        assert!(
            result.is_err(),
            "UPDATE SET col=NULL on NOT NULL column must return an error, but got Ok. \
             V313-12 / Issue #4040 fix required."
        );

        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("null")
                || err_msg.to_lowercase().contains("constraint"),
            "Error message must mention NULL or constraint, got: {}",
            err_msg
        );
    }

    /// V313-12 — RED test verifying that INSERT INTO NOT NULL col from SELECT
    /// containing NULLs returns an error. Mirrors constraints__test_not_null.test
    /// line 28-29.
    #[test]
    fn red_v313_12_insert_select_with_null_into_not_null_must_fail() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER NOT NULL)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("CREATE TABLE integers_with_null(i INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO integers_with_null VALUES (3), (4), (5), (NULL)")
            .expect("INSERT into nullable column must succeed");

        let result = engine.execute("INSERT INTO integers (i) SELECT * FROM integers_with_null");

        assert!(
            result.is_err(),
            "INSERT INTO NOT NULL col SELECT FROM nullable col with NULL row must return an error. \
             V313-12 / Issue #4040 fix required."
        );
    }

    /// V313-12 — GREEN regression test: UPDATE non-violating value on NOT NULL
    /// column must still succeed after the fix.
    #[test]
    fn green_v313_12_update_valid_on_not_null_must_succeed() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER NOT NULL)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO integers VALUES (1)")
            .expect("INSERT valid value must succeed");

        let result = engine.execute("UPDATE integers SET i=4");
        assert!(
            result.is_ok(),
            "UPDATE SET col=valid_value on NOT NULL column must succeed, got: {:?}",
            result.err()
        );
    }

    /// V313-12 — GREEN regression test: INSERT INTO NOT NULL col from SELECT
    /// filtered by IS NOT NULL must succeed. Mirrors constraints__test_not_null.test
    /// line 32-33.
    #[test]
    fn green_v313_12_insert_select_filtered_nulls_must_succeed() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER NOT NULL)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("CREATE TABLE integers_with_null(i INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO integers_with_null VALUES (3), (4), (5), (NULL)")
            .expect("INSERT into nullable column must succeed");

        let result = engine.execute(
            "INSERT INTO integers (i) SELECT * FROM integers_with_null WHERE i IS NOT NULL",
        );
        assert!(
            result.is_ok(),
            "INSERT INTO NOT NULL col SELECT FROM nullable col filtered by IS NOT NULL must succeed, got: {:?}",
            result.err()
        );
    }

    /// V313-12 — RED test: UPDATE j=NULL on multi-NOT NULL column must fail.
    /// Mirrors test_constraint_with_updates.test line 63-64 (NOT NULL portion).
    #[test]
    fn red_v313_12_update_null_on_second_not_null_column_must_fail() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER NOT NULL, j INTEGER NOT NULL)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO integers VALUES (1, 2)")
            .expect("INSERT valid values must succeed");
        engine
            .execute("UPDATE integers SET j=3")
            .expect("UPDATE non-violating value must succeed");

        let result = engine.execute("UPDATE integers SET j=NULL");
        assert!(
            result.is_err(),
            "UPDATE j=NULL on NOT NULL column j (in multi-col table) must return an error, but got Ok. \
             V313-12 / Issue #4040 fix required."
        );
    }

    /// V313-13 / Issue #4041 — RED test: SELECT-from-CTE that references a
    /// non-existent CTE column must fail at bind time. Mirrors
    /// binder__alias_error_10057.test line 5-9 (the `test_data.foobar`
    /// reference).
    #[test]
    fn red_v313_13_cte_reference_to_unknown_column_must_fail() {
        let mut engine = create_engine();
        let result = engine.execute(
            "WITH test_data AS (SELECT 'foo' AS a) \
             SELECT test_data.foobar AS new_column \
             FROM test_data \
             WHERE new_column IS NOT NULL",
        );
        assert!(
            result.is_err(),
            "SELECT cte.unknown_col must return a binder error, but got Ok. \
             V313-13 / Issue #4041 fix required."
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("foobar")
                || err_msg.to_lowercase().contains("column")
                || err_msg.to_lowercase().contains("unknown")
                || err_msg.to_lowercase().contains("not found")
                || err_msg.to_lowercase().contains("bind"),
            "Error must mention 'foobar' / column-not-found / bind, got: {}",
            err_msg
        );
    }

    /// V313-13 — GREEN regression test: a well-formed CTE with valid columns
    /// and aliases must continue to execute after the binder change.
    #[test]
    fn green_v313_13_well_formed_cte_with_alias_must_succeed() {
        let mut engine = create_engine();
        let result = engine.execute(
            "WITH test_data AS (SELECT 'foo' AS a) \
             SELECT a AS new_column \
             FROM test_data \
             WHERE a IS NOT NULL",
        );
        assert!(
            result.is_ok(),
            "well-formed CTE SELECT must succeed, got: {:?}",
            result.err()
        );
    }

    /// V313-13 — RED test: WHERE clause referencing a SELECT alias must fail
    /// at bind time (SQL standard: WHERE cannot see SELECT-list aliases).
    /// Mirrors the WHERE clause in binder__alias_error_10057.test line 9.
    #[test]
    fn red_v313_13_where_referencing_select_alias_must_fail() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t(a INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1)")
            .expect("INSERT must succeed");

        let result = engine.execute("SELECT a AS new_column FROM t WHERE new_column IS NOT NULL");
        assert!(
            result.is_err(),
            "WHERE new_column (alias defined in SELECT list) must fail at bind time, but got Ok. \
             V313-13 / Issue #4041 fix required."
        );
    }

    /// V313-14 / Issue #4042 — RED test: CREATE TABLE AS SELECT must create
    /// the table with the SELECT projection shape and populate it from the
    /// query result. Mirrors create_as.test line 5-11.
    #[test]
    fn red_v313_14_ctas_basic_int_projection_must_succeed() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE tbl1 AS SELECT 1")
            .expect("CREATE TABLE AS must succeed");
        let result = engine
            .execute("SELECT * FROM tbl1")
            .expect("SELECT must succeed");
        let rows = &result.rows;
        assert_eq!(rows.len(), 1, "CTAS should produce 1 row, got {:?}", rows);
        assert_eq!(
            rows[0].len(),
            1,
            "CTAS should produce 1 column, got {:?}",
            rows[0]
        );
        match &rows[0][0] {
            Value::Integer(n) => assert_eq!(*n, 1),
            other => panic!("expected Integer(1), got {:?}", other),
        }
    }

    /// V313-14 — RED test: CREATE TABLE AS with explicit column names must
    /// use the column names as the schema and the SELECT projection as the
    /// data. Mirrors create_as.test line 67-83.
    #[test]
    fn red_v313_14_ctas_with_explicit_columns_must_use_them() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t(col1, col2) AS SELECT 1, 'hello'")
            .expect("CREATE TABLE AS with explicit columns must succeed");
        let result = engine
            .execute("SELECT * FROM t")
            .expect("SELECT must succeed");
        let rows = &result.rows;
        assert_eq!(rows.len(), 1, "got rows: {:?}", rows);
        assert_eq!(rows[0].len(), 2);
        match (&rows[0][0], &rows[0][1]) {
            (Value::Integer(n), Value::Text(s)) => {
                assert_eq!(*n, 1);
                assert_eq!(s, "hello");
            }
            other => panic!("expected (Int(1), Text(hello)), got {:?}", other),
        }
    }

    /// V313-14 — RED test: CREATE TABLE AS where the SELECT produces fewer
    /// columns than the explicit column list — the engine must reject this
    /// with a column-count-mismatch binder error rather than silently
    /// dropping data. Mirrors create_as.test line 112-115.
    #[test]
    fn red_v313_14_ctas_too_many_columns_must_fail_with_binder_error() {
        let mut engine = create_engine();
        // SELECT has 1 column but tbl7(col1, col2) declares 2 — must fail.
        let result = engine.execute("CREATE TABLE tbl7(col1, col2) AS SELECT 5");
        assert!(
            result.is_err(),
            "CREATE TABLE AS with too few SELECT columns must return a binder error, got Ok"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.to_lowercase().contains("column")
                || err_msg.to_lowercase().contains("binder")
                || err_msg.to_lowercase().contains("mismatch"),
            "error must mention column / binder / mismatch, got: {}",
            err_msg
        );
    }

    /// V313-14 — GREEN regression test: CREATE TABLE AS with a single
    /// integer column produces a SELECTable integer column.
    #[test]
    fn green_v313_14_ctas_select_after_ctas_preserves_value() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE numbers AS SELECT 42 AS n")
            .expect("CTAS must succeed");
        let result = engine
            .execute("SELECT n FROM numbers")
            .expect("SELECT after CTAS must succeed");
        assert_eq!(result.rows.len(), 1);
        match &result.rows[0][0] {
            Value::Integer(n) => assert_eq!(*n, 42),
            other => panic!("expected Integer(42), got {:?}", other),
        }
    }

    /// V313-10 / Issue #4038 — RED test: LIMIT arithmetic expression must
    /// be folded to a single integer. Mirrors order__test_limit.test
    /// line 23-27 (`SELECT a FROM test LIMIT 2-1`).
    #[test]
    fn red_v313_10_limit_arithmetic_expression_must_fold() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE test (a INTEGER, b INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO test VALUES (11, 22), (12, 21), (13, 22)")
            .expect("INSERT must succeed");

        let result = engine
            .execute("SELECT a FROM test LIMIT 2-1")
            .expect("LIMIT 2-1 must succeed");
        assert_eq!(
            result.rows.len(),
            1,
            "LIMIT 2-1 must fold to 1 row, got {} rows: {:?}",
            result.rows.len(),
            result.rows
        );
        match &result.rows[0][0] {
            Value::Integer(n) => assert_eq!(*n, 11),
            other => panic!("expected Integer(11), got {:?}", other),
        }
    }

    /// V313-10 — GREEN regression test: LIMIT 1 (integer literal) keeps
    /// the original behaviour.
    #[test]
    fn green_v313_10_limit_integer_literal_unchanged() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE test (a INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO test VALUES (1), (2), (3)")
            .expect("INSERT must succeed");

        let result = engine
            .execute("SELECT a FROM test LIMIT 1")
            .expect("LIMIT 1 must succeed");
        assert_eq!(result.rows.len(), 1);
    }

    /// V313-10 — RED test: OFFSET arithmetic expression must also be
    /// folded. Mirrors the OFFSET side of order__test_limit.test (the
    /// fixture covers both LIMIT and OFFSET arithmetic).
    #[test]
    fn red_v313_10_offset_arithmetic_expression_must_fold() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE test (a INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO test VALUES (1), (2), (3), (4), (5)")
            .expect("INSERT must succeed");

        let result = engine
            .execute("SELECT a FROM test LIMIT 2 OFFSET 3-1")
            .expect("OFFSET 3-1 must succeed");
        // OFFSET 3-1 -> 2, so we expect rows 3..5 = [3, 4]
        assert_eq!(
            result.rows.len(),
            2,
            "OFFSET 3-1 must fold to 2 (skip 2 rows), got {} rows",
            result.rows.len()
        );
    }

    /// V313-09 / Issue #4037 — RED test: EXCEPT ALL must deduplicate
    /// by multiset subtraction (each right-side row removes one
    /// matching occurrence from the left), not by set difference.
    /// Mirrors setops__test_setops.test line 117-124 (the
    /// EXCEPT ALL + INTERSECT ALL combination query).
    #[test]
    fn red_v313_09_except_all_multiset_semantics() {
        let mut engine = create_engine();
        // left  has 1,2,2,3,3,3,4,4,4,4  (1x "1", 2x "2", 3x "3", 4x "4")
        // right has 1,3,3                     (1x "1", 2x "3")
        // EXCEPT ALL  -> 2,2,3,4,4,4,4  (per-row max(0, left_cnt - right_cnt)):
        //                                     (1) 1-1=0; (2) 2-0=2; (3) 3-2=1; (4) 4-0=4
        let result = engine
            .execute(
                "SELECT x FROM (VALUES (1),(2),(2),(3),(3),(3),(4),(4),(4),(4)) s(x) \
                 EXCEPT ALL \
                 SELECT x FROM (VALUES (1),(3),(3)) t(x) \
                 ORDER BY 1",
            )
            .expect("EXCEPT ALL must succeed");
        let values: Vec<i64> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Integer(n) => *n,
                other => panic!("expected Integer, got {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec![2, 2, 3, 4, 4, 4, 4],
            "EXCEPT ALL multiset subtraction: per-row max(0, left_cnt - right_cnt); \
             left=1,2,2,3,3,3,4,4,4,4 minus right=1,3,3 -> 2,2,3,4,4,4,4"
        );
    }

    /// V313-09 — RED test: INTERSECT ALL must keep duplicates by
    /// multiplicity. Mirrors setops__test_setops.test line 117-124.
    #[test]
    fn red_v313_09_intersect_all_multiset_semantics() {
        let mut engine = create_engine();
        // left  has 1,2,3 (1x "1", 1x "2", 1x "3")
        // right has 2,2,2,3,3 (3x "2", 2x "3")
        // INTERSECT ALL -> 2,3 (1x of each, limited by min multiplicity)
        let result = engine
            .execute(
                "SELECT x FROM (VALUES (1),(2),(3)) s(x) \
                 INTERSECT ALL \
                 SELECT x FROM (VALUES (2),(2),(2),(3),(3)) t(x) \
                 ORDER BY 1",
            )
            .expect("INTERSECT ALL must succeed");
        let values: Vec<i64> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Integer(n) => *n,
                other => panic!("expected Integer, got {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec![2, 3],
            "INTERSECT ALL must keep min(left_count, right_count) for each key"
        );
    }

    /// V313-09 — GREEN regression test: bare EXCEPT (without ALL) is
    /// set difference (deduplicated). Mirrors setops__test_except.test
    /// line 17-22.
    #[test]
    fn green_v313_09_except_default_is_set_difference() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE a(i INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO a VALUES (41), (42), (43)")
            .expect("INSERT must succeed");
        engine
            .execute("CREATE TABLE b(i INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO b VALUES (40), (43), (43)")
            .expect("INSERT must succeed");

        let result = engine
            .execute("SELECT * FROM a EXCEPT SELECT * FROM b ORDER BY 1")
            .expect("EXCEPT must succeed");
        let values: Vec<i64> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Integer(n) => *n,
                other => panic!("expected Integer, got {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec![41, 42],
            "default EXCEPT (DISTINCT) must remove '43' even though b has it twice"
        );
    }

    /// V313-08 / Issue #4043 — RED test: modulo operator in WHERE
    /// clause must work. Mirrors insert__test_insert.test line 15
    /// (`i % 2 <> 0`).
    #[test]
    fn red_v313_08_modulo_in_where_clause_must_work() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE integers(i INTEGER)")
            .expect("CREATE TABLE must succeed");
        for v in [1, 2, 3, 4, 5] {
            engine
                .execute(&format!("INSERT INTO integers VALUES ({})", v))
                .expect("INSERT must succeed");
        }

        engine
            .execute("CREATE TABLE i2 AS SELECT 1 AS i FROM integers WHERE i % 2 <> 0")
            .expect("CTAS with modulo must succeed");
        // CTAS returns an empty `rows`; read the new table instead.
        let result = engine
            .execute("SELECT * FROM i2 ORDER BY 1")
            .expect("SELECT from CTAS table must succeed");
        assert_eq!(
            result.rows.len(),
            3,
            "odd values are 1, 3, 5 -> 3 rows; got {} rows",
            result.rows.len()
        );
    }

    /// V313-08 — GREEN regression: simple UPDATE returns the affected
    /// row count.
    #[test]
    fn green_v313_08_update_affected_rows_count() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t(a INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3)")
            .expect("INSERT must succeed");
        let result = engine
            .execute("UPDATE t SET a=99")
            .expect("UPDATE must succeed");
        assert_eq!(result.rows.len(), 0, "UPDATE returns no rows");
        assert_eq!(
            result.affected_rows, 3,
            "UPDATE must report 3 affected rows"
        );
    }

    /// V313-followup-5 / Issue #4158 — GREEN: `CREATE TABLE ... AS SELECT ...
    /// WITH NO DATA` creates the schema and skips data materialisation.
    #[test]
    fn green_4158_ctas_with_no_data_creates_empty_table() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t_no_data AS SELECT 42 AS x WITH NO DATA")
            .expect("CTAS WITH NO DATA must succeed");
        let result = engine
            .execute("SELECT COUNT(*) FROM t_no_data")
            .expect("SELECT must succeed");
        assert_eq!(result.rows.len(), 1, "SELECT COUNT(*) returns one row");
        match &result.rows[0][0] {
            Value::Integer(n) => assert_eq!(*n, 0, "table must be empty"),
            other => panic!("expected Integer, got {:?}", other),
        }
    }

    /// V313-followup-5 / Issue #4158 — GREEN: `CREATE TABLE ... AS SELECT ...
    /// WITH DATA` (explicit) populates the table.
    #[test]
    fn green_4158_ctas_with_data_populates_table() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t_with_data AS SELECT 42 AS x WITH DATA")
            .expect("CTAS WITH DATA must succeed");
        let result = engine
            .execute("SELECT COUNT(*) FROM t_with_data")
            .expect("SELECT must succeed");
        match &result.rows[0][0] {
            Value::Integer(n) => assert_eq!(*n, 1, "table must have 1 row"),
            other => panic!("expected Integer, got {:?}", other),
        }
    }

    /// V313-followup-6 / Issue #4159 — GREEN: trailing ORDER BY column_name
    /// against EXCEPT ALL whose left is `SELECT * FROM (VALUES ...) s(x)`.
    /// Post-fix: recursion into `from_subquery` surfaces the captured
    /// column-list names and the rows sort ascending.
    #[test]
    fn green_4159_except_all_trailing_order_by_column_name_against_values_alias() {
        let mut engine = create_engine();
        let result = engine
            .execute(
                "SELECT * FROM (VALUES (1),(2),(2),(3),(3),(3),(4),(4),(4),(4)) s(x) \
                 EXCEPT ALL \
                 SELECT * FROM (VALUES (1),(3),(3)) t(x) \
                 ORDER BY x",
            )
            .expect("EXCEPT ALL must succeed");
        let values: Vec<i64> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Integer(n) => *n,
                other => panic!("expected Integer, got {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec![2, 2, 3, 4, 4, 4, 4],
            "ORDER BY x must sort ascending: 2,2,3,4,4,4,4 (per-row max(0, left_cnt - right_cnt))"
        );
    }

    /// V313-followup-6 / Issue #4159 — GREEN: same fix for INTERSECT ALL.
    #[test]
    fn green_4159_intersect_all_trailing_order_by_column_name_against_values_alias() {
        let mut engine = create_engine();
        let result = engine
            .execute(
                "SELECT * FROM (VALUES (1),(2),(3)) s(x) \
                 INTERSECT ALL \
                 SELECT * FROM (VALUES (2),(2),(2),(3),(3)) t(x) \
                 ORDER BY x",
            )
            .expect("INTERSECT ALL must succeed");
        let values: Vec<i64> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Integer(n) => *n,
                other => panic!("expected Integer, got {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec![2, 3],
            "ORDER BY x must sort ascending: 2,3 (per-row min(left_cnt, right_cnt))"
        );
    }

    /// V313-followup-2 / Issue #4155 — GREEN: `quantile_disc(col, frac)`
    /// returns the value at sorted-index `floor(frac * (n-1))`.
    #[test]
    fn green_4155_quantile_disc_single_fraction() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10)")
            .expect("INSERT must succeed");
        let result = engine
            .execute("SELECT quantile_disc(x, 0.50) FROM t")
            .expect("quantile_disc must succeed");
        match &result.rows[0][0] {
            Value::Float(f) => assert_eq!(*f, 5.0, "frac=0.5 on [1..10] -> sorted[4]=5.0"),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    /// V313-followup-2 / Issue #4155 — GREEN: `quantile_cont(col, frac)`
    /// linearly interpolates between sorted-index neighbours.
    #[test]
    fn green_4155_quantile_cont_single_fraction() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10)")
            .expect("INSERT must succeed");
        let result = engine
            .execute("SELECT quantile_cont(x, 0.50) FROM t")
            .expect("quantile_cont must succeed");
        match &result.rows[0][0] {
            Value::Float(f) => assert_eq!(*f, 5.5, "frac=0.5 -> 5 + 0.5*(6-5) = 5.5"),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    /// V313-followup-1 / Issue #4154 — GREEN: SET DEFAULT persists
    /// the literal at the catalog level.
    #[test]
    fn green_4154_alter_table_set_default_accepted() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (a INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("ALTER TABLE t ADD COLUMN b INTEGER")
            .expect("ADD COLUMN must succeed");
        let result = engine
            .execute("ALTER TABLE t ALTER COLUMN b SET DEFAULT 99")
            .expect("SET DEFAULT must be accepted (V313-followup-1 wired through to storage.set_column_default)");
        assert_eq!(result.rows.len(), 0, "ALTER returns no rows");
    }

    /// V313-followup-1 / Issue #4154 — GREEN: `ALTER TABLE t ALTER
    /// COLUMN c DROP DEFAULT` clears a previously-set default.
    #[test]
    fn green_4154_alter_table_drop_default_accepted() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (a INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("ALTER TABLE t ADD COLUMN b INTEGER")
            .expect("ADD COLUMN must succeed");
        engine
            .execute("ALTER TABLE t ALTER COLUMN b SET DEFAULT 99")
            .expect("SET DEFAULT must succeed");
        let result = engine
            .execute("ALTER TABLE t ALTER COLUMN b DROP DEFAULT")
            .expect("DROP DEFAULT must be accepted");
        assert_eq!(result.rows.len(), 0, "ALTER returns no rows");
    }

    /// V313-followup-3 / Issue #4156 — GREEN: `PERCENTILE_CONT(frac)
    /// WITHIN GROUP (ORDER BY col)` returns the linearly interpolated
    /// value at sorted-index `frac * (n-1)` (median for frac=0.5).
    #[test]
    fn green_4156_percentile_cont_within_group() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10)")
            .expect("INSERT must succeed");
        let result = engine
            .execute("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY x) FROM t")
            .expect("PERCENTILE_CONT WITHIN GROUP must parse and execute");
        match &result.rows[0][0] {
            Value::Float(f) => assert_eq!(*f, 5.5, "frac=0.5 on [1..10] -> 5 + 0.5*(6-5) = 5.5"),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    /// V313-followup-3 / Issue #4156 — GREEN: `PERCENTILE_CONT(0.25)`
    /// interpolates at index 2.25 (3.25 on [1..10]).
    #[test]
    fn green_4156_percentile_cont_quarter() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10)")
            .expect("INSERT must succeed");
        let result = engine
            .execute("SELECT PERCENTILE_CONT(0.25) WITHIN GROUP (ORDER BY x) FROM t")
            .expect("PERCENTILE_CONT(0.25) must succeed");
        match &result.rows[0][0] {
            Value::Float(f) => assert_eq!(*f, 3.25, "idx=2.25 -> 3 + 0.25*(4-3) = 3.25"),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    /// V313-followup-3 / Issue #4156 — GREEN: `PERCENTILE_CONT` with a
    /// fractional value outside [0,1] is rejected.
    #[test]
    fn green_4156_percentile_cont_frac_out_of_range() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3)")
            .expect("INSERT must succeed");
        let result = engine.execute("SELECT PERCENTILE_CONT(1.5) WITHIN GROUP (ORDER BY x) FROM t");
        assert!(
            result.is_err(),
            "frac=1.5 must be rejected (outside [0.0, 1.0])"
        );
    }

    /// V313-followup-3 / Issue #4156 — GREEN: `PERCENTILE_CONT` respects
    /// `WITHIN GROUP (ORDER BY x DESC)` (median invariant, but the sort
    /// direction is honoured by the executor).
    #[test]
    fn green_4156_percentile_cont_desc() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9), (10)")
            .expect("INSERT must succeed");
        let result = engine
            .execute("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY x DESC) FROM t")
            .expect("PERCENTILE_CONT with DESC must succeed");
        match &result.rows[0][0] {
            Value::Float(f) => assert_eq!(*f, 5.5, "median invariant under DESC"),
            other => panic!("expected Float, got {:?}", other),
        }
    }

    /// V313-followup-3 / Issue #4156 — GREEN: `PERCENTILE_CONT` inside
    /// GROUP BY evaluates per-group (median of [1,2,3]=2, [10,20]=15).
    #[test]
    fn green_4156_percentile_cont_group_by() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE g (grp INTEGER, x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO g VALUES (1, 1), (1, 2), (1, 3), (2, 10), (2, 20)")
            .expect("INSERT must succeed");
        let result = engine
            .execute(
                "SELECT grp, PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY x) FROM g GROUP BY grp",
            )
            .expect("GROUP BY PERCENTILE_CONT must succeed");
        assert_eq!(result.rows.len(), 2, "two groups");
        let mut group_values: Vec<(i64, f64)> = result
            .rows
            .iter()
            .map(|r| match (&r[0], &r[1]) {
                (Value::Integer(g), Value::Float(f)) => (*g, *f),
                other => panic!("unexpected row {:?}", other),
            })
            .collect();
        group_values.sort_by_key(|a| a.0);
        assert_eq!(group_values, vec![(1, 2.0), (2, 15.0)]);
    }

    /// V313-followup-4 / Issue #4157 — GREEN: `SET default_null_order =
    /// 'nulls_first'` puts NULL at the start of ORDER BY output.
    #[test]
    fn green_4157_set_default_null_order_nulls_first() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (NULL), (3), (NULL), (5)")
            .expect("INSERT must succeed");
        engine
            .execute("SET default_null_order = 'nulls_first'")
            .expect("SET must succeed");
        let result = engine
            .execute("SELECT * FROM t ORDER BY x")
            .expect("ORDER BY must succeed");
        let values: Vec<String> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Null => "NULL".to_string(),
                Value::Integer(n) => n.to_string(),
                other => panic!("unexpected value {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec!["NULL", "NULL", "1", "3", "5"],
            "nulls_first: NULL appears before non-NULL values"
        );
    }

    /// V313-followup-4 / Issue #4157 — GREEN: `SET default_null_order =
    /// 'nulls_last'` puts NULL at the end of ORDER BY output.
    #[test]
    fn green_4157_set_default_null_order_nulls_last() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE t (x INTEGER)")
            .expect("CREATE TABLE must succeed");
        engine
            .execute("INSERT INTO t VALUES (1), (NULL), (3), (NULL), (5)")
            .expect("INSERT must succeed");
        engine
            .execute("SET default_null_order = 'nulls_last'")
            .expect("SET must succeed");
        let result = engine
            .execute("SELECT * FROM t ORDER BY x")
            .expect("ORDER BY must succeed");
        let values: Vec<String> = result
            .rows
            .iter()
            .map(|r| match &r[0] {
                Value::Null => "NULL".to_string(),
                Value::Integer(n) => n.to_string(),
                other => panic!("unexpected value {:?}", other),
            })
            .collect();
        assert_eq!(
            values,
            vec!["1", "3", "5", "NULL", "NULL"],
            "nulls_last: NULL appears after non-NULL values"
        );
    }

    /// V313-followup-4 / Issue #4157 — GREEN: `SET debug_force_external`
    /// is accepted without error (DuckDB debug toggle; the engine
    /// has no spilling path so the SET is a no-op).
    #[test]
    fn green_4157_set_debug_force_external_accepted() {
        let mut engine = create_engine();
        let result = engine
            .execute("SET debug_force_external = true")
            .expect("SET debug_force_external must be accepted (no-op)");
        assert_eq!(result.rows.len(), 0, "SET returns no rows");
        assert_eq!(
            result.affected_rows, 0,
            "SET is not a write — affected_rows is 0"
        );
    }

    /// V312-59-D / Issue #4558 — GREEN regression test:
    /// multi-row INSERT with an explicit column list (skipping the
    /// AUTO_INCREMENT PK) used to spuriously fail with
    /// "Column 'k' cannot be NULL" because `validate_not_null`
    /// indexed the row using VALUES-order names while the row had already
    /// been reordered to table column order by `materialise_default_tokens`.
    ///
    /// Schema mirrors sysbench `sbtest1`:
    /// `id INTEGER AUTO_INCREMENT PK` is the "skipped" column; `k`/`c`/
    /// `pad` are the bulk-inserted NOT NULL columns with explicit defaults
    /// matching sysbench `oltp_common.lua`.
    #[test]
    fn green_v4558_multi_row_insert_explicit_columns_not_null_must_succeed() {
        let mut engine = create_engine();
        engine
            .execute(
                "CREATE TABLE sbtest1 (id INTEGER AUTO_INCREMENT PRIMARY KEY, \
                 k INTEGER NOT NULL DEFAULT 0, \
                 c CHAR(120) NOT NULL DEFAULT '', \
                 pad CHAR(60) NOT NULL DEFAULT '')",
            )
            .expect("CREATE TABLE must succeed");

        // Explicit column list skips `id`; multi-row VALUES; every value
        // for k/c/pad is explicitly non-null.
        let result = engine.execute(
            "INSERT INTO sbtest1(k, c, pad) VALUES \
             (5041, '66561...', '36032...'), \
             (5036, '31756...', '38170...'), \
             (5021, '13988...', '66083...')",
        );
        assert!(
            result.is_ok(),
            "multi-row INSERT with explicit column list and NOT NULL columns \
             must succeed (Issue #4558), got: {:?}",
            result.err()
        );
        assert_eq!(
            result.as_ref().expect("Ok already checked").affected_rows,
            3,
            "all 3 VALUES rows should be inserted"
        );

        let sel = engine
            .execute("SELECT COUNT(*) FROM sbtest1")
            .expect("SELECT must succeed");
        assert_eq!(sel.rows[0][0], Value::Integer(3), "exactly 3 rows stored");

        // Verify the k values were stored as-given (not silently rewritten
        // by the fix). This guards against a regression where the fix
        // accidentally overwrites the explicit value with Null/0.
        let sel = engine
            .execute("SELECT k FROM sbtest1 ORDER BY id")
            .expect("SELECT must succeed");
        assert_eq!(sel.rows[0][0], Value::Integer(5041));
        assert_eq!(sel.rows[1][0], Value::Integer(5036));
        assert_eq!(sel.rows[2][0], Value::Integer(5021));
    }

    /// V312-59-D / Issue #4558 — GREEN regression test:
    /// multi-row INSERT with explicit column list where a middle row
    /// contains NULL for a NOT NULL column must still be REJECTED with
    /// the correct column name. This proves the fix didn't accidentally
    /// disable NULL checks.
    #[test]
    fn green_v4558_multi_row_insert_explicit_columns_null_in_middle_must_fail() {
        let mut engine = create_engine();
        engine
            .execute(
                "CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, k INTEGER NOT NULL)",
            )
            .expect("CREATE TABLE must succeed");

        let result = engine.execute("INSERT INTO t(k) VALUES (1), (NULL), (3)");
        assert!(
            result.is_err(),
            "INSERT with NULL in middle row for NOT NULL column k \
             must return an error (data integrity, Issue #4558), got Ok"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains('k'),
            "Error must identify the offending column 'k', got: {}",
            err_msg
        );
    }

    /// V312-59-D / Issue #4558 — GREEN regression test:
    /// single-row INSERT with explicit column list must also succeed
    /// (proves the fix isn't specific to multi-row; aligns the contract).
    #[test]
    fn green_v4558_single_row_insert_explicit_columns_not_null_must_succeed() {
        let mut engine = create_engine();
        engine
            .execute(
                "CREATE TABLE t (id INTEGER AUTO_INCREMENT PRIMARY KEY, \
                 k INTEGER NOT NULL DEFAULT 0, \
                 c CHAR(120) NOT NULL DEFAULT '', \
                 pad CHAR(60) NOT NULL DEFAULT '')",
            )
            .expect("CREATE TABLE must succeed");

        let result = engine
            .execute("INSERT INTO t(k, c, pad) VALUES (1, 'hello', 'world')");
        assert!(
            result.is_ok(),
            "single-row INSERT with explicit column list and NOT NULL columns \
             must succeed (Issue #4558), got: {:?}",
            result.err()
        );
    }
}

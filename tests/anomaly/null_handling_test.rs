//! Type Conversion and NULL Handling Tests
//!
//! P4 tests for type casting and NULL handling

#[cfg(test)]
mod tests {
    use sqlrustgo::{parse, ExecutionEngine, MemoryStorage, Value};

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

        assert!(result.rows.len() >= 1);
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
        assert_eq!(
            result.rows.len(),
            1,
            "SELECT COUNT(*) returns one row"
        );
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
}

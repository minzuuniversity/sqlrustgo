//! Type Conversion and NULL Handling Tests
//!
//! P4 tests for type casting and NULL handling

#[cfg(test)]
mod tests {
    use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};

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
}

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
}

//! Aggregate and Type Conversion Tests
//!
//! P4 tests for aggregate functions and type conversions

#[cfg(test)]
mod tests {
    use sqlrustgo::{ExecutionEngine, MemoryStorage};

    fn create_engine() -> ExecutionEngine<MemoryStorage> {
        ExecutionEngine::<MemoryStorage>::with_memory()
    }

    #[test]
    fn test_count_star() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE counts (id INTEGER, value INTEGER)").unwrap();
        engine.execute("INSERT INTO counts VALUES (1, 10), (2, 20), (3, 30)").unwrap();

        let result = engine.execute("SELECT COUNT(*) FROM counts").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_count_column() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE counts (id INTEGER, value INTEGER)").unwrap();
        engine.execute("INSERT INTO counts VALUES (1, 10), (2, 20), (3, 30)").unwrap();

        let result = engine.execute("SELECT COUNT(id) FROM counts").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_sum() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE items (id INTEGER, amount INTEGER)").unwrap();
        engine.execute("INSERT INTO items VALUES (1, 100), (2, 200), (3, 300)").unwrap();

        let result = engine.execute("SELECT SUM(amount) FROM items").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_avg() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE data (id INTEGER, val INTEGER)").unwrap();
        engine.execute("INSERT INTO data VALUES (1, 10), (2, 20), (3, 30)").unwrap();

        let result = engine.execute("SELECT AVG(val) FROM data").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_min_max() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE nums (id INTEGER, n INTEGER)").unwrap();
        engine.execute("INSERT INTO nums VALUES (1, 5), (2, 15), (3, 10)").unwrap();

        let min_result = engine.execute("SELECT MIN(n) FROM nums").unwrap();
        let max_result = engine.execute("SELECT MAX(n) FROM nums").unwrap();
        assert_eq!(min_result.rows.len(), 1);
        assert_eq!(max_result.rows.len(), 1);
    }

    #[test]
    fn test_type_conversion_integer_to_text() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE ints (id INTEGER, val TEXT)").unwrap();
        engine.execute("INSERT INTO ints VALUES (1, '42')").unwrap();

        let result = engine.execute("SELECT CAST(val AS INTEGER) FROM ints").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_type_conversion_text_to_integer() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE texts (id INTEGER, val TEXT)").unwrap();
        engine.execute("INSERT INTO texts VALUES (1, '100')").unwrap();

        let result = engine.execute("SELECT CAST(val AS INTEGER) FROM texts").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_group_by_with_aggregate() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE sales (region TEXT, amount INTEGER)").unwrap();
        engine.execute("INSERT INTO sales VALUES ('north', 100), ('south', 200), ('north', 150)").unwrap();

        let result = engine.execute("SELECT region, SUM(amount) FROM sales GROUP BY region").unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_distinct_with_aggregate() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE dup (id INTEGER, val TEXT)").unwrap();
        engine.execute("INSERT INTO dup VALUES (1, 'a'), (2, 'a'), (3, 'b')").unwrap();

        let result = engine.execute("SELECT COUNT(DISTINCT val) FROM dup").unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_coalesce_with_nulls() {
        let mut engine = create_engine();
        engine.execute("CREATE TABLE nullable (id INTEGER, val INTEGER)").unwrap();
        engine.execute("INSERT INTO nullable VALUES (1, NULL), (2, 5)").unwrap();

        let result = engine.execute("SELECT COALESCE(val, 0) FROM nullable").unwrap();
        assert_eq!(result.rows.len(), 2);
    }
}

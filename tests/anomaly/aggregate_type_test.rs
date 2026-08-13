//! Aggregate and Type Conversion Tests
//!
//! P4 tests for aggregate functions and type conversions

#[cfg(test)]
mod tests {
    use sqlrustgo::{ExecutionEngine, MemoryStorage};

    use parking_lot::RwLock;
    use std::sync::Arc;

    fn create_engine() -> ExecutionEngine<MemoryStorage> {
        ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
    }

    #[test]
    fn test_count_star() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE counts (id INTEGER, value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO counts VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        let result = engine.execute("SELECT COUNT(*) FROM counts").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_count_column() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE counts (id INTEGER, value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO counts VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        let result = engine.execute("SELECT COUNT(id) FROM counts").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_sum_aggregate() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE numbers (id INTEGER, value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO numbers VALUES (1, 100), (2, 200), (3, 300)")
            .unwrap();

        let result = engine.execute("SELECT SUM(value) FROM numbers").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_avg_aggregate() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE numbers (id INTEGER, value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO numbers VALUES (1, 10), (2, 20), (3, 30)")
            .unwrap();

        let result = engine.execute("SELECT AVG(value) FROM numbers").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_min_aggregate() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE numbers (id INTEGER, value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO numbers VALUES (1, 30), (2, 10), (3, 20)")
            .unwrap();

        let result = engine.execute("SELECT MIN(value) FROM numbers").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_max_aggregate() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE numbers (id INTEGER, value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO numbers VALUES (1, 30), (2, 10), (3, 20)")
            .unwrap();

        let result = engine.execute("SELECT MAX(value) FROM numbers").unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_multiple_aggregates() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE stats (value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO stats VALUES (10), (20), (30)")
            .unwrap();

        let result = engine
            .execute("SELECT COUNT(*), SUM(value), AVG(value) FROM stats")
            .unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_aggregate_empty_table() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE empty_table (value INTEGER)")
            .unwrap();

        let result = engine
            .execute("SELECT COUNT(*), SUM(value) FROM empty_table")
            .unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_aggregate_single_row() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE single (value INTEGER)")
            .unwrap();
        engine.execute("INSERT INTO single VALUES (42)").unwrap();

        let result = engine
            .execute(
                "SELECT COUNT(*), SUM(value), AVG(value), MIN(value), MAX(value) FROM single",
            )
            .unwrap();

        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_aggregate_negative_values() {
        let mut engine = create_engine();
        engine
            .execute("CREATE TABLE negatives (value INTEGER)")
            .unwrap();
        engine
            .execute("INSERT INTO negatives VALUES (-10), (-20), (30)")
            .unwrap();

        let result = engine
            .execute("SELECT SUM(value), AVG(value) FROM negatives")
            .unwrap();

        assert_eq!(result.rows.len(), 1);
    }
}

//! Q22 cell-level comparison test.
//!
//! Compares SQLRustGo Q22 output against DuckDB baseline
//! to detect cell-level mismatches.

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use sqlrustgo::{ExecutionEngine, StorageEngine};
    use sqlrustgo_storage::{ColumnDefinition, TableInfo};
    use sqlrustgo_types::Value;
    use std::sync::Arc;

    fn setup_q22_data() -> Arc<RwLock<sqlrustgo::MemoryStorage>> {
        let storage = Arc::new(RwLock::new(sqlrustgo::MemoryStorage::new()));
        {
            let mut s = storage.write();
            s.create_table(&TableInfo {
                name: "customer".into(),
                columns: vec![
                    ColumnDefinition::new("c_custkey", "INTEGER"),
                    ColumnDefinition::new("c_name", "TEXT"),
                    ColumnDefinition::new("c_phone", "TEXT"),
                    ColumnDefinition::new("c_acctbal", "REAL"),
                ],
                ..Default::default()
            })
            .unwrap();
            s.create_table(&TableInfo {
                name: "orders".into(),
                columns: vec![
                    ColumnDefinition::new("o_orderkey", "INTEGER"),
                    ColumnDefinition::new("o_custkey", "INTEGER"),
                ],
                ..Default::default()
            })
            .unwrap();
        }
        storage
    }

    fn insert_data(storage: &Arc<RwLock<sqlrustgo::MemoryStorage>>) {
        let mut engine = ExecutionEngine::new(storage.clone());
        // Customers with matching prefix '13'
        engine
            .execute("INSERT INTO customer VALUES (1, 'C1', '13-111-1111', 100.00)")
            .unwrap();
        engine
            .execute("INSERT INTO customer VALUES (2, 'C2', '13-222-2222', 200.00)")
            .unwrap();
        engine
            .execute("INSERT INTO customer VALUES (3, 'C3', '13-333-3333', 300.00)")
            .unwrap();
        engine
            .execute("INSERT INTO customer VALUES (4, 'C4', '13-444-4444', 400.00)")
            .unwrap();
        engine
            .execute("INSERT INTO customer VALUES (5, 'C5', '13-555-5555', 500.00)")
            .unwrap();
        engine
            .execute("INSERT INTO customer VALUES (6, 'C6', '13-666-6666', 600.00)")
            .unwrap();
        // Non-matching prefix
        engine
            .execute("INSERT INTO customer VALUES (7, 'C7', '10-777-7777', 1000.00)")
            .unwrap();
        // Matching prefix '18'
        engine
            .execute("INSERT INTO customer VALUES (8, 'C8', '18-888-8888', 50.00)")
            .unwrap();
        engine
            .execute("INSERT INTO customer VALUES (9, 'C9', '18-999-9999', 800.00)")
            .unwrap();
        // Matching prefix but has order (excluded by NOT EXISTS)
        engine
            .execute("INSERT INTO customer VALUES (10, 'C10', '13-000-0000', 900.00)")
            .unwrap();
        engine.execute("INSERT INTO orders VALUES (1, 10)").unwrap();
    }

    /// DuckDB reference output for this dataset:
    ///   cntrycode='13', numcust=2, totacctbal=1100.0
    ///   cntrycode='18', numcust=1, totacctbal=800.0
    const DUCKDB_Q22: &[(&str, i64, f64)] = &[("13", 2, 1100.0), ("18", 1, 800.0)];

    const Q22_SQL: &str = "SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal FROM (SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal FROM customer WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') AND c_acctbal > (SELECT AVG(c_acctbal) FROM customer WHERE c_acctbal > 0.00 AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale GROUP BY cntrycode ORDER BY cntrycode LIMIT 100";

    #[test]
    fn test_q22_simple_data() {
        let storage = setup_q22_data();
        insert_data(&storage);
        let mut engine = ExecutionEngine::new(storage);

        let result = engine.execute(Q22_SQL).unwrap();
        assert_eq!(result.rows.len(), 2, "Q22 should return 2 rows");

        for (i, (expected_code, expected_count, expected_bal)) in DUCKDB_Q22.iter().enumerate() {
            let row = &result.rows[i];
            eprintln!("Q22 row {}: {:?}", i, row);

            // cntrycode: column 0
            let cntrycode = row[0].to_sql_string();
            assert_eq!(cntrycode, *expected_code, "Row {}: cntrycode mismatch", i);

            // numcust: column 1 (COUNT(*) -> Integer)
            match &row[1] {
                Value::Integer(n) => {
                    assert_eq!(*n, *expected_count, "Row {}: numcust mismatch", i);
                }
                other => panic!("Row {}: expected Integer for numcust, got {:?}", i, other),
            }

            // totacctbal: column 2 (SUM(REAL) -> Float)
            match &row[2] {
                Value::Float(f) => {
                    let diff = (*f - expected_bal).abs();
                    assert!(
                        diff < 0.01,
                        "Row {}: totacctbal mismatch: got {}, expected {} (diff={})",
                        i,
                        f,
                        expected_bal,
                        diff
                    );
                }
                other => panic!("Row {}: expected Float for totacctbal, got {:?}", i, other),
            }
        }
    }

    #[test]
    fn test_q22_zero_rows() {
        // Empty dataset should return 0 rows
        let storage = setup_q22_data();
        let mut engine = ExecutionEngine::new(storage);
        let result = engine.execute(Q22_SQL).unwrap();
        assert_eq!(result.rows.len(), 0, "Empty Q22 should return 0 rows");
    }

    #[test]
    fn test_q22_substr_basic() {
        let storage = setup_q22_data();
        insert_data(&storage);
        let mut engine = ExecutionEngine::new(storage);

        // SUBSTR returns first 2 chars of phone
        let r = engine
            .execute("SELECT SUBSTR(c_phone, 1, 2) FROM customer WHERE c_custkey = 1")
            .unwrap();
        assert_eq!(r.rows.len(), 1);
        assert_eq!(r.rows[0][0].to_sql_string(), "13");

        // SUBSTR with length beyond string
        let r2 = engine
            .execute("SELECT SUBSTR(c_phone, 1, 100) FROM customer WHERE c_custkey = 1")
            .unwrap();
        assert_eq!(r2.rows[0][0].to_sql_string(), "13-111-1111");

        // SUBSTR with start > length returns empty string
        let r3 = engine
            .execute("SELECT SUBSTR(c_phone, 100, 2) FROM customer WHERE c_custkey = 1")
            .unwrap();
        assert_eq!(r3.rows[0][0].to_sql_string(), "");

        // SUBSTR with start=0 should behave as start=1 (1-indexed SQL standard)
        let r4 = engine
            .execute("SELECT SUBSTR(c_phone, 0, 2) FROM customer WHERE c_custkey = 1")
            .unwrap();
        assert_eq!(r4.rows[0][0].to_sql_string(), "13");
    }

    #[test]
    fn test_q22_avg_precision() {
        // Test AVG precision on REAL columns matches DuckDB exactly
        let storage = setup_q22_data();
        insert_data(&storage);
        let mut engine = ExecutionEngine::new(storage);

        let r = engine
            .execute(
                "SELECT AVG(c_acctbal) FROM customer \
                 WHERE c_acctbal > 0.00 \
                 AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')",
            )
            .unwrap();
        assert_eq!(r.rows.len(), 1);
        match &r.rows[0][0] {
            Value::Float(f) => {
                // DuckDB avg: (100+200+300+400+500+600+50+800+900)/9 = 427.77777777777777
                let diff = (*f - 427.77777777777777).abs();
                assert!(
                    diff < 0.001,
                    "AVG mismatch: got {}, expected 427.777... (diff={})",
                    f,
                    diff
                );
            }
            other => panic!("AVG should be Float, got {:?}", other),
        }
    }
}

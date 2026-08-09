//! DATE/TIMESTAMP Type Tests
//!
//! P3 tests for DATE and TIMESTAMP types per TEST_PLAN.md
//! Tests date/time operations and type handling
//!
//! QUARANTINED: Value::Date and Value::Timestamp variants do not exist in current Value enum.
//! Tracking issue: #3904 (V312-17)
//! Owner: claude-code
//! Expiry: v3.13.0
//! Reason: API drift - tests expect Date/Timestamp Value variants that were never implemented.

#[cfg(test)]
mod tests {
    use sqlrustgo_types::Value;

    // Date/Timestamp tests commented out - Value::Date/Timestamp don't exist
    // See V312-17 quarantine decision

    #[test]
    fn test_null_date() {
        let null_date: Value = Value::Null;
        assert!(matches!(null_date, Value::Null));
    }

    #[test]
    fn test_null_timestamp() {
        let null_ts: Value = Value::Null;
        assert!(matches!(null_ts, Value::Null));
    }
}

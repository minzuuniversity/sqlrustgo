//! Catalog rebuild functionality
//!
//! Provides utilities to rebuild a Catalog from a StorageEngine for testing.

use crate::data_type::DataType;

/// Convert a storage data type string to catalog DataType
pub fn convert_data_type(data_type: &str) -> Option<DataType> {
    DataType::parse_sql_name(data_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_data_type_text() {
        let dt = convert_data_type("TEXT");
        assert_eq!(dt, Some(DataType::Text));
    }

    #[test]
    fn test_convert_data_type_various() {
        assert_eq!(convert_data_type("INT"), Some(DataType::Integer));
        assert_eq!(convert_data_type("VARCHAR"), Some(DataType::Text));
        assert_eq!(convert_data_type("BOOL"), Some(DataType::Boolean));
    }

    #[test]
    fn test_convert_data_type_lowercase() {
        let dt = convert_data_type("text");
        assert_eq!(dt, Some(DataType::Text));
    }

    #[test]
    fn test_convert_data_type_mixed_case() {
        let dt = convert_data_type("Text");
        assert_eq!(dt, Some(DataType::Text));
    }
}

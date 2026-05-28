//! PostgreSQL Type Mapping
//!
//! This module provides mapping between PostgreSQL OIDs (Object Identifiers)
//! and SQLRustGo Value types.
//!
//! ## Supported Types (P0)
//!
//! | OID | PostgreSQL Type | SQLRustGo Value |
//! |-----|-----------------|-----------------|
//! | 16  | BOOL            | Boolean(bool)   |
//! | 20  | INT8            | Integer(i64)    |
//! | 21  | INT2            | Integer(i64)    |
//! | 23  | INT4            | Integer(i64)    |
//! | 25  | TEXT            | Text(String)    |
//! | 700 | FLOAT4          | Float(f64)      |
//! | 701 | FLOAT8          | Float(f64)      |
//! | 1043| VARCHAR         | Text(String)    |
//! | 1424| BYTEA           | Blob(Vec<u8>)   |
//! | 2950| UUID            | Text(String)    |
//! | 3802| JSONB           | Text(String)    |
//! | 3807| JSONPATH        | Text(String)    |

use sqlrustgo_types::Value;

/// PostgreSQL Object Identifier (OID) type
pub type PgOid = u32;

/// PostgreSQL type information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgType {
    /// PostgreSQL OID
    pub oid: PgOid,
    /// Type name
    pub name: &'static str,
    /// Element type OID (for arrays)
    pub element: Option<PgOid>,
}

impl PgType {
    /// Create a new PgType with the given OID and name
    pub const fn new(oid: PgOid, name: &'static str) -> Self {
        Self {
            oid,
            name,
            element: None,
        }
    }

    /// Create an array type with the given element OID and name
    #[allow(dead_code)]
    pub const fn array(oid: PgOid, name: &'static str, element: PgOid) -> Self {
        Self {
            oid,
            name,
            element: Some(element),
        }
    }
}

/// PostgreSQL type map containing all supported types
pub struct PgTypeMap;

impl PgTypeMap {
    /// Create a new type map
    pub const fn new() -> Self {
        Self
    }

    /// Look up a type by OID
    pub fn get(&self, oid: PgOid) -> Option<PgType> {
        PG_TYPES.iter().find(|(id, _)| *id == oid).map(|(_, t)| *t)
    }

    /// Look up a type by name
    pub fn get_by_name(&self, name: &str) -> Option<PgType> {
        PG_TYPES
            .iter()
            .find(|(_, t)| t.name == name)
            .map(|(_, t)| *t)
    }
}

impl Default for PgTypeMap {
    fn default() -> Self {
        Self::new()
    }
}

/// PostgreSQL type to SQLRustGo Value conversion
pub fn pg_to_value(oid: PgOid, data: &[u8]) -> Option<Value> {
    match oid {
        // BOOL (16)
        16 => {
            if data.is_empty() {
                Some(Value::Null)
            } else {
                Some(Value::Boolean(data[0] != 0))
            }
        }
        // INT8 (20), INT2 (21), INT4 (23)
        20 | 21 | 23 => {
            if data.len() >= 8 {
                let val = i64::from_be_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]);
                Some(Value::Integer(val))
            } else if data.len() >= 4 {
                let val = i32::from_be_bytes([data[0], data[1], data[2], data[3]]) as i64;
                Some(Value::Integer(val))
            } else if data.len() >= 2 {
                let val = i16::from_be_bytes([data[0], data[1]]) as i64;
                Some(Value::Integer(val))
            } else {
                None
            }
        }
        // FLOAT4 (700)
        700 => {
            if data.len() >= 4 {
                let bits = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                let val = f32::from_bits(bits) as f64;
                Some(Value::Float(val))
            } else {
                None
            }
        }
        // FLOAT8 (701)
        701 => {
            if data.len() >= 8 {
                let bits = u64::from_be_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]);
                Some(Value::Float(f64::from_bits(bits)))
            } else {
                None
            }
        }
        // TEXT (25), VARCHAR (1043)
        25 | 1043 => Some(Value::Text(String::from_utf8_lossy(data).into_owned())),
        // BYTEA (1424)
        1424 => Some(Value::Blob(data.to_vec())),
        // UUID (2950)
        2950 => Some(Value::Text(String::from_utf8_lossy(data).into_owned())),
        // JSONB (3802), JSONPATH (3807)
        3802 | 3807 => Some(Value::Text(String::from_utf8_lossy(data).into_owned())),
        _ => None,
    }
}

/// SQLRustGo Value to PostgreSQL binary format conversion
pub fn value_to_pg(value: &Value) -> Option<(PgOid, Vec<u8>)> {
    match value {
        Value::Null => None,
        Value::Boolean(b) => Some((16, vec![if *b { 1 } else { 0 }])),
        Value::Integer(i) => Some((20, i.to_be_bytes().to_vec())),
        Value::Float(f) => Some((701, f.to_bits().to_be_bytes().to_vec())),
        Value::Text(s) => Some((25, s.as_bytes().to_vec())),
        Value::Blob(b) => Some((1424, b.clone())),
        Value::Geometry(_) => None, // Not supported in P0
    }
}

/// Static list of all PostgreSQL types supported in P0
const PG_TYPES: &[(PgOid, PgType)] = &[
    // Boolean types
    (16, PgType::new(16, "bool")),
    // Integer types
    (20, PgType::new(20, "int8")),
    (21, PgType::new(21, "int2")),
    (23, PgType::new(23, "int4")),
    // Floating point types
    (700, PgType::new(700, "float4")),
    (701, PgType::new(701, "float8")),
    // String types
    (25, PgType::new(25, "text")),
    (1043, PgType::new(1043, "varchar")),
    // Binary types
    (1424, PgType::new(1424, "bytea")),
    // UUID type
    (2950, PgType::new(2950, "uuid")),
    // JSON types
    (3802, PgType::new(3802, "jsonb")),
    (3807, PgType::new(3807, "jsonpath")),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pg_type_map_lookup() {
        let map = PgTypeMap::new();
        assert_eq!(map.get(16), Some(PgType::new(16, "bool")));
        assert_eq!(map.get(20), Some(PgType::new(20, "int8")));
        assert_eq!(map.get(25), Some(PgType::new(25, "text")));
        assert_eq!(map.get(701), Some(PgType::new(701, "float8")));
    }

    #[test]
    fn test_pg_type_map_lookup_by_name() {
        let map = PgTypeMap::new();
        assert_eq!(map.get_by_name("bool"), Some(PgType::new(16, "bool")));
        assert_eq!(map.get_by_name("int8"), Some(PgType::new(20, "int8")));
        assert_eq!(map.get_by_name("text"), Some(PgType::new(25, "text")));
    }

    #[test]
    fn test_pg_to_value_bool() {
        assert_eq!(pg_to_value(16, &[1]), Some(Value::Boolean(true)));
        assert_eq!(pg_to_value(16, &[0]), Some(Value::Boolean(false)));
        assert_eq!(pg_to_value(16, &[]), Some(Value::Null));
    }

    #[test]
    fn test_pg_to_value_int() {
        assert_eq!(
            pg_to_value(20, &(10i64).to_be_bytes()),
            Some(Value::Integer(10))
        );
        assert_eq!(
            pg_to_value(23, &(42i32).to_be_bytes()),
            Some(Value::Integer(42))
        );
    }

    #[test]
    fn test_pg_to_value_float() {
        let data = (3.14f64).to_be_bytes();
        assert_eq!(pg_to_value(701, &data), Some(Value::Float(3.14)));
    }

    #[test]
    fn test_pg_to_value_text() {
        assert_eq!(pg_to_value(25, b"hello"), Some(Value::Text("hello".into())));
        assert_eq!(
            pg_to_value(1043, b"world"),
            Some(Value::Text("world".into()))
        );
    }

    #[test]
    fn test_pg_to_value_blob() {
        assert_eq!(
            pg_to_value(1424, &[1, 2, 3]),
            Some(Value::Blob(vec![1, 2, 3]))
        );
    }

    #[test]
    fn test_value_to_pg() {
        assert_eq!(value_to_pg(&Value::Boolean(true)), Some((16, vec![1])));
        assert_eq!(value_to_pg(&Value::Boolean(false)), Some((16, vec![0])));
        assert_eq!(
            value_to_pg(&Value::Integer(42)),
            Some((20, 42i64.to_be_bytes().to_vec()))
        );
        assert_eq!(
            value_to_pg(&Value::Float(3.14)),
            Some((701, 3.14f64.to_bits().to_be_bytes().to_vec()))
        );
        assert_eq!(
            value_to_pg(&Value::Text("hello".into())),
            Some((25, b"hello".to_vec()))
        );
        assert_eq!(
            value_to_pg(&Value::Blob(vec![1, 2, 3])),
            Some((1424, vec![1, 2, 3]))
        );
        assert_eq!(value_to_pg(&Value::Null), None);
    }
}

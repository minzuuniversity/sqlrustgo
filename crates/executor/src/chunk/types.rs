//! Primitive SQL types for columnar execution.
use serde::{Deserialize, Serialize};

/// Primitive SQL data types supported by Chunk storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PrimitiveType {
    /// 32-bit signed integer
    Int32 = 0,
    /// 64-bit signed integer
    Int64 = 1,
    /// 64-bit IEEE 754 floating point
    Float64 = 2,
    /// UTF-8 variable-length string
    String = 3,
    /// Fixed-size binary blob
    Blob = 4,
    /// Boolean stored as u8 (0=false, 1=true)
    Bool = 5,
    /// 32-bit unsigned integer
    UInt32 = 6,
    /// Date as days since 1970-01-01
    Date = 7,
    /// Timestamp as microseconds since 1970-01-01 00:00:00
    Timestamp = 8,
}

impl PrimitiveType {
    /// Returns the size in bytes for fixed-size types, or 0 for variable-size types.
    #[inline]
    pub fn fixed_size(&self) -> usize {
        match self {
            PrimitiveType::Int32
            | PrimitiveType::UInt32
            | PrimitiveType::Bool
            | PrimitiveType::Date => 4,
            PrimitiveType::Int64 | PrimitiveType::Float64 | PrimitiveType::Timestamp => 8,
            PrimitiveType::String | PrimitiveType::Blob => 0,
        }
    }

    /// Returns true if this type has variable size (String, Blob).
    #[inline]
    pub fn is_variable_size(&self) -> bool {
        matches!(self, PrimitiveType::String | PrimitiveType::Blob)
    }

    /// Returns the default null value for this type.
    pub fn null_value(&self) -> PrimitiveValue {
        match self {
            PrimitiveType::Int32 => PrimitiveValue::Int32(0),
            PrimitiveType::Int64 => PrimitiveValue::Int64(0),
            PrimitiveType::Float64 => PrimitiveValue::Float64(f64::NAN),
            PrimitiveType::String => PrimitiveValue::String(String::new()),
            PrimitiveType::Blob => PrimitiveValue::Blob(Vec::new()),
            PrimitiveType::Bool => PrimitiveValue::Bool(false),
            PrimitiveType::UInt32 => PrimitiveValue::UInt32(0),
            PrimitiveType::Date => PrimitiveValue::Date(0),
            PrimitiveType::Timestamp => PrimitiveValue::Timestamp(0),
        }
    }
}

/// A single primitive value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveValue {
    Int32(i32),
    Int64(i64),
    Float64(f64),
    String(String),
    Blob(Vec<u8>),
    Bool(bool),
    UInt32(u32),
    Date(i32),
    Timestamp(i64), // microseconds since 1970-01-01 00:00:00
}

impl PrimitiveValue {
    /// Returns the type of this value.
    pub fn type_id(&self) -> PrimitiveType {
        match self {
            PrimitiveValue::Int32(_) => PrimitiveType::Int32,
            PrimitiveValue::Int64(_) => PrimitiveType::Int64,
            PrimitiveValue::Float64(_) => PrimitiveType::Float64,
            PrimitiveValue::String(_) => PrimitiveType::String,
            PrimitiveValue::Blob(_) => PrimitiveType::Blob,
            PrimitiveValue::Bool(_) => PrimitiveType::Bool,
            PrimitiveValue::UInt32(_) => PrimitiveType::UInt32,
            PrimitiveValue::Date(_) => PrimitiveType::Date,
            PrimitiveValue::Timestamp(_) => PrimitiveType::Timestamp,
        }
    }

    /// Returns true if this value represents null.
    pub fn is_null(&self) -> bool {
        match self {
            PrimitiveValue::Float64(f) => f.is_nan(),
            PrimitiveValue::String(s) => s.is_empty(),
            PrimitiveValue::Blob(b) => b.is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_type_fixed_size() {
        assert_eq!(PrimitiveType::Int32.fixed_size(), 4);
        assert_eq!(PrimitiveType::Int64.fixed_size(), 8);
        assert_eq!(PrimitiveType::Float64.fixed_size(), 8);
        assert_eq!(PrimitiveType::String.fixed_size(), 0);
        assert_eq!(PrimitiveType::Blob.fixed_size(), 0);
    }

    #[test]
    fn test_primitive_type_is_variable_size() {
        assert!(!PrimitiveType::Int32.is_variable_size());
        assert!(!PrimitiveType::Float64.is_variable_size());
        assert!(PrimitiveType::String.is_variable_size());
        assert!(PrimitiveType::Blob.is_variable_size());
    }

    #[test]
    fn test_primitive_value_type_id() {
        assert_eq!(PrimitiveValue::Int32(0).type_id(), PrimitiveType::Int32);
        assert_eq!(PrimitiveValue::Int64(0).type_id(), PrimitiveType::Int64);
        assert_eq!(
            PrimitiveValue::Float64(0.0).type_id(),
            PrimitiveType::Float64
        );
        assert_eq!(
            PrimitiveValue::String("".into()).type_id(),
            PrimitiveType::String
        );
    }
}

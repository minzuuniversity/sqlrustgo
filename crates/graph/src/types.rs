//! Core graph types: identifiers, properties, errors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Unique identifier for a node within a graph.
///
/// Monotonically increasing u64; never reused within a single store instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Returns the underlying raw id.
    #[inline]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "n{}", self.0)
    }
}

/// Unique identifier for an edge within a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EdgeId(pub u64);

impl EdgeId {
    /// Returns the underlying raw id.
    #[inline]
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e{}", self.0)
    }
}

/// A node label (e.g. "Person", "Document"). Multiple labels per node allowed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Label(pub String);

impl Label {
    /// Construct a label from a string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Label {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Property value — supports common scalar and structured types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    /// UTF-8 string.
    String(String),
    /// 64-bit signed integer.
    Int(i64),
    /// 64-bit IEEE float.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// Byte array (e.g. embeddings).
    Bytes(Vec<u8>),
    /// Nested list (homogeneous element type recommended).
    List(Vec<PropertyValue>),
    /// Null sentinel.
    Null,
}

impl PropertyValue {
    /// Type name for diagnostic messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            PropertyValue::String(_) => "String",
            PropertyValue::Int(_) => "Int",
            PropertyValue::Float(_) => "Float",
            PropertyValue::Bool(_) => "Bool",
            PropertyValue::Bytes(_) => "Bytes",
            PropertyValue::List(_) => "List",
            PropertyValue::Null => "Null",
        }
    }

    /// True if value is the Null variant.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, PropertyValue::Null)
    }
}

impl From<&str> for PropertyValue {
    fn from(s: &str) -> Self {
        PropertyValue::String(s.to_string())
    }
}
impl From<String> for PropertyValue {
    fn from(s: String) -> Self {
        PropertyValue::String(s)
    }
}
impl From<i64> for PropertyValue {
    fn from(v: i64) -> Self {
        PropertyValue::Int(v)
    }
}
impl From<i32> for PropertyValue {
    fn from(v: i32) -> Self {
        PropertyValue::Int(v as i64)
    }
}
impl From<f64> for PropertyValue {
    fn from(v: f64) -> Self {
        PropertyValue::Float(v)
    }
}
impl From<bool> for PropertyValue {
    fn from(v: bool) -> Self {
        PropertyValue::Bool(v)
    }
}

/// Ordered key/value property bag.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyMap {
    /// Insertion-ordered entries (BTreeMap would sort keys but loses insertion order).
    entries: Vec<(String, PropertyValue)>,
}

impl PropertyMap {
    /// Empty property bag.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Build from an iterator of (key, value) pairs.
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<PropertyValue>,
    {
        Self {
            entries: pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// Returns the value for `key` if present.
    pub fn get(&self, key: &str) -> Option<&PropertyValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Inserts or replaces a key, returning the previous value if any.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl Into<PropertyValue>,
    ) -> Option<PropertyValue> {
        let key = key.into();
        let value = value.into();
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| k == &key) {
            return Some(std::mem::replace(&mut slot.1, value));
        }
        self.entries.push((key, value));
        None
    }

    /// Number of properties.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if no properties.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over (key, value) pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PropertyValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// HashMap view (cloned) for callers that don't care about order.
    pub fn to_hash_map(&self) -> HashMap<String, PropertyValue> {
        self.entries.iter().cloned().collect()
    }
}

/// Errors returned by the graph engine.
#[derive(Debug, Error)]
pub enum GraphError {
    /// Node referenced does not exist.
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),
    /// Edge referenced does not exist.
    #[error("edge not found: {0}")]
    EdgeNotFound(EdgeId),
    /// Source or target node of a new edge does not exist.
    #[error("edge endpoint missing: src={src} dst={dst}")]
    EdgeEndpointMissing {
        /// Source endpoint NodeId that does not exist.
        src: NodeId,
        /// Target endpoint NodeId that does not exist.
        dst: NodeId,
    },
    /// I/O error during disk persistence or WAL append.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization error (bincode / serde_json).
    #[error("serialization error: {0}")]
    Serde(String),
    /// Cypher parse error.
    #[error("cypher parse error at line {line} col {col}: {message}")]
    CypherParse {
        /// Line number (1-based).
        line: usize,
        /// Column number (1-based).
        col: usize,
        /// Human-readable message.
        message: String,
    },
    /// Cypher execution error (semantic, not syntactic).
    #[error("cypher execution error: {0}")]
    CypherExec(String),
    /// Underlying storage engine error.
    #[error("storage error: {0}")]
    Storage(String),
}

impl From<bincode::Error> for GraphError {
    fn from(e: bincode::Error) -> Self {
        GraphError::Serde(e.to_string())
    }
}

impl From<serde_json::Error> for GraphError {
    fn from(e: serde_json::Error) -> Self {
        GraphError::Serde(e.to_string())
    }
}

/// Result alias.
pub type GraphResult<T> = Result<T, GraphError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_display() {
        let n = NodeId(42);
        assert_eq!(n.to_string(), "n42");
        assert_eq!(n.raw(), 42);
    }

    #[test]
    fn edge_id_display() {
        let e = EdgeId(7);
        assert_eq!(e.to_string(), "e7");
    }

    #[test]
    fn property_value_type_names() {
        assert_eq!(PropertyValue::from("hi").type_name(), "String");
        assert_eq!(PropertyValue::from(1_i64).type_name(), "Int");
        assert_eq!(PropertyValue::from(2.5_f64).type_name(), "Float");
        assert_eq!(PropertyValue::from(true).type_name(), "Bool");
        assert_eq!(PropertyValue::Null.type_name(), "Null");
    }

    #[test]
    fn property_map_insertion_order() {
        let mut m = PropertyMap::new();
        m.insert("z", 1_i64);
        m.insert("a", 2_i64);
        m.insert("m", 3_i64);
        let keys: Vec<&str> = m.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["z", "a", "m"]);
    }

    #[test]
    fn property_map_insert_overwrites_and_returns_old() {
        let mut m = PropertyMap::new();
        assert!(m.insert("k", 1_i64).is_none());
        let prev = m.insert("k", 2_i64);
        assert_eq!(prev, Some(PropertyValue::Int(1)));
        assert_eq!(m.get("k"), Some(&PropertyValue::Int(2)));
    }

    #[test]
    fn label_from_str_and_string() {
        let a: Label = "Person".into();
        let b: Label = String::from("Person").into();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "Person");
    }

    #[test]
    fn null_is_null() {
        assert!(PropertyValue::Null.is_null());
        assert!(!PropertyValue::from(0_i64).is_null());
    }
}

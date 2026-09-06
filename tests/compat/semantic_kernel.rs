use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SemanticType {
    Null,
    Boolean,
    Integer,
    Float,
    Text(Collation),
    Blob,
    Point,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Collation {
    Binary,
    NoCase,
    RTrim,
}

impl Collation {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "NOCASE" => Collation::NoCase,
            "RTRIM" => Collation::RTrim,
            _ => Collation::Binary,
        }
    }

    pub fn eq(&self, a: &str, b: &str) -> bool {
        match self {
            Collation::Binary => a == b,
            Collation::NoCase => a.to_uppercase() == b.to_uppercase(),
            Collation::RTrim => a.trim_end() == b.trim_end(),
        }
    }

    pub fn cmp(&self, a: &str, b: &str) -> std::cmp::Ordering {
        match self {
            Collation::Binary => a.cmp(b),
            Collation::NoCase => a.to_uppercase().cmp(&b.to_uppercase()),
            Collation::RTrim => a.trim_end().cmp(b.trim_end()),
        }
    }
}

impl Default for Collation {
    fn default() -> Self {
        Collation::Binary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemanticValue {
    pub type_: SemanticType,
    pub raw: String,
}

impl SemanticValue {
    pub fn null() -> Self {
        Self {
            type_: SemanticType::Null,
            raw: "NULL".to_string(),
        }
    }

    pub fn integer(n: i64) -> Self {
        Self {
            type_: SemanticType::Integer,
            raw: n.to_string(),
        }
    }

    pub fn text(s: &str) -> Self {
        Self {
            type_: SemanticType::Text(Collation::default()),
            raw: s.to_string(),
        }
    }

    pub fn text_with_collation(s: &str, collation: Collation) -> Self {
        Self {
            type_: SemanticType::Text(collation),
            raw: s.to_string(),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.type_, SemanticType::Null)
    }

    pub fn equals(&self, other: &SemanticValue) -> SemanticResult {
        if self.is_null() || other.is_null() {
            return SemanticResult::Null;
        }
        match (&self.type_, &other.type_) {
            (SemanticType::Integer, SemanticType::Integer) => {
                SemanticResult::Bool(self.raw == other.raw)
            }
            (SemanticType::Text(c1), SemanticType::Text(c2)) => {
                SemanticResult::Bool(c1.eq(&self.raw, &other.raw) && c2.eq(&self.raw, &other.raw))
            }
            _ => SemanticResult::Bool(false),
        }
    }

    pub fn not_equals(&self, other: &SemanticValue) -> SemanticResult {
        match self.equals(other) {
            SemanticResult::Bool(b) => SemanticResult::Bool(!b),
            SemanticResult::Null => SemanticResult::Null,
        }
    }

    pub fn less_than(&self, other: &SemanticValue) -> SemanticResult {
        if self.is_null() || other.is_null() {
            return SemanticResult::Null;
        }
        match (&self.type_, &other.type_) {
            (SemanticType::Integer, SemanticType::Integer) => {
                let a: i64 = self.raw.parse().unwrap_or(0);
                let b: i64 = other.raw.parse().unwrap_or(0);
                SemanticResult::Bool(a < b)
            }
            (SemanticType::Text(c1), SemanticType::Text(c2)) => {
                SemanticResult::Bool(c1.cmp(&self.raw, &other.raw).is_lt())
            }
            _ => SemanticResult::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticResult {
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub struct SemanticKernel {
    pub values: HashMap<String, SemanticValue>,
    pub collation: Collation,
}

impl SemanticKernel {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            collation: Collation::default(),
        }
    }

    pub fn with_collation(collation: Collation) -> Self {
        Self {
            values: HashMap::new(),
            collation,
        }
    }

    pub fn set(&mut self, name: &str, value: SemanticValue) {
        self.values.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&SemanticValue> {
        self.values.get(name)
    }

    pub fn evaluate(&self, condition: &str) -> SemanticResult {
        let parts: Vec<&str> = condition.split_whitespace().collect();
        if parts.len() != 3 {
            return SemanticResult::Null;
        }
        let left = self.values.get(parts[0]);
        let right = self.values.get(parts[2]);
        match (left, right) {
            (Some(l), Some(r)) => match parts[1] {
                "=" => l.equals(r),
                "!=" | "<>" => l.not_equals(r),
                "<" => l.less_than(r),
                ">" => r.less_than(l),
                _ => SemanticResult::Null,
            },
            _ => SemanticResult::Null,
        }
    }
}

impl Default for SemanticKernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collation_binary() {
        assert!(Collation::Binary.eq("hello", "hello"));
        assert!(!Collation::Binary.eq("hello", "HELLO"));
    }

    #[test]
    fn test_collation_nocase() {
        assert!(Collation::NoCase.eq("hello", "HELLO"));
        assert!(Collation::NoCase.eq("Hello", "hELLo"));
    }

    #[test]
    fn test_collation_rtrim() {
        assert!(Collation::RTrim.eq("hello", "hello   "));
        assert!(!Collation::RTrim.eq("hello", "hello world"));
    }

    #[test]
    fn test_value_null_equals() {
        let null_val = SemanticValue::null();
        let int_val = SemanticValue::integer(42);
        assert_eq!(null_val.equals(&int_val), SemanticResult::Null);
        assert_eq!(int_val.equals(&null_val), SemanticResult::Null);
    }

    #[test]
    fn test_value_integer_equals() {
        let a = SemanticValue::integer(42);
        let b = SemanticValue::integer(42);
        let c = SemanticValue::integer(43);
        assert_eq!(a.equals(&b), SemanticResult::Bool(true));
        assert_eq!(a.equals(&c), SemanticResult::Bool(false));
    }

    #[test]
    fn test_value_text_equals() {
        let a = SemanticValue::text("hello");
        let b = SemanticValue::text("hello");
        let c = SemanticValue::text("world");
        assert_eq!(a.equals(&b), SemanticResult::Bool(true));
        assert_eq!(a.equals(&c), SemanticResult::Bool(false));
    }

    #[test]
    fn test_value_text_nocase() {
        let a = SemanticValue::text_with_collation("Hello", Collation::NoCase);
        let b = SemanticValue::text_with_collation("HELLO", Collation::NoCase);
        assert_eq!(a.equals(&b), SemanticResult::Bool(true));
    }

    #[test]
    fn test_kernel_evaluate() {
        let mut kernel = SemanticKernel::new();
        kernel.set("a", SemanticValue::integer(42));
        kernel.set("b", SemanticValue::integer(42));
        kernel.set("c", SemanticValue::integer(43));

        assert_eq!(kernel.evaluate("a = b"), SemanticResult::Bool(true));
        assert_eq!(kernel.evaluate("a = c"), SemanticResult::Bool(false));
        assert_eq!(kernel.evaluate("a < c"), SemanticResult::Bool(true));
    }

    #[test]
    fn test_kernel_null_comparison() {
        let mut kernel = SemanticKernel::new();
        kernel.set("a", SemanticValue::null());
        kernel.set("b", SemanticValue::integer(42));

        assert_eq!(kernel.evaluate("a = b"), SemanticResult::Null);
        assert_eq!(kernel.evaluate("b = a"), SemanticResult::Null);
    }

    #[test]
    fn test_value_hash_consistency() {
        let v1 = SemanticValue::integer(42);
        let v2 = SemanticValue::integer(42);
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        v1.hash(&mut hasher1);
        v2.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());
    }
}
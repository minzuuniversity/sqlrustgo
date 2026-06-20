//! Test Harness

pub struct TestHarness {
    name: String,
}

impl TestHarness {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct TestFixture {
    data: std::collections::HashMap<String, Vec<Vec<sqlrustgo_types::Value>>>,
}

impl TestFixture {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    pub fn add_table(mut self, name: &str, rows: Vec<Vec<sqlrustgo_types::Value>>) -> Self {
        self.data.insert(name.to_string(), rows);
        self
    }

    pub fn get_table(&self, name: &str) -> Option<&Vec<Vec<sqlrustgo_types::Value>>> {
        self.data.get(name)
    }

    pub fn table_names(&self) -> Vec<&String> {
        self.data.keys().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_new() {
        let harness = TestHarness::new("test");
        assert_eq!(harness.name(), "test");
    }

    #[test]
    fn test_test_fixture_new() {
        let fixture = TestFixture::new();
        assert!(fixture.is_empty());
    }

    #[test]
    fn test_test_fixture_add_table() {
        let fixture =
            TestFixture::new().add_table("users", vec![vec![sqlrustgo_types::Value::Integer(1)]]);
        assert!(fixture.get_table("users").is_some());
    }

    #[test]
    fn test_test_fixture_table_names() {
        let fixture = TestFixture::new()
            .add_table("users", vec![])
            .add_table("orders", vec![]);
        let names = fixture.table_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_harness_name() {
        let harness = TestHarness::new("my_test_harness");
        assert_eq!(harness.name(), "my_test_harness");
    }

    #[test]
    fn test_fixture_get_empty_table() {
        let fixture = TestFixture::new().add_table("empty_table", vec![]);
        assert!(fixture.get_table("empty_table").is_some());
    }

    #[test]
    fn test_fixture_get_nonexistent() {
        let fixture = TestFixture::new();
        assert!(fixture.get_table("nonexistent").is_none());
    }

    #[test]
    fn test_fixture_empty_after_creation() {
        let fixture = TestFixture::new();
        assert!(fixture.is_empty());
        assert_eq!(fixture.table_names().len(), 0);
    }

    #[test]
    fn test_fixture_is_empty_false_after_add() {
        let fixture = TestFixture::new().add_table("t", vec![]);
        assert!(!fixture.is_empty());
    }

    #[test]
    fn test_test_fixture_multiple_additions() {
        let fixture = TestFixture::new()
            .add_table("t1", vec![vec![sqlrustgo_types::Value::Integer(1)]])
            .add_table("t2", vec![vec![sqlrustgo_types::Value::Text("hello".to_string())]])
            .add_table("t3", vec![]);
        assert_eq!(fixture.table_names().len(), 3);
        assert!(!fixture.is_empty());
    }

    #[test]
    fn test_test_fixture_default() {
        let fixture = TestFixture::default();
        assert!(fixture.is_empty());
    }

    #[test]
    fn test_test_fixture_default_add_table() {
        let fixture = TestFixture::default().add_table("t", vec![]);
        assert!(fixture.get_table("t").is_some());
    }
}

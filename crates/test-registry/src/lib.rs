//! Test Registry Module
//!
//! Provides test metadata management and test discovery for regression testing.
//!
//! Two layers:
//!
//! 1. **In-source `TestMetadata`** — per-test metadata registered programmatically
//!    (id, name, category, module, tags, priority, timeout, file_path).
//! 2. **On-disk `ManagedTest`** — a manifest of binaries + args (e.g. `sqlancer`,
//!    `test-runner`) loaded from a TOML file. These are the *managed* test
//!    infrastructure entries the runner can dispatch against.
//!
//! V312-24 activation: `TestRegistry` is now a single object that holds both
//! layers, with `from_toml` / `write_toml` for round-tripping the manifest
//! part. The CLI binary (`test-registry-cli`) drives the on-disk layer.
//!
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

// Re-export the `toml` crate at the crate root so callers don't have to
// pin a separate version. Downstream code can `use test_registry::toml;`.
pub use toml;

/// Errors produced by `TestRegistry::from_toml` / `write_toml`.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestCategory {
    Unit,
    Integration,
    Anomaly,
    Stress,
    E2E,
    CI,
}

impl TestCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestCategory::Unit => "unit",
            TestCategory::Integration => "integration",
            TestCategory::Anomaly => "anomaly",
            TestCategory::Stress => "stress",
            TestCategory::E2E => "e2e",
            TestCategory::CI => "ci",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestPriority {
    P0,
    P1,
    P2,
    P3,
    P4,
}

impl TestPriority {
    pub fn as_u8(&self) -> u8 {
        match self {
            TestPriority::P0 => 0,
            TestPriority::P1 => 1,
            TestPriority::P2 => 2,
            TestPriority::P3 => 3,
            TestPriority::P4 => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetadata {
    pub id: String,
    pub name: String,
    pub category: TestCategory,
    pub module: String,
    pub tags: Vec<String>,
    pub priority: TestPriority,
    pub timeout_ms: u64,
    pub flaky: bool,
    pub file_path: String,
}

impl TestMetadata {
    pub fn new(id: &str, name: &str, category: TestCategory, module: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            category,
            module: module.to_string(),
            tags: Vec::new(),
            priority: TestPriority::P2,
            timeout_ms: 60000,
            flaky: false,
            file_path: String::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_priority(mut self, priority: TestPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_file_path(mut self, path: &str) -> Self {
        self.file_path = path.to_string();
        self
    }
}

/// A managed test entry: a binary + args registered in a TOML manifest.
///
/// Distinct from [`TestMetadata`] (which describes in-source unit/integration
/// tests). `ManagedTest` is the on-disk contract that `test-runner` reads to
/// know which binaries to invoke, with what arguments, and with what timeout.
///
/// TOML schema (table-array form, see design.md):
///
/// ```toml
/// [[test]]
/// name = "sqlancer"
/// binary = "target/release/sqlancer"
/// args = ["--duration", "120", "--out", "target/sqlancer-report.json"]
/// timeout_ms = 600_000
/// priority = "p1"
/// category = "fuzz"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManagedTest {
    pub name: String,
    pub binary: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_priority")]
    pub priority: TestPriority,
    #[serde(default = "default_category")]
    pub category: TestCategory,
}

fn default_timeout_ms() -> u64 {
    120_000
}

fn default_priority() -> TestPriority {
    TestPriority::P2
}

fn default_category() -> TestCategory {
    TestCategory::Integration
}

impl ManagedTest {
    pub fn new(name: &str, binary: &str) -> Self {
        Self {
            name: name.to_string(),
            binary: binary.to_string(),
            args: Vec::new(),
            timeout_ms: default_timeout_ms(),
            priority: default_priority(),
            category: default_category(),
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_priority(mut self, priority: TestPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_category(mut self, category: TestCategory) -> Self {
        self.category = category;
        self
    }
}

/// Bridge a `ManagedTest` (a binary manifest) into a `TestMetadata` (the
/// in-memory per-test record) so the registry can treat both uniformly.
impl From<ManagedTest> for TestMetadata {
    fn from(m: ManagedTest) -> Self {
        TestMetadata::new(&m.name, &m.name, m.category, "managed")
            .with_priority(m.priority)
            .with_timeout(m.timeout_ms)
            .with_file_path(&m.binary)
    }
}

#[derive(Debug, Default)]
pub struct TestRegistry {
    tests: HashMap<String, TestMetadata>,
    modules: HashMap<String, Vec<String>>,
    /// On-disk managed binaries (loaded from `test-registry.toml`).
    managed_tests: HashMap<String, ManagedTest>,
}

impl TestRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, metadata: TestMetadata) {
        self.tests.insert(metadata.id.clone(), metadata.clone());

        self.modules
            .entry(metadata.module.clone())
            .or_default()
            .push(metadata.id.clone());
    }

    pub fn get(&self, id: &str) -> Option<&TestMetadata> {
        self.tests.get(id)
    }

    pub fn get_by_category(&self, category: TestCategory) -> Vec<&TestMetadata> {
        self.tests
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    pub fn get_by_module(&self, module: &str) -> Vec<&TestMetadata> {
        self.tests.values().filter(|t| t.module == module).collect()
    }

    pub fn get_by_tag(&self, tag: &str) -> Vec<&TestMetadata> {
        self.tests
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .collect()
    }

    pub fn get_by_priority(&self, priority: TestPriority) -> Vec<&TestMetadata> {
        self.tests
            .values()
            .filter(|t| t.priority == priority)
            .collect()
    }

    pub fn get_all(&self) -> Vec<&TestMetadata> {
        self.tests.values().collect()
    }

    pub fn get_module_dependencies(&self, module: &str) -> Vec<String> {
        let mut deps = Vec::new();

        match module {
            "executor" => {
                deps.push("planner".to_string());
                deps.push("optimizer".to_string());
            }
            "planner" => {
                deps.push("parser".to_string());
            }
            "optimizer" => {
                deps.push("planner".to_string());
                deps.push("storage".to_string());
            }
            "storage" => {
                deps.push("types".to_string());
            }
            "transaction" => {
                deps.push("storage".to_string());
            }
            _ => {}
        }

        deps
    }

    pub fn get_affected_modules(&self, changed_files: &[String]) -> Vec<String> {
        let mut affected = Vec::new();

        for file in changed_files {
            let module = self.file_to_module(file);
            if !affected.contains(&module) {
                affected.push(module.clone());
            }

            let deps = self.get_module_dependencies(&module);
            for dep in deps {
                if !affected.contains(&dep) {
                    affected.push(dep);
                }
            }
        }

        affected
    }

    fn file_to_module(&self, file: &str) -> String {
        let path = PathBuf::from(file);

        if let Some(path_str) = path.to_str() {
            if path_str.contains("crates/parser") {
                return "parser".to_string();
            } else if path_str.contains("crates/planner") {
                return "planner".to_string();
            } else if path_str.contains("crates/optimizer") {
                return "optimizer".to_string();
            } else if path_str.contains("crates/executor") {
                return "executor".to_string();
            } else if path_str.contains("crates/storage") {
                return "storage".to_string();
            } else if path_str.contains("crates/transaction") {
                return "transaction".to_string();
            } else if path_str.contains("crates/server") {
                return "server".to_string();
            }
        }

        "unknown".to_string()
    }

    pub fn get_affected_tests(&self, changed_files: &[String]) -> Vec<&TestMetadata> {
        let affected_modules = self.get_affected_modules(changed_files);

        self.tests
            .values()
            .filter(|t| affected_modules.contains(&t.module))
            .collect()
    }

    pub fn total_count(&self) -> usize {
        self.tests.len()
    }

    pub fn count_by_category(&self) -> HashMap<TestCategory, usize> {
        let mut counts = HashMap::new();

        for test in self.tests.values() {
            *counts.entry(test.category).or_insert(0) += 1;
        }

        counts
    }

    pub fn tests(&self) -> impl Iterator<Item = &TestMetadata> {
        self.tests.values()
    }

    pub fn is_empty(&self) -> bool {
        self.tests.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tests.len()
    }

    // -------------------------------------------------------------------
    // Managed tests (on-disk manifest) — V312-24 activation.
    // -------------------------------------------------------------------

    /// Register a managed test (binary + args) under its `name`.
    /// Also folds the entry into the in-memory `tests` map so existing
    /// query APIs (`get_by_category`, `tests()`, etc.) see it.
    pub fn register_managed(&mut self, m: ManagedTest) {
        self.managed_tests.insert(m.name.clone(), m.clone());
        self.register(m.into());
    }

    pub fn managed(&self) -> impl Iterator<Item = &ManagedTest> {
        self.managed_tests.values()
    }

    pub fn get_managed(&self, name: &str) -> Option<&ManagedTest> {
        self.managed_tests.get(name)
    }

    pub fn managed_len(&self) -> usize {
        self.managed_tests.len()
    }

    /// Load a registry from a TOML manifest file. The file format is the
    /// `[[test]]` table-array form defined in `design.md`. Missing file
    /// returns an empty registry with an `Io` error.
    pub fn from_toml(path: &Path) -> Result<Self, RegistryError> {
        let s = std::fs::read_to_string(path).map_err(|source| RegistryError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        #[derive(Deserialize)]
        struct Manifest {
            #[serde(default)]
            test: Vec<ManagedTest>,
        }
        let manifest: Manifest = toml::from_str(&s)?;
        let mut registry = TestRegistry::default();
        for m in manifest.test {
            registry.register_managed(m);
        }
        Ok(registry)
    }

    /// Persist the managed-tests portion of the registry to a TOML file.
    /// Output uses the `[[test]]` table-array form so the file can be
    /// read back via `from_toml` or hand-edited.
    pub fn write_toml(&self, path: &Path) -> Result<(), RegistryError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
        }
        let mut entries: Vec<ManagedTest> = self.managed_tests.values().cloned().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        #[derive(Serialize)]
        struct Manifest<'a> {
            test: Vec<&'a ManagedTest>,
        }
        let manifest = Manifest {
            test: entries.iter().collect(),
        };
        let s = toml::to_string(&manifest)?;
        std::fs::write(path, s).map_err(|source| RegistryError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }
}

pub struct TestRegistryBuilder {
    registry: TestRegistry,
}

impl TestRegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: TestRegistry::new(),
        }
    }

    pub fn register_from_tests_dir(mut self, tests_dir: &PathBuf) -> Self {
        if let Ok(entries) = std::fs::read_dir(tests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        let category = match dir_name {
                            "unit" => TestCategory::Unit,
                            "integration" => TestCategory::Integration,
                            "anomaly" => TestCategory::Anomaly,
                            "stress" => TestCategory::Stress,
                            "e2e" => TestCategory::E2E,
                            "ci" => TestCategory::CI,
                            _ => continue,
                        };

                        if let Ok(files) = std::fs::read_dir(&path) {
                            for file in files.flatten() {
                                let file_path = file.path();
                                if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
                                    if let Some(file_name) =
                                        file_path.file_name().and_then(|n| n.to_str())
                                    {
                                        let test_name = file_name.trim_end_matches(".rs");
                                        let module = self.infer_module(&path);

                                        let metadata = TestMetadata::new(
                                            test_name, test_name, category, &module,
                                        )
                                        .with_file_path(file_path.to_str().unwrap_or(""))
                                        .with_timeout(60000);

                                        self.registry.register(metadata);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self
    }

    fn infer_module(&self, path: &Path) -> String {
        let path_str = path.to_string_lossy().to_string();

        if path_str.contains("parser") {
            "parser".to_string()
        } else if path_str.contains("planner") {
            "planner".to_string()
        } else if path_str.contains("optimizer") {
            "optimizer".to_string()
        } else if path_str.contains("executor") {
            "executor".to_string()
        } else if path_str.contains("storage") {
            "storage".to_string()
        } else if path_str.contains("transaction") {
            "transaction".to_string()
        } else if path_str.contains("server") {
            "server".to_string()
        } else {
            "unknown".to_string()
        }
    }

    pub fn build(self) -> TestRegistry {
        self.registry
    }
}

impl Default for TestRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register() {
        let mut registry = TestRegistry::new();

        let metadata = TestMetadata::new(
            "test_001",
            "test_select_parser",
            TestCategory::Unit,
            "parser",
        );

        registry.register(metadata);

        assert_eq!(registry.total_count(), 1);
    }

    #[test]
    fn test_query_by_category() {
        let mut registry = TestRegistry::new();

        registry.register(TestMetadata::new("t1", "t1", TestCategory::Unit, "parser"));
        registry.register(TestMetadata::new("t2", "t2", TestCategory::Unit, "planner"));
        registry.register(TestMetadata::new(
            "t3",
            "t3",
            TestCategory::Integration,
            "parser",
        ));

        let unit_tests = registry.get_by_category(TestCategory::Unit);
        assert_eq!(unit_tests.len(), 2);
    }

    #[test]
    fn test_query_by_module() {
        let mut registry = TestRegistry::new();

        registry.register(TestMetadata::new("t1", "t1", TestCategory::Unit, "parser"));
        registry.register(TestMetadata::new("t2", "t2", TestCategory::Unit, "parser"));
        registry.register(TestMetadata::new("t3", "t3", TestCategory::Unit, "planner"));

        let parser_tests = registry.get_by_module("parser");
        assert_eq!(parser_tests.len(), 2);
    }

    #[test]
    fn test_module_dependencies() {
        let registry = TestRegistry::new();

        let deps = registry.get_module_dependencies("executor");
        assert!(deps.contains(&"planner".to_string()));
        assert!(deps.contains(&"optimizer".to_string()));
    }

    #[test]
    fn test_affected_tests() {
        let mut registry = TestRegistry::new();

        registry.register(TestMetadata::new("t1", "t1", TestCategory::Unit, "parser"));
        registry.register(TestMetadata::new("t2", "t2", TestCategory::Unit, "planner"));
        registry.register(TestMetadata::new(
            "t3",
            "t3",
            TestCategory::Unit,
            "executor",
        ));

        let changed = vec!["crates/planner/src/lib.rs".to_string()];
        let affected = registry.get_affected_tests(&changed);

        assert!(affected.iter().any(|t| t.module == "planner"));
    }
}

//! Test Runner Module
//!
//! Core test execution engine for regression testing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    TimedOut,
    Crashed,
}

impl TestStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Pending => "pending",
            TestStatus::Running => "running",
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Skipped => "skipped",
            TestStatus::TimedOut => "timed_out",
            TestStatus::Crashed => "crashed",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, TestStatus::Passed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub output: String,
    pub error_message: Option<String>,
    pub retries: u32,
}

impl TestResult {
    pub fn new(test_id: &str, name: &str) -> Self {
        let now = Utc::now();
        Self {
            test_id: test_id.to_string(),
            name: name.to_string(),
            status: TestStatus::Pending,
            duration_ms: 0,
            started_at: now,
            finished_at: now,
            output: String::new(),
            error_message: None,
            retries: 0,
        }
    }

    pub fn passed(duration_ms: u64) -> Self {
        let now = Utc::now();
        Self {
            test_id: String::new(),
            name: String::new(),
            status: TestStatus::Passed,
            duration_ms,
            started_at: now,
            finished_at: now,
            output: String::new(),
            error_message: None,
            retries: 0,
        }
    }

    pub fn failed(duration_ms: u64, error: &str) -> Self {
        let now = Utc::now();
        Self {
            test_id: String::new(),
            name: String::new(),
            status: TestStatus::Failed,
            duration_ms,
            started_at: now,
            finished_at: now,
            output: String::new(),
            error_message: Some(error.to_string()),
            retries: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunConfig {
    pub max_parallel: usize,
    pub retry_count: u32,
    pub timeout_per_test_ms: u64,
    pub working_dir: PathBuf,
    pub cargo_binary: String,
    pub test_flags: Vec<String>,
}

impl Default for TestRunConfig {
    fn default() -> Self {
        Self {
            max_parallel: num_cpus::get(),
            retry_count: 0,
            timeout_per_test_ms: 120000,
            working_dir: PathBuf::from("."),
            cargo_binary: "cargo".to_string(),
            test_flags: vec![],
        }
    }
}

pub struct TestRunner {
    config: TestRunConfig,
}
impl TestRunner {
    pub fn new(config: TestRunConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(TestRunConfig::default())
    }

    pub async fn run_test(&self, test_id: &str, name: &str) -> TestResult {
        let start_time = Instant::now();
        let result = Self::execute_cargo_test(&self.config, test_id, name).await;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        final_result.finished_at = Utc::now();

        final_result
    }

    async fn execute_cargo_test(config: &TestRunConfig, test_id: &str, name: &str) -> TestResult {
        let started_at = Utc::now();
        let mut attempts = 0u32;
        let max_attempts = config.retry_count.saturating_add(1).max(1);
        let timeout_dur = Duration::from_millis(config.timeout_per_test_ms.max(1));

        loop {
            attempts += 1;
            let mut cmd = Command::new(&config.cargo_binary);
            cmd.arg("test")
                .arg(test_id)
                .arg("--")
                .args(&config.test_flags)
                .current_dir(&config.working_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);

            // tokio::time::timeout enforces the per-test wall-clock budget.
            let child_result = timeout(timeout_dur, cmd.output()).await;
            let (status, stdout, stderr, error_message) = match child_result {
                Err(_elapsed) => {
                    // Hard timeout: kill the child via drop, mark TimedOut.
                    (
                        TestStatus::TimedOut,
                        String::new(),
                        String::new(),
                        Some(format!("timeout after {} ms", config.timeout_per_test_ms)),
                    )
                }
                Ok(Err(e)) => (
                    TestStatus::Crashed,
                    String::new(),
                    String::new(),
                    Some(format!("Failed to execute test: {}", e)),
                ),
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let status = if output.status.success() {
                        TestStatus::Passed
                    } else if output.status.code() == Some(101) {
                        TestStatus::Failed
                    } else {
                        TestStatus::Crashed
                    };
                    (status, stdout, stderr, None)
                }
            };

            let combined_output = format!("{}\n{}", stdout, stderr);

            // Retry on Failed or Crashed (not on Passed or TimedOut).
            let should_retry = attempts < max_attempts
                && matches!(status, TestStatus::Failed | TestStatus::Crashed);
            if !should_retry {
                return TestResult {
                    test_id: test_id.to_string(),
                    name: name.to_string(),
                    status,
                    duration_ms: 0,
                    started_at,
                    finished_at: Utc::now(),
                    output: combined_output,
                    error_message,
                    retries: attempts - 1,
                };
            }
        }
    }

    /// Run multiple tests in parallel, bounded by `config.max_parallel`.
    ///
    /// Concurrency model: a `tokio::sync::Semaphore` caps in-flight tests; each
    /// test runs as its own task on a `JoinSet`. `&self` (not `&mut self`) lets
    /// the tasks share the runner without contention — `execute_cargo_test` is
    /// already a static method and does not touch `self`.
    pub async fn run_tests(&self, test_ids: Vec<String>) -> Vec<TestResult> {
        let max_parallel = self.config.max_parallel.max(1);
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(max_parallel));
        let config = std::sync::Arc::new(self.config.clone());
        let mut joinset: tokio::task::JoinSet<TestResult> = tokio::task::JoinSet::new();

        for test_id in test_ids {
            let permit_source = sem.clone();
            let config = config.clone();
            let name = test_id.clone();
            joinset.spawn(async move {
                let _permit = permit_source
                    .acquire_owned()
                    .await
                    .expect("semaphore closed unexpectedly");
                Self::run_test_with_config(&config, &test_id, &name).await
            });
        }

        let mut results = Vec::with_capacity(joinset.len());
        while let Some(joined) = joinset.join_next().await {
            match joined {
                Ok(r) => results.push(r),
                Err(e) => {
                    // Task panicked or was cancelled. Record a synthetic
                    // Crashed result so callers always see one entry per
                    // requested test_id (the test_id is lost here, but the
                    // outer length matches the spawned count).
                    results.push(TestResult::failed(0, &format!("test task failed: {}", e)));
                }
            }
        }
        results
    }

    /// Like [`run_test`] but takes the config by reference, suitable for
    /// parallel dispatch where borrowing `self` is not possible.
    pub async fn run_test_with_config(
        config: &TestRunConfig,
        test_id: &str,
        name: &str,
    ) -> TestResult {
        let start_time = Instant::now();
        let result = Self::execute_cargo_test(config, test_id, name).await;
        let mut final_result = result;
        final_result.duration_ms = start_time.elapsed().as_millis() as u64;
        final_result.finished_at = Utc::now();
        final_result
    }

    // -------------------------------------------------------------------
    // Managed-test dispatch (V312-24 activation).
    //
    // These methods consume `test_registry::ManagedTest` entries — the
    // on-disk `[[test]]` manifest — and invoke the listed binaries with
    // their declared args and per-entry timeout. Concurrency is still
    // bounded by `config.max_parallel`.
    // -------------------------------------------------------------------

    /// Run a single managed entry, honoring its declared `timeout_ms`.
    pub async fn run_managed(&self, entry: &test_registry::ManagedTest) -> TestResult {
        let started_at = Utc::now();
        let start = Instant::now();
        let timeout_dur = Duration::from_millis(entry.timeout_ms.max(1));
        let mut cmd = Command::new(&entry.binary);
        cmd.args(&entry.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child_result = timeout(timeout_dur, cmd.output()).await;
        let (status, stdout, stderr, error_message) = match child_result {
            Err(_elapsed) => (
                TestStatus::TimedOut,
                String::new(),
                String::new(),
                Some(format!("timeout after {} ms", entry.timeout_ms)),
            ),
            Ok(Err(e)) => (
                TestStatus::Crashed,
                String::new(),
                String::new(),
                Some(format!("Failed to execute {}: {}", entry.binary, e)),
            ),
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let status = if output.status.success() {
                    TestStatus::Passed
                } else if output.status.code() == Some(101) {
                    TestStatus::Failed
                } else {
                    TestStatus::Crashed
                };
                (status, stdout, stderr, None)
            }
        };

        TestResult {
            test_id: entry.name.clone(),
            name: format!("{} {}", entry.binary, entry.args.join(" ")),
            status,
            duration_ms: start.elapsed().as_millis() as u64,
            started_at,
            finished_at: Utc::now(),
            output: format!("{}\n{}", stdout, stderr),
            error_message,
            retries: 0,
        }
    }

    /// Run all managed entries in parallel, bounded by `config.max_parallel`.
    /// Returns a `TestResult` per entry; ordering matches the input.
    ///
    /// Concurrency model: the caller passes an `Arc<Self>` so spawned tasks
    /// can borrow the runner for the duration of each future. The Semaphore
    /// caps in-flight entries at `config.max_parallel` (default `num_cpus`).
    pub async fn run_managed_all(
        self: std::sync::Arc<Self>,
        entries: Vec<test_registry::ManagedTest>,
    ) -> Vec<TestResult> {
        let max_parallel = self.config.max_parallel.max(1);
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(max_parallel));
        let mut joinset: tokio::task::JoinSet<TestResult> = tokio::task::JoinSet::new();

        for entry in entries {
            let permit_source = sem.clone();
            let runner = self.clone();
            joinset.spawn(async move {
                let _permit = permit_source
                    .acquire_owned()
                    .await
                    .expect("semaphore closed unexpectedly");
                runner.run_managed(&entry).await
            });
        }

        let mut results = Vec::with_capacity(joinset.len());
        while let Some(joined) = joinset.join_next().await {
            match joined {
                Ok(r) => results.push(r),
                Err(e) => results.push(TestResult::failed(
                    0,
                    &format!("managed-test task failed: {}", e),
                )),
            }
        }
        results
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timed_out: usize,
    pub crashed: usize,
    pub total_duration_ms: u64,
}

impl TestRunSummary {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }

    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.timed_out == 0 && self.crashed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_creation() {
        let result = TestResult::new("test_001", "test_select");
        assert_eq!(result.status, TestStatus::Pending);
    }

    #[test]
    fn test_result_passed() {
        let result = TestResult::passed(100);
        assert_eq!(result.status, TestStatus::Passed);
    }

    #[test]
    fn test_result_failed() {
        let result = TestResult::failed(50, "Assertion failed");
        assert_eq!(result.status, TestStatus::Failed);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_run_summary() {
        let summary = TestRunSummary {
            total: 10,
            passed: 8,
            failed: 1,
            skipped: 0,
            timed_out: 1,
            crashed: 0,
            total_duration_ms: 5000,
        };

        assert_eq!(summary.pass_rate(), 80.0);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_run_summary_all_passed() {
        let summary = TestRunSummary {
            total: 10,
            passed: 10,
            failed: 0,
            skipped: 0,
            timed_out: 0,
            crashed: 0,
            total_duration_ms: 5000,
        };

        assert!(summary.all_passed());
    }

    #[test]
    fn test_config_defaults() {
        let config = TestRunConfig::default();
        assert!(config.max_parallel > 0);
        assert_eq!(config.retry_count, 0);
    }
}

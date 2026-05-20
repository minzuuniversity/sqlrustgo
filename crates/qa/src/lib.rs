//! SQLRustGo QA Module
//!
//! AI-powered testing tools including:
//! - AI Test Generator: LLM-based test case generation
//! - Intelligent Test Selection: ML-based test selection for PRs
//! - Bug Triage AI: Automated bug classification and routing
//! - Flaky Test Detector: Statistical flaky test detection

pub mod ai_test_generator;
pub mod bug_triage;
pub mod flaky_detector;
pub mod intelligent_test_selector;

pub use ai_test_generator::{AITestGenerator, TestCase, TestContext};
pub use bug_triage::{BugReport, BugTriageAI, Severity, TriageResult};
pub use flaky_detector::{FlakyAction, FlakyTestDetector};
pub use intelligent_test_selector::IntelligentTestSelector;

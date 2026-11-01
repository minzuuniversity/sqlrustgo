//! P0-1: D6 Test Inventory Gate validation
//!
//! **Issue**: #2874, #3847
//! **Source**: COMPREHENSIVE_FEATURE_TRACKING.md (DAG Node N1)
//! **Change**: openspec/changes/p0-1-integrate-35-tests
//!
//! From issue #3847 the D6b gate no longer re-executes the test suite:
//! it is a **static consistency check** only. The authoritative pass/fail
//! counts are produced by the `test` job (`check_test_target_execution.sh`
//! plus its job log).
//!
//! These tests therefore assert two things at the same time:
//!   1. the inventory is still collected and still produces evidence, and
//!   2. the script can no longer execute a test binary — that is the
//!      property #3847 introduced, so it needs a regression guard.

fn script_path() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_test_inventory.sh")
}

fn script_content() -> String {
    std::fs::read_to_string(script_path()).expect("check_test_inventory.sh not found")
}

/// Strip shell comments: whole-line comments and trailing `# ...` comments.
fn strip_comments(content: &str) -> String {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let cut = line.find(" #").unwrap_or(line.len());
        out.push(&line[..cut]);
    }
    out.join("\n")
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every `cargo <subcommand>` occurrence in the comment-free script.
///
/// Returns the subcommand token (lowercase letters and dashes) that follows
/// the word `cargo`. `command -v cargo >/dev/null` yields no token because the
/// next non-space character is not a word character.
fn cargo_subcommands(content: &str) -> Vec<String> {
    let mut found = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let is_cargo = i + 4 < chars.len()
            && chars[i] == 'c'
            && chars[i + 1..i + 5] == ['a', 'r', 'g', 'o'];
        if is_cargo {
            let before_ok = i == 0 || !is_word_char(chars[i - 1]);
            let after = chars[i + 5];
            let after_ok = !after.is_alphanumeric() && after != '_';
            if before_ok && after_ok {
                let mut j = i + 5;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let start = j;
                while j < chars.len() && (chars[j].is_ascii_lowercase() || chars[j] == '-') {
                    j += 1;
                }
                if j > start {
                    found.push(chars[start..j].iter().collect::<String>());
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    found
}

#[test]
fn test_d6_collects_all_test_files() {
    let content = script_content();
    assert!(content.contains("TEST_FILES"), "must collect TEST_FILES");
    assert!(content.contains("find tests"), "must find all test files");
}

#[test]
fn test_d6_generates_evidence() {
    let content = script_content();
    assert!(
        content.contains("d6_test_inventory.json"),
        "must write evidence JSON"
    );
    assert!(content.contains("artifacts/gate"), "must use artifacts dir");
}

#[test]
fn test_d6_handles_subdir_tests() {
    let content = script_content();
    // D6b resolves subdir test names via a PATH_TO_NAME map built from
    // [[test]] blocks in Cargo.toml (so tests/ci/buffer_pool_test.rs uses the
    // canonical name "buffer_pool_test", not the path-derived "ci_buffer_pool_test").
    assert!(
        content.contains("PATH_TO_NAME"),
        "must build PATH_TO_NAME map from [[test]] blocks"
    );
    assert!(
        content.contains("awk") && content.contains("name = "),
        "must parse [[test]] name = entries"
    );
}

#[test]
fn test_d6_delegates_to_manifest_honesty_gate() {
    let content = script_content();
    // The static verdict comes from the manifest honesty gate, which already
    // implements C1..C6: one source of truth instead of a drifting copy.
    assert!(
        content.contains("check_test_manifest_honesty.sh"),
        "must delegate to check_test_manifest_honesty.sh"
    );
    assert!(
        content.contains("cargo metadata"),
        "must cross-check the declared targets against cargo metadata"
    );
}

#[test]
fn test_d6_is_static_only_marker() {
    let content = script_content();
    let marker = "D6B_MODE: STATIC_ONLY_NO_CARGO";
    assert_eq!(
        content.matches(marker).count(),
        1,
        "the static-only guard marker must appear exactly once"
    );
}

#[test]
fn test_d6_never_executes_a_test_binary() {
    let stripped = strip_comments(&script_content());
    for sub in cargo_subcommands(&stripped) {
        assert!(
            !matches!(
                sub.as_str(),
                "test" | "run" | "bench" | "nextest" | "miri" | "fuzz"
            ),
            "D6b must never execute a test binary, found: cargo {}",
            sub
        );
    }
}

#[test]
fn test_d6_cargo_subcommand_whitelist() {
    let stripped = strip_comments(&script_content());
    for sub in cargo_subcommands(&stripped) {
        assert!(
            sub == "metadata" || sub == "--version",
            "cargo subcommand '{}' is not on the whitelist (metadata, --version)",
            sub
        );
    }
}

#[test]
fn test_d6_has_no_timeout_wrapper() {
    let stripped = strip_comments(&script_content());
    assert!(
        !stripped.contains("timeout "),
        "the static-only gate must not wrap anything in a timeout runner"
    );
    assert!(
        !stripped.contains("TIMEOUT"),
        "the static-only gate must not report a TIMEOUT status"
    );
}

#[test]
fn test_d6_reports_degraded_when_cargo_unavailable() {
    let content = script_content();
    // "could not measure" must never be reported as "measured and fine".
    assert!(
        content.contains("DEGRADED"),
        "must report DEGRADED when the cargo cross-check cannot be performed"
    );
    assert!(content.contains("exit 2"), "must use exit code 2 for DEGRADED");
}

#[test]
fn test_d6_executable() {
    let script = script_path();
    let metadata = std::fs::metadata(&script).expect("script must exist");
    let permissions = metadata.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = permissions.mode();
        assert!(mode & 0o111 != 0, "must be executable");
    }
}

#[test]
fn test_d6_smoke_gate_is_minimal_real_execution() {
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/gate/check_d6_smoke.sh");
    let content = std::fs::read_to_string(&script).expect("check_d6_smoke.sh not found");
    assert_eq!(
        content.matches("D6_SMOKE_MODE: MINIMAL_REAL_EXECUTION").count(),
        1,
        "the smoke gate must carry its guard marker exactly once"
    );
    assert!(
        content.contains("SMOKE_TARGETS=("),
        "must declare an explicit smoke target list"
    );
    assert!(
        content.contains("PER_TARGET_TIMEOUT"),
        "must bound each smoke target with a timeout"
    );
    // The smoke set must stay small: a growing list is how the #3847 cost
    // problem comes back.
    let listed = content
        .split("SMOKE_TARGETS=(")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .unwrap_or("");
    let count = listed
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .count();
    assert!(
        (3..=6).contains(&count),
        "the smoke set must stay between 3 and 6 targets, found {}",
        count
    );
}

#[test]
fn test_d6_smoke_gate_is_wired_into_postcheck() {
    let workflow = std::env::current_dir()
        .unwrap()
        .join(".gitea/workflows/ci.yml");
    let content = std::fs::read_to_string(&workflow).expect("ci.yml not found");
    assert!(
        content.contains("check_d6_smoke.sh"),
        "the postcheck job must run the D6 smoke gate"
    );
    assert!(
        content.contains("D6 Smoke Gate"),
        "the smoke gate must be a visible, named CI step"
    );
}

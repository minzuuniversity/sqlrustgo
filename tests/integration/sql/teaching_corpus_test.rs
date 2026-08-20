//! Tests for the SQLRustGo teaching SQL corpus (V312-56B / #4252).
//!
//! V312-56B acceptance criteria require:
//! - `tests/compat/teaching_sql_v3_12/manifest.yml` exists and lists
//!   every file's oracle, expected status, owner, and stage.
//! - Every file has at least a SQLite oracle (MySQL/PostgreSQL optional).
//! - Every FAIL/SKIP entry carries `issue_link`, `owner`, `expiry`,
//!   and a close boundary.
//! - The corpus does not silently skip any test via the Rust ignore attribute.
//!
//! These tests are intentionally meta-tests: they verify manifest
//! integrity, file presence, and the structural oracle contract
//! before any execution. The actual oracle comparison (SQLite vs
//! SQLRustGo row sets) is delegated to the gate script
//! `scripts/gate/check_sqllogictest_v312.sh --corpus teaching`.
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const CORPUS_DIR: &str = "tests/compat/teaching_sql_v3_12";
const MANIFEST_PATH: &str = "tests/compat/teaching_sql_v3_12/manifest.yml";

fn corpus_root() -> PathBuf {
    // tests are run from the workspace root, so this is a relative path.
    PathBuf::from(CORPUS_DIR)
}

fn manifest_path() -> PathBuf {
    PathBuf::from(MANIFEST_PATH)
}

#[test]
fn test_teaching_corpus_manifest_exists() {
    assert!(
        manifest_path().exists(),
        "teaching corpus manifest missing: {}",
        MANIFEST_PATH
    );
}

#[test]
fn test_teaching_corpus_manifest_required_keys() {
    let raw = fs::read_to_string(manifest_path()).expect("read manifest");
    // Minimal hand-rolled YAML check: avoid pulling in `serde_yaml` for
    // this meta-test. The required top-level fields must appear at
    // column zero so they are not list items.
    for required in ["version:", "owner:", "stage:", "oracles:", "files:"] {
        assert!(
            raw.lines().any(|l| l.starts_with(required)),
            "manifest missing top-level key '{required}'"
        );
    }
}

#[test]
fn test_teaching_corpus_files_match_manifest() {
    // Build the on-disk file set
    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    for entry in walk_sql(corpus_root()) {
        if let Ok(rel) = entry.strip_prefix(corpus_root()) {
            on_disk.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    // Parse manifest entries (simple parse: paths appear as `- path:` lines)
    let raw = fs::read_to_string(manifest_path()).expect("read manifest");
    let mut in_files = false;
    let mut in_entry = false;
    let mut current_path: Option<String> = None;
    let mut manifest_paths: BTreeSet<String> = BTreeSet::new();
    for line in raw.lines() {
        if line.starts_with("files:") {
            in_files = true;
            continue;
        }
        if in_files {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") {
                // start a new entry
                if let Some(prev) = current_path.take() {
                    manifest_paths.insert(prev);
                }
                in_entry = true;
                // "- path: foo/bar.sql" or "- path: \"foo bar.sql\""
                if let Some(rest) = trimmed.trim_start_matches("- ").strip_prefix("path:") {
                    current_path = Some(unquote(rest.trim()));
                }
            } else if in_entry && !trimmed.is_empty() && !line.starts_with(' ') {
                // top-level key like `ignore_rules:` — close the section
                if let Some(prev) = current_path.take() {
                    manifest_paths.insert(prev);
                }
                in_files = false;
                in_entry = false;
            }
        }
    }
    if let Some(prev) = current_path.take() {
        manifest_paths.insert(prev);
    }

    let missing_files: Vec<_> = manifest_paths.difference(&on_disk).cloned().collect();
    let extra_files: Vec<_> = on_disk.difference(&manifest_paths).cloned().collect();

    assert!(
        missing_files.is_empty(),
        "manifest references files that do not exist on disk: {missing_files:?}"
    );
    assert!(
        extra_files.is_empty(),
        "disk has SQL files not listed in manifest: {extra_files:?}"
    );
}

#[test]
fn test_teaching_corpus_files_have_headers() {
    for entry in walk_sql(corpus_root()) {
        let raw = fs::read_to_string(&entry).expect("read sql file");
        let has_name = raw.lines().any(|l| l.trim_start().starts_with("# name:"));
        let has_expect = raw.lines().any(|l| l.trim_start().starts_with("# expect:"));
        assert!(has_name, "{}: missing `# name:` header", entry.display());
        assert!(
            has_expect,
            "{}: missing `# expect:` header (PASS/FAIL/SKIP)",
            entry.display()
        );
    }
}

#[test]
fn test_teaching_corpus_fail_entries_have_governance_fields() {
    let raw = fs::read_to_string(manifest_path()).expect("read manifest");
    let entries = parse_entries(&raw);
    for entry in entries {
        if entry.expected != "FAIL" && entry.expected != "SKIP" {
            continue;
        }
        assert!(
            entry.issue_link.is_some(),
            "{}: FAIL/SKIP entries must carry issue_link",
            entry.path
        );
        assert!(
            entry.owner.is_some(),
            "{}: FAIL/SKIP entries must carry owner",
            entry.path
        );
        assert!(
            entry.expiry.is_some(),
            "{}: FAIL/SKIP entries must carry expiry",
            entry.path
        );
        // Block placeholders
        if let Some(link) = &entry.issue_link {
            assert_ne!(
                link, "TBD",
                "{}: issue_link is still TBD; resolve before claiming V312-56B PASS",
                entry.path
            );
        }
        if let Some(o) = &entry.owner {
            assert_ne!(o, "TBD", "{}: owner is still TBD", entry.path);
        }
    }
}

#[test]
fn test_teaching_corpus_oracles_declared() {
    let raw = fs::read_to_string(manifest_path()).expect("read manifest");
    // The `oracles:` block must list at least sqlite
    let mut in_oracles = false;
    let mut saw_sqlite = false;
    for line in raw.lines() {
        if line.starts_with("oracles:") {
            in_oracles = true;
            continue;
        }
        if in_oracles {
            if !line.starts_with(' ') && !line.is_empty() {
                // left the oracles section
                break;
            }
            if line.contains("sqlite") {
                saw_sqlite = true;
            }
        }
    }
    assert!(
        saw_sqlite,
        "manifest `oracles:` block must declare sqlite (V312-56B minimum)"
    );
}

// -------- helpers --------

fn walk_sql(root: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(&root) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_sql(path));
        } else if path.extension().map(|s| s == "sql").unwrap_or(false) {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn unquote(s: &str) -> String {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Default)]
struct Entry {
    path: String,
    oracle: Option<String>,
    expected: String,
    issue_link: Option<String>,
    owner: Option<String>,
    expiry: Option<String>,
}

/// Minimal YAML entry parser. We avoid pulling `serde_yaml` so this
/// stays dependency-free and runs even when the lockfile is mid-edit.
/// The grammar we accept: a list under `files:` whose items are
///   - path: "..."
///     oracle: sqlite
///     expected: PASS
///     issue_link: ...
///     owner: ...
///     expiry: ...
fn parse_entries(raw: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut in_files = false;
    let mut current: Option<Entry> = None;
    for line in raw.lines() {
        if line.starts_with("files:") {
            in_files = true;
            continue;
        }
        if in_files {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") {
                if let Some(prev) = current.take() {
                    entries.push(prev);
                }
                if let Some(rest) = trimmed.trim_start_matches("- ").strip_prefix("path:") {
                    current = Some(Entry {
                        path: unquote(rest.trim()),
                        expected: "PASS".to_string(),
                        ..Default::default()
                    });
                }
            } else if let Some(ref mut e) = current {
                if let Some(v) = trimmed.strip_prefix("oracle:") {
                    e.oracle = Some(unquote(v.trim()));
                } else if let Some(v) = trimmed.strip_prefix("expected:") {
                    e.expected = unquote(v.trim());
                } else if let Some(v) = trimmed.strip_prefix("issue_link:") {
                    e.issue_link = Some(unquote(v.trim()));
                } else if let Some(v) = trimmed.strip_prefix("owner:") {
                    e.owner = Some(unquote(v.trim()));
                } else if let Some(v) = trimmed.strip_prefix("expiry:") {
                    e.expiry = Some(unquote(v.trim()));
                } else if !trimmed.is_empty() {
                    // left the files section
                    if let Some(prev) = current.take() {
                        entries.push(prev);
                    }
                    in_files = false;
                }
            }
        }
    }
    if let Some(prev) = current.take() {
        entries.push(prev);
    }
    entries
}

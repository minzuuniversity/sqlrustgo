//! Detect whether a CLI positional argument looks like a DB path
//! (vs a flag or subcommand name) for the implicit `sqlrustgo <path>` alias.

#![allow(dead_code)]

use std::path::Path;

const SUBCOMMAND_NAMES: &[&str] = &[
    "serve",
    "exec",
    "repl",
    "bench",
    "gmp",
    "diag",
    "backup",
    "restore",
    "cli",
    "soak",
    "sqlite",
    "help",
    "--help",
    "-h",
    "--version",
    "-V",
];

pub fn looks_like_db_path(arg: &str) -> bool {
    if arg.is_empty() {
        return false;
    }
    // Any --flag is not a DB path
    if arg.starts_with('-') {
        return false;
    }
    // Known subcommand names are not DB paths
    if SUBCOMMAND_NAMES.contains(&arg) {
        return false;
    }
    // Suffix-based detection (sqlite3 muscle memory)
    if arg.ends_with(".db") || arg.ends_with(".sqlite") || arg.ends_with(".sqlite3") {
        return true;
    }
    // Path separator present
    if arg.contains('/') || arg.contains('\\') {
        return true;
    }
    // Already-existing directory
    if Path::new(arg).is_dir() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_suffix_detected() {
        assert!(looks_like_db_path("edu.db"));
        assert!(looks_like_db_path("test.sqlite"));
    }

    #[test]
    fn slash_path_detected() {
        assert!(looks_like_db_path("./edu.db"));
        assert!(looks_like_db_path("/tmp/edu.db"));
    }

    #[test]
    fn existing_directory_detected() {
        // tmp dir always exists
        assert!(looks_like_db_path("/tmp"));
    }

    #[test]
    fn flags_not_detected() {
        assert!(!looks_like_db_path("--help"));
        assert!(!looks_like_db_path("-h"));
        assert!(!looks_like_db_path("--version"));
    }

    #[test]
    fn subcommand_names_not_detected() {
        assert!(!looks_like_db_path("serve"));
        assert!(!looks_like_db_path("repl"));
        assert!(!looks_like_db_path("sqlite"));
    }

    #[test]
    fn empty_string_not_detected() {
        assert!(!looks_like_db_path(""));
    }
}

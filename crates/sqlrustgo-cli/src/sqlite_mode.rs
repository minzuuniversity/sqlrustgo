//! SQLite-compatible mode for CLI.
//!
//! Provides SqliteMode struct that holds execution engine + state for
//! SQLite-like CLI operations.

use crate::error::CliError;
use crate::output::{OutputMode, OutputTarget};
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SqliteState {
    pub mode: OutputMode,
    pub headers: bool,
    pub timer: bool,
    pub explain: bool,
    pub output: OutputTarget,
}

impl Default for SqliteState {
    fn default() -> Self {
        Self {
            mode: OutputMode::Table,
            headers: true,
            timer: false,
            explain: false,
            output: OutputTarget::Stdout,
        }
    }
}

#[allow(dead_code)]
pub struct SqliteMode {
    pub engine: ExecutionEngine<FileStorage>,
    pub state: SqliteState,
    pub error_seen: bool,
    pub continue_on_error: bool,
    pub db_path: PathBuf,
}

#[allow(dead_code)]
impl SqliteMode {
    pub fn open(db: &Path, state: SqliteState, continue_on_error: bool) -> Result<Self, CliError> {
        // Resolve storage path: if db is a file path, use its parent + basename as subdir.
        // If db is a directory, use as-is.
        let dir = if db.is_dir() {
            db.to_path_buf()
        } else {
            let parent = db.parent().unwrap_or(Path::new("."));
            let name = db.file_name().unwrap_or_else(|| std::ffi::OsStr::new("db"));
            parent.join(name)
        };

        std::fs::create_dir_all(&dir).map_err(|e| {
            CliError::Io(format!(
                "cannot create DB directory {}: {}",
                dir.display(),
                e
            ))
        })?;

        let storage = Arc::new(parking_lot::RwLock::new(
            FileStorage::new(dir.clone()).map_err(|e| {
                CliError::Io(format!("cannot open storage {}: {}", dir.display(), e))
            })?,
        ));
        let engine = ExecutionEngine::with_catalog(
            storage,
            Arc::new(parking_lot::RwLock::new(sqlrustgo_catalog::Catalog::new(
                "main",
            ))),
        );

        Ok(Self {
            engine,
            state,
            error_seen: false,
            continue_on_error,
            db_path: dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_new_db_path() {
        let tmp = std::env::temp_dir().join("v31257_open_creates");
        let _ = std::fs::remove_dir_all(&tmp);
        let mode = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open");
        assert!(!mode.error_seen);
        assert!(!mode.continue_on_error);
        // Path should exist after open
        assert!(tmp.exists(), "open should create the DB path");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn open_existing_db_path_succeeds() {
        let tmp = std::env::temp_dir().join("v31257_open_existing");
        let _ = std::fs::remove_dir_all(&tmp);
        // First open creates
        let _ = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open1");
        // Second open succeeds on existing
        let _ = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open2");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn default_state_has_table_mode_headers_on() {
        let s = SqliteState::default();
        assert_eq!(s.mode, OutputMode::Table);
        assert!(s.headers);
        assert!(!s.timer);
        assert!(!s.explain);
        assert_eq!(s.output, OutputTarget::Stdout);
    }
}

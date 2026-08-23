//! JSON → BINT v3 lazy migration.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableFormat {
    Binary,
    Json,
    Missing,
}

/// Detect whether a table's files on disk are BIN, JSON, or absent.
pub fn detect_table_format(data_dir: &Path, table: &str) -> TableFormat {
    let root_bin = data_dir.join(format!("{}.root.bin", table));
    if root_bin.exists() {
        return TableFormat::Binary;
    }
    let json = data_dir.join(format!("{}.json", table));
    if json.exists() {
        return TableFormat::Json;
    }
    TableFormat::Missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_json_only() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("t1.json"), b"{}").unwrap();
        assert!(matches!(detect_table_format(dir.path(), "t1"), TableFormat::Json));
    }

    #[test]
    fn test_detect_bin_only() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("t1.root.bin"), b"\0\0\0\0").unwrap();
        assert!(matches!(detect_table_format(dir.path(), "t1"), TableFormat::Binary));
    }

    #[test]
    fn test_detect_both_prefers_bin() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("t1.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("t1.root.bin"), b"\0\0\0\0").unwrap();
        assert!(matches!(detect_table_format(dir.path(), "t1"), TableFormat::Binary));
    }

    #[test]
    fn test_detect_missing() {
        let dir = tempdir().unwrap();
        assert!(matches!(detect_table_format(dir.path(), "missing"), TableFormat::Missing));
    }
}

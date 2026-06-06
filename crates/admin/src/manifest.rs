use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub version: u32,
    pub created_at: String,
    pub sqlrustgo_version: String,
    pub data_files: Vec<FileEntry>,
    pub wal_file: Option<FileEntry>,
    pub total_size_bytes: u64,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
            sqlrustgo_version: env!("CARGO_PKG_VERSION").to_string(),
            data_files: Vec::new(),
            wal_file: None,
            total_size_bytes: 0,
        }
    }

    pub fn scan(data_dir: &Path, wal_path: Option<&Path>) -> std::io::Result<Self> {
        let mut manifest = Self::new();
        let mut total: u64 = 0;

        if data_dir.exists() {
            for entry in walk_files(data_dir)? {
                let rel = entry
                    .strip_prefix(data_dir)
                    .unwrap_or(&entry)
                    .to_string_lossy()
                    .replace('\\', "/");
                let meta = fs::metadata(&entry)?;
                let size = meta.len();
                let sha = sha256_file(&entry)?;
                total += size;
                manifest.data_files.push(FileEntry {
                    path: rel,
                    size,
                    sha256: sha,
                });
            }
        }

        if let Some(wp) = wal_path {
            if wp.exists() {
                let meta = fs::metadata(wp)?;
                let size = meta.len();
                let sha = sha256_file(wp)?;
                total += size;
                manifest.wal_file = Some(FileEntry {
                    path: "sqlrustgo.wal".to_string(),
                    size,
                    sha256: sha,
                });
            }
        }

        manifest.total_size_bytes = total;
        Ok(manifest)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut f = fs::File::create(path)?;
        f.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn read_from(path: &Path) -> std::io::Result<Self> {
        let mut s = String::new();
        fs::File::open(path)?.read_to_string(&mut s)?;
        Self::from_json(&s)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_lower(&hasher.finalize()))
}

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_lower(&hasher.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn walk_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_recursive(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_recursive(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_new() {
        let m = Manifest::new();
        assert_eq!(m.version, 1);
        assert!(m.data_files.is_empty());
        assert!(m.wal_file.is_none());
    }

    #[test]
    fn test_sha256_bytes() {
        let h = sha256_bytes(b"hello");
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha256_file() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        let mut fh = std::fs::File::create(&f).unwrap();
        fh.write_all(b"hello world").unwrap();
        drop(fh);
        let h = sha256_file(&f).unwrap();
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn test_walk_files_empty() {
        let dir = TempDir::new().unwrap();
        let files = walk_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_walk_files_nested() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(dir.path().join("a.txt"), b"a").unwrap();
        std::fs::write(sub.join("b.txt"), b"b").unwrap();
        let files = walk_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_manifest_scan_with_data() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("t1.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("t2.json"), b"[]").unwrap();
        let m = Manifest::scan(dir.path(), None).unwrap();
        assert_eq!(m.data_files.len(), 2);
        assert!(m.wal_file.is_none());
        assert_eq!(m.total_size_bytes, 4);
    }

    #[test]
    fn test_manifest_scan_with_wal() {
        let data = TempDir::new().unwrap();
        let wal_dir = TempDir::new().unwrap();
        std::fs::write(data.path().join("data.json"), b"x").unwrap();
        let wal = wal_dir.path().join("sqlrustgo.wal");
        std::fs::write(&wal, b"wal-data").unwrap();
        let m = Manifest::scan(data.path(), Some(&wal)).unwrap();
        assert_eq!(m.data_files.len(), 1);
        assert!(m.wal_file.is_some());
        assert_eq!(m.wal_file.as_ref().unwrap().path, "sqlrustgo.wal");
    }

    #[test]
    fn test_manifest_serialize_roundtrip() {
        let mut m = Manifest::new();
        m.data_files.push(FileEntry {
            path: "a.json".to_string(),
            size: 10,
            sha256: "abc".to_string(),
        });
        let s = m.to_json().unwrap();
        let m2 = Manifest::from_json(&s).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn test_manifest_write_read_file() {
        let dir = TempDir::new().unwrap();
        let mf = dir.path().join("manifest.json");
        let m = Manifest::new();
        m.write_to(&mf).unwrap();
        let m2 = Manifest::read_from(&mf).unwrap();
        assert_eq!(m.version, m2.version);
    }
}

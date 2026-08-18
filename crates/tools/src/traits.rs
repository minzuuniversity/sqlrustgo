//! Abstraction layer for file-system I/O.

use std::path::Path;

/// Abstracts the file-system operations used by the backup/restore pipeline.
pub trait SqlRustGoIo: Send + Sync {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;
    fn write(&self, path: &Path, contents: &[u8]) -> std::io::Result<()>;
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn remove_file(&self, path: &Path) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
}

// ---------------------------------------------------------------------------

/// Production I/O: delegates straight to `std::fs`.
pub struct RealIo;

impl SqlRustGoIo for RealIo {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }
    fn write(&self, path: &Path, contents: &[u8]) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }
    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }
    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

// ---------------------------------------------------------------------------

/// Test double with injectable error responses per operation.
pub struct MockIo {
    pub read_err: parking_lot::Mutex<Option<std::io::Error>>,
    pub write_err: parking_lot::Mutex<Option<std::io::Error>>,
    pub create_dir_all_err: parking_lot::Mutex<Option<std::io::Error>>,
    pub remove_dir_all_err: parking_lot::Mutex<Option<std::io::Error>>,
    pub remove_file_err: parking_lot::Mutex<Option<std::io::Error>>,
    pub exists_return: bool,
}

impl Default for MockIo {
    fn default() -> Self {
        Self::happy()
    }
}

impl MockIo {
    /// All operations succeed; `exists` returns `true`.
    pub fn happy() -> Self {
        Self {
            read_err: parking_lot::Mutex::new(None),
            write_err: parking_lot::Mutex::new(None),
            create_dir_all_err: parking_lot::Mutex::new(None),
            remove_dir_all_err: parking_lot::Mutex::new(None),
            remove_file_err: parking_lot::Mutex::new(None),
            exists_return: true,
        }
    }

    /// Configure `read` to fail with `kind`.
    pub fn with_read_err(self, kind: std::io::ErrorKind) -> Self {
        *self.read_err.lock() = Some(std::io::Error::new(kind, "mock read error"));
        self
    }

    /// Configure `create_dir_all` to fail with `kind`.
    pub fn with_create_dir_err(self, kind: std::io::ErrorKind) -> Self {
        *self.create_dir_all_err.lock() = Some(std::io::Error::new(kind, "mock dir error"));
        self
    }

    /// Configure `exists` to return `false`.
    pub fn with_missing(mut self) -> Self {
        self.exists_return = false;
        self
    }

    fn take_err(lock: &parking_lot::Mutex<Option<std::io::Error>>) -> Option<std::io::Error> {
        let mut guard = lock.lock();
        guard.take()
    }
}

impl SqlRustGoIo for MockIo {
    fn read(&self, _path: &Path) -> std::io::Result<Vec<u8>> {
        match Self::take_err(&self.read_err) {
            Some(e) => Err(e),
            None => Ok(Vec::new()),
        }
    }

    fn write(&self, _path: &Path, _contents: &[u8]) -> std::io::Result<()> {
        match Self::take_err(&self.write_err) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn create_dir_all(&self, _path: &Path) -> std::io::Result<()> {
        match Self::take_err(&self.create_dir_all_err) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn remove_dir_all(&self, _path: &Path) -> std::io::Result<()> {
        match Self::take_err(&self.remove_dir_all_err) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn remove_file(&self, _path: &Path) -> std::io::Result<()> {
        match Self::take_err(&self.remove_file_err) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn exists(&self, _path: &Path) -> bool {
        self.exists_return
    }
}

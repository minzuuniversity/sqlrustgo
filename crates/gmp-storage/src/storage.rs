//! GMP Storage engine - Manages document storage, FTS, Vector, and Graph indexes

use gmp_core::{Error, Result};
use std::path::Path;

/// Storage engine configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub data_dir: String,
}

/// Storage engine instance
#[derive(Debug)]
pub struct StorageEngine {
    config: StorageConfig,
}

impl StorageEngine {
    /// Create a new storage engine
    pub fn new(config: StorageConfig) -> Result<Self> {
        let path = Path::new(&config.data_dir);
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| Error::Storage(e.to_string()))?;
        }
        Ok(Self { config })
    }

    /// Get storage data directory
    pub fn data_dir(&self) -> &str {
        &self.config.data_dir
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
        }
    }
}

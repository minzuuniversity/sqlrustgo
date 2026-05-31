//! Engine builder - factory methods for creating ExecutionEngine instances
//! Extracted from execution_engine.rs to reduce file size

#![allow(unused_variables, unused_imports)]

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use sqlrustgo_catalog::Catalog;
use sqlrustgo_storage::{
    recovery_engine::{RecoveryEngine, RecoveryEngineImpl, RecoveryReport},
    FileBackedWalManager, FileStorage, MemoryStorage, StorageEngine, WalStorage,
};
use sqlrustgo_transaction::{IsolationLevel as TmIsolationLevel, TransactionManager};

use crate::execution_engine::{ExecutionEngine, ExecutionStats, TxStatus};
use crate::{SqlError, SqlResult};

// =============================================================================
// LAYER 1 — Plain MemoryStorage
// =============================================================================

impl ExecutionEngine<MemoryStorage> {
    /// Create a new execution engine backed by MemoryStorage with CBO enabled
    pub fn with_memory() -> Self {
        Self {
            storage: Arc::new(RwLock::new(MemoryStorage::new())),
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
        }
    }

    /// Create a new execution engine backed by MemoryStorage with custom CBO setting
    pub fn with_memory_and_cbo(cbo_enabled: bool) -> Self {
        Self {
            storage: Arc::new(RwLock::new(MemoryStorage::new())),
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
        }
    }

    /// Create a new execution engine with catalog
    pub fn with_memory_and_catalog(catalog: Arc<RwLock<Catalog>>) -> Self {
        Self {
            storage: Arc::new(RwLock::new(MemoryStorage::new())),
            catalog: Some(catalog),
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
        }
    }
}

// =============================================================================
// LAYER 2 — WAL integration stub layer
// Does NOT write real WAL entries — flush hook exists, replay mocked
// Use for: WAL interface exists, flush hook exists, replay mocked
// =============================================================================

impl ExecutionEngine<MemoryStorage> {
    pub fn with_wal_stub(
    ) -> ExecutionEngine<WalStorage<MemoryStorage, sqlrustgo_storage::MemoryWalManager>> {
        let inner = MemoryStorage::new();
        let wal = sqlrustgo_storage::MemoryWalManager::new();
        let wal_storage = WalStorage::new(inner, wal).unwrap();
        ExecutionEngine {
            storage: Arc::new(RwLock::new(wal_storage)),
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
        }
    }
}

// =============================================================================
// LAYER 3 — Full WAL layer (Beta Gate required)
// WAL path: wal_path/.wal
// Use for: WAL-001~005, RECOVERY-001~008, B1~B3 integration
// =============================================================================

impl ExecutionEngine<MemoryStorage> {
    /// Create a WAL-backed execution engine with full WAL enabled
    /// WalStorage::new(inner, wal_manager) initializes with given WAL manager
    pub fn with_wal(
        wal_path: PathBuf,
    ) -> SqlResult<
        ExecutionEngine<WalStorage<MemoryStorage, sqlrustgo_storage::FileBackedWalManager>>,
    > {
        let inner = MemoryStorage::new();
        let wal_manager = sqlrustgo_storage::FileBackedWalManager::new(wal_path)?;
        let wal = WalStorage::new(inner, wal_manager)?;
        Ok(ExecutionEngine {
            storage: Arc::new(RwLock::new(wal)),
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
        })
    }

    /// Create a WAL-backed engine with persistent FileStorage (clean boot)
    ///
    /// Creates storage and WAL manager, wraps in WalStorage, returns Engine.
    /// Does NOT run WAL recovery — call recover_wal() after crash recovery.
    pub fn with_wal_file(
        data_dir: PathBuf,
    ) -> SqlResult<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>> {
        let inner = FileStorage::new_with_wal(data_dir.clone())
            .map_err(|e| SqlError::ExecutionError(format!("FileStorage init failed: {}", e)))?;
        let wal_path = data_dir.join("sqlrustgo.wal");
        let wal_manager = FileBackedWalManager::new(wal_path)?;
        let wal_storage = WalStorage::new(inner, wal_manager)?;

        Ok(ExecutionEngine {
            storage: Arc::new(RwLock::new(wal_storage)),
            catalog: None,
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            cbo_enabled: true,
            transaction_manager: TransactionManager::new(),
            current_tx_id: None,
            tx_status: TxStatus::Idle,
            default_isolation: TmIsolationLevel::default(),
            current_role: None,
        })
    }

    /// Create a WAL-backed engine with persistent FileStorage and automatic recovery.
    /// This constructor creates storage and WAL manager, then runs recovery automatically.
    /// For production use with WAL persistence.
    pub fn with_wal_recovery(
        data_dir: PathBuf,
    ) -> SqlResult<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>> {
        let mut engine = Self::with_wal_file(data_dir)?;
        recover_wal(&mut engine)?;
        Ok(engine)
    }
}

/// Recover a WAL-backed engine after crash: replay committed WAL entries
pub fn recover_wal(
    engine: &mut ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>,
) -> SqlResult<RecoveryReport> {
    let storage = &mut *engine.storage.write().map_err(|e| {
        SqlError::ExecutionError(format!("Failed to lock storage for recovery: {:?}", e))
    })?;
    let (inner, wal_mgr) = storage.split();
    let mut recovery = RecoveryEngineImpl;
    RecoveryEngine::recover(&mut recovery, inner, wal_mgr)
}

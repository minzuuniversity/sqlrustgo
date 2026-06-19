//! Engine builder - factory methods for creating ExecutionEngine instances
//! Extracted from execution_engine.rs to reduce file size

#![allow(unused_variables, unused_imports)]

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use sqlrustgo_catalog::Catalog;
use sqlrustgo_storage::{
    recovery_engine::{RecoveryEngine, RecoveryEngineImpl, RecoveryReport, StatefulRecoveryEngine},
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
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
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
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
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
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
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
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
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
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
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
            checkpoint_manager: None,
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
        })
    }

    /// Create a WAL-backed engine with CheckpointManager for WAL lifecycle control.
    pub fn with_wal_and_checkpoint(
        data_dir: PathBuf,
        checkpoint_dir: PathBuf,
    ) -> SqlResult<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>> {
        let inner = FileStorage::new_with_wal(data_dir.clone())
            .map_err(|e| SqlError::ExecutionError(format!("FileStorage init failed: {}", e)))?;
        let wal_path = data_dir.join("sqlrustgo.wal");
        let wal_manager = FileBackedWalManager::new(wal_path)?;
        let wal_storage = WalStorage::new(inner, wal_manager)?;

        let checkpoint_manager =
            sqlrustgo_storage::CheckpointManager::with_dir(checkpoint_dir).ok();

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
            checkpoint_manager: checkpoint_manager.map(|cp| Arc::new(RwLock::new(cp))),
            parallel_degree: 1,
            stmt_cache: sqlrustgo_cache::PreparedStatementCache::new(100),
        })
    }

    /// Create a WAL-backed engine with persistent FileStorage and automatic recovery.
    /// This constructor creates storage and WAL manager, then runs recovery automatically.
    /// For production use with WAL persistence.
    pub fn with_wal_recovery(
        data_dir: PathBuf,
    ) -> SqlResult<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>> {
        let mut engine = Self::with_wal_file(data_dir)?;
        // PR-842: clear in-memory rows loaded from t.json before replay so
        // the WAL is the sole source of truth. Without this the rows
        // persisted during normal operation would be reapplied by the
        // recovery engine, producing duplicates on every restart.
        {
            let mut storage = engine.storage.write().map_err(|e| {
                SqlError::ExecutionError(format!("Failed to lock storage: {:?}", e))
            })?;
            let (inner, _wal_mgr) = storage.split();
            inner.clear_all_tables();
        }
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
    let mut recovery = StatefulRecoveryEngine::new();
    let report = RecoveryEngine::recover(&mut recovery, inner, wal_mgr)?;

    // Flush any data accumulated in FileStorage's insert buffer during replay
    // so the post-recovery scan can see the recovered rows. Without this,
    // `storage.scan()` would only see `data.rows` (which may have been
    // truncated by row-level DELETE replay) and miss the inserted rows
    // sitting in the buffer.
    storage.flush()?;

    log::info!(
        "WAL recovery completed: {} committed txns, {} rolled back, {} incomplete, {} entries total",
        report.committed_txns,
        report.rolled_back_txns,
        report.incomplete_txns,
        report.entries_total
    );

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use sqlrustgo_storage::MemoryStorage;
    use tempfile::TempDir;

    #[test]
    fn test_with_memory_creates_working_engine() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory();
        engine
            .execute("CREATE TABLE t (id INTEGER, name TEXT)")
            .expect("CREATE TABLE should succeed");
        engine
            .execute("INSERT INTO t VALUES (1, 'alice')")
            .expect("INSERT should succeed");
        let r = engine
            .execute("SELECT name FROM t WHERE id = 1")
            .expect("SELECT should succeed");
        assert_eq!(r.rows.len(), 1, "expected 1 row");
        assert_eq!(r.rows[0][0], Value::Text("alice".to_string()));
    }

    #[test]
    fn test_with_memory_and_cbo_disabled_still_executes() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_memory_and_cbo(false);
        engine.execute("CREATE TABLE t (x INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (42)").unwrap();
        let r = engine.execute("SELECT x FROM t").unwrap();
        assert_eq!(r.rows, vec![vec![Value::Integer(42)]]);
    }

    #[test]
    fn test_with_wal_stub_creates_engine_and_executes_sql() {
        let mut engine = ExecutionEngine::<MemoryStorage>::with_wal_stub();
        engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (1)").unwrap();
        engine.execute("INSERT INTO t VALUES (2)").unwrap();
        let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
        assert_eq!(r.affected_rows, 1);
        assert_eq!(r.rows[0][0], Value::Integer(2));
    }

    #[test]
    fn test_with_wal_file_creates_persistent_engine() {
        let dir = TempDir::new().expect("tempdir");
        let mut engine = ExecutionEngine::<MemoryStorage>::with_wal_file(dir.path().to_path_buf())
            .expect("with_wal_file should succeed");
        engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (100)").unwrap();
        let r = engine.execute("SELECT id FROM t").unwrap();
        assert_eq!(r.rows, vec![vec![Value::Integer(100)]]);

        let wal_path = dir.path().join("sqlrustgo.wal");
        assert!(wal_path.exists(), "WAL file should exist at {:?}", wal_path);
    }

    #[test]
    fn test_with_wal_and_checkpoint_creates_engine() {
        let dir = TempDir::new().expect("tempdir");
        let mut engine = ExecutionEngine::<MemoryStorage>::with_wal_and_checkpoint(
            dir.path().to_path_buf(),
            dir.path().join("checkpoints"),
        )
        .expect("with_wal_and_checkpoint should succeed");
        engine.execute("CREATE TABLE t (id INTEGER)").unwrap();
        engine.execute("INSERT INTO t VALUES (7)").unwrap();
        let r = engine.execute("SELECT id FROM t").unwrap();
        assert_eq!(r.rows, vec![vec![Value::Integer(7)]]);
    }

    #[test]
    fn test_recover_wal_on_empty_wal_returns_zero_report() {
        let dir = TempDir::new().expect("tempdir");
        let mut engine =
            ExecutionEngine::<MemoryStorage>::with_wal_file(dir.path().to_path_buf()).unwrap();
        let report = recover_wal(&mut engine).expect("recover_wal on empty WAL should succeed");
        assert_eq!(report.entries_total, 0, "fresh WAL should have 0 entries");
        assert_eq!(
            report.committed_txns, 0,
            "fresh WAL should have 0 committed txns"
        );
    }
}

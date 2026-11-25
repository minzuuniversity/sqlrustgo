use sqlrustgo_storage::{ColumnDefinition, FileStorage, StorageEngine, TableInfo, Value};
use sqlrustgo_storage::append_only_storage::AppendOnlyStorage;
use sqlrustgo_transaction::{TransactionManager, IsolationLevel};
use sqlrustgo_transaction::savepoint::UndoRecord;
use sqlrustgo_storage::{MemoryStorage, MemoryWalManager, WalStorage, SqlResult};
use sqlrustgo_storage::wal::{WalManager, WalEntry};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

struct CountWal { inner: MemoryWalManager, syncs: Arc<AtomicUsize>, fail_recover: bool }
impl WalManager for CountWal {
    fn append(&mut self, e: WalEntry) -> SqlResult<u64> { self.inner.append(e) }
    fn flush(&mut self) -> SqlResult<()> { self.inner.flush() }
    fn sync(&mut self) -> SqlResult<()> { self.syncs.fetch_add(1, Ordering::SeqCst); Ok(()) }
    fn recover(&mut self) -> SqlResult<Vec<WalEntry>> {
        if self.fail_recover { Err(sqlrustgo_storage::engine::SqlError::ExecutionError("HARA recovery I/O failure".into())) }
        else { self.inner.recover() }
    }
    fn truncate_before(&mut self, n: u64) -> SqlResult<()> { self.inner.truncate_before(n) }
    fn current_lsn(&self) -> u64 { self.inner.current_lsn() }
}

fn info() -> TableInfo {
    TableInfo {
        name: "t".into(), columns: vec![ColumnDefinition {
            name: "id".into(), data_type: "INTEGER".into(), nullable: false,
            primary_key: false, char_max_length: None, collation: None,
            default_value: None, auto_increment: false,
        }], foreign_keys: vec![], unique_constraints: vec![], check_constraints: vec![],
        partition_info: None, collations: Default::default(), compression: None,
        original_sql: String::new(), clustered: false,
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--pending-child") {
        let path = std::path::PathBuf::from(std::env::args().nth(2).unwrap());
        let mut s = FileStorage::new_with_wal(path).unwrap();
        s.create_table(&info()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("t", vec![vec![Value::Integer(42)]]).unwrap();
        StorageEngine::flush(&mut s).unwrap();
        std::process::exit(0);
    }
    {
        use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
        let dir = tempfile::tempdir().unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--pending-child").arg(dir.path()).status().unwrap();
        let mut s = FileStorage::new_with_wal(dir.path().into()).unwrap();
        let mut wal = sqlrustgo_storage::FileBackedWalManager::new(dir.path().join("sqlrustgo.wal")).unwrap();
        let mut r = StatefulRecoveryEngine::<FileStorage>::new();
        println!("HARA pending_restart child={child} recovery={:?} expected=0 actual={}", r.recover(&mut s,&mut wal),s.scan("t").unwrap().len());
    }
    {
        let dir = tempfile::tempdir().unwrap();
        let pool = Arc::new(sqlrustgo_storage::BufferPool::new(16));
        let mut s = FileStorage::new_with_bufferpool(dir.path().into(), pool).unwrap();
        s.create_table(&info()).unwrap();
        s.begin_transaction().unwrap();
        s.insert("t", vec![vec![Value::Integer(42)]]).unwrap();
        s.rollback_transaction().unwrap();
        println!("HARA bufferpool_rollback expected=0 actual={}",s.scan("t").unwrap().len());
    }
    for lockfree in [false, true] {
        let syncs = Arc::new(AtomicUsize::new(0));
        let wal = CountWal { inner: MemoryWalManager::new(), syncs: syncs.clone(), fail_recover: false };
        let mut s = WalStorage::new(MemoryStorage::new(), wal).unwrap();
        s.begin_transaction().unwrap();
        syncs.store(0, Ordering::SeqCst);
        if lockfree { s.commit_transaction_lockfree().unwrap(); }
        else { s.commit_transaction().unwrap(); }
        println!("HARA commit_sync lockfree={lockfree} expected_at_least=1 actual={}",syncs.load(Ordering::SeqCst));
    }
    {
        use sqlrustgo_storage::recovery_engine::{RecoveryEngine, StatefulRecoveryEngine};
        let mut wal = CountWal { inner: MemoryWalManager::new(), syncs: Arc::new(AtomicUsize::new(0)), fail_recover: true };
        let mut r = StatefulRecoveryEngine::<MemoryStorage>::new();
        let mut s = MemoryStorage::new();
        println!("HARA recovery_failure_first expected=Err actual={:?}",r.recover(&mut s,&mut wal));
        println!("HARA recovery_failure_second expected=Err actual={:?}",r.recover(&mut s,&mut wal));
    }
    for (name, ddl, seed, dml) in [
        ("delete", "CREATE TABLE t (id INT PRIMARY KEY)", "INSERT INTO t VALUES (1)", "DELETE FROM t WHERE id=1"),
        ("pk_update", "CREATE TABLE t (id INT PRIMARY KEY)", "INSERT INTO t VALUES (1)", "UPDATE t SET id=2 WHERE id=1"),
        ("replace", "CREATE TABLE t (id INT PRIMARY KEY, v INT)", "INSERT INTO t VALUES (1,10)", "REPLACE INTO t VALUES (1,20)"),
        ("odku", "CREATE TABLE t (id INT PRIMARY KEY, v INT)", "INSERT INTO t VALUES (1,10)", "INSERT INTO t VALUES (1,20) ON DUPLICATE KEY UPDATE v=20"),
        ("truncate", "CREATE TABLE t (id INT PRIMARY KEY)", "INSERT INTO t VALUES (1)", "TRUNCATE TABLE t"),
        ("clustered", "CREATE TABLE t (id INT PRIMARY KEY) ENGINE=InnoDB CLUSTERED", "INSERT INTO t VALUES (1)", "INSERT INTO t VALUES (2)"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(dir.path().into()).unwrap();
        let mut e = sqlrustgo::ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(storage)));
        e.execute(ddl).unwrap();
        e.execute(seed).unwrap();
        e.execute("BEGIN").unwrap();
        let changed = e.execute(dml);
        let rollback = e.execute("ROLLBACK");
        let expected = if name == "replace" || name == "odku" { "[[1,10]]" } else { "[[1]]" };
        println!("HARA sql_{name} dml={:?} rollback={:?} expected={expected} actual={:?}", changed.as_ref().map(|_| ()), rollback.as_ref().map(|_| ()), e.execute("SELECT * FROM t").map(|r|r.rows));
    }
    for flush in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new_with_buffer_config(dir.path().into(), usize::MAX, true).unwrap();
        s.create_table(&info()).unwrap();
        let tx = s.begin_transaction().unwrap();
        s.insert("t", vec![vec![Value::Integer(42)]]).unwrap();
        if flush { StorageEngine::flush(&mut s).unwrap(); }
        s.rollback_transaction_for(tx).unwrap();
        let n = s.scan("t").unwrap().len();
        println!("HARA flush_insert_rollback flush={flush} expected=0 actual={n}");
    }
    {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new_with_buffer_config(dir.path().into(), usize::MAX, true).unwrap();
        s.create_table(&info()).unwrap();
        s.insert("t", vec![vec![Value::Integer(42)]]).unwrap();
        let tx = s.begin_transaction().unwrap();
        s.delete("t", &[]).unwrap();
        s.rollback_transaction_for(tx).unwrap();
        println!("HARA buffered_delete_all_rollback expected=1 actual={}", s.scan("t").unwrap().len());
    }
    {
        let dir = tempfile::tempdir().unwrap();
        let mut s = AppendOnlyStorage::new(dir.path().into()).unwrap();
        s.create_table(&info()).unwrap();
        let tx = s.begin_transaction().unwrap();
        s.insert("t", vec![vec![Value::Integer(42)]]).unwrap();
        let result = s.rollback_transaction_for(tx);
        println!("HARA append_rollback result={result:?} expected=0 actual={}",s.scan("t").unwrap().len());
    }
    {
        let mut tm = TransactionManager::new();
        let tx = tm.begin_transaction(IsolationLevel::SnapshotIsolation).unwrap();
        tm.add_undo_record(tx, UndoRecord::Insert { table: "t".into(), key: vec![], row: vec![] }).unwrap();
        let result = tm.rollback_with_undo(tx, |_| Err("HARA injected undo failure".into()));
        println!("HARA undo_failure expected=Err actual={result:?}");
    }
}

//! A06 (#5228) fail-point reachability: AfterWALAppend / AfterIndexUpdate.
//!
//! These two points extend the B-1 framework (`crash_commit_fsync_b1.rs`
//! covers `AfterCommitFsync`) to the WAL-append and index-update boundaries
//! the audit requires. Each test arms one point in a background thread,
//! lets the production call site reach it, and reads the signal file the
//! background thread writes — proving the hook is reachable, not decorative.

use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::fault_injection::{FaultPoint, Signal};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::wal_legacy::{WalEntry, WalEntryType, WalWriter};
use sqlrustgo_storage::{StorageEngine, Value as SqlValue};
use std::sync::{mpsc, Mutex, OnceLock};

use std::time::{Duration, Instant};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Arm `point`, run `body` in a background thread, and return the signal
/// file contents once that thread has written it.
///
/// `reach` blocks (up to 120s) so it must run off the main thread. The
/// signal file is the only observable: its presence plus a `run_id` match
/// is what a crash test uses to decide it reached the boundary.
///
/// The whole arm→spawn→wait window is serialized under [`env_lock`]:
/// `std::env::set_var` is process-global, so two tests setting different
/// fault points in parallel would clobber each other's `SQLRUSTGO_FAULT_*`
/// and one would spin forever waiting for a signal that never fires.
fn arm_and_capture<F>(point: FaultPoint, run_id: &str, body: F) -> String
where
    F: FnOnce() + Send + 'static,
{
    let _guard = env_lock().lock().expect("env lock poisoned");
    let dir = std::env::temp_dir().join(format!("a06_reach_{run_id}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir signal dir");

    std::env::set_var("SQLRUSTGO_FAULT_POINT", point.as_str());
    std::env::set_var("SQLRUSTGO_FAULT_SIGNAL_DIR", &dir);
    std::env::set_var("SQLRUSTGO_FAULT_RUN_ID", run_id);

    let (tx, rx) = mpsc::channel();
    let point_in_thread = point;
    std::thread::spawn(move || {
        body();
        sqlrustgo_storage::fault_injection::reach(point_in_thread);
        let _ = tx.send(());
    });

    let signal_file = dir.join(format!("{}.reached", point.as_str()));
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if signal_file.exists() {
            let contents = std::fs::read_to_string(&signal_file).expect("read signal");
            let _ = rx.recv_timeout(Duration::from_secs(1));
            return contents;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "fault point {} was never reached: signal file {} did not appear",
        point.as_str(),
        signal_file.display()
    );
}

fn table_info() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "a".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: false,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

#[test]
fn after_wal_append_is_reachable() {
    let run_id = format!("a06_wal_{}", std::process::id());
    let contents = arm_and_capture(FaultPoint::AfterWALAppend, &run_id, || {
        let path = std::env::temp_dir().join(format!("a06_wal_{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut w = WalWriter::new(&path).expect("open WAL writer");
        let entry = WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 1,
            key: None,
            data: None,
            lsn: 0,
            timestamp: 0,
            table_name: Some("t".into()),
            database_name: None,
        };
        w.append(&entry).expect("append");
    });

    let sig = Signal::parse(&contents).expect("parse signal");
    assert_eq!(sig.point, FaultPoint::AfterWALAppend.as_str());
    assert_eq!(sig.run_id, run_id);
}

#[test]
fn after_index_update_is_reachable() {
    let run_id = format!("a06_idx_{}", std::process::id());
    let contents = arm_and_capture(FaultPoint::AfterIndexUpdate, &run_id, || {
        let dir = std::env::temp_dir().join(format!("a06_idx_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut s = FileStorage::new(dir).expect("open storage");
        s.create_table(&table_info()).expect("create table");
        s.insert("t", vec![vec![SqlValue::Integer(1)]])
            .expect("insert row");
        s.create_index("t", "a", 0).expect("create index");
    });

    let sig = Signal::parse(&contents).expect("parse signal");
    assert_eq!(sig.point, FaultPoint::AfterIndexUpdate.as_str());
    assert_eq!(sig.run_id, run_id);
}

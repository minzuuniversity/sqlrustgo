//! Disk-backed graph store with WAL persistence.
//!
//! `DiskGraphStore` wraps an in-memory state for fast queries, and on every mutation
//! appends a record to the on-disk WAL. The WAL survives crashes via CRC32 checksums
//! and torn-write detection in [`crate::wal::Wal::replay`].
//!
//! ## Crash recovery contract
//!
//! - Writes are `fsync`-ed on every append (configurable; see [`DiskGraphStore::set_sync_mode`]).
//! - On [`DiskGraphStore::open`], if a snapshot file exists, it is loaded; then WAL
//!   records **after** the snapshot's high-water-mark are replayed.
//! - Torn or CRC-corrupt records at the tail of the WAL are silently truncated.
//!
//! ## Snapshotting
//!
//! Call [`DiskGraphStore::flush`] to write a full snapshot of the in-memory state,
//! rewrite the WAL to contain a single `SnapshotMark` record, and fsync. This bounds
//! replay time as the store grows.

use crate::store::{Edge, GraphStore, InMemoryGraphStore, Node};
use crate::types::{EdgeId, GraphResult, Label, NodeId, PropertyMap};
use crate::wal::{Wal, WalRecord};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// File names within the store directory.
const WAL_FILENAME: &str = "graph.wal";
const SNAPSHOT_FILENAME: &str = "graph.snapshot";

/// On-disk snapshot: full in-memory state plus the high-water LSN and id counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Snapshot {
    /// Schema version. Currently 1; reject unknown on load.
    version: u32,
    /// Last LSN that this snapshot's state reflects.
    last_lsn: u64,
    /// All nodes.
    nodes: Vec<(NodeId, Vec<Label>, PropertyMap)>,
    /// All edges: (edge_id, source, target, rel_type, properties).
    edges: Vec<(EdgeId, NodeId, NodeId, Label, PropertyMap)>,
    /// Next node id counter (high-water mark).
    next_node_id: u64,
    /// Next edge id counter (high-water mark).
    next_edge_id: u64,
}

const SNAPSHOT_VERSION: u32 = 1;

/// Synchronisation mode for the WAL appender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// `fsync` on every append — strongest durability, lower throughput.
    EveryWrite,
    /// Only flush userspace buffer on every append; no `fsync`. Faster but
    /// last few records may be lost on power loss (not on clean process exit).
    Buffered,
}

impl Default for SyncMode {
    fn default() -> Self {
        SyncMode::EveryWrite
    }
}

/// Disk-backed graph store.
///
/// Thread-safe: reads via the inner `RwLock`, mutations serialised through the
/// `Mutex<Wal>` and the inner store's write lock.
///
/// `DiskGraphStore` maintains its own monotonic id counters
/// (`next_node_id` / `next_edge_id`) rather than relying on `InMemoryGraphStore`'s
/// internal counters. This is critical for snapshot correctness: snapshot
/// persistence must capture the high-water mark of *allocated* ids, not just
/// ids that happen to still be live.
pub struct DiskGraphStore {
    dir: PathBuf,
    inner: RwLock<InnerState>,
    wal: Mutex<Wal>,
    sync_mode: RwLock<SyncMode>,
}

#[derive(Debug, Default)]
struct InnerState {
    store: InMemoryGraphStore,
    last_lsn: u64,
    next_node_id: u64,
    next_edge_id: u64,
}

impl DiskGraphStore {
    /// Opens (or creates) the store rooted at `dir`.
    ///
    /// On startup:
    /// 1. Load snapshot if present (sets `last_lsn` and id counters).
    /// 2. Replay WAL records with `lsn > last_lsn` to catch up.
    pub fn open(dir: impl Into<PathBuf>) -> GraphResult<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;

        let wal_path = dir.join(WAL_FILENAME);
        let snap_path = dir.join(SNAPSHOT_FILENAME);

        let mut inner = InnerState::default();

        // Step 1: load snapshot if any.
        if snap_path.exists() {
            let snap_bytes = std::fs::read(&snap_path)?;
            let snap: Snapshot = bincode::deserialize(&snap_bytes)
                .map_err(|e| crate::types::GraphError::Serde(format!("snapshot decode: {e}")))?;
            if snap.version != SNAPSHOT_VERSION {
                return Err(crate::types::GraphError::Storage(format!(
                    "snapshot version {} not supported (this build supports {})",
                    snap.version, SNAPSHOT_VERSION
                )));
            }
            apply_snapshot(&mut inner, snap);
        }

        // Step 2: open WAL and replay anything new.
        let wal = Wal::open(&wal_path)?;
        let replayed = Wal::replay(&wal_path)?;
        for entry in &replayed {
            if entry.lsn > inner.last_lsn {
                apply_wal_record(&mut inner, &entry.record)?;
                inner.last_lsn = entry.lsn;
            }
        }

        Ok(Self {
            dir,
            inner: RwLock::new(inner),
            wal: Mutex::new(wal),
            sync_mode: RwLock::new(SyncMode::default()),
        })
    }

    /// Path to the store directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Last LSN applied to the in-memory state.
    pub fn last_lsn(&self) -> u64 {
        self.inner.read().last_lsn
    }

    /// Switches sync mode at runtime. Takes effect on the next append.
    pub fn set_sync_mode(&self, mode: SyncMode) {
        *self.sync_mode.write() = mode;
    }

    /// Returns the active sync mode.
    pub fn sync_mode(&self) -> SyncMode {
        *self.sync_mode.read()
    }

    /// Writes a snapshot, rewrites the WAL to a single `SnapshotMark` record,
    /// and fsyncs. Returns the new high-water-mark LSN.
    pub fn flush(&self) -> GraphResult<u64> {
        let mut inner = self.inner.write();
        let snap_lsn: u64 = 1;

        let snap = build_snapshot(&inner, snap_lsn)?;
        let snap_bytes = bincode::serialize(&snap)
            .map_err(|e| crate::types::GraphError::Serde(format!("snapshot encode: {e}")))?;
        let tmp = self.dir.join(format!("{SNAPSHOT_FILENAME}.tmp"));
        {
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp)?;
            f.write_all(&snap_bytes)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, self.dir.join(SNAPSHOT_FILENAME))?;

        rewrite_wal_with_snapshot_mark(&self.dir.join(WAL_FILENAME), snap_lsn)?;
        let last_lsn;
        {
            let mut wal = self.wal.lock();
            *wal = Wal::open(self.dir.join(WAL_FILENAME))?;
            last_lsn = snap_lsn;
            inner.last_lsn = last_lsn;
        }
        Ok(last_lsn)
    }

    fn alloc_node_id(&self) -> GraphResult<NodeId> {
        let mut inner = self.inner.write();
        let id = NodeId(inner.next_node_id);
        inner.next_node_id = inner
            .next_node_id
            .checked_add(1)
            .ok_or_else(|| crate::types::GraphError::Storage("node id space exhausted".into()))?;
        Ok(id)
    }

    fn alloc_edge_id(&self) -> GraphResult<EdgeId> {
        let mut inner = self.inner.write();
        let id = EdgeId(inner.next_edge_id);
        inner.next_edge_id = inner
            .next_edge_id
            .checked_add(1)
            .ok_or_else(|| crate::types::GraphError::Storage("edge id space exhausted".into()))?;
        Ok(id)
    }

    fn append_wal(&self, record: WalRecord) -> GraphResult<u64> {
        let mut wal = self.wal.lock();
        let lsn = wal.append(record)?;
        let mode = *self.sync_mode.read();
        if mode == SyncMode::EveryWrite {
            wal.flush()?;
        }
        Ok(lsn)
    }
}

fn apply_wal_record(state: &mut InnerState, rec: &WalRecord) -> GraphResult<()> {
    match rec {
        WalRecord::CreateNode {
            node_id,
            labels,
            properties,
        } => match state.store.get_node(*node_id) {
            Ok(_) => {}
            Err(crate::types::GraphError::NodeNotFound(_)) => {
                state
                    .store
                    .insert_node_with_id(*node_id, labels.clone(), properties.clone())?;
                if state.next_node_id <= node_id.0 {
                    state.next_node_id = node_id.0 + 1;
                }
            }
            Err(e) => return Err(e),
        },
        WalRecord::UpdateNode {
            node_id,
            labels,
            properties,
        } => {
            state
                .store
                .update_node(*node_id, labels.clone(), properties.clone())?;
        }
        WalRecord::DeleteNode { node_id } => {
            let _ = state.store.delete_node(*node_id);
        }
        WalRecord::CreateEdge {
            edge_id,
            source,
            target,
            rel_type,
            properties,
        } => match state.store.get_edge(*edge_id) {
            Ok(_) => {}
            Err(crate::types::GraphError::EdgeNotFound(_)) => {
                state.store.insert_edge_with_id(
                    *edge_id,
                    *source,
                    *target,
                    rel_type.clone(),
                    properties.clone(),
                )?;
                if state.next_edge_id <= edge_id.0 {
                    state.next_edge_id = edge_id.0 + 1;
                }
            }
            Err(e) => return Err(e),
        },
        WalRecord::UpdateEdge {
            edge_id,
            properties,
        } => {
            state.store.update_edge(*edge_id, properties.clone())?;
        }
        WalRecord::DeleteEdge { edge_id } => {
            let _ = state.store.delete_edge(*edge_id);
        }
        WalRecord::SetNextIds {
            next_node_id,
            next_edge_id,
        } => {
            if state.next_node_id < *next_node_id {
                state.next_node_id = *next_node_id;
            }
            if state.next_edge_id < *next_edge_id {
                state.next_edge_id = *next_edge_id;
            }
        }
        WalRecord::SnapshotMark { .. } => {
            // No-op; state already reflects the snapshot.
        }
    }
    Ok(())
}

fn build_snapshot(state: &InnerState, last_lsn: u64) -> GraphResult<Snapshot> {
    let mut nodes = Vec::with_capacity(state.store.node_count() as usize);
    for id in state.store.all_node_ids() {
        let n = state.store.get_node(id)?;
        nodes.push((n.id, n.labels, n.properties));
    }
    let mut edges = Vec::with_capacity(state.store.edge_count() as usize);
    for id in state.store.all_edge_ids() {
        let e = state.store.get_edge(id)?;
        edges.push((e.id, e.source, e.target, e.rel_type, e.properties));
    }

    Ok(Snapshot {
        version: SNAPSHOT_VERSION,
        last_lsn,
        nodes,
        edges,
        next_node_id: state.next_node_id,
        next_edge_id: state.next_edge_id,
    })
}

fn apply_snapshot(state: &mut InnerState, snap: Snapshot) {
    for (id, labels, properties) in snap.nodes {
        if state.store.get_node(id).is_err() {
            let _ = state.store.insert_node_with_id(id, labels, properties);
        }
    }
    for (id, src, dst, rel, props) in snap.edges {
        if state.store.get_edge(id).is_err() {
            let _ = state.store.insert_edge_with_id(id, src, dst, rel, props);
        }
    }
    if state.next_node_id < snap.next_node_id {
        state.next_node_id = snap.next_node_id;
    }
    if state.next_edge_id < snap.next_edge_id {
        state.next_edge_id = snap.next_edge_id;
    }
    state.last_lsn = snap.last_lsn;
}

fn rewrite_wal_with_snapshot_mark(wal_path: &Path, up_to_lsn: u64) -> GraphResult<()> {
    let tmp = wal_path.with_extension("wal.tmp");
    {
        let mut w = Wal::open(&tmp)?;
        w.append(WalRecord::SnapshotMark { up_to_lsn })?;
        w.flush()?;
    }
    std::fs::rename(&tmp, wal_path)?;
    Ok(())
}

impl GraphStore for DiskGraphStore {
    fn create_node(&self, labels: Vec<Label>, properties: PropertyMap) -> GraphResult<NodeId> {
        let id = self.alloc_node_id()?;
        self.inner
            .write()
            .store
            .insert_node_with_id(id, labels.clone(), properties.clone())?;
        self.append_wal(WalRecord::CreateNode {
            node_id: id,
            labels,
            properties,
        })?;
        Ok(id)
    }

    fn create_edge(
        &self,
        source: NodeId,
        target: NodeId,
        rel_type: Label,
        properties: PropertyMap,
    ) -> GraphResult<EdgeId> {
        if self.inner.read().store.get_node(source).is_err() {
            return Err(crate::types::GraphError::EdgeEndpointMissing {
                src: source,
                dst: target,
            });
        }
        if self.inner.read().store.get_node(target).is_err() {
            return Err(crate::types::GraphError::EdgeEndpointMissing {
                src: source,
                dst: target,
            });
        }
        let id = self.alloc_edge_id()?;
        self.inner.write().store.insert_edge_with_id(
            id,
            source,
            target,
            rel_type.clone(),
            properties.clone(),
        )?;
        self.append_wal(WalRecord::CreateEdge {
            edge_id: id,
            source,
            target,
            rel_type,
            properties,
        })?;
        Ok(id)
    }

    fn get_node(&self, id: NodeId) -> GraphResult<Node> {
        self.inner.read().store.get_node(id)
    }

    fn get_edge(&self, id: EdgeId) -> GraphResult<Edge> {
        self.inner.read().store.get_edge(id)
    }

    fn update_node(
        &self,
        id: NodeId,
        labels: Vec<Label>,
        properties: PropertyMap,
    ) -> GraphResult<()> {
        self.inner
            .write()
            .store
            .update_node(id, labels.clone(), properties.clone())?;
        self.append_wal(WalRecord::UpdateNode {
            node_id: id,
            labels,
            properties,
        })?;
        Ok(())
    }

    fn update_edge(&self, id: EdgeId, properties: PropertyMap) -> GraphResult<()> {
        self.inner
            .write()
            .store
            .update_edge(id, properties.clone())?;
        self.append_wal(WalRecord::UpdateEdge {
            edge_id: id,
            properties,
        })?;
        Ok(())
    }

    fn delete_node(&self, id: NodeId) -> GraphResult<()> {
        self.inner.write().store.delete_node(id)?;
        self.append_wal(WalRecord::DeleteNode { node_id: id })?;
        Ok(())
    }

    fn delete_edge(&self, id: EdgeId) -> GraphResult<()> {
        self.inner.write().store.delete_edge(id)?;
        self.append_wal(WalRecord::DeleteEdge { edge_id: id })?;
        Ok(())
    }

    fn node_count(&self) -> u64 {
        self.inner.read().store.node_count()
    }

    fn edge_count(&self) -> u64 {
        self.inner.read().store.edge_count()
    }

    fn all_node_ids(&self) -> Vec<NodeId> {
        self.inner.read().store.all_node_ids()
    }

    fn all_edge_ids(&self) -> Vec<EdgeId> {
        self.inner.read().store.all_edge_ids()
    }

    fn outgoing_edges(&self, node: NodeId) -> GraphResult<Vec<Edge>> {
        self.inner.read().store.outgoing_edges(node)
    }

    fn incoming_edges(&self, node: NodeId) -> GraphResult<Vec<Edge>> {
        self.inner.read().store.incoming_edges(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn tmp_store() -> (tempfile::TempDir, DiskGraphStore) {
        let dir = tempdir().unwrap();
        let s = DiskGraphStore::open(dir.path()).unwrap();
        (dir, s)
    }

    fn make_props(pairs: &[(&str, &str)]) -> PropertyMap {
        let mut m = PropertyMap::new();
        for (k, v) in pairs {
            m.insert(*k, *v);
        }
        m
    }

    #[test]
    fn open_empty_store() {
        let (_dir, s) = tmp_store();
        assert_eq!(s.node_count(), 0);
        assert_eq!(s.edge_count(), 0);
    }

    #[test]
    fn create_node_then_reopen_preserves() {
        let dir = tempdir().unwrap();
        let id;
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            id = s
                .create_node(vec!["Person".into()], make_props(&[("name", "Alice")]))
                .unwrap();
            assert_eq!(s.node_count(), 1);
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        assert_eq!(s2.node_count(), 1);
        let n = s2.get_node(id).unwrap();
        assert!(n.has_label(&"Person".into()));
        assert_eq!(
            n.properties.get("name"),
            Some(&crate::types::PropertyValue::String("Alice".into()))
        );
    }

    #[test]
    fn create_edge_then_reopen_preserves_with_cascade() {
        let dir = tempdir().unwrap();
        let (a, b);
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            a = s
                .create_node(vec!["Person".into()], make_props(&[("name", "Alice")]))
                .unwrap();
            b = s
                .create_node(vec!["Person".into()], make_props(&[("name", "Bob")]))
                .unwrap();
            s.create_edge(a, b, "KNOWS".into(), PropertyMap::new())
                .unwrap();
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        assert_eq!(s2.node_count(), 2);
        assert_eq!(s2.edge_count(), 1);
        let outs = s2.outgoing_edges(a).unwrap();
        assert_eq!(outs.len(), 1);
        assert_eq!(outs[0].target, b);
    }

    #[test]
    fn delete_node_then_reopen_preserves_deletion() {
        let dir = tempdir().unwrap();
        let a;
        let b;
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            a = s.create_node(vec![], PropertyMap::new()).unwrap();
            b = s.create_node(vec![], PropertyMap::new()).unwrap();
            s.create_edge(a, b, "X".into(), PropertyMap::new()).unwrap();
            s.delete_node(a).unwrap();
            assert_eq!(s.node_count(), 1);
            assert_eq!(s.edge_count(), 0);
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        assert_eq!(s2.node_count(), 1);
        assert_eq!(s2.edge_count(), 0);
        assert!(s2.get_node(a).is_err());
    }

    #[test]
    fn update_node_then_reopen_preserves() {
        let dir = tempdir().unwrap();
        let id;
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            id = s
                .create_node(vec!["Person".into()], PropertyMap::new())
                .unwrap();
            s.update_node(id, vec!["Friend".into()], make_props(&[("nick", "Al")]))
                .unwrap();
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        let n = s2.get_node(id).unwrap();
        assert_eq!(n.labels.len(), 1);
        assert!(n.has_label(&"Friend".into()));
    }

    #[test]
    fn flush_writes_snapshot_and_truncates_wal() {
        let dir = tempdir().unwrap();
        let s = DiskGraphStore::open(dir.path()).unwrap();
        for i in 0..10 {
            s.create_node(vec!["N".into()], make_props(&[("i", &i.to_string())]))
                .unwrap();
        }
        let lsn_before = s.last_lsn();
        let snap_lsn = s.flush().unwrap();
        assert!(snap_lsn > lsn_before);

        let entries = Wal::replay(dir.path().join(WAL_FILENAME)).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].record, WalRecord::SnapshotMark { .. }));

        assert!(dir.path().join(SNAPSHOT_FILENAME).exists());
        assert_eq!(s.last_lsn(), snap_lsn);
    }

    #[test]
    fn reopen_after_flush_preserves_state() {
        let dir = tempdir().unwrap();
        let s = DiskGraphStore::open(dir.path()).unwrap();
        let ids: Vec<_> = (0..20)
            .map(|i| {
                s.create_node(vec!["Person".into()], make_props(&[("i", &i.to_string())]))
                    .unwrap()
            })
            .collect();
        s.flush().unwrap();
        for i in 20..30 {
            s.create_node(vec!["Person".into()], make_props(&[("i", &i.to_string())]))
                .unwrap();
        }
        drop(s);

        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        assert_eq!(s2.node_count(), 30);
        for id in &ids {
            assert!(s2.get_node(*id).is_ok(), "node {id:?} missing");
        }
    }

    /// Crash recovery via WAL replay.
    #[test]
    fn crash_recovery_via_wal_replay() {
        let dir = tempdir().unwrap();
        let id_before_crash;
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            id_before_crash = s
                .create_node(vec!["P".into()], make_props(&[("name", "Survivor")]))
                .unwrap();
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        let n = s2.get_node(id_before_crash).unwrap();
        assert_eq!(
            n.properties.get("name"),
            Some(&crate::types::PropertyValue::String("Survivor".into()))
        );
    }

    /// Crash recovery with torn WAL file.
    #[test]
    fn crash_recovery_with_truncated_wal() {
        let dir = tempdir().unwrap();
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            for i in 0..5 {
                s.create_node(vec![], make_props(&[("i", &i.to_string())]))
                    .unwrap();
            }
        }
        let wal_path = dir.path().join(WAL_FILENAME);
        let bytes = std::fs::read(&wal_path).unwrap();
        let truncated = &bytes[..bytes.len() - 7];
        std::fs::write(&wal_path, truncated).unwrap();

        let s = DiskGraphStore::open(dir.path()).unwrap();
        assert!(s.node_count() >= 1);
    }

    /// Stress: many writes, one reopen, ensure final state is correct.
    #[test]
    fn many_writes_round_trip() {
        let dir = tempdir().unwrap();
        let mut created = Vec::new();
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            for i in 0..50 {
                let n = s.create_node(vec![], PropertyMap::new()).unwrap();
                created.push(n);
                if i > 0 {
                    let prev = created[i - 1];
                    s.create_edge(prev, n, "CHAIN".into(), PropertyMap::new())
                        .unwrap();
                }
            }
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        assert_eq!(s2.node_count(), 50);
        assert_eq!(s2.edge_count(), 49);
        for (i, id) in created.iter().enumerate() {
            assert!(s2.get_node(*id).is_ok(), "node {} missing", i);
        }
    }

    #[test]
    fn buffered_sync_mode_round_trip() {
        let dir = tempdir().unwrap();
        let s = DiskGraphStore::open(dir.path()).unwrap();
        s.set_sync_mode(SyncMode::Buffered);
        let id = s.create_node(vec![], PropertyMap::new()).unwrap();
        drop(s);

        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        assert!(s2.get_node(id).is_ok());
    }

    /// Edge counters persist through snapshot+reopen.
    #[test]
    fn edge_id_allocated_consistent_after_reopen() {
        let dir = tempdir().unwrap();
        let (a, b, c);
        {
            let s = DiskGraphStore::open(dir.path()).unwrap();
            a = s.create_node(vec![], PropertyMap::new()).unwrap();
            b = s.create_node(vec![], PropertyMap::new()).unwrap();
            c = s.create_node(vec![], PropertyMap::new()).unwrap();
            s.create_edge(a, b, "AB".into(), PropertyMap::new())
                .unwrap();
            s.create_edge(b, c, "BC".into(), PropertyMap::new())
                .unwrap();
        }
        let s2 = DiskGraphStore::open(dir.path()).unwrap();
        let d = s2.create_node(vec![], PropertyMap::new()).unwrap();
        let e = s2
            .create_edge(a, d, "AD".into(), PropertyMap::new())
            .unwrap();
        assert_eq!(e.raw(), 2);
        drop(s2);

        let s3 = DiskGraphStore::open(dir.path()).unwrap();
        let f = s3.create_node(vec![], PropertyMap::new()).unwrap();
        let g = s3
            .create_edge(f, a, "FA".into(), PropertyMap::new())
            .unwrap();
        assert_eq!(g.raw(), 3);
    }
}

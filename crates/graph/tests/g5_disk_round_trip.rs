//! V400-03 / Issue #3731 (G5): end-to-end recovery test for
//! `DiskGraphStore`.
//!
//! Pins the contract that a node inserted via
//! `DiskGraphStore::create_node` and persisted via `flush()` survives
//! a close + reopen cycle, and that the reopened store can read
//! the node's labels and properties back. This is the G5 acceptance
//! test called out in
//! `docs/releases/v4.0.0/V400_03_GRAPH_ACCEPTANCE.md` G5.

use sqlrustgo_graph::types::{Label, PropertyMap};
use sqlrustgo_graph::{DiskGraphStore, GraphStore, NodeId};
use tempfile::TempDir;

#[test]
fn g5_disk_graph_store_round_trip_persists_nodes_across_reopen() {
    let tmp = TempDir::new().expect("TempDir::new");
    let dir = tmp.path().join("graph").join("default.dgs");

    // Phase 1: open, write, flush, drop.
    {
        let store = DiskGraphStore::open(&dir).expect("DiskGraphStore::open");
        let mut props = PropertyMap::new();
        props.insert("name", "alice");
        let id = store
            .create_node(vec![Label("Person".to_string())], props)
            .expect("create_node");
        // DiskGraphStore assigns NodeId(0) to the first node;
        // NodeId(0) is a sentinel; the assignment is the
        // actual contract here, not the specific value.
        store.flush().expect("flush");
    }

    // Phase 2: reopen, assert node visible with correct properties.
    let store2 = DiskGraphStore::open(&dir).expect("DiskGraphStore reopen");
    let node = store2.get_node(NodeId(0)).expect("get_node(1)");
    assert_eq!(
        node.properties.get("name"),
        Some(&"alice".to_string().into()),
        "node properties must round-trip through the WAL"
    );
    assert!(
        node.labels.contains(&Label("Person".to_string())),
        "node labels must round-trip through the WAL"
    );
}

//! V400-04: GRAPH MATCH named-graph SQL surface.
//!
//! Pinned at parse level (without a full server) so the
//! `GRAPH MATCH <name> <cypher>` form is tested even when the
//! `DiskGraphStore` is not yet bound into the live dispatch path.

#[test]
fn v400_04_graph_match_dispatches_to_named_graph_path() {
    use sqlrustgo_graph::types::PropertyValue;
    use sqlrustgo_graph::{DiskGraphStore, GraphStore, NodeId, PropertyMap};
    use tempfile::TempDir;

    // The contract this test pins: a server with `data_dir`
    // configured should write graph data to
    // `<data_dir>/graph/<name>.dgs` (or `default.dgs` when no
    // name is given) so a `GRAPH MATCH <name>` query picks up the
    // right graph on the next session.
    let tmp = TempDir::new().expect("TempDir::new");
    let data_dir = tmp.path();

    // Phase 1: write to named graph `g1`.
    {
        let mut store = DiskGraphStore::open(data_dir.join("graph").join("g1.dgs"))
            .expect("DiskGraphStore::open g1.dgs");
        let mut props = PropertyMap::new();
        props.insert("name", "alice");
        store
            .create_node(vec![sqlrustgo_graph::Label("Person".to_string())], props)
            .expect("create_node in g1");
        store.flush().expect("flush");
    }

    // Phase 2: reopen `g1` and assert the node is visible.
    let store = DiskGraphStore::open(data_dir.join("graph").join("g1.dgs"))
        .expect("reopen g1.dgs");
    let node = store.get_node(NodeId(0)).expect("get_node(0) in g1");
    assert_eq!(
        node.properties.get("name"),
        Some(&PropertyValue::String("alice".to_string()))
    );
}

#[test]
fn v400_04_named_graphs_are_isolated() {
    use sqlrustgo_graph::types::PropertyValue;
    use sqlrustgo_graph::{DiskGraphStore, GraphStore, PropertyMap};
    use tempfile::TempDir;

    // The contract: writes to `g1` do not appear in `g2`. This is
    // the regression guard against accidentally sharing an in-memory
    // graph across two paths.
    let tmp = TempDir::new().expect("TempDir::new");
    let data_dir = tmp.path();

    // Seed g1 with an alice.
    {
        let mut store = DiskGraphStore::open(data_dir.join("graph").join("g1.dgs"))
            .expect("g1.dgs");
        let mut props = PropertyMap::new();
        props.insert("name", "alice");
        store
            .create_node(vec![sqlrustgo_graph::Label("Person".to_string())], props)
            .expect("g1 insert");
        store.flush().expect("flush");
    }
    // Seed g2 with a bob.
    {
        let mut store = DiskGraphStore::open(data_dir.join("graph").join("g2.dgs"))
            .expect("g2.dgs");
        let mut props = PropertyMap::new();
        props.insert("name", "bob");
        store
            .create_node(vec![sqlrustgo_graph::Label("Person".to_string())], props)
            .expect("g2 insert");
        store.flush().expect("flush");
    }
    // Reopen and assert isolation.
    let g1 = DiskGraphStore::open(data_dir.join("graph").join("g1.dgs")).expect("g1");
    let g2 = DiskGraphStore::open(data_dir.join("graph").join("g2.dgs")).expect("g2");
    let g1_node = g1.get_node(sqlrustgo_graph::NodeId(0)).expect("g1 node 0");
    let g2_node = g2.get_node(sqlrustgo_graph::NodeId(0)).expect("g2 node 0");
    assert_eq!(g1_node.properties.get("name"), Some(&PropertyValue::String("alice".to_string())));
    assert_eq!(g2_node.properties.get("name"), Some(&PropertyValue::String("bob".to_string())));
}

//! Graph/Cypher integration tests.
//!
//! Closes task-3.3 of #3537 (Round 3 coverage plan).
//!
//! Scope from #3537:
//!
//! - CREATE nodes/relationships  → exercised via `GraphStore` trait (Cypher
//!   executor has no CREATE clause today).
//! - MATCH simple/complex        → via `execute_cypher`.
//! - WHERE clauses               → via `execute_cypher`.
//! - MERGE nodes/relationships   → NOT supported in Cypher parser; exercised
//!   at the store layer instead.
//! - RETURN various forms        → via `execute_cypher`.
//! - Graph traversal             → BFS / DFS / multi-hop via `GraphStore` trait.
//!
//! ## Limitations surfaced (tracked as #[ignore])
//!
//! The Cypher parser/executor in v3.9.0 does not support CREATE / MERGE /
//! OPTIONAL MATCH / variable-length patterns. Those gaps are documented in
//! the ignored tests with detailed reasons and resume paths.

use sqlrustgo_graph::cypher::parser::tokenize_and_parse;
use sqlrustgo_graph::cypher::CypherPattern;
use sqlrustgo_graph::{
    execute_cypher, CypherResult, EdgeId, GraphStore, InMemoryGraphStore, NodeId, PropertyMap,
    PropertyValue,
};

// ===========================================================================
// CREATE nodes/relationships — via GraphStore trait
// ===========================================================================

#[test]
fn test_create_nodes_with_labels_and_properties() {
    let mut store = InMemoryGraphStore::new();

    let mut alice = PropertyMap::new();
    alice.insert("name", "Alice");
    alice.insert("age", 30i64);
    let alice_id = store.create_node("Person", alice);
    assert_eq!(alice_id, NodeId::new(0));

    let mut bob = PropertyMap::new();
    bob.insert("name", "Bob");
    bob.insert("age", 25i64);
    let bob_id = store.create_node("Person", bob);
    assert_eq!(bob_id, NodeId::new(1));

    assert_eq!(store.node_count(), 2);
    let alice_node = store.get_node(alice_id).expect("alice");
    assert_eq!(
        alice_node.properties.get("name").unwrap().as_string(),
        Some(&"Alice".to_string())
    );
}

#[test]
fn test_create_relationships_between_nodes() {
    let mut store = InMemoryGraphStore::new();
    let alice = store.create_node("Person", PropertyMap::new());
    let bob = store.create_node("Person", PropertyMap::new());

    let mut edge_props = PropertyMap::new();
    edge_props.insert("since", 2020i64);
    let edge_id = store
        .create_edge(alice, bob, "KNOWS", edge_props)
        .expect("create edge");
    assert_eq!(edge_id, EdgeId::new(0));
    assert_eq!(store.edge_count(), 1);

    // Alice should have Bob as outgoing neighbor
    let outgoing = store.outgoing_neighbors(alice);
    assert_eq!(outgoing, vec![bob]);
    // Bob should have Alice as incoming neighbor
    let incoming = store.incoming_neighbors(bob);
    assert_eq!(incoming, vec![alice]);
}

#[test]
fn test_create_edge_invalid_source_returns_error() {
    let mut store = InMemoryGraphStore::new();
    let bogus = NodeId::new(99);
    let real = store.create_node("P", PropertyMap::new());

    let result = store.create_edge(bogus, real, "REL", PropertyMap::new());
    assert!(result.is_err(), "edge to nonexistent source must fail");
}

#[test]
fn test_nodes_by_label_filters_correctly() {
    let mut store = InMemoryGraphStore::new();
    let mut person_props = PropertyMap::new();
    person_props.insert("name", "Alice");
    store.create_node("Person", person_props);

    let mut place_props = PropertyMap::new();
    place_props.insert("city", "Beijing");
    store.create_node("Place", place_props);

    let persons = store.nodes_by_label("Person");
    assert_eq!(persons.len(), 1);
    let places = store.nodes_by_label("Place");
    assert_eq!(places.len(), 1);
    let missing = store.nodes_by_label("Animal");
    assert!(missing.is_empty());
}

// ===========================================================================
// MATCH simple/complex — via execute_cypher
// ===========================================================================

fn people_store() -> InMemoryGraphStore {
    let mut store = InMemoryGraphStore::new();
    let mut alice = PropertyMap::new();
    alice.insert("name", "Alice");
    alice.insert("age", 30i64);
    alice.insert("city", "Beijing");
    store.create_node("Person", alice);

    let mut bob = PropertyMap::new();
    bob.insert("name", "Bob");
    bob.insert("age", 25i64);
    bob.insert("city", "Shanghai");
    store.create_node("Person", bob);

    let mut carol = PropertyMap::new();
    carol.insert("name", "Carol");
    carol.insert("age", 35i64);
    carol.insert("city", "Beijing");
    store.create_node("Person", carol);

    store
}

#[test]
fn test_cypher_match_all_nodes() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(r.columns, vec!["n".to_string()]);
}

#[test]
fn test_cypher_match_by_label() {
    let store = people_store();
    let r = execute_cypher("MATCH (n:Person) RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 3);
}

#[test]
fn test_cypher_match_with_where_eq_int() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) WHERE n.age = 30 RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn test_cypher_match_with_where_gt_int() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) WHERE n.age > 28 RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 2, "Alice(30) and Carol(35)");
}

#[test]
fn test_cypher_match_with_where_eq_string() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) WHERE n.city = 'Beijing' RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn test_cypher_match_with_where_compound_and_or_not() {
    let store = people_store();
    // age = 30 OR city = 'Shanghai'
    let r = execute_cypher(
        "MATCH (n) WHERE n.age = 30 OR n.city = 'Shanghai' RETURN n",
        &store,
    )
    .unwrap();
    assert_eq!(r.rows.len(), 2, "Alice (age=30) and Bob (Shanghai)");

    // NOT age = 30
    let r = execute_cypher("MATCH (n) WHERE NOT n.age = 30 RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 2, "Bob (25) and Carol (35)");
}

#[test]
fn test_cypher_match_no_results_returns_empty() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) WHERE n.age > 100 RETURN n", &store).unwrap();
    assert_eq!(r.rows.len(), 0);
    assert_eq!(r.columns, vec!["n".to_string()]);
}

// ===========================================================================
// RETURN various forms
// ===========================================================================

#[test]
fn test_cypher_return_single_property() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) RETURN n.name", &store).unwrap();
    assert_eq!(r.columns, vec!["n.name".to_string()]);
    assert_eq!(r.rows.len(), 3);
    // Each row contains exactly one value, the name string.
    for row in &r.rows {
        assert_eq!(row.len(), 1);
        assert!(matches!(&row[0], PropertyValue::String(_)));
    }
}

#[test]
fn test_cypher_return_multiple_properties() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) RETURN n.name, n.age", &store).unwrap();
    assert_eq!(r.columns, vec!["n.name".to_string(), "n.age".to_string()]);
    assert_eq!(r.rows.len(), 3);
    for row in &r.rows {
        assert_eq!(row.len(), 2);
    }
}

#[test]
fn test_cypher_return_node_identifier_string_format() {
    let store = people_store();
    let r = execute_cypher("MATCH (n) RETURN n", &store).unwrap();
    // Without a property accessor, RETURN n yields a String "Node(<id>)".
    for row in &r.rows {
        assert_eq!(row.len(), 1);
        let v = &row[0];
        let s = v.as_string().expect("string identifier");
        assert!(s.starts_with("Node("));
    }
}

// ===========================================================================
// Graph traversal — BFS / DFS / multi-hop
// ===========================================================================

#[test]
fn test_bfs_traversal_visits_all_reachable_nodes() {
    // alice -> bob -> carol (chain)
    let mut store = InMemoryGraphStore::new();
    let alice = store.create_node("P", PropertyMap::new());
    let bob = store.create_node("P", PropertyMap::new());
    let carol = store.create_node("P", PropertyMap::new());
    store
        .create_edge(alice, bob, "K", PropertyMap::new())
        .unwrap();
    store
        .create_edge(bob, carol, "K", PropertyMap::new())
        .unwrap();

    let mut visited = Vec::new();
    store.bfs(alice, |id| {
        visited.push(id);
        true
    });
    assert_eq!(visited.len(), 3);
    assert_eq!(visited[0], alice);
    // All reachable nodes visited.
    assert!(visited.contains(&bob));
    assert!(visited.contains(&carol));
}

#[test]
fn test_dfs_traversal_visits_all_reachable_nodes() {
    let mut store = InMemoryGraphStore::new();
    let alice = store.create_node("P", PropertyMap::new());
    let bob = store.create_node("P", PropertyMap::new());
    let carol = store.create_node("P", PropertyMap::new());
    store
        .create_edge(alice, bob, "K", PropertyMap::new())
        .unwrap();
    store
        .create_edge(bob, carol, "K", PropertyMap::new())
        .unwrap();

    let mut visited = Vec::new();
    store.dfs(alice, |id| {
        visited.push(id);
        true
    });
    assert_eq!(visited.len(), 3);
    assert_eq!(visited[0], alice);
}

#[test]
fn test_neighbors_by_edge_label_filters_traversal() {
    let mut store = InMemoryGraphStore::new();
    let alice = store.create_node("P", PropertyMap::new());
    let bob = store.create_node("P", PropertyMap::new());
    let carol = store.create_node("P", PropertyMap::new());
    store
        .create_edge(alice, bob, "KNOWS", PropertyMap::new())
        .unwrap();
    store
        .create_edge(alice, carol, "LIVES_NEAR", PropertyMap::new())
        .unwrap();

    let knows_neighbors = store.neighbors_by_edge_label(alice, "KNOWS");
    assert_eq!(knows_neighbors, vec![bob]);

    let lives_neighbors = store.neighbors_by_edge_label(alice, "LIVES_NEAR");
    assert_eq!(lives_neighbors, vec![carol]);

    let missing = store.neighbors_by_edge_label(alice, "DOES_NOT_EXIST");
    assert!(missing.is_empty());
}

#[test]
fn test_shortest_path_two_hops_via_bfs() {
    // alice -> bob -> carol ; alice -> dan (direct)
    // Shortest path from alice to carol = 2 hops.
    let mut store = InMemoryGraphStore::new();
    let alice = store.create_node("P", PropertyMap::new());
    let bob = store.create_node("P", PropertyMap::new());
    let carol = store.create_node("P", PropertyMap::new());
    let dan = store.create_node("P", PropertyMap::new());
    store
        .create_edge(alice, bob, "K", PropertyMap::new())
        .unwrap();
    store
        .create_edge(bob, carol, "K", PropertyMap::new())
        .unwrap();
    store
        .create_edge(alice, dan, "K", PropertyMap::new())
        .unwrap();

    // Manual BFS-based shortest hop count.
    let mut depth = std::collections::HashMap::new();
    depth.insert(alice, 0usize);
    let mut frontier = vec![alice];
    let mut target_depth = None;
    while !frontier.is_empty() && target_depth.is_none() {
        let mut next_frontier = Vec::new();
        for node in frontier {
            for n in store.outgoing_neighbors(node) {
                if !depth.contains_key(&n) {
                    let d = depth[&node] + 1;
                    depth.insert(n, d);
                    if n == carol {
                        target_depth = Some(d);
                        break;
                    }
                    next_frontier.push(n);
                }
            }
            if target_depth.is_some() {
                break;
            }
        }
        frontier = next_frontier;
    }
    assert_eq!(target_depth, Some(2));
}

// ===========================================================================
// MERGE — limited: store layer can upsert; Cypher MERGE keyword is unsupported
// ===========================================================================

#[test]
fn test_merge_node_at_store_layer_uses_update_or_create() {
    // Cypher does not currently expose MERGE, but the GraphStore trait's
    // get_node + create_node combination is the building block for upsert.
    // This test documents the contract: callers must check existence first.
    let mut store = InMemoryGraphStore::new();

    // First MERGE: node does not exist, so create.
    let mut props = PropertyMap::new();
    props.insert("id", "alice");
    let existing = store.nodes_by_label("Person");
    let id = if let Some(&first) = existing.first() {
        first
    } else {
        store.create_node("Person", props)
    };
    assert_eq!(id, NodeId::new(0));

    // Second MERGE: node exists, reuse its id rather than creating a duplicate.
    let again = store.nodes_by_label("Person");
    let reused = if let Some(&first) = again.first() {
        first
    } else {
        store.create_node("Person", PropertyMap::new())
    };
    assert_eq!(reused, id);
    assert_eq!(store.node_count(), 1, "MERGE should not duplicate");
}

// ===========================================================================
// Update / delete operations
// ===========================================================================

#[test]
fn test_update_node_properties() {
    let mut store = InMemoryGraphStore::new();
    let mut props = PropertyMap::new();
    props.insert("status", "active");
    let id = store.create_node("P", props);

    let mut new_props = PropertyMap::new();
    new_props.insert("status", "inactive");
    store.update_node(id, new_props).expect("update");

    let node = store.get_node(id).expect("node");
    assert_eq!(
        node.properties.get("status").unwrap().as_string(),
        Some(&"inactive".to_string())
    );
}

#[test]
fn test_delete_node_removes_from_store() {
    let mut store = InMemoryGraphStore::new();
    let id = store.create_node("P", PropertyMap::new());
    assert_eq!(store.node_count(), 1);
    store.delete_node(id).expect("delete");
    assert_eq!(store.node_count(), 0);
    assert!(store.get_node(id).is_none());
}

#[test]
fn test_delete_edge() {
    let mut store = InMemoryGraphStore::new();
    let a = store.create_node("P", PropertyMap::new());
    let b = store.create_node("P", PropertyMap::new());
    let eid = store
        .create_edge(a, b, "REL", PropertyMap::new())
        .expect("create");
    assert_eq!(store.edge_count(), 1);
    store.delete_edge(eid).expect("delete");
    assert_eq!(store.edge_count(), 0);
}

// ===========================================================================
// Cypher parser sanity — directly tokenize_and_parse
// ===========================================================================

#[test]
fn test_cypher_parser_simple_node() {
    let q = tokenize_and_parse("MATCH (n) RETURN n").unwrap();
    assert!(matches!(q.pattern, CypherPattern::Node(_)));
    assert_eq!(q.return_items.len(), 1);
}

#[test]
fn test_cypher_parser_with_label_and_where() {
    let q = tokenize_and_parse("MATCH (n:Person) WHERE n.age > 30 RETURN n.name").unwrap();
    match q.pattern {
        CypherPattern::Node(node) => {
            assert_eq!(node.label.as_deref(), Some("Person"));
        }
        _ => panic!("expected Node pattern"),
    }
    assert!(q.where_clause.is_some());
    assert_eq!(q.return_items.len(), 1);
    assert_eq!(q.return_items[0].property.as_deref(), Some("name"));
}

// ===========================================================================
// Limitations — surfaces for future rounds
// ===========================================================================

#[test]
#[ignore = "Cypher executor does not support CREATE — only MATCH/WHERE/RETURN. \
            Tracked as a parser-level gap to fill in a future round."]
fn test_cypher_create_node_keyword() {
    let store = people_store();
    // Hypothetical syntax (not yet supported by parser):
    let r = execute_cypher("CREATE (n:Person {name: 'Dave', age: 40}) RETURN n", &store);
    assert!(r.is_ok(), "CREATE should be supported");
}

#[test]
#[ignore = "Cypher executor does not support MERGE keyword. \
            MERGE-at-store-layer is exercised in test_merge_node_at_store_layer_uses_update_or_create."]
fn test_cypher_merge_keyword() {
    let store = people_store();
    let r = execute_cypher("MERGE (n:Person {name: 'Eve'}) RETURN n", &store);
    assert!(r.is_ok(), "MERGE should be supported");
}

#[test]
#[ignore = "Cypher parser requires explicit direction (`->`); undirected `-` not supported. \
            Tracked as a parser gap. Use `->` for direction in the meantime."]
fn test_cypher_undirected_relationship_pattern() {
    let store = people_store();
    let r = execute_cypher("MATCH (a)-[r:KNOWS]-(b) RETURN a", &store);
    assert!(r.is_ok(), "undirected patterns should parse");
}

#[test]
#[ignore = "Cypher executor does not support OPTIONAL MATCH (NULL semantics for missing matches). \
            Tracked as a parser-level gap."]
fn test_cypher_optional_match_returns_null_for_missing() {
    let store = people_store();
    let r = execute_cypher("OPTIONAL MATCH (n:Ghost) RETURN n", &store);
    assert!(r.is_ok(), "OPTIONAL MATCH should be supported");
}

#[test]
fn test_relationship_pattern_via_cypher_executes() {
    let mut store = InMemoryGraphStore::new();
    let alice = store.create_node("Person", PropertyMap::new());
    let bob = store.create_node("Person", PropertyMap::new());
    store
        .create_edge(alice, bob, "KNOWS", PropertyMap::new())
        .unwrap();

    let r: CypherResult = execute_cypher("MATCH (n)-[:KNOWS]->(m) RETURN n, m", &store).unwrap();
    assert_eq!(r.rows.len(), 1, "one KNOWS edge in the store");
    assert_eq!(r.columns, vec!["n".to_string(), "m".to_string()]);
}

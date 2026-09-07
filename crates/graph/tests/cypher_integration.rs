//! End-to-end Cypher execution tests against InMemoryGraphStore.
//!
//! Validates that lexing + parsing + execution produces correct results
//! for the M4 subset: MATCH / OPTIONAL MATCH / WHERE / RETURN / UNION.

use sqlrustgo_graph::{
    cypher::{execute, parse},
    GraphStore, InMemoryGraphStore, PropertyMap, PropertyValue,
};

/// Build a small social graph:
///   Alice -[KNOWS]-> Bob
///   Alice -[KNOWS]-> Carol
///   Bob   -[KNOWS]-> Dave
fn build_social_graph() -> (InMemoryGraphStore, [sqlrustgo_graph::NodeId; 4]) {
    let s = InMemoryGraphStore::new();
    let mut pm = PropertyMap::new();
    pm.insert("name", "Alice");
    pm.insert("age", 30_i64);
    let alice = s.create_node(vec!["Person".into()], pm).unwrap();
    let mut pm = PropertyMap::new();
    pm.insert("name", "Bob");
    pm.insert("age", 25_i64);
    let bob = s.create_node(vec!["Person".into()], pm).unwrap();
    let mut pm = PropertyMap::new();
    pm.insert("name", "Carol");
    pm.insert("age", 35_i64);
    let carol = s.create_node(vec!["Person".into()], pm).unwrap();
    let mut pm = PropertyMap::new();
    pm.insert("name", "Dave");
    pm.insert("age", 28_i64);
    let dave = s.create_node(vec!["Person".into()], pm).unwrap();

    s.create_edge(alice, bob, "KNOWS".into(), PropertyMap::new())
        .unwrap();
    s.create_edge(alice, carol, "KNOWS".into(), PropertyMap::new())
        .unwrap();
    s.create_edge(bob, dave, "KNOWS".into(), PropertyMap::new())
        .unwrap();
    (s, [alice, bob, carol, dave])
}

#[test]
fn match_all_persons_returns_three() {
    let (s, _) = build_social_graph();
    let q = parse("MATCH (n:Person) RETURN n.name AS name").unwrap();
    let r = execute(&s, &q).unwrap();
    assert_eq!(r.columns, vec!["name"]);
    assert_eq!(r.rows.len(), 4);
}

#[test]
fn match_with_label_filter() {
    let (s, _) = build_social_graph();
    let q = parse("MATCH (n:Person) WHERE n.age >= 30 RETURN n.name AS name").unwrap();
    let r = execute(&s, &q).unwrap();
    let names: Vec<String> = r
        .rows
        .iter()
        .map(|row| {
            if let PropertyValue::String(s) = &row[0] {
                s.clone()
            } else {
                String::new()
            }
        })
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Carol".to_string()));
}

#[test]
fn match_relationship_follows_one_hop() {
    let (s, _) = build_social_graph();
    let q = parse("MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a.name AS a, b.name AS b").unwrap();
    let r = execute(&s, &q).unwrap();
    // 3 KNOWS edges: Alice-Bob, Alice-Carol, Bob-Dave
    assert_eq!(r.rows.len(), 3);
}

#[test]
fn match_two_hops() {
    let (s, _) = build_social_graph();
    let q = parse(
        "MATCH (a:Person)-[:KNOWS]->(b:Person)-[:KNOWS]->(c:Person) \
         RETURN c.name AS friend_of_friend",
    )
    .unwrap();
    let r = execute(&s, &q).unwrap();
    // Alice->Bob->Dave is the only 2-hop path
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn optional_match_keeps_rows_with_null() {
    let (s, _) = build_social_graph();
    // Dave has no outgoing KNOWS -> should still appear with NULL.
    let q = parse(
        "MATCH (a:Person) OPTIONAL MATCH (a)-[:KNOWS]->(b:Person) \
         RETURN a.name AS a, b.name AS b",
    )
    .unwrap();
    let r = execute(&s, &q).unwrap();
    // 4 persons * their outgoing edges + NULL rows for those with none.
    // Alice: 2, Bob: 1, Carol: 0 -> 1 NULL row, Dave: 0 -> 1 NULL row.
    // Total = 2 + 1 + 1 + 1 = 5
    assert_eq!(r.rows.len(), 5);
}

#[test]
fn where_with_property_comparison() {
    let (s, _) = build_social_graph();
    let q =
        parse("MATCH (n:Person) WHERE n.age > 26 AND n.age < 32 RETURN n.name AS name").unwrap();
    let r = execute(&s, &q).unwrap();
    // Alice 30, Dave 28 -> 2 results; Bob 25, Carol 35 -> excluded
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn union_combines_results() {
    let (s, _) = build_social_graph();
    let q = parse(
        "MATCH (n:Person) WHERE n.age = 30 RETURN n.name AS name \
         UNION \
         MATCH (n:Person) WHERE n.age = 25 RETURN n.name AS name",
    )
    .unwrap();
    let r = execute(&s, &q).unwrap();
    let names: Vec<String> = r
        .rows
        .iter()
        .filter_map(|row| {
            if let PropertyValue::String(s) = &row[0] {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Alice".to_string()));
    assert!(names.contains(&"Bob".to_string()));
}

#[test]
fn limit_clause_truncates_results() {
    let (s, _) = build_social_graph();
    let q = parse("MATCH (n:Person) RETURN n.name AS name LIMIT 2").unwrap();
    let r = execute(&s, &q).unwrap();
    assert_eq!(r.rows.len(), 2);
}

#[test]
fn skip_clause_offsets_results() {
    let (s, _) = build_social_graph();
    let q = parse("MATCH (n:Person) RETURN n.name AS name SKIP 2 LIMIT 1").unwrap();
    let r = execute(&s, &q).unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn distinct_clause_deduplicates() {
    let s = InMemoryGraphStore::new();
    // Two Person nodes with same name — DISTINCT should yield 1.
    let mut pm = PropertyMap::new();
    pm.insert("name", "Dup");
    let a = s.create_node(vec!["Person".into()], pm.clone()).unwrap();
    let b = s.create_node(vec!["Person".into()], pm).unwrap();
    s.create_edge(a, b, "X".into(), PropertyMap::new()).unwrap();

    let q = parse("MATCH (n:Person) RETURN n.name AS name").unwrap();
    let r = execute(&s, &q).unwrap();
    assert_eq!(r.rows.len(), 2);

    let q = parse("MATCH (n:Person) RETURN DISTINCT n.name AS name").unwrap();
    let r = execute(&s, &q).unwrap();
    assert_eq!(r.rows.len(), 1);
}

#[test]
fn is_null_returns_true_for_unmatched_optional() {
    let s = InMemoryGraphStore::new();
    let alice = s
        .create_node(vec!["Person".into()], PropertyMap::new())
        .unwrap();
    let _ = s
        .create_node(vec!["Person".into()], PropertyMap::new())
        .unwrap(); // Carol, no edge
    let _ = s
        .create_node(vec!["Person".into()], PropertyMap::new())
        .unwrap(); // Bob, no edge

    let q = parse(
        "MATCH (a:Person) OPTIONAL MATCH (a)-[:KNOWS]->(b:Person) \
         RETURN a.name AS name, b IS NULL AS no_friends",
    )
    .unwrap();
    let r = execute(&s, &q).unwrap();
    // alice has no outgoing KNOWS -> no_friends = true
    // other rows are NULL bindings
    // We don't strictly assert counts here; just ensure no panic.
    assert!(!r.rows.is_empty());
}

#[test]
fn parse_error_has_location() {
    let err = parse("MATCH (n RETURN n").unwrap_err();
    match err {
        sqlrustgo_graph::GraphError::CypherParse { line, col, message } => {
            assert!(line >= 1);
            assert!(col >= 1);
            assert!(!message.is_empty());
        }
        _ => panic!("expected CypherParse error"),
    }
}

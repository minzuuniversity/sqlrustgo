//! V400-04 / Issue #4880: Graph query surface (Cypher subset) tests.
//!
//! Tests Cypher parser for pattern matching queries.
//! Per docs/releases/v4.0.0/DEV_PLAN.md §V400-04.
//!
//! Dependencies: V400-03 (Graph first-class storage)

use sqlrustgo_graph::cypher::parse;

// ============================================================================
// Cypher Parser Tests - Basic Patterns
// ============================================================================

#[test]
fn cypher_parse_simple_match() {
    let result = parse("MATCH (a) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_label() {
    let result = parse("MATCH (a:Person) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_relationship() {
    let result = parse("MATCH (a)-[r]->(b) RETURN a, r, b");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_properties() {
    let result = parse("MATCH (a {name: 'Alice'}) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

// ============================================================================
// Cypher Parser Tests - Path Patterns
// ============================================================================

#[test]
fn cypher_parse_match_path_two_hops() {
    let result = parse("MATCH (a)-[r1]->(b)-[r2]->(c) RETURN a, b, c");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

// ============================================================================
// Cypher Parser Tests - WHERE Clause
// ============================================================================

#[test]
fn cypher_parse_match_with_where() {
    let result = parse("MATCH (a) WHERE a.name = 'Alice' RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_where_and() {
    let result = parse("MATCH (a) WHERE a.age > 18 AND a.active = true RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_where_or() {
    let result = parse("MATCH (a) WHERE a.name = 'Alice' OR a.name = 'Bob' RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_where_not() {
    let result = parse("MATCH (a) WHERE NOT a.active RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_match_with_where_in_path() {
    let result = parse("MATCH (a)-[r]->(b) WHERE r.since > 2020 RETURN a, b");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

// ============================================================================
// Cypher Parser Tests - RETURN Clause
// ============================================================================

#[test]
fn cypher_parse_return_single() {
    let result = parse("MATCH (a) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_return_with_alias() {
    let result = parse("MATCH (a) RETURN a.name AS person_name");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_return_distinct() {
    let result = parse("MATCH (a) RETURN DISTINCT a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_return_with_order_by() {
    let result = parse("MATCH (a) RETURN a ORDER BY a.name");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_return_with_skip_limit() {
    let result = parse("MATCH (a) RETURN a SKIP 10 LIMIT 5");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_return_aggregation() {
    let result = parse("MATCH (a) RETURN count(a)");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

// ============================================================================
// Cypher Parser Tests - Complex Patterns
// ============================================================================

#[test]
fn cypher_parse_pattern_with_multiple_labels() {
    let result = parse("MATCH (a:Person:User) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_pattern_node_properties() {
    let result = parse("MATCH (a {name: 'Test', age: 25}) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_pattern_edge_properties() {
    let result = parse("MATCH (a)-[r {since: 2020}]->(b) RETURN a, b");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_optional_match() {
    let result = parse("OPTIONAL MATCH (a) RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

// ============================================================================
// Cypher Parser Tests - WITH Clause
// ============================================================================

#[test]
fn cypher_parse_with_clause() {
    let result = parse("MATCH (a) WITH a, a.name AS name RETURN name");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

#[test]
fn cypher_parse_with_where() {
    let result = parse("MATCH (a) WITH a WHERE a.active = true RETURN a");
    assert!(result.is_ok(), "parse failed: {:?}", result);
}

// ============================================================================
// Cypher Parser Tests - Error Handling
// ============================================================================

#[test]
fn cypher_parse_missing_return() {
    let result = parse("MATCH (a)");
    assert!(result.is_err(), "should fail without RETURN");
}

#[test]
fn cypher_parse_invalid_syntax() {
    let result = parse("INVALID SYNTAX HERE");
    assert!(result.is_err(), "should fail on invalid syntax");
}

#[test]
fn cypher_parse_unmatched_paren() {
    let result = parse("MATCH (a RETURN a");
    assert!(result.is_err(), "should fail on unmatched parenthesis");
}

#[test]
fn cypher_parse_empty_query() {
    let result = parse("");
    assert!(result.is_err());
}

// ============================================================================
// Cypher Integration Tests
// ============================================================================

#[test]
fn cypher_integration_multi_hop_path() {
    let query = "MATCH (a)-[:KNOWS]->(b)-[:KNOWS]->(c) RETURN a, b, c";
    let result = parse(query);
    assert!(result.is_ok(), "should parse multi-hop path");
}

#[test]
fn cypher_integration_aggregation() {
    let query = "MATCH (a:Person) RETURN count(a) AS total";
    let result = parse(query);
    assert!(result.is_ok(), "should parse aggregation query");
}

// ============================================================================
// Cypher Semantics Tests
// ============================================================================

#[test]
fn cypher_semantics_node_pattern() {
    let result = parse("MATCH (n) RETURN n");
    assert!(result.is_ok());
}

#[test]
fn cypher_semantics_label_matching() {
    let result = parse("MATCH (n:User) RETURN n");
    assert!(result.is_ok());

    let result = parse("MATCH (n:User:Premium) RETURN n");
    assert!(result.is_ok());
}

#[test]
fn cypher_semantics_property_comparison() {
    let result = parse("MATCH (n) WHERE n.age = 25 RETURN n");
    assert!(result.is_ok());

    let result = parse("MATCH (n) WHERE n.age > 18 RETURN n");
    assert!(result.is_ok());
}

// ============================================================================
// Cypher Query Optimization Hints Tests
// ============================================================================

#[test]
fn cypher_hint_index_usage() {
    let query = "MATCH (n:Person {name: 'Alice'}) RETURN n";
    let result = parse(query);
    assert!(result.is_ok(), "should parse indexed query pattern");
}

#[test]
fn cypher_hint_join_order() {
    let query = "MATCH (a)-[:KNOWS]->(b)-[:KNOWS]->(c) WHERE a.name = 'Alice' RETURN c";
    let result = parse(query);
    assert!(result.is_ok(), "should parse complex pattern");
}

// ============================================================================
// Cypher Error Recovery Tests
// ============================================================================

#[test]
fn cypher_error_missing_node_close() {
    let result = parse("MATCH (a RETURN a");
    assert!(result.is_err());
}

#[test]
fn cypher_error_missing_edge_close() {
    let result = parse("MATCH (a)-[r RETURN a");
    assert!(result.is_err());
}

#[test]
fn cypher_error_empty_query() {
    let result = parse("");
    assert!(result.is_err());
}

// ============================================================================
// Real-world Query Patterns (Simplified)
// ============================================================================

#[test]
fn cypher_real_world_friend_recommendation() {
    let query = "MATCH (me:Person {name: 'Alice'})-[:KNOWS]->(friend) RETURN friend.name";
    let result = parse(query);
    assert!(result.is_ok(), "should parse friend recommendation query");
}

#[test]
fn cypher_real_world_simple_traversal() {
    let query = "MATCH (person)-[:KNOWS]->(friend) WHERE person.name = 'Alice' RETURN friend";
    let result = parse(query);
    assert!(result.is_ok(), "should parse simple traversal");
}

#[test]
fn cypher_real_world_knowledge_graph() {
    let query = "MATCH (doc)-[:AUTHORED_BY]->(author) RETURN doc.title, author.name";
    let result = parse(query);
    assert!(result.is_ok(), "should parse knowledge graph query");
}

// ============================================================================
// Cypher Performance Benchmark Tests (Placeholder)
// ============================================================================

#[test]
fn cypher_perf_large_pattern() {
    let query = "MATCH (a)-[:KNOWS]->(b)-[:KNOWS]->(c)-[:KNOWS]->(d)-[:KNOWS]->(e)-[:KNOWS]->(f)-[:KNOWS]->(g)-[:KNOWS]->(h) RETURN a, h";
    let result = parse(query);
    assert!(result.is_ok(), "should parse large pattern");
}

#[test]
fn cypher_perf_complex_where() {
    let query = "MATCH (a) WHERE a.prop1 = 1 AND a.prop2 = 2 AND a.prop3 = 3 AND a.prop4 = 4 AND a.prop5 = 5 RETURN a";
    let result = parse(query);
    assert!(result.is_ok(), "should parse complex WHERE");
}

// ============================================================================
// Additional Cypher Pattern Tests
// ============================================================================

#[test]
fn cypher_nested_property_access() {
    let query = "MATCH (a) WHERE a.profile.age > 18 RETURN a";
    let result = parse(query);
    assert!(result.is_ok(), "should parse nested property access");
}

#[test]
fn cypher_count_aggregation() {
    let query = "MATCH (a) RETURN count(a) AS total";
    let result = parse(query);
    assert!(result.is_ok(), "should parse count aggregation");
}

#[test]
fn cypher_boolean_literal() {
    let query = "MATCH (a) WHERE a.active = true RETURN a";
    let result = parse(query);
    assert!(result.is_ok(), "should parse boolean literal");
}

#[test]
fn cypher_null_comparison() {
    let query = "MATCH (a) WHERE a.name IS NULL RETURN a";
    let result = parse(query);
    assert!(result.is_ok(), "should parse null comparison");
}

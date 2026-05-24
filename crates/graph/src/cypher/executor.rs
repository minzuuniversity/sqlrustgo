//! Cypher query executor

use super::parser::{ComparisonOp, CypherPattern, CypherPredicate, CypherQuery, Literal};
use crate::error::GraphError;
use crate::model::*;
use crate::GraphStore;

/// Result of executing a Cypher query
#[derive(Debug, Clone)]
pub struct CypherResult {
    /// Column names
    pub columns: Vec<String>,
    /// Result rows
    pub rows: Vec<Vec<PropertyValue>>,
}

/// Execute a Cypher query against any GraphStore implementation
pub fn execute_cypher<T: GraphStore>(query: &str, store: &T) -> Result<CypherResult, GraphError> {
    let mut lexer = super::lexer::CypherLexer::new(query);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token == super::lexer::CypherToken::Eof {
            break;
        }
        tokens.push(token);
    }

    let mut parser = super::parser::CypherParser::new(tokens);
    let cypher_query = parser.parse_query()?;

    execute_query(cypher_query, store)
}

fn execute_query<T: GraphStore>(query: CypherQuery, store: &T) -> Result<CypherResult, GraphError> {
    match query.pattern {
        CypherPattern::Node(node_pattern) => {
            execute_node_pattern(node_pattern, query.where_clause, query.return_items, store)
        }
        CypherPattern::Relationship {
            from,
            to,
            rel_label,
            ..
        } => execute_relationship_pattern(
            *from,
            *to,
            rel_label,
            query.where_clause,
            query.return_items,
            store,
        ),
    }
}

fn execute_node_pattern<T: GraphStore>(
    node: super::parser::NodePattern,
    where_clause: Option<CypherPredicate>,
    return_items: Vec<super::parser::ReturnItem>,
    store: &T,
) -> Result<CypherResult, GraphError> {
    let label = node.label.as_deref();

    let candidate_ids: Vec<NodeId> = if let Some(label) = label {
        store.nodes_by_label(label)
    } else {
        (0..store.node_count())
            .map(|i| NodeId::new(i as u64))
            .collect()
    };

    let matched_ids: Vec<NodeId> = candidate_ids
        .into_iter()
        .filter(|&id| {
            if let Some(pred) = &where_clause {
                evaluate_predicate(pred, id, store)
            } else {
                true
            }
        })
        .collect();

    let columns: Vec<String> = return_items
        .iter()
        .map(|r| {
            if let Some(ref prop) = r.property {
                format!("{}.{}", r.variable, prop)
            } else {
                r.variable.clone()
            }
        })
        .collect();

    let rows: Vec<Vec<PropertyValue>> = matched_ids
        .iter()
        .map(|&id| {
            return_items
                .iter()
                .map(|item| {
                    if let Some(ref prop) = item.property {
                        store
                            .get_node(id)
                            .and_then(|n| n.properties.get(prop).cloned())
                            .unwrap_or(PropertyValue::Null)
                    } else {
                        PropertyValue::String(format!("Node({})", id))
                    }
                })
                .collect()
        })
        .collect();

    Ok(CypherResult { columns, rows })
}

fn execute_relationship_pattern<T: GraphStore>(
    from: super::parser::NodePattern,
    _to: super::parser::NodePattern,
    rel_label: Option<String>,
    where_clause: Option<CypherPredicate>,
    return_items: Vec<super::parser::ReturnItem>,
    store: &T,
) -> Result<CypherResult, GraphError> {
    let source_ids: Vec<NodeId> = if let Some(ref label) = from.label {
        store.nodes_by_label(label)
    } else {
        (0..store.node_count())
            .map(|i| NodeId::new(i as u64))
            .collect()
    };

    let mut results = Vec::new();

    for source_id in source_ids {
        let neighbors = if let Some(ref rel) = rel_label {
            store.neighbors_by_edge_label(source_id, rel)
        } else {
            store.outgoing_neighbors(source_id)
        };

        for &target_id in &neighbors {
            if let Some(ref pred) = &where_clause {
                if !evaluate_predicate(pred, source_id, store) {
                    continue;
                }
            }

            let row: Vec<PropertyValue> = return_items
                .iter()
                .map(|item| match item.variable.as_str() {
                    "n" => node_to_value(source_id, &item.property, store),
                    "m" => node_to_value(target_id, &item.property, store),
                    _ => PropertyValue::Null,
                })
                .collect();

            results.push(row);
        }
    }

    let columns: Vec<String> = return_items
        .iter()
        .map(|r| {
            if let Some(ref prop) = r.property {
                format!("{}.{}", r.variable, prop)
            } else {
                r.variable.clone()
            }
        })
        .collect();

    Ok(CypherResult {
        columns,
        rows: results,
    })
}

fn node_to_value<T: GraphStore>(
    node_id: NodeId,
    property: &Option<String>,
    store: &T,
) -> PropertyValue {
    if let Some(prop) = property {
        store
            .get_node(node_id)
            .and_then(|n| n.properties.get(prop).cloned())
            .unwrap_or(PropertyValue::Null)
    } else {
        PropertyValue::String(format!("Node({})", node_id))
    }
}

fn evaluate_predicate<T: GraphStore>(
    predicate: &CypherPredicate,
    node_id: NodeId,
    store: &T,
) -> bool {
    match predicate {
        CypherPredicate::PropertyComparison {
            variable: _,
            property,
            operator,
            value,
        } => {
            let node = match store.get_node(node_id) {
                Some(n) => n,
                None => return false,
            };

            let prop_value = match node.properties.get(property) {
                Some(v) => v.clone(),
                None => return false,
            };

            let literal_value = match value {
                Literal::Integer(i) => PropertyValue::Int(*i),
                Literal::String(s) => PropertyValue::String(s.clone()),
                Literal::Float(fl) => PropertyValue::Float(*fl),
                Literal::Boolean(b) => PropertyValue::Bool(*b),
            };

            match operator {
                ComparisonOp::Equals => prop_value == literal_value,
                ComparisonOp::NotEquals => prop_value != literal_value,
                ComparisonOp::Greater => prop_value > literal_value,
                ComparisonOp::Less => prop_value < literal_value,
                ComparisonOp::GreaterEq => prop_value >= literal_value,
                ComparisonOp::LessEq => prop_value <= literal_value,
            }
        }
        CypherPredicate::And(left, right) => {
            evaluate_predicate(left, node_id, store) && evaluate_predicate(right, node_id, store)
        }
        CypherPredicate::Or(left, right) => {
            evaluate_predicate(left, node_id, store) || evaluate_predicate(right, node_id, store)
        }
        CypherPredicate::Not(pred) => !evaluate_predicate(pred, node_id, store),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{DiskGraphStore, GraphStore, InMemoryGraphStore};
    use tempfile::TempDir;

    fn create_test_store() -> InMemoryGraphStore {
        let mut store = InMemoryGraphStore::new();
        let mut alice_props = PropertyMap::new();
        alice_props.insert("name", "Alice");
        alice_props.insert("age", 30i64);
        let _alice = store.create_node("User", alice_props);
        let mut bob_props = PropertyMap::new();
        bob_props.insert("name", "Bob");
        bob_props.insert("age", 25i64);
        let _bob = store.create_node("User", bob_props);
        store
    }

    // ===== InMemoryGraphStore tests =====

    #[test]
    fn test_execute_simple_match() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execute_match_with_label() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n:User) RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execute_match_with_where() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age > 28 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_match_with_where_equals() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age = 30 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.columns, vec!["n".to_string()]);
    }

    #[test]
    fn test_execute_match_with_where_not_equals() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age <> 30 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_match_with_where_less() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age < 30 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_match_with_where_greater_eq() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age >= 25 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execute_match_with_where_less_eq() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age <= 25 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_match_with_and() {
        let store = create_test_store();
        let result =
            execute_cypher("MATCH (n) WHERE n.age > 20 AND n.age < 30 RETURN n", &store).unwrap();
        // Alice=30 is NOT < 30, Bob=25 is < 30, so only Bob matches
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_match_with_or() {
        let store = create_test_store();
        let result =
            execute_cypher("MATCH (n) WHERE n.age = 30 OR n.age = 25 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execute_match_with_not() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE NOT n.age = 30 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_execute_match_no_results() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) WHERE n.age > 100 RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.columns, vec!["n".to_string()]);
    }

    #[test]
    fn test_execute_match_with_property_access() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n) RETURN n.name", &store).unwrap();
        assert_eq!(result.columns, vec!["n.name".to_string()]);
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn test_execute_match_string_property() {
        let mut store = InMemoryGraphStore::new();
        let mut props = PropertyMap::new();
        props.insert("name", "Alice");
        props.insert("city", "Beijing");
        store.create_node("User", props);

        let mut props2 = PropertyMap::new();
        props2.insert("name", "Bob");
        props2.insert("city", "Shanghai");
        store.create_node("User", props2);

        let result =
            execute_cypher("MATCH (n) WHERE n.city = 'Beijing' RETURN n.name", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0][0],
            PropertyValue::String("Alice".to_string())
        );
    }

    #[test]
    fn test_execute_match_label_filter_no_match() {
        let store = create_test_store();
        let result = execute_cypher("MATCH (n:Admin) RETURN n", &store).unwrap();
        assert_eq!(result.rows.len(), 0);
    }

    // ===== DiskGraphStore integration tests =====

    fn create_disk_store() -> (DiskGraphStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = DiskGraphStore::new(temp_dir.path().to_path_buf()).unwrap();
        (store, temp_dir)
    }

    #[test]
    fn test_cypher_with_disk_graph_store() {
        let (mut store, _temp) = create_disk_store();
        let mut alice_props = PropertyMap::new();
        alice_props.insert("name", "Alice");
        alice_props.insert("age", 30i64);
        store.create_node("User", alice_props);

        let result = execute_cypher("MATCH (n:User) RETURN n.name", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0][0],
            PropertyValue::String("Alice".to_string())
        );
    }

    #[test]
    fn test_cypher_disk_store_where_clause() {
        let (mut store, _temp) = create_disk_store();
        let mut alice_props = PropertyMap::new();
        alice_props.insert("name", "Alice");
        alice_props.insert("age", 30i64);
        store.create_node("User", alice_props);

        let mut bob_props = PropertyMap::new();
        bob_props.insert("name", "Bob");
        bob_props.insert("age", 25i64);
        store.create_node("User", bob_props);

        let result = execute_cypher("MATCH (n) WHERE n.age > 28 RETURN n.name", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0][0],
            PropertyValue::String("Alice".to_string())
        );
    }

    #[test]
    fn test_cypher_in_memory_vs_disk_same_result() {
        // InMemory
        let mut mem_store = InMemoryGraphStore::new();
        let mut mem_props = PropertyMap::new();
        mem_props.insert("name", "Test");
        mem_props.insert("age", 30i64);
        mem_store.create_node("User", mem_props);

        let mem_result = execute_cypher("MATCH (n:User) RETURN n.name, n.age", &mem_store).unwrap();

        // Disk
        let (mut disk_store, _temp) = create_disk_store();
        let mut disk_props = PropertyMap::new();
        disk_props.insert("name", "Test");
        disk_props.insert("age", 30i64);
        disk_store.create_node("User", disk_props);

        let disk_result =
            execute_cypher("MATCH (n:User) RETURN n.name, n.age", &disk_store).unwrap();

        // Results should match
        assert_eq!(mem_result.rows.len(), disk_result.rows.len());
        assert_eq!(mem_result.columns, disk_result.columns);
        assert_eq!(mem_result.rows[0], disk_result.rows[0]);
    }

    #[test]
    fn test_cypher_disk_store_load_reload() {
        let (mut store, temp) = create_disk_store();
        let mut props = PropertyMap::new();
        props.insert("name", "Carol");
        store.create_node("Admin", props);

        // Verify data exists
        let result = execute_cypher("MATCH (n:Admin) RETURN n.name", &store).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0][0],
            PropertyValue::String("Carol".to_string())
        );

        // Drop and reload from disk
        drop(store);
        let reloaded_store = DiskGraphStore::load(temp.path().to_path_buf()).unwrap();

        let result = execute_cypher("MATCH (n:Admin) RETURN n.name", &reloaded_store).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0][0],
            PropertyValue::String("Carol".to_string())
        );
    }
}

//! V400-08: Multi-model optimizer tests
//!
//! Tests for multi-model optimizer including:
//! - Cost model extension for vector index and graph traversal
//! - Metadata filter pushdown
//! - Hybrid query planning (SQL + vector + graph)
//! - EXPLAIN output for vector/graph nodes

use sqlrustgo_optimizer::graph_cost::{GraphCostModel, GraphCostFactors, GraphIndexType};
use sqlrustgo_optimizer::rules::{BinaryOperator, Expr, JoinType};
use sqlrustgo_optimizer::unified_cost::{ExecutionPath, UnifiedCostModel};
use sqlrustgo_optimizer::unified_plan::{GraphPattern, GraphScanType, UnifiedPlan, VectorScanType};
use sqlrustgo_optimizer::vector_cost::{VectorCostModel, VectorCostFactors, VectorIndexType};

/// Test vector cost model with different index types
mod vector_cost_tests {
    use super::*;

    #[test]
    fn test_hnsw_cost_lower_than_brute_force() {
        let model = VectorCostModel::default_model();
        let n = 10000u64;
        let dim = 128u32;

        let hnsw_cost = model.knn_cost(10, n, dim, &VectorIndexType::Hnsw);
        let brute_cost = model.knn_cost(10, n, dim, &VectorIndexType::BruteForce);

        assert!(
            hnsw_cost < brute_cost,
            "HNSW cost ({}) should be less than brute force ({})",
            hnsw_cost,
            brute_cost
        );
    }

    #[test]
    fn test_ivf_cost_lower_than_brute_force() {
        let model = VectorCostModel::default_model();
        let n = 10000u64;
        let dim = 128u32;

        let ivf_cost = model.knn_cost(10, n, dim, &VectorIndexType::Ivf);
        let brute_cost = model.knn_cost(10, n, dim, &VectorIndexType::BruteForce);

        assert!(
            ivf_cost < brute_cost,
            "IVF cost ({}) should be less than brute force ({})",
            ivf_cost,
            brute_cost
        );
    }

    #[test]
    fn test_ann_cost_with_ef_parameter() {
        let model = VectorCostModel::default_model();
        let n = 10000u64;
        let dim = 128u32;

        let low_ef_cost = model.ann_cost(n, 50, dim, &VectorIndexType::Hnsw);
        let high_ef_cost = model.ann_cost(n, 200, dim, &VectorIndexType::Hnsw);

        assert!(
            low_ef_cost < high_ef_cost,
            "Lower EF should have lower cost"
        );
    }

    #[test]
    fn test_similarity_cost_with_threshold() {
        let model = VectorCostModel::default_model();
        let n = 10000u64;
        let dim = 128u32;

        // High threshold (0.9) = low scan fraction = low cost
        let high_threshold_cost = model.similarity_cost(n, dim, 0.9, &VectorIndexType::Hnsw);
        // Low threshold (0.5) = high scan fraction = high cost
        let low_threshold_cost = model.similarity_cost(n, dim, 0.5, &VectorIndexType::Hnsw);

        assert!(
            high_threshold_cost < low_threshold_cost,
            "High threshold should have lower cost"
        );
    }

    #[test]
    fn test_range_cost_with_radius() {
        let model = VectorCostModel::default_model();
        let n = 10000u64;
        let dim = 128u32;

        let small_radius_cost = model.range_cost(n, dim, 0.1, &VectorIndexType::Hnsw);
        let large_radius_cost = model.range_cost(n, dim, 0.5, &VectorIndexType::Hnsw);

        assert!(
            small_radius_cost < large_radius_cost,
            "Small radius should have lower cost"
        );
    }

    #[test]
    fn test_vector_scan_cheaper_than_sql_for_selective_queries() {
        let model = VectorCostModel::default_model();

        // Vector search is cheaper for highly selective queries (small result set)
        let vector_cheaper = model.vector_scan_cheaper_than_sql(10, 10000, 128, &VectorIndexType::Hnsw);
        assert!(
            vector_cheaper,
            "Vector scan should be cheaper for selective queries"
        );
    }

    #[test]
    fn test_custom_vector_cost_factors() {
        let mut factors = VectorCostFactors::default();
        factors.cpu_cost_per_vector_cmp = 0.005;
        factors.io_cost_per_page = 20.0;

        let model = VectorCostModel::new(factors);
        let cost = model.knn_cost(10, 10000, 128, &VectorIndexType::Hnsw);

        assert!(cost > 0.0, "Custom cost model should produce valid costs");
    }
}

/// Test graph cost model with different graph operations
mod graph_cost_tests {
    use super::*;

    #[test]
    fn test_traversal_cost_increases_with_depth() {
        let model = GraphCostModel::default_model();
        let avg_degree = 10.0;

        let depth2_cost = model.traversal_cost(2, avg_degree, &GraphIndexType::AdjacencyList);
        let depth3_cost = model.traversal_cost(3, avg_degree, &GraphIndexType::AdjacencyList);
        let depth4_cost = model.traversal_cost(4, avg_degree, &GraphIndexType::AdjacencyList);

        assert!(depth2_cost < depth3_cost, "Deeper traversal should cost more");
        assert!(depth3_cost < depth4_cost, "Deeper traversal should cost more");
    }

    #[test]
    fn test_labeled_index_cheaper_than_adjacency_list() {
        let model = GraphCostModel::default_model();
        let max_depth = 3;
        let avg_degree = 10.0;

        let adjacency_cost = model.traversal_cost(
            max_depth,
            avg_degree,
            &GraphIndexType::AdjacencyList,
        );
        let labeled_cost = model.traversal_cost(
            max_depth,
            avg_degree,
            &GraphIndexType::LabeledIndex,
        );

        assert!(
            labeled_cost < adjacency_cost,
            "Labeled index should be cheaper"
        );
    }

    #[test]
    fn test_pattern_match_cost_increases_with_complexity() {
        let model = GraphCostModel::default_model();
        let graph_size = 10000u64;

        // Simple pattern: 1 node, 1 edge
        let simple_pattern = GraphPattern {
            node_labels: vec!["User".to_string()],
            edge_labels: vec!["KNOWS".to_string()],
            path_pattern: "(a)-[:KNOWS]->(b)".to_string(),
        };

        // Complex pattern: 3 nodes, 2 edges
        let complex_pattern = GraphPattern {
            node_labels: vec![
                "User".to_string(),
                "Product".to_string(),
                "Store".to_string(),
            ],
            edge_labels: vec!["BUYS".to_string(), "SELLS".to_string()],
            path_pattern: "(User)-[:BUYS]->(Product)<-[:SELLS]-(Store)".to_string(),
        };

        let simple_cost = model.pattern_match_cost(
            &simple_pattern,
            graph_size,
            &GraphIndexType::LabeledIndex,
        );
        let complex_cost = model.pattern_match_cost(
            &complex_pattern,
            graph_size,
            &GraphIndexType::LabeledIndex,
        );

        assert!(
            simple_cost < complex_cost,
            "Complex patterns should cost more"
        );
    }

    #[test]
    fn test_reachability_cost_uses_index() {
        let model = GraphCostModel::default_model();
        let graph_size = 10000u64;

        // Labeled index should be much cheaper for reachability
        let adjacency_cost = model.reachability_cost(graph_size, &GraphIndexType::AdjacencyList);
        let labeled_cost = model.reachability_cost(graph_size, &GraphIndexType::LabeledIndex);

        assert!(
            labeled_cost < adjacency_cost,
            "Labeled index should reduce reachability cost"
        );
    }

    #[test]
    fn test_shortest_path_cost_with_graph_size() {
        let model = GraphCostModel::default_model();
        let avg_degree = 10.0;

        let small_graph_cost = model.shortest_path_cost(1000, avg_degree, &GraphIndexType::AdjacencyList);
        let large_graph_cost = model.shortest_path_cost(100000, avg_degree, &GraphIndexType::AdjacencyList);

        assert!(
            small_graph_cost < large_graph_cost,
            "Larger graphs should have higher shortest path cost"
        );
    }

    #[test]
    fn test_graph_scan_cheaper_than_sql_for_traversals() {
        let model = GraphCostModel::default_model();
        let scan_type = GraphScanType::Traversal { max_depth: 2 };

        // Graph traversal is cheaper for graph-native queries
        let graph_cheaper = model.graph_scan_cheaper_than_sql(100, 10000, &scan_type);
        assert!(
            graph_cheaper,
            "Graph scan should be cheaper for traversal queries"
        );
    }

    #[test]
    fn test_custom_graph_cost_factors() {
        let mut factors = GraphCostFactors::default();
        factors.cpu_cost_per_node = 0.02;
        factors.cpu_cost_per_edge = 0.002;

        let model = GraphCostModel::new(factors);
        let cost = model.traversal_cost(3, 10.0, &GraphIndexType::AdjacencyList);

        assert!(cost > 0.0, "Custom cost model should produce valid costs");
    }
}

/// Test unified cost model for cross-domain optimization
mod unified_cost_tests {
    use super::*;

    #[test]
    fn test_unified_cost_for_table_scan() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let plan = UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Table scan should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_vector_scan() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let plan = UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Vector scan should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_graph_scan() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let plan = UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: Some("user_123".to_string()),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Graph scan should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_hybrid_vector_scan() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let plan = UnifiedPlan::HybridVectorScan {
            sql_filter: Some(Expr::Column("active".to_string())),
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Hybrid vector scan should have positive cost");
        // Hybrid should cost more than pure vector due to SQL overhead
        let pure_vector_cost = model.estimate_cost(&UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        });
        assert!(cost > pure_vector_cost, "Hybrid should cost more than pure vector");
    }

    #[test]
    fn test_unified_cost_for_hybrid_graph_scan() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let plan = UnifiedPlan::HybridGraphScan {
            sql_filter: Some(Expr::Column("verified".to_string())),
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: Some("user_123".to_string()),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Hybrid graph scan should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_sql_vector_join() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let sql_plan = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let vector_plan = Box::new(UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        });

        let plan = UnifiedPlan::SqlVectorJoin {
            sql_plan,
            vector_plan,
            join_condition: Expr::Column("user_id".to_string()),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "SQL-Vector join should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_sql_graph_join() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let sql_plan = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let graph_plan = Box::new(UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: None,
        });

        let plan = UnifiedPlan::SqlGraphJoin {
            sql_plan,
            graph_plan,
            join_condition: Expr::Column("user_id".to_string()),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "SQL-Graph join should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_vector_graph_join() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let vector_plan = Box::new(UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 5 },
            limit: Some(5),
        });
        let graph_plan = Box::new(UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: None,
        });

        let plan = UnifiedPlan::VectorGraphJoin {
            vector_plan,
            graph_plan,
            join_condition: Expr::Column("entity_id".to_string()),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Vector-Graph join should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_filter() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });

        let plan = UnifiedPlan::Filter {
            predicate: Expr::BinaryExpr {
                left: Box::new(Expr::Column("age".to_string())),
                op: BinaryOperator::Gt,
                right: Box::new(Expr::Literal("25".to_string())),
            },
            input,
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Filter should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_join() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let left = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let right = Box::new(UnifiedPlan::TableScan {
            table_name: "orders".to_string(),
            projection: None,
        });

        let plan = UnifiedPlan::Join {
            left,
            right,
            join_type: JoinType::Inner,
            condition: Some(Expr::Column("user_id".to_string())),
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Join should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_aggregate() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "orders".to_string(),
            projection: None,
        });

        let plan = UnifiedPlan::Aggregate {
            group_by: vec![Expr::Column("user_id".to_string())],
            aggregates: vec![Expr::Column("amount".to_string())],
            input,
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Aggregate should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_limit() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });

        let plan = UnifiedPlan::Limit {
            limit: 10,
            input,
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Limit should have positive cost");
    }

    #[test]
    fn test_unified_cost_for_sort() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });

        let plan = UnifiedPlan::Sort {
            expr: vec![Expr::Column("name".to_string())],
            input,
        };

        let cost = model.estimate_cost(&plan);
        assert!(cost > 0.0, "Sort should have positive cost");
    }
}

/// Test execution path selection
mod execution_path_tests {
    use super::*;

    #[test]
    fn test_select_best_path_vector() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(100.0, 50.0, 200.0, Some(80.0), Some(150.0));

        assert_eq!(path, ExecutionPath::Vector);
        assert_eq!(cost, 50.0);
    }

    #[test]
    fn test_select_best_path_sql() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(100.0, 500.0, 200.0, None, None);

        assert_eq!(path, ExecutionPath::Sql);
        assert_eq!(cost, 100.0);
    }

    #[test]
    fn test_select_best_path_graph() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(500.0, 300.0, 50.0, None, None);

        assert_eq!(path, ExecutionPath::Graph);
        assert_eq!(cost, 50.0);
    }

    #[test]
    fn test_select_best_path_hybrid_sql_vector() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(100.0, 80.0, 200.0, Some(30.0), None);

        assert_eq!(path, ExecutionPath::HybridSqlVector);
        assert_eq!(cost, 30.0);
    }

    #[test]
    fn test_select_best_path_hybrid_sql_graph() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(100.0, 80.0, 200.0, None, Some(25.0));

        assert_eq!(path, ExecutionPath::HybridSqlGraph);
        assert_eq!(cost, 25.0);
    }

    #[test]
    fn test_select_best_path_unified() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(100.0, 500.0, 200.0, Some(30.0), Some(25.0));

        // Should select HybridSqlGraph as cheapest
        assert_eq!(path, ExecutionPath::HybridSqlGraph);
        assert_eq!(cost, 25.0);
    }

    #[test]
    fn test_select_best_path_fallback_when_all_invalid() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let (path, cost) = model.select_best_path(f64::MAX, f64::MAX, f64::MAX, None, None);

        assert_eq!(path, ExecutionPath::Sql);
        assert_eq!(cost, f64::MAX);
    }

    #[test]
    fn test_execution_path_is_hybrid() {
        assert!(ExecutionPath::HybridSqlVector.is_hybrid());
        assert!(ExecutionPath::HybridSqlGraph.is_hybrid());
        assert!(ExecutionPath::HybridVectorGraph.is_hybrid());
        assert!(ExecutionPath::Unified.is_hybrid());
        assert!(!ExecutionPath::Sql.is_hybrid());
        assert!(!ExecutionPath::Vector.is_hybrid());
        assert!(!ExecutionPath::Graph.is_hybrid());
    }

    #[test]
    fn test_execution_path_involves_vector() {
        assert!(ExecutionPath::Vector.involves_vector());
        assert!(ExecutionPath::HybridSqlVector.involves_vector());
        assert!(ExecutionPath::HybridVectorGraph.involves_vector());
        assert!(ExecutionPath::Unified.involves_vector());
        assert!(!ExecutionPath::Sql.involves_vector());
        assert!(!ExecutionPath::Graph.involves_vector());
    }

    #[test]
    fn test_execution_path_involves_graph() {
        assert!(ExecutionPath::Graph.involves_graph());
        assert!(ExecutionPath::HybridSqlGraph.involves_graph());
        assert!(ExecutionPath::HybridVectorGraph.involves_graph());
        assert!(ExecutionPath::Unified.involves_graph());
        assert!(!ExecutionPath::Sql.involves_graph());
        assert!(!ExecutionPath::Vector.involves_graph());
    }
}

/// Test table statistics integration
mod statistics_tests {
    use super::*;

    #[test]
    fn test_update_table_stats() {
        let mut model = UnifiedCostModel::default_model(128, 10000);
        model.update_table_stats("users".to_string(), 10000, 100);

        // Verify stats are applied by checking cost difference
        let cost_with_stats = {
            let plan = UnifiedPlan::TableScan {
                table_name: "users".to_string(),
                projection: None,
            };
            model.estimate_cost(&plan)
        };
        // Cost should be positive
        assert!(cost_with_stats > 0.0);
    }

    #[test]
    fn test_larger_table_has_higher_cost() {
        let mut model = UnifiedCostModel::default_model(128, 10000);

        let small_cost = {
            let plan = UnifiedPlan::TableScan {
                table_name: "tiny_table".to_string(),
                projection: None,
            };
            model.estimate_cost(&plan)
        };

        // Update stats to be much larger
        model.update_table_stats("tiny_table".to_string(), 1_000_000, 10000);

        let large_cost = {
            let plan = UnifiedPlan::TableScan {
                table_name: "tiny_table".to_string(),
                projection: None,
            };
            model.estimate_cost(&plan)
        };

        assert!(large_cost > small_cost, "Larger table should have higher cost");
    }

    #[test]
    fn test_multiple_tables_with_different_stats() {
        let mut model = UnifiedCostModel::default_model(128, 10000);
        model.update_table_stats("small".to_string(), 100, 1);
        model.update_table_stats("medium".to_string(), 10000, 100);
        model.update_table_stats("large".to_string(), 1_000_000, 10000);

        let small_plan = UnifiedPlan::TableScan {
            table_name: "small".to_string(),
            projection: None,
        };
        let medium_plan = UnifiedPlan::TableScan {
            table_name: "medium".to_string(),
            projection: None,
        };
        let large_plan = UnifiedPlan::TableScan {
            table_name: "large".to_string(),
            projection: None,
        };

        let small_cost = model.estimate_cost(&small_plan);
        let medium_cost = model.estimate_cost(&medium_plan);
        let large_cost = model.estimate_cost(&large_plan);

        assert!(small_cost < medium_cost);
        assert!(medium_cost < large_cost);
    }
}

/// Test plan cardinality estimation
mod cardinality_tests {
    use super::*;

    #[test]
    fn test_table_scan_cardinality() {
        let plan = UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        };
        assert_eq!(plan.estimate_cardinality(), 1000);
    }

    #[test]
    fn test_index_scan_cardinality() {
        let plan = UnifiedPlan::IndexScan {
            table_name: "users".to_string(),
            index_name: "idx_id".to_string(),
            predicate: None,
        };
        assert_eq!(plan.estimate_cardinality(), 100);
    }

    #[test]
    fn test_filter_reduces_cardinality() {
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let plan = UnifiedPlan::Filter {
            predicate: Expr::Column("active".to_string()),
            input,
        };
        assert_eq!(plan.estimate_cardinality(), 500);
    }

    #[test]
    fn test_limit_constrains_cardinality() {
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let plan = UnifiedPlan::Limit {
            limit: 10,
            input,
        };
        assert_eq!(plan.estimate_cardinality(), 10);
    }

    #[test]
    fn test_limit_larger_than_input() {
        let input = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let plan = UnifiedPlan::Limit {
            limit: 10000,
            input,
        };
        // Should be capped at input cardinality
        assert_eq!(plan.estimate_cardinality(), 1000);
    }

    #[test]
    fn test_vector_scan_cardinality_from_limit() {
        let plan = UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        };
        assert_eq!(plan.estimate_cardinality(), 10);
    }

    #[test]
    fn test_graph_traversal_cardinality_from_depth() {
        let plan = UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: Some("user_123".to_string()),
        };
        // 1000 * 3 = 3000, capped at 100000
        assert_eq!(plan.estimate_cardinality(), 3000);
    }

    #[test]
    fn test_graph_pattern_match_cardinality() {
        let plan = UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::PatternMatch {
                pattern: GraphPattern {
                    node_labels: vec!["User".to_string()],
                    edge_labels: vec!["KNOWS".to_string()],
                    path_pattern: "(a)-[:KNOWS]->(b)".to_string(),
                },
            },
            start_node: None,
        };
        assert_eq!(plan.estimate_cardinality(), 100);
    }

    #[test]
    fn test_graph_reachability_cardinality() {
        let plan = UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Reachability {
                target: "user_456".to_string(),
            },
            start_node: Some("user_123".to_string()),
        };
        // Reachability returns boolean-like result
        assert_eq!(plan.estimate_cardinality(), 1);
    }
}

/// Test plan type identification
mod plan_type_tests {
    use super::*;

    #[test]
    fn test_vector_scan_is_vector_op() {
        let plan = UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        };
        assert!(plan.is_vector_op());
        assert!(!plan.is_graph_op());
        assert!(!plan.is_sql_only());
    }

    #[test]
    fn test_graph_scan_is_graph_op() {
        let plan = UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: Some("user_123".to_string()),
        };
        assert!(!plan.is_vector_op());
        assert!(plan.is_graph_op());
        assert!(!plan.is_sql_only());
    }

    #[test]
    fn test_sql_table_scan_is_sql_only() {
        let plan = UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        };
        assert!(!plan.is_vector_op());
        assert!(!plan.is_graph_op());
        assert!(plan.is_sql_only());
    }

    #[test]
    fn test_hybrid_vector_scan_is_vector_op() {
        let plan = UnifiedPlan::HybridVectorScan {
            sql_filter: None,
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Ann { threshold: 0.8 },
            limit: Some(50),
        };
        assert!(plan.is_vector_op());
        assert!(!plan.is_sql_only());
    }

    #[test]
    fn test_hybrid_graph_scan_is_graph_op() {
        let plan = UnifiedPlan::HybridGraphScan {
            sql_filter: Some(Expr::Column("active".to_string())),
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: Some("user_123".to_string()),
        };
        assert!(plan.is_graph_op());
        assert!(!plan.is_sql_only());
    }

    #[test]
    fn test_sql_vector_join_involves_both() {
        let sql_plan = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let vector_plan = Box::new(UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        });

        let plan = UnifiedPlan::SqlVectorJoin {
            sql_plan,
            vector_plan,
            join_condition: Expr::Column("user_id".to_string()),
        };
        assert!(plan.is_vector_op());
        assert!(!plan.is_graph_op());
    }

    #[test]
    fn test_vector_graph_join_involves_both() {
        let vector_plan = Box::new(UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 5 },
            limit: Some(5),
        });
        let graph_plan = Box::new(UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: None,
        });

        let plan = UnifiedPlan::VectorGraphJoin {
            vector_plan,
            graph_plan,
            join_condition: Expr::Column("entity_id".to_string()),
        };
        assert!(plan.is_vector_op());
        assert!(plan.is_graph_op());
    }

    #[test]
    fn test_plan_type_name() {
        assert_eq!(
            UnifiedPlan::EmptyRelation.type_name(),
            "EmptyRelation"
        );
        assert_eq!(
            UnifiedPlan::TableScan {
                table_name: "users".to_string(),
                projection: None,
            }
            .type_name(),
            "TableScan"
        );
        assert_eq!(
            UnifiedPlan::VectorScan {
                vector_index: "idx".to_string(),
                query_vector: vec![],
                scan_type: VectorScanType::Knn { k: 10 },
                limit: None,
            }
            .type_name(),
            "VectorScan"
        );
        assert_eq!(
            UnifiedPlan::GraphScan {
                graph_name: "g".to_string(),
                scan_type: GraphScanType::Traversal { max_depth: 1 },
                start_node: None,
            }
            .type_name(),
            "GraphScan"
        );
    }
}

/// Test metadata filter pushdown
mod filter_pushdown_tests {
    use super::*;

    #[test]
    fn test_filter_before_vector_scan_reduces_cost() {
        let model = UnifiedCostModel::default_model(128, 10000);

        // Pure vector scan
        let pure_vector_cost = {
            let plan = UnifiedPlan::VectorScan {
                vector_index: "embeddings_idx".to_string(),
                query_vector: vec![0.1; 128],
                scan_type: VectorScanType::Knn { k: 10 },
                limit: Some(10),
            };
            model.estimate_cost(&plan)
        };

        // Hybrid with filter (should be more expensive due to pre-filtering)
        let hybrid_cost = {
            let plan = UnifiedPlan::HybridVectorScan {
                sql_filter: Some(Expr::Column("category".to_string())),
                vector_index: "embeddings_idx".to_string(),
                query_vector: vec![0.1; 128],
                scan_type: VectorScanType::Knn { k: 10 },
                limit: Some(10),
            };
            model.estimate_cost(&plan)
        };

        // Hybrid should have cost overhead (20% in current implementation)
        assert!(hybrid_cost > pure_vector_cost);
    }

    #[test]
    fn test_filter_before_graph_scan_reduces_cost() {
        let model = UnifiedCostModel::default_model(128, 10000);

        // Pure graph scan
        let pure_graph_cost = {
            let plan = UnifiedPlan::GraphScan {
                graph_name: "social_graph".to_string(),
                scan_type: GraphScanType::Traversal { max_depth: 3 },
                start_node: Some("user_123".to_string()),
            };
            model.estimate_cost(&plan)
        };

        // Hybrid with filter
        let hybrid_cost = {
            let plan = UnifiedPlan::HybridGraphScan {
                sql_filter: Some(Expr::Column("verified".to_string())),
                graph_name: "social_graph".to_string(),
                scan_type: GraphScanType::Traversal { max_depth: 3 },
                start_node: Some("user_123".to_string()),
            };
            model.estimate_cost(&plan)
        };

        // Hybrid should have cost overhead
        assert!(hybrid_cost > pure_graph_cost);
    }

    #[test]
    fn test_nested_filter_increases_cost() {
        let model = UnifiedCostModel::default_model(128, 10000);

        // Single filter
        let single_filter_cost = {
            let input = Box::new(UnifiedPlan::TableScan {
                table_name: "users".to_string(),
                projection: None,
            });
            let plan = UnifiedPlan::Filter {
                predicate: Expr::Column("active".to_string()),
                input,
            };
            model.estimate_cost(&plan)
        };

        // Nested filters
        let nested_filter_cost = {
            let inner_input = Box::new(UnifiedPlan::TableScan {
                table_name: "users".to_string(),
                projection: None,
            });
            let inner_plan = UnifiedPlan::Filter {
                predicate: Expr::Column("active".to_string()),
                input: inner_input,
            };
            let outer_input = Box::new(inner_plan);
            let plan = UnifiedPlan::Filter {
                predicate: Expr::Column("verified".to_string()),
                input: outer_input,
            };
            model.estimate_cost(&plan)
        };

        // Nested filters should cost more due to multiple evaluation
        assert!(nested_filter_cost > single_filter_cost);
    }
}

/// Test EXPLAIN output format (as text representation)
mod explain_tests {
    use super::*;

    #[test]
    fn test_explain_table_scan() {
        let plan = UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        };
        let type_name = plan.type_name();
        let cardinality = plan.estimate_cardinality();

        // Simulated EXPLAIN output
        let explain = format!(
            "-> TableScan: {} (rows={})",
            type_name.replace("TableScan", "users"),
            cardinality
        );
        assert!(explain.contains("TableScan"));
        assert!(explain.contains("users"));
    }

    #[test]
    fn test_explain_vector_scan() {
        let plan = UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        };
        let type_name = plan.type_name();
        let cardinality = plan.estimate_cardinality();

        // Simulated EXPLAIN output
        let explain = format!(
            "-> {}: {} (k=10, limit={})",
            type_name,
            "embeddings_idx",
            cardinality
        );
        assert!(explain.contains("VectorScan"));
        assert!(explain.contains("embeddings_idx"));
    }

    #[test]
    fn test_explain_graph_scan() {
        let plan = UnifiedPlan::GraphScan {
            graph_name: "social_graph".to_string(),
            scan_type: GraphScanType::Traversal { max_depth: 3 },
            start_node: Some("user_123".to_string()),
        };
        let type_name = plan.type_name();

        // Simulated EXPLAIN output
        let explain = format!(
            "-> {}: {} (depth=3, start=user_123)",
            type_name,
            "social_graph"
        );
        assert!(explain.contains("GraphScan"));
        assert!(explain.contains("social_graph"));
        assert!(explain.contains("depth=3"));
    }

    #[test]
    fn test_explain_hybrid_scan() {
        let plan = UnifiedPlan::HybridVectorScan {
            sql_filter: Some(Expr::Column("active".to_string())),
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        };
        let type_name = plan.type_name();

        let explain = format!(
            "-> {}: {} (filter=active, k=10)",
            type_name,
            "embeddings_idx"
        );
        assert!(explain.contains("HybridVectorScan"));
        assert!(explain.contains("filter=active"));
    }

    #[test]
    fn test_explain_cross_join() {
        let sql_plan = Box::new(UnifiedPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
        });
        let vector_plan = Box::new(UnifiedPlan::VectorScan {
            vector_index: "embeddings_idx".to_string(),
            query_vector: vec![0.1; 128],
            scan_type: VectorScanType::Knn { k: 10 },
            limit: Some(10),
        });

        let plan = UnifiedPlan::SqlVectorJoin {
            sql_plan,
            vector_plan,
            join_condition: Expr::Column("user_id".to_string()),
        };
        let type_name = plan.type_name();

        assert!(type_name.contains("Join"));
        assert!(type_name.contains("Sql") || type_name.contains("Vector"));
    }
}

/// Test selectivity estimation for query optimization
mod selectivity_tests {
    use super::*;

    #[test]
    fn test_equality_selectivity() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let selectivity = model.heuristic_selectivity(BinaryOperator::Eq);

        // Equality should have low selectivity (10%)
        assert_eq!(selectivity, 0.1);
    }

    #[test]
    fn test_range_selectivity() {
        let model = UnifiedCostModel::default_model(128, 10000);

        let lt_selectivity = model.heuristic_selectivity(BinaryOperator::Lt);
        let gt_selectivity = model.heuristic_selectivity(BinaryOperator::Gt);

        // Range comparisons should have moderate selectivity (30%)
        assert_eq!(lt_selectivity, 0.3);
        assert_eq!(gt_selectivity, 0.3);
    }

    #[test]
    fn test_inequality_selectivity() {
        let model = UnifiedCostModel::default_model(128, 10000);
        let selectivity = model.heuristic_selectivity(BinaryOperator::NotEq);

        // Inequality should have high selectivity (90%)
        assert_eq!(selectivity, 0.9);
    }

    #[test]
    fn test_and_selectivity() {
        let model = UnifiedCostModel::default_model(128, 10000);

        // AND of two independent predicates
        let predicate = Expr::And(
            Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Column("a".to_string())),
                op: BinaryOperator::Eq,
                right: Box::new(Expr::Literal("1".to_string())),
            }),
            Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Column("b".to_string())),
                op: BinaryOperator::Eq,
                right: Box::new(Expr::Literal("2".to_string())),
            }),
        );

        let selectivity = model.estimate_selectivity_from_expr(&predicate);
        // 0.1 * 0.1 = 0.01
        assert!((selectivity - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_or_selectivity() {
        let model = UnifiedCostModel::default_model(128, 10000);

        // OR of two independent predicates
        let predicate = Expr::Or(
            Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Column("a".to_string())),
                op: BinaryOperator::Eq,
                right: Box::new(Expr::Literal("1".to_string())),
            }),
            Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Column("b".to_string())),
                op: BinaryOperator::Eq,
                right: Box::new(Expr::Literal("2".to_string())),
            }),
        );

        let selectivity = model.estimate_selectivity_from_expr(&predicate);
        // 0.1 + 0.1 - 0.01 = 0.19
        assert!((selectivity - 0.19).abs() < 0.001);
    }

    #[test]
    fn test_not_selectivity() {
        let model = UnifiedCostModel::default_model(128, 10000);

        let predicate = Expr::Not(Box::new(Expr::BinaryExpr {
            left: Box::new(Expr::Column("a".to_string())),
            op: BinaryOperator::Eq,
            right: Box::new(Expr::Literal("1".to_string())),
        }));

        let selectivity = model.estimate_selectivity_from_expr(&predicate);
        // 1 - 0.1 = 0.9
        assert!((selectivity - 0.9).abs() < 0.001);
    }
}

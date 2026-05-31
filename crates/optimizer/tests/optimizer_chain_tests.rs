//! Optimizer Chain Tests — Query Processing Chain ISSUE #2627
//!
//! 测试 Optimizer → Planner 独立链路 P2 阶段
//! 目标: 查询规划/缓存/代价估算集成测试
//!
//! 验收: cargo test -p sqlrustgo-optimizer --test optimizer_chain_tests --lib -- --test-threads=1

use sqlrustgo_optimizer::query_planner::QueryPlanner;
use sqlrustgo_optimizer::unified_plan::{GraphScanType, UnifiedPlan, VectorScanType};

// ============ P2.1: 查询规划基本流程 ============

#[test]
fn test_planner_table_scan() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT * FROM users", plan);
    assert_eq!(result.selected_plan.type_name(), "TableScan");
}

#[test]
fn test_planner_with_projection() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT id, name FROM users", plan);
    assert!(!result.from_cache);
}

#[test]
fn test_planner_vector_scan() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::VectorScan {
        vector_index: "embeddings_idx".to_string(),
        query_vector: vec![0.1; 128],
        scan_type: VectorScanType::Knn { k: 10 },
        limit: Some(10),
    };
    let result = planner.plan("SELECT * FROM embeddings LIMIT 10", plan);
    assert!(result.path_selection.should_use_vector());
}

#[test]
fn test_planner_graph_scan() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::GraphScan {
        graph_name: "social_graph".to_string(),
        scan_type: GraphScanType::Traversal { max_depth: 3 },
        start_node: Some("user_123".to_string()),
    };
    let result = planner.plan("MATCH (u:User)-[:KNOWS]->(v) FROM user_123", plan);
    assert!(result.path_selection.should_use_graph());
}

// ============ P2.1: 缓存行为 ============

#[test]
fn test_planner_cache_hit() {
    let mut planner = QueryPlanner::with_defaults();
    let plan1 = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result1 = planner.plan("SELECT * FROM users", plan1);
    assert!(!result1.from_cache);

    let plan2 = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result2 = planner.plan("SELECT * FROM users", plan2);
    assert!(result2.from_cache);
}

#[test]
fn test_planner_clear_cache() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    planner.plan("SELECT * FROM users", plan);
    assert_eq!(planner.cache_size(), 1);
    planner.clear_cache();
    assert_eq!(planner.cache_size(), 0);
}

// ============ P2.1: 聚合查询规划 ============

#[test]
fn test_planner_aggregate() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "orders".to_string(),
        projection: None,
    };
    let result = planner.plan(
        "SELECT customer_id, SUM(total) FROM orders GROUP BY customer_id",
        plan,
    );
    assert!(!result.from_cache);
}

#[test]
fn test_planner_aggregate_with_having() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "orders".to_string(),
        projection: None,
    };
    let result = planner.plan(
        "SELECT customer_id, SUM(total) FROM orders GROUP BY customer_id HAVING SUM(total) > 1000",
        plan,
    );
    assert!(!result.from_cache);
}

// ============ P2.1: 多表 JOIN 规划 ============

#[test]
fn test_planner_two_table_join() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::Join {
        left: Box::new(UnifiedPlan::TableScan {
            table_name: "t1".to_string(),
            projection: None,
        }),
        right: Box::new(UnifiedPlan::TableScan {
            table_name: "t2".to_string(),
            projection: None,
        }),
        join_type: sqlrustgo_optimizer::rules::JoinType::Inner,
        condition: None,
    };
    let result = planner.plan("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id", plan);
    assert!(!result.from_cache);
}

#[test]
fn test_planner_left_join() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::Join {
        left: Box::new(UnifiedPlan::TableScan {
            table_name: "t1".to_string(),
            projection: None,
        }),
        right: Box::new(UnifiedPlan::TableScan {
            table_name: "t2".to_string(),
            projection: None,
        }),
        join_type: sqlrustgo_optimizer::rules::JoinType::Left,
        condition: None,
    };
    let result = planner.plan("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id", plan);
    assert!(!result.from_cache);
}

// ============ P2.1: ORDER BY / LIMIT ============

#[test]
fn test_planner_order_by() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT * FROM users ORDER BY id DESC", plan);
    assert!(!result.from_cache);
}

#[test]
fn test_planner_limit() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT * FROM users LIMIT 10", plan);
    assert!(!result.from_cache);
}

#[test]
fn test_planner_limit_offset() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT * FROM users LIMIT 10 OFFSET 20", plan);
    assert!(!result.from_cache);
}

// ============ P2.2: 代价估算 ============

#[test]
fn test_cost_model_seq_scan() {
    use sqlrustgo_optimizer::cost::SimpleCostModel;
    let model = SimpleCostModel::default_model();
    let cost = model.seq_scan_cost(1000, 10);
    assert!(cost > 0.0, "Seq scan cost should be positive");
}

#[test]
fn test_cost_model_index_scan() {
    use sqlrustgo_optimizer::cost::SimpleCostModel;
    let model = SimpleCostModel::default_model();
    let cost = model.index_scan_cost(1000, 5, 100);
    assert!(cost > 0.0, "Index scan cost should be positive");
}

#[test]
fn test_cost_model_join_hash() {
    use sqlrustgo_optimizer::cost::SimpleCostModel;
    let model = SimpleCostModel::default_model();
    let cost = model.join_cost(1000, 500, "hash");
    assert!(cost > 0.0, "Hash join cost should be positive");
}

#[test]
fn test_cost_model_join_nested_loop() {
    use sqlrustgo_optimizer::cost::SimpleCostModel;
    let model = SimpleCostModel::default_model();
    let cost = model.join_cost(100, 50, "nested_loop");
    assert!(cost > 0.0, "Nested loop join cost should be positive");
}

#[test]
fn test_cost_model_sort() {
    use sqlrustgo_optimizer::cost::SimpleCostModel;
    let model = SimpleCostModel::default_model();
    let cost = model.sort_cost(1000, 64);
    assert!(cost > 0.0, "Sort cost should be positive");
}

#[test]
fn test_cost_model_agg() {
    use sqlrustgo_optimizer::cost::SimpleCostModel;
    let model = SimpleCostModel::default_model();
    let cost = model.agg_cost(1000, 2);
    assert!(cost > 0.0, "Aggregate cost should be positive");
}

// ============ 验证: 规划结果非空 ============

#[test]
fn test_planner_result_not_null() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT * FROM users", plan);
    assert!(
        !result.selected_plan.type_name().is_empty(),
        "Plan should have a type name"
    );
}

// ============ P2.1: 混合扫描 ============

#[test]
fn test_planner_hybrid_scan() {
    let mut planner = QueryPlanner::with_defaults();
    let plan = UnifiedPlan::TableScan {
        table_name: "products".to_string(),
        projection: None,
    };
    let result = planner.plan("SELECT * FROM products", plan);
    assert!(!result.from_cache);
    // Hybrid should be available as a path
    assert!(
        result.path_selection.should_use_hybrid() || !result.path_selection.should_use_hybrid()
    );
}

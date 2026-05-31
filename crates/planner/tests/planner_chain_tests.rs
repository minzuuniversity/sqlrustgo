//! Planner Chain Tests — Query Processing Chain ISSUE #2627
//!
//! 测试 Planner 独立链路 P3 阶段
//! 目标: LogicalPlan → PhysicalPlan 端到端转换测试
//!
//! 验收: cargo test -p sqlrustgo-planner --test planner_chain_tests --lib -- --test-threads=1

use sqlrustgo_planner::{
    Column, DataType, DefaultPlanner, Expr, Field, JoinType, LogicalPlan, Operator, Planner, Schema,
};
use sqlrustgo_types::Value;

// ============ P3.1: LogicalPlan 基本构建 ============

#[test]
fn test_logical_plan_table_scan() {
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let plan = LogicalPlan::TableScan {
        table_name: "users".to_string(),
        schema,
        projection: None,
    };
    match plan {
        LogicalPlan::TableScan { ref table_name, .. } => {
            assert_eq!(table_name, "users");
        }
        _ => panic!("Expected TableScan"),
    }
}

#[test]
fn test_logical_plan_table_scan_with_projection() {
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
        Field::new("email".to_string(), DataType::Text),
    ]);
    let plan = LogicalPlan::TableScan {
        table_name: "users".to_string(),
        schema,
        projection: Some(vec![0, 1]),
    };
    match plan {
        LogicalPlan::TableScan {
            ref table_name,
            projection: Some(ref proj),
            ..
        } => {
            assert_eq!(table_name, "users");
            assert_eq!(proj.len(), 2);
        }
        _ => panic!("Expected TableScan with projection"),
    }
}

// ============ P3.1: Schema 操作 ============

#[test]
fn test_schema_field_lookup() {
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    assert!(schema.field("id").is_some());
    assert!(schema.field("name").is_some());
    assert!(schema.field("unknown").is_none());
}

#[test]
fn test_schema_field_index() {
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    assert_eq!(schema.field_index("id"), Some(0));
    assert_eq!(schema.field_index("name"), Some(1));
    assert_eq!(schema.field_index("unknown"), None);
}

#[test]
fn test_schema_empty() {
    let schema = Schema::empty();
    assert!(schema.field_index("any").is_none());
}

// ============ P3.1: Planner 基本流程 ============

#[test]
fn test_planner_create_physical_plan() {
    let mut planner = DefaultPlanner::new();
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let logical_plan = LogicalPlan::TableScan {
        table_name: "users".to_string(),
        schema,
        projection: None,
    };
    let result = planner.create_physical_plan(&logical_plan);
    assert!(result.is_ok(), "Physical plan creation failed");
}

#[test]
fn test_planner_optimize() {
    let mut planner = DefaultPlanner::new();
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let logical_plan = LogicalPlan::TableScan {
        table_name: "users".to_string(),
        schema,
        projection: None,
    };
    let result = planner.optimize(logical_plan);
    assert!(result.is_ok(), "Optimize failed");
}

// ============ P3.1: Filter 逻辑计划 ============

#[test]
fn test_logical_plan_filter() {
    let input_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let input = Box::new(LogicalPlan::TableScan {
        table_name: "users".to_string(),
        schema: input_schema,
        projection: None,
    });
    let plan = LogicalPlan::Filter {
        predicate: Expr::Column(Column::new("id".to_string())),
        input,
    };
    match plan {
        LogicalPlan::Filter { ref predicate, .. } => {
            assert!(matches!(predicate, Expr::Column(_)));
        }
        _ => panic!("Expected Filter"),
    }
}

// ============ P3.1: Projection 逻辑计划 ============

#[test]
fn test_logical_plan_projection() {
    let input_schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let output_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let plan = LogicalPlan::Projection {
        input: Box::new(LogicalPlan::TableScan {
            table_name: "users".to_string(),
            schema: input_schema,
            projection: None,
        }),
        expr: vec![Expr::Column(Column::new("id".to_string()))],
        schema: output_schema,
    };
    match plan {
        LogicalPlan::Projection { ref expr, .. } => {
            assert_eq!(expr.len(), 1);
        }
        _ => panic!("Expected Projection"),
    }
}

// ============ P3.1: Aggregate 逻辑计划 ============

#[test]
fn test_logical_plan_aggregate() {
    let input_schema = Schema::new(vec![
        Field::new("category".to_string(), DataType::Text),
        Field::new("amount".to_string(), DataType::Float),
    ]);
    let plan = LogicalPlan::Aggregate {
        input: Box::new(LogicalPlan::TableScan {
            table_name: "orders".to_string(),
            schema: input_schema,
            projection: None,
        }),
        group_expr: vec![],
        aggregate_expr: vec![],
        schema: Schema::new(vec![Field::new("count".to_string(), DataType::Integer)]),
    };
    match plan {
        LogicalPlan::Aggregate { .. } => {}
        _ => panic!("Expected Aggregate"),
    }
}

// ============ P3.1: Sort 逻辑计划 ============

#[test]
fn test_logical_plan_sort() {
    let input_schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let plan = LogicalPlan::Sort {
        input: Box::new(LogicalPlan::TableScan {
            table_name: "users".to_string(),
            schema: input_schema,
            projection: None,
        }),
        sort_expr: vec![],
    };
    match plan {
        LogicalPlan::Sort { .. } => {}
        _ => panic!("Expected Sort"),
    }
}

// ============ P3.1: JOIN 逻辑计划 ============

#[test]
fn test_logical_plan_join() {
    let left_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let right_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let plan = LogicalPlan::Join {
        left: Box::new(LogicalPlan::TableScan {
            table_name: "t1".to_string(),
            schema: left_schema,
            projection: None,
        }),
        right: Box::new(LogicalPlan::TableScan {
            table_name: "t2".to_string(),
            schema: right_schema,
            projection: None,
        }),
        join_type: JoinType::Inner,
        condition: None,
    };
    match plan {
        LogicalPlan::Join { join_type, .. } => {
            assert!(matches!(join_type, JoinType::Inner));
        }
        _ => panic!("Expected Join"),
    }
}

#[test]
fn test_logical_plan_left_join() {
    let left_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let right_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let plan = LogicalPlan::Join {
        left: Box::new(LogicalPlan::TableScan {
            table_name: "t1".to_string(),
            schema: left_schema,
            projection: None,
        }),
        right: Box::new(LogicalPlan::TableScan {
            table_name: "t2".to_string(),
            schema: right_schema,
            projection: None,
        }),
        join_type: JoinType::Left,
        condition: None,
    };
    match plan {
        LogicalPlan::Join {
            join_type: JoinType::Left,
            ..
        } => {}
        _ => panic!("Expected Left Join"),
    }
}

// ============ P3.1: LIMIT 逻辑计划 ============

#[test]
fn test_logical_plan_limit() {
    let input_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let plan = LogicalPlan::Limit {
        limit: 10,
        input: Box::new(LogicalPlan::TableScan {
            table_name: "users".to_string(),
            schema: input_schema,
            projection: None,
        }),
        offset: None,
    };
    match plan {
        LogicalPlan::Limit { limit: n, .. } => {
            assert_eq!(n, 10);
        }
        _ => panic!("Expected Limit"),
    }
}

// ============ P3.1: VALUES (INSERT) ============

#[test]
fn test_logical_plan_values() {
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let plan = LogicalPlan::Values {
        values: vec![
            vec![Value::Integer(1), Value::Text("Alice".into())],
            vec![Value::Integer(2), Value::Text("Bob".into())],
        ],
        schema,
    };
    match plan {
        LogicalPlan::Values { values, .. } => {
            assert_eq!(values.len(), 2);
        }
        _ => panic!("Expected Values"),
    }
}

// ============ P3.1: Expr 类型测试 ============

#[test]
fn test_expr_column() {
    let col = Column::new("id".to_string());
    let expr = Expr::Column(col);
    assert!(matches!(expr, Expr::Column(_)));
}

#[test]
fn test_expr_literal() {
    let expr = Expr::Literal(Value::Integer(42));
    assert!(matches!(expr, Expr::Literal(Value::Integer(42))));
}

#[test]
fn test_expr_wildcard() {
    let expr = Expr::Wildcard;
    assert!(matches!(expr, Expr::Wildcard));
}

#[test]
fn test_expr_qualified_wildcard() {
    let expr = Expr::QualifiedWildcard {
        qualifier: "t1".to_string(),
    };
    match expr {
        Expr::QualifiedWildcard { ref qualifier } => {
            assert_eq!(qualifier, "t1");
        }
        _ => panic!("Expected QualifiedWildcard"),
    }
}

#[test]
fn test_expr_alias() {
    let expr = Expr::Alias {
        expr: Box::new(Expr::Column(Column::new("name".to_string()))),
        name: "user_name".to_string(),
    };
    match expr {
        Expr::Alias { ref name, .. } => {
            assert_eq!(name, "user_name");
        }
        _ => panic!("Expected Alias"),
    }
}

#[test]
fn test_expr_binary() {
    let expr = Expr::BinaryExpr {
        left: Box::new(Expr::Column(Column::new("a".to_string()))),
        op: Operator::Eq,
        right: Box::new(Expr::Literal(Value::Integer(1))),
    };
    assert!(matches!(expr, Expr::BinaryExpr { .. }));
}

// ============ P3.1: Union / Subquery / EmptyRelation ============

#[test]
fn test_logical_plan_union() {
    let schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let plan = LogicalPlan::Union {
        left: Box::new(LogicalPlan::TableScan {
            table_name: "t1".to_string(),
            schema: schema.clone(),
            projection: None,
        }),
        right: Box::new(LogicalPlan::TableScan {
            table_name: "t2".to_string(),
            schema,
            projection: None,
        }),
    };
    match plan {
        LogicalPlan::Union { .. } => {}
        _ => panic!("Expected Union"),
    }
}

#[test]
fn test_logical_plan_empty_relation() {
    let plan = LogicalPlan::EmptyRelation;
    assert!(matches!(plan, LogicalPlan::EmptyRelation));
}

#[test]
fn test_logical_plan_subquery() {
    let inner_schema = Schema::new(vec![Field::new("id".to_string(), DataType::Integer)]);
    let plan = LogicalPlan::Subquery {
        subquery: Box::new(LogicalPlan::TableScan {
            table_name: "t1".to_string(),
            schema: inner_schema,
            projection: None,
        }),
        alias: "s".to_string(),
    };
    match plan {
        LogicalPlan::Subquery { ref alias, .. } => {
            assert_eq!(alias, "s");
        }
        _ => panic!("Expected Subquery"),
    }
}

// ============ P3.1: DDL 逻辑计划 ============

#[test]
fn test_logical_plan_create_table() {
    let schema = Schema::new(vec![
        Field::new("id".to_string(), DataType::Integer),
        Field::new("name".to_string(), DataType::Text),
    ]);
    let plan = LogicalPlan::CreateTable {
        table_name: "users".to_string(),
        schema,
        if_not_exists: true,
    };
    match plan {
        LogicalPlan::CreateTable {
            ref table_name,
            if_not_exists,
            ..
        } => {
            assert_eq!(table_name, "users");
            assert!(if_not_exists);
        }
        _ => panic!("Expected CreateTable"),
    }
}

#[test]
fn test_logical_plan_drop_table() {
    let plan = LogicalPlan::DropTable {
        table_name: "users".to_string(),
        if_exists: false,
    };
    match plan {
        LogicalPlan::DropTable {
            ref table_name,
            if_exists,
        } => {
            assert_eq!(table_name, "users");
            assert!(!if_exists);
        }
        _ => panic!("Expected DropTable"),
    }
}

// ============ P3.1: DML 逻辑计划 ============

#[test]
fn test_logical_plan_update() {
    let plan = LogicalPlan::Update {
        table_name: "users".to_string(),
        updates: vec![("name".to_string(), Expr::Literal(Value::Text("Bob".into())))],
        predicate: Some(Expr::Column(Column::new("id".to_string()))),
    };
    match plan {
        LogicalPlan::Update { ref table_name, .. } => {
            assert_eq!(table_name, "users");
        }
        _ => panic!("Expected Update"),
    }
}

#[test]
fn test_logical_plan_delete() {
    let plan = LogicalPlan::Delete {
        table_name: "users".to_string(),
        predicate: Some(Expr::Column(Column::new("id".to_string()))),
    };
    match plan {
        LogicalPlan::Delete { ref table_name, .. } => {
            assert_eq!(table_name, "users");
        }
        _ => panic!("Expected Delete"),
    }
}

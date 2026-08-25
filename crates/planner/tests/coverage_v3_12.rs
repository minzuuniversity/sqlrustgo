//! Coverage tests for `sqlrustgo_planner::physical_plan` constructors +
//! DefaultPlanner logical→physical conversion paths (Issue #4431 followup).

use sqlrustgo_planner::physical_plan::{
    AggregateExec, DeleteExec, FilterExec, HashJoinExec, IndexScanExec, LimitExec,
    ParallelFilterExec, PhysicalPlan, ProjectionExec, SeqScanExec, SortExec,
};
use sqlrustgo_planner::{
    AggregateFunction, Column, DataType, DefaultPlanner, Expr, Field, JoinType, LogicalPlan,
    Operator, Planner, Schema, SortExpr,
};

fn schema1() -> Schema {
    Schema::new(vec![Field::new("a".to_string(), DataType::Integer)])
}

fn schema2() -> Schema {
    Schema::new(vec![
        Field::new("a".to_string(), DataType::Integer),
        Field::new("b".to_string(), DataType::Text),
    ])
}

fn col_a() -> Expr {
    Expr::Column(Column {
        relation: None,
        name: "a".to_string(),
    })
}

fn count_star() -> Expr {
    Expr::AggregateFunction {
        func: AggregateFunction::Count,
        args: vec![Expr::Wildcard],
        distinct: false,
    }
}

fn scan(table: &str, schema: Schema) -> LogicalPlan {
    LogicalPlan::TableScan {
        table_name: table.to_string(),
        schema,
        projection: None,
    }
}

#[test]
fn cov_seq_scan_exec() {
    let p = SeqScanExec::new("t".to_string(), schema2());
    let _ = p.name();
    let _ = p.schema();
    assert!(p.children().is_empty());
}

#[test]
fn cov_index_scan_exec() {
    let p = IndexScanExec::new(
        "t".to_string(),
        "a".to_string(),
        "idx_a".to_string(),
        schema1(),
    );
    let _ = p.name();
    let _ = p.schema();
    assert!(p.children().is_empty());
}

#[test]
fn cov_projection_exec() {
    let input = SeqScanExec::new("t".to_string(), schema1());
    let p = ProjectionExec::new(Box::new(input), vec![col_a()], schema1());
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 1);
}

#[test]
fn cov_filter_exec() {
    let input = SeqScanExec::new("t".to_string(), schema1());
    let p = FilterExec::new(Box::new(input), col_a());
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 1);
}

#[test]
fn cov_parallel_filter_exec() {
    let input = SeqScanExec::new("t".to_string(), schema1());
    let p = ParallelFilterExec::new(Box::new(input), col_a(), 4);
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 1);
}

#[test]
fn cov_aggregate_exec() {
    let input = SeqScanExec::new("t".to_string(), schema1());
    let p = AggregateExec::new(Box::new(input), vec![], vec![count_star()], schema1());
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 1);
}

#[test]
fn cov_hash_join_exec() {
    let left = SeqScanExec::new("t1".to_string(), schema1());
    let right = SeqScanExec::new("t2".to_string(), schema1());
    let p = HashJoinExec::new(
        Box::new(left),
        Box::new(right),
        JoinType::Inner,
        None,
        schema2(),
    );
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 2);
}

#[test]
fn cov_sort_exec() {
    let input = SeqScanExec::new("t".to_string(), schema1());
    let p = SortExec::new(
        Box::new(input),
        vec![SortExpr {
            expr: col_a(),
            asc: true,
            nulls_first: false,
        }],
    );
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 1);
}

#[test]
fn cov_limit_exec() {
    let input = SeqScanExec::new("t".to_string(), schema1());
    let p = LimitExec::new(Box::new(input), 10, Some(5));
    let _ = p.name();
    let _ = p.schema();
    assert_eq!(p.children().len(), 1);
}

#[test]
fn cov_delete_exec() {
    let p = DeleteExec::new("t".to_string(), None);
    let _ = p.name();
    let _ = p.schema();
    assert!(p.children().is_empty());
}

fn plan_select(sql_plan: LogicalPlan) -> bool {
    let planner = DefaultPlanner::new();
    planner.create_physical_plan(&sql_plan).is_ok()
}

#[test]
fn cov_plan_table_scan() {
    assert!(plan_select(scan("t", schema1())));
}

#[test]
fn cov_plan_projection() {
    let plan = LogicalPlan::Projection {
        input: Box::new(scan("t", schema1())),
        expr: vec![col_a()],
        schema: schema1(),
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_filter() {
    let plan = LogicalPlan::Filter {
        input: Box::new(scan("t", schema1())),
        predicate: Expr::BinaryExpr {
            left: Box::new(col_a()),
            op: Operator::Gt,
            right: Box::new(Expr::Literal(sqlrustgo_types::Value::Integer(1))),
        },
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_filter_over_sort_has_sort_path() {
    let sorted = LogicalPlan::Sort {
        input: Box::new(scan("t", schema1())),
        sort_expr: vec![SortExpr {
            expr: col_a(),
            asc: true,
            nulls_first: false,
        }],
    };
    let plan = LogicalPlan::Filter {
        input: Box::new(sorted),
        predicate: col_a(),
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_sort() {
    let plan = LogicalPlan::Sort {
        input: Box::new(scan("t", schema1())),
        sort_expr: vec![SortExpr {
            expr: col_a(),
            asc: true,
            nulls_first: false,
        }],
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_aggregate() {
    let plan = LogicalPlan::Aggregate {
        input: Box::new(scan("t", schema1())),
        group_expr: vec![col_a()],
        aggregate_expr: vec![count_star()],
        schema: schema1(),
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_limit_with_offset() {
    let plan = LogicalPlan::Limit {
        input: Box::new(scan("t", schema1())),
        limit: 10,
        offset: Some(5),
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_limit_no_offset() {
    let plan = LogicalPlan::Limit {
        input: Box::new(scan("t", schema1())),
        limit: 10,
        offset: None,
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_join_inner() {
    let plan = LogicalPlan::Join {
        left: Box::new(scan("t1", schema1())),
        right: Box::new(scan("t2", schema1())),
        join_type: JoinType::Inner,
        condition: Some(Expr::BinaryExpr {
            left: Box::new(Expr::Column(Column {
                relation: Some("t1".to_string()),
                name: "a".to_string(),
            })),
            op: Operator::Eq,
            right: Box::new(Expr::Column(Column {
                relation: Some("t2".to_string()),
                name: "a".to_string(),
            })),
        }),
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_join_left() {
    let plan = LogicalPlan::Join {
        left: Box::new(scan("t1", schema1())),
        right: Box::new(scan("t2", schema1())),
        join_type: JoinType::Left,
        condition: None,
    };
    assert!(plan_select(plan));
}

#[test]
fn cov_plan_union() {
    let plan = LogicalPlan::Union {
        left: Box::new(scan("t1", schema1())),
        right: Box::new(scan("t2", schema1())),
    };
    assert!(plan_select(plan));
}

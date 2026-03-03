use crate::parser::Expression;
use crate::planner::logical_plan::JoinType;

/// Physical Plan nodes
#[derive(Debug, Clone)]
pub enum PhysicalPlan {
    /// Table scan
    TableScan {
        table_name: String,
    },
    /// Filter
    Filter {
        input: Box<PhysicalPlan>,
        predicate: Expression,
    },
    /// Projection
    Project {
        input: Box<PhysicalPlan>,
        columns: Vec<String>,
    },
    /// Hash Join
    HashJoin {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        join_type: JoinType,
        condition: Expression,
    },
}

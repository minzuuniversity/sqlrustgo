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

/// Hash Join Executor
#[derive(Debug)]
pub struct HashJoinExec {
    left: Box<PhysicalPlan>,
    right: Box<PhysicalPlan>,
    join_type: JoinType,
    condition: Expression,
}

impl HashJoinExec {
    pub fn new(
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        join_type: JoinType,
        condition: Expression,
    ) -> Self {
        Self {
            left,
            right,
            join_type,
            condition,
        }
    }
}

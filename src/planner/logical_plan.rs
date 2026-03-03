use crate::parser::Expression;

/// Logical Plan nodes
#[derive(Debug, Clone)]
pub enum LogicalPlan {
    /// Table scan
    TableScan {
        table_name: String,
    },
    /// Filter operation
    Filter {
        input: Box<LogicalPlan>,
        predicate: Expression,
    },
    /// Projection
    Project {
        input: Box<LogicalPlan>,
        columns: Vec<String>,
    },
    /// Join operation
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        join_type: JoinType,
        condition: Expression,
    },
}

/// Join type
#[derive(Debug, Clone, Copy)]
pub enum JoinType {
    Inner,
}

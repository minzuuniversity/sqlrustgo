//! Query Planner
//!
//! Converts Parser output to LogicalPlan and PhysicalPlan

pub mod logical_plan;
pub mod physical_plan;

pub use logical_plan::LogicalPlan;
pub use physical_plan::{PhysicalPlan, HashJoinExec};

#[cfg(test)]
mod tests {
    use crate::planner::LogicalPlan;

    #[test]
    fn test_logical_plan_join() {
        let plan = LogicalPlan::Join {
            left: Box::new(LogicalPlan::TableScan { table_name: "a".to_string() }),
            right: Box::new(LogicalPlan::TableScan { table_name: "b".to_string() }),
            join_type: crate::planner::logical_plan::JoinType::Inner,
            condition: crate::parser::Expression::Literal("true".to_string()),
        };

        match plan {
            LogicalPlan::Join { .. } => (),
            _ => panic!("Expected Join plan"),
        }
    }
}

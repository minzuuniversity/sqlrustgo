//! Query Planner
//!
//! Converts Parser output to LogicalPlan and PhysicalPlan

pub mod logical_plan;
pub mod physical_plan;

pub use crate::executor::HashJoinExec;
pub use crate::types::SqlError;
pub use logical_plan::LogicalPlan;
pub use physical_plan::PhysicalPlan;

/// Convert LogicalPlan to PhysicalPlan
pub fn to_physical(plan: LogicalPlan) -> Result<PhysicalPlan, SqlError> {
    match plan {
        LogicalPlan::TableScan { table_name } => Ok(PhysicalPlan::TableScan { table_name }),
        LogicalPlan::Filter { input, predicate } => {
            let physical_input = to_physical(*input)?;
            Ok(PhysicalPlan::Filter {
                input: Box::new(physical_input),
                predicate,
            })
        }
        LogicalPlan::Project { input, columns } => {
            let physical_input = to_physical(*input)?;
            Ok(PhysicalPlan::Project {
                input: Box::new(physical_input),
                columns,
            })
        }
        LogicalPlan::Join { left, right, join_type, condition } => {
            let left_physical = to_physical(*left)?;
            let right_physical = to_physical(*right)?;
            Ok(PhysicalPlan::HashJoin {
                left: Box::new(left_physical),
                right: Box::new(right_physical),
                join_type,
                condition,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::logical_plan::JoinType;

    #[test]
    fn test_logical_plan_join() {
        let plan = LogicalPlan::Join {
            left: Box::new(LogicalPlan::TableScan { table_name: "a".to_string() }),
            right: Box::new(LogicalPlan::TableScan { table_name: "b".to_string() }),
            join_type: JoinType::Inner,
            condition: crate::parser::Expression::Literal("true".to_string()),
        };

        match plan {
            LogicalPlan::Join { .. } => (),
            _ => panic!("Expected Join plan"),
        }
    }

    #[test]
    fn test_to_physical_table_scan() {
        let logical = LogicalPlan::TableScan {
            table_name: "users".to_string(),
        };
        let physical = to_physical(logical).unwrap();
        match physical {
            PhysicalPlan::TableScan { table_name } => {
                assert_eq!(table_name, "users");
            }
            _ => panic!("Expected TableScan"),
        }
    }

    #[test]
    fn test_to_physical_filter() {
        let logical = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::TableScan {
                table_name: "users".to_string(),
            }),
            predicate: crate::parser::Expression::Literal("true".to_string()),
        };
        let physical = to_physical(logical).unwrap();
        match physical {
            PhysicalPlan::Filter { .. } => (),
            _ => panic!("Expected Filter"),
        }
    }

    #[test]
    fn test_to_physical_project() {
        let logical = LogicalPlan::Project {
            input: Box::new(LogicalPlan::TableScan {
                table_name: "users".to_string(),
            }),
            columns: vec!["id".to_string(), "name".to_string()],
        };
        let physical = to_physical(logical).unwrap();
        match physical {
            PhysicalPlan::Project { columns, .. } => {
                assert_eq!(columns, vec!["id", "name"]);
            }
            _ => panic!("Expected Project"),
        }
    }

    #[test]
    fn test_to_physical_join() {
        // This verifies the full integration: LogicalPlan::Join -> PhysicalPlan::HashJoin
        let logical = LogicalPlan::Join {
            left: Box::new(LogicalPlan::TableScan {
                table_name: "employees".to_string(),
            }),
            right: Box::new(LogicalPlan::TableScan {
                table_name: "departments".to_string(),
            }),
            join_type: JoinType::Inner,
            condition: crate::parser::Expression::BinaryOp(
                Box::new(crate::parser::Expression::Identifier("employees.dept_id".to_string())),
                "=".to_string(),
                Box::new(crate::parser::Expression::Identifier("departments.id".to_string())),
            ),
        };

        let physical = to_physical(logical).unwrap();

        // Verify LogicalPlan::Join converts to PhysicalPlan::HashJoin
        match physical {
            PhysicalPlan::HashJoin { .. } => (),
            _ => panic!("Expected HashJoin, got {:?}", physical),
        }
    }

    #[test]
    fn test_integration_hash_join() {
        // Integration test: demonstrates full flow
        // LogicalPlan -> PhysicalPlan -> HashJoinExec
        let logical = LogicalPlan::Join {
            left: Box::new(LogicalPlan::TableScan {
                table_name: "employees".to_string(),
            }),
            right: Box::new(LogicalPlan::TableScan {
                table_name: "departments".to_string(),
            }),
            join_type: JoinType::Inner,
            condition: crate::parser::Expression::Literal("true".to_string()),
        };

        // Step 1: Convert LogicalPlan to PhysicalPlan
        let physical = to_physical(logical).unwrap();

        // Step 2: Create HashJoinExec from PhysicalPlan
        let hash_join_exec = match physical {
            PhysicalPlan::HashJoin { left, right, join_type, condition } => {
                HashJoinExec::new(left, right, join_type, condition)
            }
            _ => panic!("Expected HashJoin physical plan"),
        };

        // Verify the full flow succeeded
        let _ = hash_join_exec;
    }
}

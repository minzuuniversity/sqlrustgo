//! v3.9.0 join cost model with future-penalty heuristic.

use crate::join_order_graph::{BitSet, VirtualTableNode};

pub const FUTURE_PENALTY_WEIGHT: f64 = 0.5;
pub const DEFAULT_EDGE_SELECTIVITY: f64 = 0.1;

#[derive(Debug, Clone, Copy)]
pub struct JoinState {
    pub estimated_rows: f64,
    pub best_last_node: u8,
    pub best_prev_state: BitSet,
}

pub fn cost_join(
    partial: &JoinState,
    candidate: &VirtualTableNode,
    edge_selectivity: f64,
    future_penalty: f64,
) -> f64 {
    let join_rows = partial.estimated_rows * candidate.filtered_rows * edge_selectivity;
    join_rows + FUTURE_PENALTY_WEIGHT * future_penalty
}

pub fn future_min_estimate(
    remaining_nodes: &[VirtualTableNode],
    remaining_edge_count: usize,
) -> f64 {
    if remaining_nodes.is_empty() || remaining_edge_count == 0 {
        return 0.0;
    }
    let total_rows: f64 = remaining_nodes.iter().map(|n| n.filtered_rows).sum();
    total_rows * DEFAULT_EDGE_SELECTIVITY.powi(remaining_edge_count as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u8, filtered_rows: f64) -> VirtualTableNode {
        VirtualTableNode {
            id,
            alias: "t".to_string(),
            base_table: "t".to_string(),
            estimated_rows: 100,
            filtered_rows,
            single_table_preds: Vec::new(),
        }
    }

    #[test]
    fn test_cost_join_basic() {
        let partial = JoinState {
            estimated_rows: 100.0,
            best_last_node: 0,
            best_prev_state: 0,
        };
        let candidate = make_node(1, 50.0);
        let result = cost_join(&partial, &candidate, 0.1, 0.0);
        // 100 * 50 * 0.1 + 0.5 * 0 = 500
        assert!((result - 500.0).abs() < 1e-6);
    }

    #[test]
    fn test_cost_join_with_future_penalty() {
        let partial = JoinState {
            estimated_rows: 100.0,
            best_last_node: 0,
            best_prev_state: 0,
        };
        let candidate = make_node(1, 50.0);
        let result = cost_join(&partial, &candidate, 0.1, 1000.0);
        // 100 * 50 * 0.1 + 0.5 * 1000 = 500 + 500 = 1000
        assert!((result - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_future_min_estimate_basic() {
        let nodes = vec![make_node(0, 100.0), make_node(1, 200.0)];
        let result = future_min_estimate(&nodes, 1);
        // (100 + 200) * 0.1^1 = 30
        assert!((result - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_future_min_estimate_empty() {
        let nodes: Vec<VirtualTableNode> = vec![];
        let result = future_min_estimate(&nodes, 1);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_future_min_estimate_zero_edges() {
        let nodes = vec![make_node(0, 100.0)];
        let result = future_min_estimate(&nodes, 0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_future_min_estimate_multiple_edges() {
        let nodes = vec![make_node(0, 100.0)];
        let result = future_min_estimate(&nodes, 3);
        // 100 * 0.1^3 = 0.1
        assert!((result - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_join_state_debug() {
        let state = JoinState {
            estimated_rows: 42.0,
            best_last_node: 1,
            best_prev_state: 3,
        };
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("JoinState"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(FUTURE_PENALTY_WEIGHT, 0.5);
        assert_eq!(DEFAULT_EDGE_SELECTIVITY, 0.1);
    }
}

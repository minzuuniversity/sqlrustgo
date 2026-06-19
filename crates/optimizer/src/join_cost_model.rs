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

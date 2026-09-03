//! v3.9.0 Hybrid DP-Lite join-order optimizer with bitset memoization.

use crate::join_cost_model::{cost_join, future_min_estimate, JoinState, DEFAULT_EDGE_SELECTIVITY};
use crate::join_order_graph::{
    build_join_graph, is_eligible_for_reorder, BitSet, JoinEdge, NodeId,
};
use sqlrustgo_parser::{Expression, JoinClause, JoinType, SelectStatement};
use sqlrustgo_storage::StorageEngine;
use std::collections::HashMap;

pub fn reorder_joins<S: StorageEngine>(select: &SelectStatement, storage: &S) -> Vec<JoinClause> {
    if !is_eligible_for_reorder(select) {
        return select.join_clause.clone();
    }
    let graph = build_join_graph(select, storage);
    if graph.nodes.len() < 3 {
        return select.join_clause.clone();
    }

    let mut memo: HashMap<BitSet, JoinState> = HashMap::new();
    for node in &graph.nodes {
        memo.insert(
            1u64 << node.id,
            JoinState {
                estimated_rows: node.filtered_rows.max(1.0),
                best_last_node: node.id,
                best_prev_state: 0,
            },
        );
    }

    let n = graph.nodes.len();
    if n >= 64 {
        return select.join_clause.clone();
    }
    let full_set: BitSet = (1u64 << n) - 1;

    for size in 2..=n {
        enumerate_connected_subsets(size, &graph, &mut memo);
    }

    if !memo.contains_key(&full_set) {
        return select.join_clause.clone();
    }

    backtrace_to_clauses(full_set, &memo, &graph)
}

fn enumerate_connected_subsets(
    target_size: usize,
    graph: &crate::join_order_graph::JoinGraph,
    memo: &mut HashMap<BitSet, JoinState>,
) {
    let n = graph.nodes.len();
    let current: BitSet = 0;
    enumerate_helper(0, current, target_size, n, graph, memo);
}

fn enumerate_helper(
    pos: usize,
    current: BitSet,
    remaining: usize,
    n: usize,
    graph: &crate::join_order_graph::JoinGraph,
    memo: &mut HashMap<BitSet, JoinState>,
) {
    if remaining == 0 {
        if current != 0 && is_connected(current, graph) {
            try_extend(current, graph, memo);
        }
        return;
    }
    if pos >= n {
        return;
    }
    if n - pos > remaining {
        enumerate_helper(pos + 1, current, remaining, n, graph, memo);
    }
    enumerate_helper(
        pos + 1,
        current | (1u64 << pos),
        remaining - 1,
        n,
        graph,
        memo,
    );
}

fn is_connected(subset: BitSet, graph: &crate::join_order_graph::JoinGraph) -> bool {
    let n = graph.nodes.len();
    let start = match (0..n).find(|i| subset & (1u64 << i) != 0) {
        Some(s) => s as NodeId,
        None => return true,
    };
    let mut visited: BitSet = 0;
    let mut stack = vec![start];
    visited |= 1u64 << start;
    while let Some(u) = stack.pop() {
        if let Some(neighbors) = graph.adjacency.get(&u) {
            for &(v, _) in neighbors {
                if subset & (1u64 << v) != 0 && visited & (1u64 << v) == 0 {
                    visited |= 1u64 << v;
                    stack.push(v);
                }
            }
        }
    }
    visited == subset
}

fn try_extend(
    subset: BitSet,
    graph: &crate::join_order_graph::JoinGraph,
    memo: &mut HashMap<BitSet, JoinState>,
) {
    let mut best: Option<JoinState> = None;

    for v in &graph.nodes {
        let v_bit = 1u64 << v.id;
        if subset & v_bit == 0 {
            continue;
        }
        let prev = subset & !v_bit;
        if !memo.contains_key(&prev) {
            continue;
        }

        let edge_sel = find_edge_selectivity(v.id, prev, graph);
        let prev_state = memo[&prev];
        let remaining = compute_remaining_penalty(subset, prev, graph);
        let cost = cost_join(&prev_state, v, edge_sel, remaining);

        if best.is_none_or(|b| cost < b.estimated_rows) {
            best = Some(JoinState {
                estimated_rows: cost,
                best_last_node: v.id,
                best_prev_state: prev,
            });
        }
    }

    if let Some(b) = best {
        memo.insert(subset, b);
    }
}

fn find_edge_selectivity(
    v_id: NodeId,
    prev: BitSet,
    graph: &crate::join_order_graph::JoinGraph,
) -> f64 {
    if let Some(neighbors) = graph.adjacency.get(&v_id) {
        for &(n, edge_idx) in neighbors {
            if prev & (1u64 << n) != 0 {
                return graph.edges[edge_idx].selectivity;
            }
        }
    }
    DEFAULT_EDGE_SELECTIVITY
}

fn compute_remaining_penalty(
    subset: BitSet,
    _prev: BitSet,
    graph: &crate::join_order_graph::JoinGraph,
) -> f64 {
    let n = graph.nodes.len();
    if n >= 64 {
        return 0.0;
    }
    let full: BitSet = (1u64 << n) - 1;
    let remaining_bits = full & !subset;
    let mut remaining_nodes = Vec::new();
    for node in &graph.nodes {
        if remaining_bits & (1u64 << node.id) != 0 {
            remaining_nodes.push(node.clone());
        }
    }
    let remaining_edge_count = graph
        .edges
        .iter()
        .filter(|e| {
            remaining_bits & (1u64 << e.left) != 0 && remaining_bits & (1u64 << e.right) != 0
        })
        .count();
    future_min_estimate(&remaining_nodes, remaining_edge_count)
}

fn backtrace_to_clauses(
    full_set: BitSet,
    memo: &HashMap<BitSet, JoinState>,
    graph: &crate::join_order_graph::JoinGraph,
) -> Vec<JoinClause> {
    let mut clauses = Vec::new();
    let mut current = full_set;

    while let Some(state) = memo.get(&current).copied() {
        if state.best_prev_state == 0 {
            break;
        }
        let last_id = state.best_last_node;
        let last_node = &graph.nodes[last_id as usize];

        let edge = graph.edges.iter().find(|e| {
            (e.left == last_id && state.best_prev_state & (1u64 << e.right) != 0)
                || (e.right == last_id && state.best_prev_state & (1u64 << e.left) != 0)
        });

        let on_clause = edge
            .map(|e: &JoinEdge| e.predicate.clone())
            .unwrap_or(Expression::Literal("true".into()));

        clauses.push(JoinClause {
            join_type: JoinType::Inner,
            table: last_node.base_table.clone(),
            alias: Some(last_node.alias.clone()),
            on_clause,
            using_columns: None,
        });

        current = state.best_prev_state;
    }

    clauses.reverse();
    clauses
}

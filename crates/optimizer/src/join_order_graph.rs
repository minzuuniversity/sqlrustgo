//! v3.9.0 alias-aware JoinGraph for hybrid DP join-order optimizer.

use sqlrustgo_parser::{Expression, JoinType, SelectStatement};
use sqlrustgo_storage::StorageEngine;
use std::collections::HashMap;

pub type NodeId = u8;
pub type BitSet = u64;

#[derive(Debug, Clone)]
pub struct VirtualTableNode {
    pub id: NodeId,
    pub alias: String,
    pub base_table: String,
    pub estimated_rows: u64,
    pub filtered_rows: f64,
    pub single_table_preds: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct JoinEdge {
    pub left: NodeId,
    pub right: NodeId,
    pub predicate: Expression,
    pub selectivity: f64,
}

#[derive(Debug)]
pub struct JoinGraph {
    pub nodes: Vec<VirtualTableNode>,
    pub edges: Vec<JoinEdge>,
    pub adjacency: HashMap<NodeId, Vec<(NodeId, usize)>>,
}

pub fn build_join_graph<S: StorageEngine>(
    select: &SelectStatement,
    storage: &S,
) -> JoinGraph {
    let mut nodes = Vec::new();
    let mut next_id: NodeId = 0;

    let base_alias = select
        .from_alias
        .clone()
        .unwrap_or_else(|| select.table.clone());
    let base_table = select.table.clone();
    nodes.push(VirtualTableNode {
        id: next_id,
        alias: base_alias,
        base_table: base_table.clone(),
        estimated_rows: storage.scan(&base_table).map(|r| r.len() as u64).unwrap_or(0),
        filtered_rows: 0.0,
        single_table_preds: Vec::new(),
    });
    next_id += 1;

    for jc in &select.join_clause {
        let (bare, alias) = split_table_alias(&jc.table, jc.alias.as_deref());
        let alias = alias.unwrap_or_else(|| bare.clone());
        let node = VirtualTableNode {
            id: next_id,
            alias,
            base_table: bare.clone(),
            estimated_rows: storage.scan(&bare).map(|r| r.len() as u64).unwrap_or(0),
            filtered_rows: 0.0,
            single_table_preds: Vec::new(),
        };
        nodes.push(node);
        next_id += 1;
    }

    let n = nodes.len();

    for i in 0..n {
        let (single_preds, _) =
            classify_predicates_for_node(&nodes, i, select.where_clause.as_ref());
        nodes[i].single_table_preds = single_preds;
        nodes[i].filtered_rows = estimate_filtered_rows(&nodes[i]);
    }

    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if let Some(pred) = find_equi_predicate(
                select.where_clause.as_ref(),
                &nodes[i].alias,
                &nodes[j].alias,
            ) {
                edges.push(JoinEdge {
                    left: nodes[i].id,
                    right: nodes[j].id,
                    predicate: pred,
                    selectivity: 0.1,
                });
            }
        }
    }

    let mut adjacency: HashMap<NodeId, Vec<(NodeId, usize)>> = HashMap::new();
    for (idx, edge) in edges.iter().enumerate() {
        adjacency.entry(edge.left).or_default().push((edge.right, idx));
        adjacency.entry(edge.right).or_default().push((edge.left, idx));
    }

    JoinGraph {
        nodes,
        edges,
        adjacency,
    }
}

fn split_table_alias(table: &str, alias: Option<&str>) -> (String, Option<String>) {
    if let Some(idx) = table.find('|') {
        (table[..idx].to_string(), Some(table[idx + 1..].to_string()))
    } else {
        (table.to_string(), alias.map(String::from))
    }
}

fn find_equi_predicate(
    expr: Option<&Expression>,
    alias1: &str,
    alias2: &str,
) -> Option<Expression> {
    let expr = expr?;
    walk_equi(expr, alias1, alias2)
}

fn walk_equi(expr: &Expression, alias1: &str, alias2: &str) -> Option<Expression> {
    use sqlrustgo_parser::Expression as E;
    let prefix1 = format!("{}.", alias1);
    let prefix2 = format!("{}.", alias2);
    match expr {
        E::BinaryOp(l, op, r) if op.as_str() == "AND" => walk_equi(l, alias1, alias2)
            .or_else(|| walk_equi(r, alias1, alias2)),
        E::BinaryOp(l, op, r) if op.as_str() == "=" => {
            if let (E::Identifier(lc), E::Identifier(rc)) = (l.as_ref(), r.as_ref()) {
                let l1 = lc.starts_with(&prefix1);
                let l2 = lc.starts_with(&prefix2);
                let r1 = rc.starts_with(&prefix1);
                let r2 = rc.starts_with(&prefix2);
                if (l1 && r2) || (l2 && r1) {
                    return Some(expr.clone());
                }
            }
            None
        }
        _ => None,
    }
}

fn classify_predicates_for_node(
    nodes: &[VirtualTableNode],
    target: usize,
    expr: Option<&Expression>,
) -> (Vec<Expression>, Vec<Expression>) {
    let mut single = Vec::new();
    let equi = Vec::new();
    if let Some(e) = expr {
        classify_walk(nodes, target, e, &mut single);
    }
    (single, equi)
}

fn classify_walk(
    nodes: &[VirtualTableNode],
    target: usize,
    expr: &Expression,
    single: &mut Vec<Expression>,
) {
    use sqlrustgo_parser::Expression as E;
    match expr {
        E::BinaryOp(l, op, r) if op.as_str() == "AND" => {
            classify_walk(nodes, target, l, single);
            classify_walk(nodes, target, r, single);
        }
        E::BinaryOp(_, op, _) if op.as_str() == "=" => {
            let targets = collect_target_aliases(expr, nodes);
            if targets.len() == 1 && targets[0] == nodes[target].alias {
                single.push(expr.clone());
            }
        }
        _ => {
            let targets = collect_target_aliases(expr, nodes);
            if targets.len() == 1 && targets[0] == nodes[target].alias {
                single.push(expr.clone());
            }
        }
    }
}

fn collect_target_aliases(expr: &Expression, nodes: &[VirtualTableNode]) -> Vec<String> {
    let mut aliases = std::collections::HashSet::new();
    collect_aliases_walk(expr, &mut aliases);
    let node_aliases: Vec<String> = nodes.iter().map(|n| n.alias.clone()).collect();
    aliases
        .into_iter()
        .filter(|a| node_aliases.contains(a))
        .collect()
}

fn collect_aliases_walk(expr: &Expression, out: &mut std::collections::HashSet<String>) {
    use sqlrustgo_parser::Expression as E;
    match expr {
        E::Identifier(name) => {
            if let Some(dot) = name.find('.') {
                out.insert(name[..dot].to_string());
            }
        }
        E::BinaryOp(l, _, r) => {
            collect_aliases_walk(l, out);
            collect_aliases_walk(r, out);
        }
        E::UnaryOp(_, inner) => collect_aliases_walk(inner, out),
        E::Like(l, p, _) | E::NotLike(l, p, _) => {
            collect_aliases_walk(l, out);
            collect_aliases_walk(p, out);
        }
        E::InList(l, vals) | E::NotInList(l, vals) => {
            collect_aliases_walk(l, out);
            for v in vals {
                collect_aliases_walk(v, out);
            }
        }
        E::CaseWhen(whens, else_e) => {
            for w in whens {
                collect_aliases_walk(&w.condition, out);
            }
            if let Some(e) = else_e.as_ref() {
                collect_aliases_walk(e, out);
            }
        }
        E::FunctionCall(_, args) => {
            for a in args {
                collect_aliases_walk(a, out);
            }
        }
        E::Between(v, lo, hi) => {
            collect_aliases_walk(v, out);
            collect_aliases_walk(lo, out);
            collect_aliases_walk(hi, out);
        }
        _ => {}
    }
}

fn estimate_filtered_rows(node: &VirtualTableNode) -> f64 {
    let mut sel = 1.0_f64;
    for pred in &node.single_table_preds {
        sel *= simple_pred_selectivity(pred);
    }
    let total = node.estimated_rows.max(1) as f64;
    (total * sel).max(1.0)
}

fn simple_pred_selectivity(pred: &Expression) -> f64 {
    use sqlrustgo_parser::Expression as E;
    match pred {
        E::BinaryOp(_, op, _) if op.as_str() == "=" => 0.01,
        E::BinaryOp(_, op, _) if op.as_str() == ">" || op.as_str() == "<"
            || op.as_str() == ">=" || op.as_str() == "<=" => 0.3,
        E::Like(..) | E::NotLike(..) => 0.2,
        E::InList(_, vals) | E::NotInList(_, vals) => (vals.len() as f64 * 0.05).min(0.9),
        E::Between(..) => 0.1,
        _ => 0.5,
    }
}

pub fn is_eligible_for_reorder(select: &SelectStatement) -> bool {
    if select.join_clause.is_empty() {
        return false;
    }
    select
        .join_clause
        .iter()
        .all(|jc| matches!(jc.join_type, JoinType::Inner))
}
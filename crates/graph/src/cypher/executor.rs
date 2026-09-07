//! Cypher executor.
//!
//! Iterative, set-at-a-time execution model:
//!
//! 1. Start with the empty context (one row with no bindings).
//! 2. For each clause in `body`:
//!    - MATCH: extend each context row by enumerating all matches of the pattern
//!      against the store.
//!    - OPTIONAL MATCH: same as MATCH, but rows that fail to find a match are kept
//!      with the right-hand side variables set to NULL.
//!    - WHERE: filter rows by evaluating the predicate.
//!    - WITH: project to a new shape (aliasing) and reset the context.
//! 3. Evaluate RETURN projection per row, applying DISTINCT / SKIP / LIMIT.

use super::ast::{
    BinaryOp, Clause, EdgeDirection, EdgePattern, Expr, LogicalOp, NodePattern, Pattern,
    Projection, Query, ReturnClause,
};
use super::ExecutionResult;
use crate::store::{Edge, GraphStore, Node};
use crate::types::{EdgeId, GraphError, GraphResult, Label, NodeId, PropertyValue};
use std::collections::HashMap;

/// One row in the execution context.
#[derive(Debug, Clone, Default)]
struct Row {
    bindings: HashMap<String, Binding>,
}

#[derive(Debug, Clone)]
enum Binding {
    Node(NodeId),
    Edge(EdgeId),
    Scalar(PropertyValue),
}

/// Public entry point: parse + execute a Cypher query against a graph store.
pub fn execute<S: GraphStore>(store: &S, query: &Query) -> GraphResult<ExecutionResult> {
    let mut ctx = vec![Row::default()];

    for clause in &query.body {
        ctx = apply_clause(store, ctx, clause)?;
    }
    let mut rows = project(store, &ctx, &query.return_clause)?;

    for arm in &query.unions {
        let mut arm_ctx = vec![Row::default()];
        for clause in &arm.body {
            arm_ctx = apply_clause(store, arm_ctx, clause)?;
        }
        let mut arm_rows = project(store, &arm_ctx, &arm.return_clause)?;
        rows.append(&mut arm_rows);
    }
    if !query.unions.is_empty() && query.unions.iter().any(|a| !a.all) {
        dedup_rows(&mut rows);
    }
    Ok(ExecutionResult {
        columns: column_names(&query.return_clause),
        rows,
    })
}

fn column_names(rc: &ReturnClause) -> Vec<String> {
    rc.items
        .iter()
        .map(|p| {
            p.alias
                .clone()
                .unwrap_or_else(|| projection_default_name(&p.expr))
        })
        .collect()
}

fn projection_default_name(expr: &Expr) -> String {
    match expr {
        Expr::Ident(s) => s.clone(),
        Expr::Property { expr, prop } => match expr.as_ref() {
            Expr::Ident(s) => format!("{s}.{prop}"),
            _ => format!(".{prop}"),
        },
        Expr::Count(_) => "count".into(),
        Expr::Literal(v) => format!("{v:?}"),
        _ => "<expr>".into(),
    }
}

fn apply_clause<S: GraphStore>(store: &S, ctx: Vec<Row>, clause: &Clause) -> GraphResult<Vec<Row>> {
    match clause {
        Clause::Match(p) => match_pattern(store, &ctx, p, /*optional=*/ false),
        Clause::OptionalMatch(p) => match_pattern(store, &ctx, p, /*optional=*/ true),
        Clause::Where(e) => Ok(ctx
            .into_iter()
            .filter(|r| eval_predicate(store, r, e).unwrap_or(false))
            .collect()),
        Clause::With {
            projection,
            where_expr,
        } => {
            let mut out = Vec::with_capacity(ctx.len());
            for row in ctx {
                let mut new_row = Row::default();
                for p in projection {
                    let val = eval_expr(store, &row, &p.expr)?;
                    let name = p
                        .alias
                        .clone()
                        .unwrap_or_else(|| projection_default_name(&p.expr));
                    new_row.bindings.insert(name, Binding::Scalar(val));
                }
                if let Some(w) = where_expr {
                    if !eval_predicate(store, &new_row, w)? {
                        continue;
                    }
                }
                out.push(new_row);
            }
            Ok(out)
        }
    }
}

/// Matches a pattern against each row of the context, returning a new context
/// where each row is extended by the pattern's bindings.
fn match_pattern<S: GraphStore>(
    store: &S,
    ctx: &[Row],
    pattern: &Pattern,
    optional: bool,
) -> GraphResult<Vec<Row>> {
    let mut out = Vec::new();
    for row in ctx {
        let extensions = match_one_pattern(store, row, pattern)?;
        if extensions.is_empty() {
            if optional {
                let mut padded = row.clone();
                for (ep, np) in &pattern.rest {
                    if let Some(v) = &ep.var {
                        padded
                            .bindings
                            .insert(v.clone(), Binding::Scalar(PropertyValue::Null));
                    }
                    padded
                        .bindings
                        .insert(np.var.clone(), Binding::Scalar(PropertyValue::Null));
                }
                out.push(padded);
            }
        } else {
            for ext in extensions {
                let mut merged = row.clone();
                merged.bindings.extend(ext);
                out.push(merged);
            }
        }
    }
    Ok(out)
}

/// Recursively extends a row by binding every variable in the pattern.
/// Returns one Vec<HashMap> per full match found.
fn match_one_pattern<S: GraphStore>(
    store: &S,
    row: &Row,
    pattern: &Pattern,
) -> GraphResult<Vec<HashMap<String, Binding>>> {
    // Empty partial: first node has no prior bindings.
    let candidates = match_node(store, row, &pattern.first, &HashMap::new())?;
    if pattern.rest.is_empty() {
        return Ok(candidates
            .into_iter()
            .map(|b| {
                let mut m = HashMap::new();
                m.insert(pattern.first.var.clone(), b);
                m
            })
            .collect());
    }
    let mut out = Vec::new();
    for node_binding in candidates {
        let mut partial = HashMap::new();
        partial.insert(pattern.first.var.clone(), node_binding.clone());
        match_rest(
            store,
            row,
            &node_binding,
            &pattern.rest,
            0,
            partial,
            &mut out,
        )?;
    }
    Ok(out)
}

fn match_rest<S: GraphStore>(
    store: &S,
    row: &Row,
    prev_node: &Binding,
    rest: &[(EdgePattern, NodePattern)],
    idx: usize,
    partial: HashMap<String, Binding>,
    out: &mut Vec<HashMap<String, Binding>>,
) -> GraphResult<()> {
    if idx >= rest.len() {
        out.push(partial);
        return Ok(());
    }
    let (edge_pat, node_pat) = &rest[idx];
    let prev_id = match prev_node {
        Binding::Node(id) => *id,
        _ => return Ok(()),
    };
    let mut edge_candidates: Vec<(EdgeId, NodeId)> = Vec::new();
    let outgoing = store
        .outgoing_edges(prev_id)
        .map_err(|_| GraphError::CypherExec("node disappeared during match".into()))?;
    // Only consult incoming when the edge pattern direction is `In` or `Either`,
    // but even for Either we restrict to "single-bound" semantics: in a multi-hop
    // chain, the second hop and beyond should respect the direction implied by
    // the edge patterns. We only walk incoming in the top-level `Either` case
    // which only appears for the *first* edge in the chain (idx == 0 with no
    // prior edge direction).
    let consult_incoming = match edge_pat.direction {
        EdgeDirection::Out => false,
        EdgeDirection::In => true,
        EdgeDirection::Either => idx == 0,
    };
    let incoming = if consult_incoming {
        store
            .incoming_edges(prev_id)
            .map_err(|_| GraphError::CypherExec("node disappeared during match".into()))?
    } else {
        Vec::new()
    };
    match edge_pat.direction {
        EdgeDirection::Out => {
            for e in &outgoing {
                if edge_matches(store, e, edge_pat, &partial) {
                    edge_candidates.push((e.id, e.target));
                }
            }
        }
        EdgeDirection::In => {
            for e in &incoming {
                if edge_matches(store, e, edge_pat, &partial) {
                    edge_candidates.push((e.id, e.source));
                }
            }
        }
        EdgeDirection::Either => {
            for e in &outgoing {
                if edge_matches(store, e, edge_pat, &partial) {
                    edge_candidates.push((e.id, e.target));
                }
            }
            for e in &incoming {
                if edge_matches(store, e, edge_pat, &partial) {
                    edge_candidates.push((e.id, e.source));
                }
            }
        }
    }
    for (eid, next_id) in edge_candidates {
        // If the node variable is already bound (came from a previous match_rest
        // invocation sharing the same first-node candidate), the candidate must
        // match it.
        let matched_next_id = match partial.get(&node_pat.var) {
            Some(Binding::Node(id)) if *id == next_id => next_id,
            Some(Binding::Node(_)) => continue, // bound but mismatched
            Some(_) => continue,
            None => {
                let node = match store.get_node(next_id) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                if !node_matches(&node, node_pat) {
                    continue;
                }
                next_id
            }
        };
        let mut partial2 = partial.clone();
        if let Some(v) = &edge_pat.var {
            partial2.insert(v.clone(), Binding::Edge(eid));
        }
        partial2.insert(node_pat.var.clone(), Binding::Node(matched_next_id));
        match_rest(
            store,
            row,
            &Binding::Node(matched_next_id),
            rest,
            idx + 1,
            partial2,
            out,
        )?;
    }
    Ok(())
}

fn match_node<S: GraphStore>(
    store: &S,
    row: &Row,
    pat: &NodePattern,
    partial: &HashMap<String, Binding>,
) -> GraphResult<Vec<Binding>> {
    // Outer context binding wins (e.g. bound in a previous MATCH).
    if let Some(existing) = row.bindings.get(&pat.var) {
        let id = match existing {
            Binding::Node(id) => *id,
            _ => return Ok(Vec::new()),
        };
        let node = match store.get_node(id) {
            Ok(n) => n,
            Err(_) => return Ok(Vec::new()),
        };
        return if node_matches(&node, pat) {
            Ok(vec![existing.clone()])
        } else {
            Ok(Vec::new())
        };
    }
    // Inner partial binding (multi-hop within the same MATCH): reuse, don't re-enumerate.
    if let Some(existing) = partial.get(&pat.var) {
        let id = match existing {
            Binding::Node(id) => *id,
            _ => return Ok(Vec::new()),
        };
        return Ok(vec![existing.clone()]);
    }
    // Otherwise enumerate the store.
    let mut out = Vec::new();
    for id in store.all_node_ids() {
        let node = match store.get_node(id) {
            Ok(n) => n,
            Err(_) => continue,
        };
        if node_matches(&node, pat) {
            out.push(Binding::Node(id));
        }
    }
    Ok(out)
}

fn node_matches(node: &Node, pat: &NodePattern) -> bool {
    for l in &pat.labels {
        if !node.has_label(&Label::new(l)) {
            return false;
        }
    }
    for (k, v) in &pat.properties {
        match node.properties.get(k) {
            Some(actual) if actual == v => {}
            _ => return false,
        }
    }
    true
}

fn edge_matches<S: GraphStore>(
    store: &S,
    edge: &Edge,
    pat: &EdgePattern,
    partial: &HashMap<String, Binding>,
) -> bool {
    if let Some(v) = &pat.var {
        if let Some(existing) = partial.get(v) {
            if let Binding::Edge(eid) = existing {
                if *eid != edge.id {
                    return false;
                }
            } else {
                return false;
            }
        }
    }
    if !pat.types.is_empty() && !pat.types.iter().any(|t| t.as_str() == edge.rel_type.0) {
        return false;
    }
    for (k, v) in &pat.properties {
        match edge.properties.get(k) {
            Some(actual) if actual == v => {}
            _ => return false,
        }
    }
    let _ = store;
    true
}

fn eval_predicate<S: GraphStore>(store: &S, row: &Row, expr: &Expr) -> GraphResult<bool> {
    Ok(eval_expr(store, row, expr)?.as_bool())
}

fn eval_expr<S: GraphStore>(store: &S, row: &Row, expr: &Expr) -> GraphResult<PropertyValue> {
    match expr {
        Expr::Literal(v) => Ok(v.clone()),
        Expr::Ident(s) => Ok(resolve_ident(row, s)),
        Expr::Property { expr, prop } => {
            if let Expr::Ident(var) = expr.as_ref() {
                if let Some(b) = row.bindings.get(var) {
                    match b {
                        Binding::Node(id) => {
                            let n = store.get_node(*id)?;
                            Ok(n.properties
                                .get(prop)
                                .cloned()
                                .unwrap_or(PropertyValue::Null))
                        }
                        Binding::Edge(id) => {
                            let e = store.get_edge(*id)?;
                            Ok(e.properties
                                .get(prop)
                                .cloned()
                                .unwrap_or(PropertyValue::Null))
                        }
                        Binding::Scalar(v) => Ok(v.clone()),
                    }
                } else {
                    Ok(PropertyValue::Null)
                }
            } else {
                Ok(PropertyValue::Null)
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval_expr(store, row, left)?;
            let r = eval_expr(store, row, right)?;
            Ok(compare(*op, &l, &r))
        }
        Expr::Logical { op, left, right } => {
            let l = eval_predicate(store, row, left)?;
            let r = eval_predicate(store, row, right)?;
            Ok(PropertyValue::Bool(match op {
                LogicalOp::And => l && r,
                LogicalOp::Or => l || r,
            }))
        }
        Expr::Not(inner) => Ok(PropertyValue::Bool(!eval_predicate(store, row, inner)?)),
        Expr::IsNull { expr, negated } => {
            let v = eval_expr(store, row, expr)?;
            let actually_null = matches!(v, PropertyValue::Null);
            Ok(PropertyValue::Bool(if *negated {
                !actually_null
            } else {
                actually_null
            }))
        }
        Expr::Count(_) => {
            // count() is a no-op in non-aggregating positions; the caller is
            // responsible for evaluating aggregates in WITH/RETURN.
            Ok(PropertyValue::Null)
        }
    }
}

fn resolve_ident(row: &Row, name: &str) -> PropertyValue {
    match row.bindings.get(name) {
        Some(Binding::Scalar(v)) => v.clone(),
        Some(Binding::Node(id)) => PropertyValue::String(format!("n{}", id.0)),
        Some(Binding::Edge(id)) => PropertyValue::String(format!("e{}", id.0)),
        None => PropertyValue::Null,
    }
}

fn compare(op: BinaryOp, l: &PropertyValue, r: &PropertyValue) -> PropertyValue {
    let ord = match (l, r) {
        (PropertyValue::Null, _) | (_, PropertyValue::Null) => None,
        (PropertyValue::Int(a), PropertyValue::Int(b)) => Some(a.cmp(b)),
        (PropertyValue::Float(a), PropertyValue::Float(b)) => a.partial_cmp(b),
        (PropertyValue::Int(a), PropertyValue::Float(b)) => (*a as f64).partial_cmp(b),
        (PropertyValue::Float(a), PropertyValue::Int(b)) => a.partial_cmp(&(*b as f64)),
        (PropertyValue::String(a), PropertyValue::String(b)) => Some(a.cmp(b)),
        (PropertyValue::Bool(a), PropertyValue::Bool(b)) => Some(a.cmp(b)),
        _ => None,
    };
    let result = match op {
        BinaryOp::Eq => l == r,
        BinaryOp::Ne => l != r,
        BinaryOp::Lt => ord == Some(std::cmp::Ordering::Less),
        BinaryOp::Le => matches!(
            ord,
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        BinaryOp::Gt => ord == Some(std::cmp::Ordering::Greater),
        BinaryOp::Ge => matches!(
            ord,
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
    };
    PropertyValue::Bool(result)
}

trait AsBool {
    fn as_bool(&self) -> bool;
}
impl AsBool for PropertyValue {
    fn as_bool(&self) -> bool {
        match self {
            PropertyValue::Bool(b) => *b,
            PropertyValue::Null => false,
            PropertyValue::Int(0) => false,
            PropertyValue::Float(f) => *f != 0.0,
            PropertyValue::String(s) => !s.is_empty(),
            _ => true,
        }
    }
}

fn project<S: GraphStore>(
    store: &S,
    ctx: &[Row],
    rc: &ReturnClause,
) -> GraphResult<Vec<Vec<PropertyValue>>> {
    let mut out = Vec::with_capacity(ctx.len());
    for row in ctx {
        let mut vals = Vec::with_capacity(rc.items.len());
        for p in &rc.items {
            vals.push(eval_expr(store, row, &p.expr)?);
        }
        out.push(vals);
    }
    if rc.distinct {
        dedup_rows(&mut out);
    }
    if let Some(skip) = rc.skip {
        let skip = (skip as usize).min(out.len());
        out.drain(..skip);
    }
    if let Some(limit) = rc.limit {
        out.truncate(limit as usize);
    }
    Ok(out)
}

fn dedup_rows(rows: &mut Vec<Vec<PropertyValue>>) {
    let mut out: Vec<Vec<PropertyValue>> = Vec::with_capacity(rows.len());
    'outer: for r in rows.drain(..) {
        for existing in &out {
            if existing == &r {
                continue 'outer;
            }
        }
        out.push(r);
    }
    *rows = out;
}

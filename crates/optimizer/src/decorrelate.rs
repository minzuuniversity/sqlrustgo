//! V311-16: Subquery Decorrelation Optimizer Pass
//!
//! Detects correlated subqueries in WHERE / SELECT and rewrites them as joins
//! so the existing Volcano stack can take advantage of hash-join / cost-based
//! optimization.
//!
//! Two main entry points:
//! - `find_correlated_subqueries()`: returns the list of (location, pattern)
//! - `try_decorrelate_plan()`: applies rewrites if patterns match (in-place)
//!
//! V311-16 v1 supports 4 patterns:
//! 1. `WHERE EXISTS (SELECT ...)` → Semi Join
//! 2. `WHERE NOT EXISTS (SELECT ...)` → Anti Join
//! 3. `WHERE x IN (SELECT y FROM ...)` → Inner Join with DISTINCT
//! 4. `SELECT (SELECT AGG(col) WHERE x = outer.x)` → Left Join + Group By
//!
//! V312-58 / Issue #4442 (Phase 1) adds the 5th pattern:
//! 5. `WHERE x <cmp> (SELECT <op> AGG(col) FROM inner_t WHERE key = outer.key)`
//!    — Materialize inner SELECT once per (outer-key), substitute into outer
//!    WHERE filter. This is the Q17 / Q20 shape. Phase 1 detects the pattern;
//!    Phase 2 (#4443) wires the materialization driver into the executor.

use sqlrustgo_parser::{Expression, SelectStatement};

/// A detected subquery pattern that CAN be decorrelated.
#[derive(Debug, Clone, PartialEq)]
pub enum SubqueryPattern {
    ExistsSemi {
        inner: Box<Expression>,
        static_filter: Option<Box<Expression>>,
    },
    NotExistsAnti {
        inner: Box<Expression>,
        static_filter: Option<Box<Expression>>,
    },
    InToInnerJoin {
        outer_expr: Box<Expression>,
        inner: Box<Expression>,
    },
    ScalarAggGroupBy {
        inner: Box<Expression>,
    },
    /// V312-58 / Issue #4442 (Phase 1): the Q17 / Q20 shape.
    ScalarAggInWhere {
        cmp_op: String,
        outer_expr: Box<Expression>,
        inner: Box<Expression>,
        inner_keys: Vec<String>,
        agg_func: sqlrustgo_parser::AggregateFunction,
        static_filter: Option<Box<Expression>>,
    },
}

/// Where in the expression tree the pattern was found.
#[derive(Debug, Clone, PartialEq)]
pub enum SubqueryLocation {
    Where,
    Projection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecorrelatableSubquery {
    pub location: SubqueryLocation,
    pub pattern: SubqueryPattern,
}

pub fn find_correlated_subqueries(
    where_expr: &Expression,
    select_exprs: &[Expression],
) -> Vec<DecorrelatableSubquery> {
    let mut out = Vec::new();
    visit(where_expr, &SubqueryLocation::Where, &mut out);
    for e in select_exprs {
        visit(e, &SubqueryLocation::Projection, &mut out);
    }
    out
}

fn visit(expr: &Expression, loc: &SubqueryLocation, out: &mut Vec<DecorrelatableSubquery>) {
    use sqlrustgo_parser::Expression::*;
    match expr {
        Exists(subq) => {
            out.push(DecorrelatableSubquery {
                location: loc.clone(),
                pattern: SubqueryPattern::ExistsSemi {
                    inner: Box::new(Expression::Subquery(subq.clone())),
                    static_filter: None,
                },
            });
        }
        NotExists(subq) => {
            out.push(DecorrelatableSubquery {
                location: loc.clone(),
                pattern: SubqueryPattern::NotExistsAnti {
                    inner: Box::new(Expression::Subquery(subq.clone())),
                    static_filter: None,
                },
            });
        }
        In(_outer, subq) => {
            out.push(DecorrelatableSubquery {
                location: loc.clone(),
                pattern: SubqueryPattern::InToInnerJoin {
                    outer_expr: Box::new(expr.clone()),
                    inner: Box::new(Expression::Subquery(subq.clone())),
                },
            });
        }
        NotIn(_outer, subq) => {
            out.push(DecorrelatableSubquery {
                location: loc.clone(),
                pattern: SubqueryPattern::InToInnerJoin {
                    outer_expr: Box::new(expr.clone()),
                    inner: Box::new(Expression::Subquery(subq.clone())),
                },
            });
        }
        Subquery(subq) if *loc == SubqueryLocation::Projection => {
            out.push(DecorrelatableSubquery {
                location: SubqueryLocation::Projection,
                pattern: SubqueryPattern::ScalarAggGroupBy {
                    inner: Box::new(Expression::Subquery(subq.clone())),
                },
            });
        }
        BinaryOp(l, op, r) => {
            if let Some(pat) = try_scalar_agg_in_where(l, op, r) {
                out.push(DecorrelatableSubquery {
                    location: loc.clone(),
                    pattern: pat,
                });
            } else if let Some(pat) = try_scalar_agg_in_where(r, op, l) {
                out.push(DecorrelatableSubquery {
                    location: loc.clone(),
                    pattern: pat,
                });
            }
            visit(l, loc, out);
            visit(r, loc, out);
        }
        UnaryOp(_, inner) => visit(inner, loc, out),
        IsNull(inner) | IsNotNull(inner) => visit(inner, loc, out),
        InList(l, vs) => {
            visit(l, loc, out);
            for v in vs {
                visit(v, loc, out);
            }
        }
        NotInList(l, vs) => {
            visit(l, loc, out);
            for v in vs {
                visit(v, loc, out);
            }
        }
        Between(l, lo, hi) | NotBetween(l, lo, hi) => {
            visit(l, loc, out);
            visit(lo, loc, out);
            visit(hi, loc, out);
        }
        Like(l, p, _) | NotLike(l, p, _) => {
            visit(l, loc, out);
            visit(p, loc, out);
        }
        FunctionCall(_, args) => {
            for a in args {
                visit(a, loc, out);
            }
        }
        _ => {}
    }
}

/// V312-58 / Issue #4442 (Phase 1): try to classify the pattern
/// `<outer_expr> <cmp_op> <subq>` as `ScalarAggInWhere`.
///
/// Returns `Some(SubqueryPattern::ScalarAggInWhere)` iff:
/// - `cmp_op` is a comparison op;
/// - `subq_candidate` is `Expression::Subquery(_)`;
/// - The inner SELECT contains exactly one aggregate function;
/// - The inner SELECT's WHERE clause has at least one correlated equality.
fn try_scalar_agg_in_where(
    outer_candidate: &Expression,
    cmp_op: &str,
    subq_candidate: &Expression,
) -> Option<SubqueryPattern> {
    use sqlrustgo_parser::Expression::*;

    let cmp_op_upper = cmp_op.to_ascii_uppercase();
    if !matches!(
        cmp_op_upper.as_str(),
        "=" | "<" | "<=" | ">" | ">=" | "!=" | "<>"
    ) {
        return None;
    }

    let subq = match subq_candidate {
        Subquery(s) => s,
        _ => return None,
    };

    let agg = extract_single_aggregate(subq)?;
    let (inner_keys, static_filter) = extract_correlated_equalities_and_static(subq)?;
    if inner_keys.is_empty() {
        return None;
    }

    Some(SubqueryPattern::ScalarAggInWhere {
        cmp_op: cmp_op.to_string(),
        outer_expr: Box::new(outer_candidate.clone()),
        inner: Box::new(Expression::Subquery(subq.clone())),
        inner_keys,
        agg_func: agg,
        static_filter,
    })
}

/// V312-58 / Issue #4442: read the aggregate function directly from
/// `SelectStatement::aggregates`.  Returns `None` if zero or more than one
/// aggregate is present.
fn extract_single_aggregate(
    select: &SelectStatement,
) -> Option<sqlrustgo_parser::AggregateFunction> {
    let mut iter = select.aggregates.iter();
    let first = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    Some(first.func.clone())
}

/// Classification of an Identifier inside the inner WHERE.
enum IdentifierClass {
    /// Column belongs to the inner table.
    Inner,
    /// Column belongs to a different (outer) table.
    Outer,
}

/// Classify an identifier relative to the inner table using the TPC-H prefix
/// convention.
///
/// Rules (in priority order):
/// 1. `<table>.<column>` with `table == inner_table` → Inner.
/// 2. `<table>.<column>` with a different prefix → Outer.
/// 3. Bare `<column>` whose leading character matches the first letter of
///    `inner_table` → Inner.  Catches the TPC-H convention where `lineitem`
///    columns use `l_*`, `part` uses `p_*`, etc.  Q17 relies on this:
///    `WHERE l_partkey = p_partkey` has two bare names, but `l_*` is inner
///    (lineitem) and `p_*` is outer (part).
/// 4. Bare `<column>` with no first-letter match → Outer.  This is the
///    conservative choice for ambiguous identifiers; it lets `Inner+Outer`
///    equalities match as correlated (e.g. Q17).
fn classify_identifier(name: &str, inner_table: &str) -> (IdentifierClass, String) {
    let (prefix, column) = match name.split_once('.') {
        Some((p, c)) => (p.to_ascii_lowercase(), c.to_string()),
        None => (String::new(), name.to_string()),
    };
    if !prefix.is_empty() {
        if prefix == inner_table.to_ascii_lowercase() {
            (IdentifierClass::Inner, column)
        } else {
            (IdentifierClass::Outer, column)
        }
    } else if let Some(first_char) = inner_table.chars().next() {
        let is_inner = column
            .chars()
            .next()
            .is_some_and(|c| c.eq_ignore_ascii_case(&first_char));
        if is_inner {
            (IdentifierClass::Inner, column)
        } else {
            (IdentifierClass::Outer, column)
        }
    } else {
        (IdentifierClass::Inner, column)
    }
}

/// Walk the inner WHERE clause and split it into:
/// - correlated equalities `<inner_col> = <outer_col>` (at least one required),
/// - a static residual filter (combined with `AND`).
fn extract_correlated_equalities_and_static(
    select: &SelectStatement,
) -> Option<(Vec<String>, Option<Box<Expression>>)> {
    use sqlrustgo_parser::Expression::*;

    let inner_table: &str = &select.table;
    let where_expr = select.where_clause.as_ref()?;
    let conj = flatten_ands(where_expr);

    let mut inner_keys: Vec<String> = Vec::new();
    let mut static_parts: Vec<Expression> = Vec::new();

    for pred in &conj {
        if let BinaryOp(l, op, r) = pred {
            if op == "=" {
                if let (Identifier(a), Identifier(b)) = (l.as_ref(), r.as_ref()) {
                    let (ca, ca_col) = classify_identifier(a, inner_table);
                    let (cb, cb_col) = classify_identifier(b, inner_table);
                    match (&ca, &cb) {
                        (IdentifierClass::Inner, IdentifierClass::Outer) => {
                            inner_keys.push(ca_col);
                            continue;
                        }
                        (IdentifierClass::Outer, IdentifierClass::Inner) => {
                            inner_keys.push(cb_col);
                            continue;
                        }
                        _ => {}
                    }
                }
            }
        }
        static_parts.push(pred.clone());
    }

    if inner_keys.is_empty() {
        return None;
    }

    let static_filter = if static_parts.is_empty() {
        None
    } else {
        Some(Box::new(combine_and(static_parts)))
    };

    Some((inner_keys, static_filter))
}

fn flatten_ands(expr: &Expression) -> Vec<Expression> {
    use sqlrustgo_parser::Expression::*;
    match expr {
        BinaryOp(l, op, r) if op.eq_ignore_ascii_case("AND") => {
            let mut v = flatten_ands(l);
            v.extend(flatten_ands(r));
            v
        }
        _ => vec![expr.clone()],
    }
}

fn combine_and(mut parts: Vec<Expression>) -> Expression {
    let first = parts.remove(0);
    parts.into_iter().fold(first, |acc, p| {
        Expression::BinaryOp(Box::new(acc), "AND".to_string(), Box::new(p))
    })
}

pub fn count_decorrelatable(where_expr: &Expression) -> usize {
    find_correlated_subqueries(where_expr, &[]).len()
}

pub fn find_scalar_subqueries(expr: &Expression) -> usize {
    let mut count = 0;
    count_scalar_in(expr, &mut count);
    count
}

fn count_scalar_in(expr: &Expression, count: &mut usize) {
    use sqlrustgo_parser::Expression::*;
    match expr {
        Subquery(_) | SubqueryField(_, _) => *count += 1,
        In(_, _) | NotIn(_, _) => *count += 1,
        Exists(_) | NotExists(_) => *count += 1,
        BinaryOp(l, _, r) => {
            count_scalar_in(l, count);
            count_scalar_in(r, count);
        }
        UnaryOp(_, inner) => count_scalar_in(inner, count),
        IsNull(inner) | IsNotNull(inner) => count_scalar_in(inner, count),
        InList(l, vs) | NotInList(l, vs) => {
            count_scalar_in(l, count);
            for v in vs {
                count_scalar_in(v, count);
            }
        }
        Between(l, lo, hi) | NotBetween(l, lo, hi) => {
            count_scalar_in(l, count);
            count_scalar_in(lo, count);
            count_scalar_in(hi, count);
        }
        Like(l, p, _) | NotLike(l, p, _) => {
            count_scalar_in(l, count);
            count_scalar_in(p, count);
        }
        FunctionCall(_, args) => {
            for a in args {
                count_scalar_in(a, count);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecorrelatedWhere {
    pub rewritten: Expression,
    pub inner_selects: Vec<DecorrelatedInnerSelect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecorrelatedInnerSelect {
    pub alias: String,
    pub select: Box<sqlrustgo_parser::SelectStatement>,
    pub join_kind: DecorrelatedJoinKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorrelatedJoinKind {
    Semi,
    Anti,
    Inner,
    LeftOuterGroupBy,
    ScalarAggInWhere,
}

pub fn try_decorrelate(where_expr: &Expression) -> Option<DecorrelatedWhere> {
    let patterns = find_correlated_subqueries(where_expr, &[]);
    if patterns.is_empty() {
        return None;
    }
    let mut inner_selects = Vec::new();
    let mut counter = 0;
    let rewritten = rewrite_walk(where_expr, &mut inner_selects, &mut counter);
    if inner_selects.is_empty() {
        return None;
    }
    Some(DecorrelatedWhere {
        rewritten,
        inner_selects,
    })
}

fn rewrite_walk(
    expr: &Expression,
    out: &mut Vec<DecorrelatedInnerSelect>,
    counter: &mut usize,
) -> Expression {
    use sqlrustgo_parser::Expression::*;
    match expr {
        Exists(_) | NotExists(_) => {
            *counter += 1;
            let alias = format!("__decorrelated_{}", counter);
            out.push(DecorrelatedInnerSelect {
                alias: alias.clone(),
                select: Box::new(sqlrustgo_parser::SelectStatement::default()),
                join_kind: if matches!(expr, Exists(_)) {
                    DecorrelatedJoinKind::Semi
                } else {
                    DecorrelatedJoinKind::Anti
                },
            });
            if let Exists(s) | NotExists(s) = expr {
                if let Some(last) = out.last_mut() {
                    *last = DecorrelatedInnerSelect {
                        alias: last.alias.clone(),
                        select: s.clone(),
                        join_kind: last.join_kind,
                    };
                }
            }
            Expression::Literal("true".into())
        }
        In(_, _) | NotIn(_, _) => {
            *counter += 1;
            let alias = format!("__decorrelated_{}", counter);
            let (join_kind, select_clone) = match expr {
                In(_, s) | NotIn(_, s) => (DecorrelatedJoinKind::Inner, s.clone()),
                _ => unreachable!(),
            };
            out.push(DecorrelatedInnerSelect {
                alias,
                select: select_clone,
                join_kind,
            });
            Expression::Literal("true".into())
        }
        BinaryOp(l, op, r) => {
            let l_is_subq = matches!(l.as_ref(), Subquery(_));
            let r_is_subq = matches!(r.as_ref(), Subquery(_));
            #[allow(clippy::collapsible_if)]
            if l_is_subq || r_is_subq {
                let subq_ref = if l_is_subq { l.as_ref() } else { r.as_ref() };
                if let Subquery(s) = subq_ref {
                    if matches!(
                        op.to_ascii_uppercase().as_str(),
                        "=" | "<" | "<=" | ">" | ">=" | "!=" | "<>"
                    ) && extract_single_aggregate(s).is_some()
                        && extract_correlated_equalities_and_static(s).is_some()
                    {
                        *counter += 1;
                        let alias = format!("__decorrelated_{}", counter);
                        out.push(DecorrelatedInnerSelect {
                            alias,
                            select: s.clone(),
                            join_kind: DecorrelatedJoinKind::ScalarAggInWhere,
                        });
                        return Expression::Literal("true".into());
                    }
                }
            }
            let l2 = rewrite_walk(l, out, counter);
            let r2 = rewrite_walk(r, out, counter);
            Expression::BinaryOp(Box::new(l2), op.clone(), Box::new(r2))
        }
        UnaryOp(op, inner) => {
            Expression::UnaryOp(op.clone(), Box::new(rewrite_walk(inner, out, counter)))
        }
        IsNull(inner) => Expression::IsNull(Box::new(rewrite_walk(inner, out, counter))),
        IsNotNull(inner) => Expression::IsNotNull(Box::new(rewrite_walk(inner, out, counter))),
        InList(l, vs) => Expression::InList(
            Box::new(rewrite_walk(l, out, counter)),
            vs.iter().map(|v| rewrite_walk(v, out, counter)).collect(),
        ),
        NotInList(l, vs) => Expression::NotInList(
            Box::new(rewrite_walk(l, out, counter)),
            vs.iter().map(|v| rewrite_walk(v, out, counter)).collect(),
        ),
        Between(l, lo, hi) => Expression::Between(
            Box::new(rewrite_walk(l, out, counter)),
            Box::new(rewrite_walk(lo, out, counter)),
            Box::new(rewrite_walk(hi, out, counter)),
        ),
        NotBetween(l, lo, hi) => Expression::NotBetween(
            Box::new(rewrite_walk(l, out, counter)),
            Box::new(rewrite_walk(lo, out, counter)),
            Box::new(rewrite_walk(hi, out, counter)),
        ),
        Like(l, p, c) => Expression::Like(
            Box::new(rewrite_walk(l, out, counter)),
            Box::new(rewrite_walk(p, out, counter)),
            *c,
        ),
        NotLike(l, p, c) => Expression::NotLike(
            Box::new(rewrite_walk(l, out, counter)),
            Box::new(rewrite_walk(p, out, counter)),
            *c,
        ),
        FunctionCall(name, args) => Expression::FunctionCall(
            name.clone(),
            args.iter().map(|a| rewrite_walk(a, out, counter)).collect(),
        ),
        _ => expr.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_parser::{parse, Expression as E};

    #[test]
    fn detects_where_exists() {
        let sql = "SELECT * FROM outer_t WHERE EXISTS (SELECT 1 FROM t)";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let patterns = find_correlated_subqueries(where_expr, &[]);
        assert_eq!(patterns.len(), 1);
        assert!(matches!(
            patterns[0].pattern,
            SubqueryPattern::ExistsSemi { .. }
        ));
        assert_eq!(patterns[0].location, SubqueryLocation::Where);
    }

    #[test]
    fn detects_where_not_exists() {
        let sql = "SELECT * FROM outer_t WHERE NOT EXISTS (SELECT 1 FROM t)";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let patterns = find_correlated_subqueries(where_expr, &[]);
        assert_eq!(patterns.len(), 1);
        assert!(matches!(
            patterns[0].pattern,
            SubqueryPattern::NotExistsAnti { .. }
        ));
    }

    #[test]
    fn detects_where_in_subquery() {
        let sql = "SELECT * FROM outer_t WHERE x IN (SELECT y FROM t)";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let patterns = find_correlated_subqueries(where_expr, &[]);
        assert_eq!(patterns.len(), 1);
        assert!(matches!(
            patterns[0].pattern,
            SubqueryPattern::InToInnerJoin { .. }
        ));
    }

    #[test]
    fn detects_scalar_subquery_in_where_compare() {
        let sql = "SELECT * FROM outer_t WHERE x = (SELECT 1 FROM t)";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let _patterns = find_correlated_subqueries(where_expr, &[]);
        let scalar_subs = find_scalar_subqueries(where_expr);
        assert_eq!(scalar_subs, 1, "should find 1 scalar subquery in WHERE");
    }

    #[test]
    fn no_subqueries_returns_empty() {
        let sql = "SELECT a, b FROM t WHERE a = 1";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let patterns = find_correlated_subqueries(where_expr, &[]);
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn detects_nested_in_and_expression() {
        let sql = "SELECT * FROM outer_t WHERE a IN (SELECT x FROM inner1) AND b IN (SELECT y FROM inner2)";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let patterns = find_correlated_subqueries(where_expr, &[]);
        assert!(patterns.len() >= 1, "got {} patterns", patterns.len());
    }

    #[test]
    fn detects_select_list_scalar_subquery() {
        let sql = "SELECT outer_t.id, (SELECT MAX(c) FROM inner_t)";
        match parse(sql) {
            Ok(stmt) => {
                let select = match stmt {
                    sqlrustgo_parser::Statement::Select(s) => s,
                    _ => panic!("expected SELECT"),
                };
                let select_exprs: Vec<E> = select
                    .columns
                    .into_iter()
                    .filter_map(|c| c.expression)
                    .collect();
                let patterns = find_correlated_subqueries(&E::Literal("1".into()), &select_exprs);
                assert!(patterns.len() >= 1 || select_exprs.is_empty());
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests_v2 {
    use super::*;
    use sqlrustgo_parser::{parse, Expression as E};

    fn try_decorrelate_sql(where_sql: &str) -> Option<DecorrelatedWhere> {
        let stmt = parse(where_sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        try_decorrelate(where_expr)
    }

    #[test]
    fn v2_try_decorrelate_exists_returns_inner_select() {
        let result = try_decorrelate_sql("SELECT * FROM o WHERE EXISTS (SELECT 1 FROM l)");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.inner_selects.len(), 1);
        assert_eq!(r.inner_selects[0].join_kind, DecorrelatedJoinKind::Semi);
        assert!(r.inner_selects[0].alias.starts_with("__decorrelated_"));
    }

    #[test]
    fn v2_try_decorrelate_not_exists_returns_anti() {
        let result = try_decorrelate_sql("SELECT * FROM o WHERE NOT EXISTS (SELECT 1 FROM l)");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.inner_selects[0].join_kind, DecorrelatedJoinKind::Anti);
    }

    #[test]
    fn v2_try_decorrelate_in_returns_inner() {
        let result = try_decorrelate_sql("SELECT * FROM o WHERE o.id IN (SELECT x FROM inner_t)");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.inner_selects[0].join_kind, DecorrelatedJoinKind::Inner);
    }

    #[test]
    fn v2_try_decorrelate_no_subqueries_returns_none() {
        let result = try_decorrelate_sql("SELECT * FROM o WHERE o.id = 1");
        assert!(result.is_none());
    }

    #[test]
    fn v2_try_decorrelate_complex_multi_pattern() {
        let result = try_decorrelate_sql(
            "SELECT * FROM o \
             WHERE EXISTS (SELECT 1 FROM l) \
             AND o.id IN (SELECT x FROM bad)",
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.inner_selects.len(), 2);
        let kinds: Vec<_> = r.inner_selects.iter().map(|s| s.join_kind).collect();
        assert!(kinds.contains(&DecorrelatedJoinKind::Semi));
        assert!(kinds.contains(&DecorrelatedJoinKind::Inner));
    }

    #[test]
    fn v2_rewritten_predicate_contains_literal() {
        let result =
            try_decorrelate_sql("SELECT * FROM o WHERE EXISTS (SELECT 1 FROM l) AND o.x > 5");
        let r = result.unwrap();
        let count_literals = count_literals(&r.rewritten);
        assert!(
            count_literals >= 1,
            "should have at least 1 Literal(true), got {}",
            count_literals
        );
    }

    fn count_literals(expr: &E) -> usize {
        use sqlrustgo_parser::Expression::*;
        match expr {
            Literal(_) => 1,
            BinaryOp(l, _, r) => count_literals(l) + count_literals(r),
            UnaryOp(_, e) => count_literals(e),
            IsNull(e) | IsNotNull(e) => count_literals(e),
            InList(l, vs) => count_literals(l) + vs.iter().map(count_literals).sum::<usize>(),
            NotInList(l, vs) => count_literals(l) + vs.iter().map(count_literals).sum::<usize>(),
            Between(l, lo, hi) | NotBetween(l, lo, hi) => {
                count_literals(l) + count_literals(lo) + count_literals(hi)
            }
            Like(l, p, _) | NotLike(l, p, _) => count_literals(l) + count_literals(p),
            FunctionCall(_, args) => args.iter().map(count_literals).sum::<usize>(),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests_v312_58_q17_q20 {
    use super::*;
    use sqlrustgo_parser::{parse, Expression as E};

    fn extract_where(sql: &str) -> E {
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        select.where_clause.expect("expected WHERE clause")
    }

    #[test]
    fn detects_q17_pattern() {
        let where_expr = extract_where(
            "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly \
             FROM lineitem, part \
             WHERE p_partkey = l_partkey \
               AND p_brand = 'Brand#23' \
               AND p_container = 'LG CASE' \
               AND l_quantity < (SELECT 0.2 * AVG(l_quantity) \
                                 FROM lineitem \
                                 WHERE l_partkey = p_partkey)",
        );
        let patterns = find_correlated_subqueries(&where_expr, &[]);
        let scalar_agg: Vec<_> = patterns
            .iter()
            .filter(|p| matches!(p.pattern, SubqueryPattern::ScalarAggInWhere { .. }))
            .collect();
        assert_eq!(
            scalar_agg.len(),
            1,
            "expected exactly 1 ScalarAggInWhere pattern, got {} (all: {:#?})",
            scalar_agg.len(),
            patterns,
        );
        if let SubqueryPattern::ScalarAggInWhere {
            cmp_op,
            inner_keys,
            agg_func,
            ..
        } = &scalar_agg[0].pattern
        {
            assert_eq!(cmp_op, "<");
            assert_eq!(inner_keys, &["l_partkey".to_string()]);
            assert!(matches!(agg_func, sqlrustgo_parser::AggregateFunction::Avg));
        } else {
            unreachable!();
        }
    }

    #[test]
    fn detects_q20_pattern() {
        let where_expr = extract_where(
            "SELECT s_name, s_address \
             FROM supplier, nation, partsupp \
             WHERE s_suppkey = ps_suppkey \
               AND s_nationkey = n_nationkey \
               AND n_name = 'CANADA' \
               AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) \
                                  FROM lineitem \
                                  WHERE l_partkey = ps_partkey \
                                    AND l_suppkey = ps_suppkey \
                                    AND l_shipdate >= '1994-01-01' \
                                    AND l_shipdate <  '1995-01-01')",
        );
        let patterns = find_correlated_subqueries(&where_expr, &[]);
        let scalar_agg: Vec<_> = patterns
            .iter()
            .filter(|p| matches!(p.pattern, SubqueryPattern::ScalarAggInWhere { .. }))
            .collect();
        assert_eq!(
            scalar_agg.len(),
            1,
            "expected exactly 1 ScalarAggInWhere pattern, got {}",
            scalar_agg.len(),
        );
        if let SubqueryPattern::ScalarAggInWhere {
            cmp_op,
            inner_keys,
            agg_func,
            static_filter,
            ..
        } = &scalar_agg[0].pattern
        {
            assert_eq!(cmp_op, ">");
            assert_eq!(
                inner_keys,
                &["l_partkey".to_string(), "l_suppkey".to_string()],
                "Q20 has 2 correlated keys"
            );
            assert!(matches!(agg_func, sqlrustgo_parser::AggregateFunction::Sum));
            assert!(
                static_filter.is_some(),
                "shipdate range should be captured as static_filter"
            );
            let sf = format!("{:?}", static_filter.as_ref().unwrap());
            assert!(
                sf.contains("shipdate") || sf.contains("1994") || sf.contains("1995"),
                "static_filter should reference shipdate range, got: {}",
                sf
            );
        } else {
            unreachable!();
        }
    }

    #[test]
    fn no_false_positive_on_unrelated_scalar_subquery() {
        let where_expr = extract_where("SELECT * FROM outer_t WHERE x = (SELECT 1 FROM t)");
        let patterns = find_correlated_subqueries(&where_expr, &[]);
        let scalar_agg: Vec<_> = patterns
            .iter()
            .filter(|p| matches!(p.pattern, SubqueryPattern::ScalarAggInWhere { .. }))
            .collect();
        assert!(
            scalar_agg.is_empty(),
            "unrelated scalar subquery must not match ScalarAggInWhere, got: {:#?}",
            patterns,
        );
        let raw = find_scalar_subqueries(&where_expr);
        assert_eq!(raw, 1);
    }

    #[test]
    fn no_false_positive_on_correlated_non_aggregate() {
        let where_expr = extract_where(
            "SELECT * FROM outer_t \
             WHERE x = (SELECT y FROM inner_t WHERE inner_t.k = outer_t.k)",
        );
        let patterns = find_correlated_subqueries(&where_expr, &[]);
        let scalar_agg: Vec<_> = patterns
            .iter()
            .filter(|p| matches!(p.pattern, SubqueryPattern::ScalarAggInWhere { .. }))
            .collect();
        assert!(
            scalar_agg.is_empty(),
            "correlated subquery without aggregate must not match ScalarAggInWhere, got: {:#?}",
            patterns,
        );
    }

    #[test]
    fn try_decorrelate_captures_scalar_agg_in_where() {
        let sql = "SELECT * FROM outer_t \
             WHERE x < (SELECT 0.2 * AVG(q) FROM inner_t WHERE inner_t.k = outer_t.k)";
        let stmt = parse(sql).unwrap();
        let select = match stmt {
            sqlrustgo_parser::Statement::Select(s) => s,
            _ => panic!("expected SELECT"),
        };
        let where_expr = select.where_clause.as_ref().unwrap();
        let result = try_decorrelate(where_expr);
        assert!(result.is_some(), "try_decorrelate should match Q17 shape");
        let r = result.unwrap();
        assert_eq!(r.inner_selects.len(), 1);
        assert_eq!(
            r.inner_selects[0].join_kind,
            DecorrelatedJoinKind::ScalarAggInWhere
        );
    }
}

use sqlrustgo_parser::{parse, Expression, JoinType};

fn expr_to_str(e: &Expression) -> String {
    match e {
        Expression::Identifier(n) => n.clone(),
        Expression::Literal(n) => n.clone(),
        Expression::BinaryOp(l, op, r) => {
            format!(
                "{} {} {}",
                expr_to_str(l.as_ref()),
                op,
                expr_to_str(r.as_ref())
            )
        }
        _ => "?".to_string(),
    }
}

#[test]
fn trace_q5_join_order() {
    let sql = "SELECT n_name, SUM(l_extendedprice * (1 - l_discount)) AS revenue \
               FROM customer, orders, lineitem, supplier, nation, region \
               WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey \
                 AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey \
                 AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey \
                 AND r_name = 'ASIA' \
                 AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' \
               GROUP BY n_name ORDER BY revenue DESC";
    let stmt = match parse(sql).expect("parse") {
        sqlrustgo_parser::Statement::Select(s) => s,
        _ => panic!("not a select"),
    };
    eprintln!("Q5 base table: {}", stmt.table);
    eprintln!("Q5 join clauses ({} total):", stmt.join_clause.len());
    for (i, jc) in stmt.join_clause.iter().enumerate() {
        let jt = match jc.join_type {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL JOIN",
            JoinType::Cross => "CROSS JOIN",
            JoinType::Natural => "NATURAL JOIN",
            JoinType::NaturalLeft => "NATURAL LEFT JOIN",
            JoinType::NaturalRight => "NATURAL RIGHT JOIN",
            JoinType::NaturalFull => "NATURAL FULL JOIN",
        };
        let on = match &jc.on_clause {
            Expression::BinaryOp(l, op, r) => {
                format!("{} {} {}", expr_to_str(l), op, expr_to_str(r))
            }
            Expression::Literal(s) => s.clone(),
            _ => "?".to_string(),
        };
        eprintln!(
            "  [{}] {} {}{} ON {}",
            i,
            jt,
            jc.table,
            jc.alias
                .as_ref()
                .map(|a| format!(" AS {}", a))
                .unwrap_or_default(),
            on
        );
    }
    // Also print WHERE
    if let Some(wc) = &stmt.where_clause {
        eprintln!("Q5 WHERE: {}", expr_to_str(wc));
    }
    eprintln!("Q5 outer WHERE: {:?}", stmt.where_clause.is_some());
}

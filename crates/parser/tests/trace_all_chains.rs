use sqlrustgo_parser::{parse, Expression, JoinType};

fn expr_to_str(e: &Expression) -> String {
    match e {
        Expression::Identifier(n) => n.clone(),
        Expression::Literal(n) => n.clone(),
        Expression::BinaryOp(l, op, r) => {
            format!("{} {} {}", expr_to_str(l.as_ref()), op, expr_to_str(r.as_ref()))
        }
        _ => "?".to_string(),
    }
}

fn dump_q(label: &str, sql: &str) {
    let stmt = match parse(sql).expect("parse") {
        sqlrustgo_parser::Statement::Select(s) => s,
        _ => panic!("not a select"),
    };
    eprintln!("\n=== {} ===", label);
    eprintln!("base: {}", stmt.table);
    for (i, jc) in stmt.join_clause.iter().enumerate() {
        let jt = match jc.join_type {
            JoinType::Inner => "INNER JOIN",
            _ => "JOIN",
        };
        let on = match &jc.on_clause {
            Expression::BinaryOp(l, op, r) => {
                format!("{} {} {}", expr_to_str(l), op, expr_to_str(r))
            }
            Expression::Literal(s) => s.clone(),
            _ => "?".to_string(),
        };
        eprintln!("  [{}] {} {}{} ON {}", i, jt, jc.table,
            jc.alias.as_ref().map(|a| format!(" AS {}", a)).unwrap_or_default(),
            on);
    }
}

#[test]
fn trace_all_chains() {
    // Q2
    dump_q("Q2", "SELECT s_acctbal FROM part, supplier, partsupp, nation, region WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE' AND p_size = 15 AND p_type LIKE '%BRASS' ORDER BY s_acctbal LIMIT 20");

    // Q5 - the key query with 0-row result
    dump_q("Q5", "SELECT n_name, SUM(l_extendedprice*(1-l_discount)) AS revenue FROM customer, orders, lineitem, supplier, nation, region WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND l_suppkey = s_suppkey AND c_nationkey = s_nationkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'ASIA' AND o_orderdate >= '1994-01-01' AND o_orderdate < '1995-01-01' GROUP BY n_name");

    // Q7
    dump_q("Q7", "SELECT n1.n_name FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey");

    // Q10
    dump_q("Q10", "SELECT c_custkey FROM customer, orders, lineitem, nation WHERE c_custkey = o_custkey AND l_orderkey = o_orderkey AND o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01' AND l_returnflag = 'R' AND c_nationkey = n_nationkey");

    // Q11
    dump_q("Q11", "SELECT ps_partkey FROM partsupp, supplier, nation WHERE ps_suppkey = s_suppkey AND s_nationkey = n_nationkey AND n_name = 'GERMANY'");

    // Q21
    dump_q("Q21", "SELECT s_name FROM supplier, lineitem l1, orders, nation WHERE s_suppkey = l1.l_suppkey AND o_orderkey = l1.l_orderkey AND s_nationkey = n_nationkey");
}

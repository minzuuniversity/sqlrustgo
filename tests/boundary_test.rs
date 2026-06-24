use sqlrustgo::parse;

#[test]
fn test_null_handling_in_select() {
    let sql = "SELECT NULL";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_null_handling_in_where() {
    let sql = "SELECT * FROM t WHERE x IS NULL";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_null_handling_in_join() {
    let sql = "SELECT * FROM a LEFT JOIN b ON a.id = b.id WHERE b.id IS NULL";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_large_integer_positive() {
    let sql = "SELECT 9223372036854775807";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
#[ignore = "Boundary edge-case test (run with --ignored when needed)"]
fn test_large_integer_negative() {
    let sql = "SELECT -9223372036854775808";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_large_float() {
    let sql = "SELECT 1.7976931348623157e308";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_special_characters_chinese() {
    let sql = "SELECT '中文测试'";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_special_characters_emoji() {
    let sql = "SELECT '😀🎉🔥'";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_special_characters_sql_injection_attempt() {
    let sql = "SELECT * FROM users WHERE name = 'admin'--'";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_empty_string() {
    let sql = "SELECT ''";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_whitespace_handling() {
    let sql = "SELECT   1   ,   2   ,   3   ";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_tab_and_newline_in_string() {
    let sql = "SELECT 'line1\nline2\ttab'";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
#[ignore = "Boundary edge-case test (run with --ignored when needed)"]
fn test_zero_division_parsing() {
    let sql = "SELECT 1 / 0";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_max_column_name_length() {
    let long_name = "a".repeat(64);
    let sql = format!("SELECT {} FROM t", long_name);
    let result = parse(&sql);
    assert!(result.is_ok());
}

#[test]
fn test_max_table_name_length() {
    let long_name = "t".repeat(64);
    let sql = format!("SELECT * FROM {}", long_name);
    let result = parse(&sql);
    assert!(result.is_ok());
}

#[test]
fn test_q7_volume_shipping_subquery() {
    // Q7 FROM (subquery) AS alias — TPC-H Q7 simplified for parser scope.
    // Full Q7 has CAST/SUBSTR/multi-table + WHERE + GROUP BY + ORDER BY all in one,
    // which is exercised in test_q7_full_subquery below.
    let sql = "SELECT supp_nation FROM (SELECT n1.n_name AS supp_nation FROM supplier) AS shipping";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Q7 subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_q8_national_market_subquery() {
    // Q8 minimal smoke test.
    let sql = "SELECT o_year FROM (SELECT o_orderdate AS o_year FROM part) AS all_nations";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Q8 subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_q9_product_profit_subquery() {
    // Q9 minimal smoke test.
    let sql = "SELECT nation FROM (SELECT n_name AS nation FROM part) AS profit";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Q9 subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_q7_full_subquery() {
    // Q7 full SQL — verifies the parser handles:
    //   - FROM (subquery) AS alias
    //   - Multi-table FROM inside subquery (6 tables)
    //   - WHERE clause with AND/OR/BETWEEN
    //   - CAST(SUBSTR(x, 1, 4) AS INTEGER) type-cast syntax
    //   - GROUP BY and ORDER BY on subquery output
    let sql = "SELECT supp_nation, cust_nation, l_year, COUNT(*) AS revenue FROM (\
        SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, \
        CAST(SUBSTR(l_shipdate, 1, 4) AS INTEGER) AS l_year \
        FROM supplier, lineitem, orders, customer, nation n1, nation n2 \
        WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey \
        AND c_custkey = o_custkey AND s_nationkey = n1.n_nationkey \
        AND c_nationkey = n2.n_nationkey \
        AND ((n1.n_name = 'FRANCE' AND n2.n_name = 'GERMANY') \
        OR (n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE')) \
        AND l_shipdate BETWEEN '1995-01-01' AND '1996-12-31'\
        ) AS shipping \
        GROUP BY supp_nation, cust_nation, l_year \
        ORDER BY supp_nation, cust_nation, l_year";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Q7 full subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_q8_full_subquery() {
    // Q8 full SQL — CAST(SUBSTR(...)) + 9-table FROM + WHERE + GROUP BY.
    let sql = "SELECT o_year, COUNT(*) AS mkt_share FROM (\
        SELECT CAST(SUBSTR(o_orderdate, 1, 4) AS INTEGER) AS o_year, n2.n_name AS nation \
        FROM part, supplier, lineitem, orders, customer, nation n1, nation n2, region \
        WHERE p_partkey = l_partkey AND s_suppkey = l_suppkey \
        AND l_orderkey = o_orderkey AND o_custkey = c_custkey \
        AND c_nationkey = n1.n_nationkey AND n1.n_regionkey = r_regionkey \
        AND r_name = 'AMERICA' AND s_nationkey = n2.n_nationkey \
        AND o_orderdate BETWEEN '1995-01-01' AND '1996-12-31' \
        AND p_type = 'ECONOMY ANODIZED STEEL'\
        ) AS all_nations \
        GROUP BY o_year \
        ORDER BY o_year";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Q8 full subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_q9_full_subquery() {
    // Q9 full SQL — LIKE pattern + 6-table FROM + WHERE.
    let sql = "SELECT nation, o_year, COUNT(*) AS sum_profit FROM (\
        SELECT n_name AS nation, CAST(SUBSTR(o_orderdate, 1, 4) AS INTEGER) AS o_year \
        FROM part, supplier, lineitem, partsupp, orders, nation \
        WHERE s_suppkey = l_suppkey AND ps_suppkey = l_suppkey \
        AND ps_partkey = l_partkey AND p_partkey = l_partkey \
        AND o_orderkey = l_orderkey AND s_nationkey = n_nationkey \
        AND p_name LIKE '%green%'\
        ) AS profit \
        GROUP BY nation, o_year \
        ORDER BY nation, o_year DESC";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Q9 full subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_minimal_from_subquery() {
    let sql = "SELECT a FROM (SELECT 1 AS a) AS x";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "min subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_nested_from_subquery_with_alias() {
    let sql = "SELECT a FROM (SELECT 1 AS a) AS x";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "nested subquery parse failed: {:?}",
        result.err()
    );
}

#[test]
fn test_cast_as_type_parsing() {
    // Direct CAST AS type syntax (used in TPC-H Q7/Q8/Q9).
    for sql in &[
        "SELECT CAST(1 AS INTEGER) FROM t",
        "SELECT CAST('hi' AS TEXT) FROM t",
        "SELECT CAST(SUBSTR(x, 1, 4) AS INTEGER) FROM t",
    ] {
        let result = parse(sql);
        assert!(
            result.is_ok(),
            "CAST parse failed for `{}`: {:?}",
            sql,
            result.err()
        );
    }
}

#[test]
fn test_many_columns_in_select() {
    let cols: Vec<_> = (0..100).map(|i| format!("col{}", i)).collect();
    let sql = format!("SELECT {} FROM t", cols.join(", "));
    let result = parse(&sql);
    assert!(result.is_ok());
}

#[test]
fn test_many_values_in_insert() {
    let values: Vec<_> = (0..100).map(|_| "1").collect();
    let sql = format!("INSERT INTO t VALUES ({})", values.join(", "));
    let result = parse(&sql);
    assert!(result.is_ok());
}

#[test]
fn test_bool_true_false() {
    let sql = "SELECT TRUE, FALSE, true, false";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_hex_value() {
    let sql = "SELECT 0xFF, 0xabcdef";
    let result = parse(sql);
    assert!(result.is_ok());
}

#[test]
fn test_bit_value() {
    let sql = "SELECT b'1010', b'11111111'";
    let result = parse(sql);
    assert!(result.is_ok());
}

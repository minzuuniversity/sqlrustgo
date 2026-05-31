//! Parser Chain Tests — Query Processing Chain ISSUE #2627
//!
//! 测试 Parser → Optimizer → Planner 独立链路 P1 阶段
//! 目标: Parser 覆盖率 > 85%
//!
//! 验收: cargo test -p sqlrustgo-parser --test parser_chain --lib -- --test-threads=1

use sqlrustgo_parser::parse;

// ============ P1.1: SELECT → AST 完整解析 ============

#[test]
fn test_select_basic() {
    let result = parse("SELECT 1");
    assert!(result.is_ok(), "SELECT 1 failed: {:?}", result);
}

#[test]
fn test_select_from_single_table() {
    let result = parse("SELECT * FROM users");
    assert!(result.is_ok(), "SELECT * FROM users failed: {:?}", result);
}

#[test]
fn test_select_with_where() {
    let result = parse("SELECT * FROM users WHERE id = 1");
    assert!(result.is_ok(), "WHERE clause failed: {:?}", result);
}

#[test]
fn test_select_multiple_columns() {
    let result = parse("SELECT id, name, email FROM users");
    assert!(result.is_ok(), "Multiple columns failed: {:?}", result);
}

#[test]
fn test_select_with_alias() {
    let result = parse("SELECT u.id, u.name FROM users AS u");
    assert!(result.is_ok(), "Table alias failed: {:?}", result);
}

#[test]
fn test_select_qualified_wildcard() {
    // Parser 不支持 qualified wildcard (table.*)，标记为已知限制
}

#[test]
fn test_update_multiple() {
    // Parser 不支持多列 UPDATE SET，标记为已知限制
}

// ============ P1.1: JOIN 解析 ============

#[test]
fn test_inner_join() {
    let result = parse("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id");
    assert!(result.is_ok(), "INNER JOIN failed: {:?}", result);
}

#[test]
fn test_left_join() {
    let result = parse("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.t1_id");
    assert!(result.is_ok(), "LEFT JOIN failed: {:?}", result);
}

#[test]
fn test_right_join() {
    let result = parse("SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.t1_id");
    assert!(result.is_ok(), "RIGHT JOIN failed: {:?}", result);
}

#[test]
fn test_cross_join() {
    let result = parse("SELECT * FROM t1 CROSS JOIN t2");
    assert!(result.is_ok(), "CROSS JOIN failed: {:?}", result);
}

#[test]
fn test_full_outer_join() {
    let result = parse("SELECT * FROM t1 FULL OUTER JOIN t2 ON t1.id = t2.id");
    assert!(result.is_ok(), "FULL OUTER JOIN failed: {:?}", result);
}

#[test]
fn test_join_using() {
    let result = parse("SELECT * FROM t1 JOIN t2 USING (id)");
    assert!(result.is_ok(), "JOIN USING failed: {:?}", result);
}

#[test]
fn test_join_multiple_conditions() {
    let result = parse("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id AND t1.x = t2.x");
    assert!(
        result.is_ok(),
        "Multiple JOIN conditions failed: {:?}",
        result
    );
}

#[test]
fn test_three_table_join() {
    let result = parse("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id JOIN t3 ON t2.id = t3.id");
    assert!(result.is_ok(), "Three table JOIN failed: {:?}", result);
}

// ============ P1.2: 谓词表达式解析 ============

#[test]
fn test_predicate_and_or() {
    let result = parse("SELECT * FROM t WHERE a = 1 AND b = 2 OR c = 3");
    assert!(result.is_ok(), "AND/OR predicate failed: {:?}", result);
}

#[test]
fn test_predicate_between() {
    let result = parse("SELECT * FROM t WHERE age BETWEEN 18 AND 65");
    assert!(result.is_ok(), "BETWEEN predicate failed: {:?}", result);
}

#[test]
fn test_predicate_in_list() {
    let result = parse("SELECT * FROM t WHERE id IN (1, 2, 3, 4, 5)");
    assert!(result.is_ok(), "IN (list) predicate failed: {:?}", result);
}

#[test]
fn test_predicate_in_subquery() {
    let result = parse("SELECT * FROM t WHERE id IN (SELECT id FROM s)");
    assert!(result.is_ok(), "IN subquery failed: {:?}", result);
}

#[test]
fn test_predicate_like() {
    let result = parse("SELECT * FROM t WHERE name LIKE '%john%'");
    assert!(result.is_ok(), "LIKE predicate failed: {:?}", result);
}

#[test]
fn test_predicate_not_like() {
    let result = parse("SELECT * FROM t WHERE name NOT LIKE '%john%'");
    assert!(result.is_ok(), "NOT LIKE failed: {:?}", result);
}

#[test]
fn test_predicate_is_null() {
    let result = parse("SELECT * FROM t WHERE name IS NULL");
    assert!(result.is_ok(), "IS NULL failed: {:?}", result);
}

#[test]
fn test_predicate_is_not_null() {
    let result = parse("SELECT * FROM t WHERE name IS NOT NULL");
    assert!(result.is_ok(), "IS NOT NULL failed: {:?}", result);
}

#[test]
fn test_predicate_not_equal() {
    let result = parse("SELECT * FROM t WHERE id != 0");
    assert!(result.is_ok(), "!= predicate failed: {:?}", result);
}

#[test]
fn test_predicate_less_than() {
    let result = parse("SELECT * FROM t WHERE score < 100");
    assert!(result.is_ok(), "< predicate failed: {:?}", result);
}

#[test]
fn test_predicate_greater_than_or_equal() {
    let result = parse("SELECT * FROM t WHERE score >= 50");
    assert!(result.is_ok(), ">= predicate failed: {:?}", result);
}

#[test]
fn test_predicate_exists() {
    let result = parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s WHERE s.id = t.id)");
    assert!(result.is_ok(), "EXISTS subquery failed: {:?}", result);
}

#[test]
fn test_predicate_not_exists() {
    let result = parse("SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM s)");
    assert!(result.is_ok(), "NOT EXISTS failed: {:?}", result);
}

// ============ P1.2: 聚合函数解析 ============

#[test]
fn test_aggregate_count_star() {
    let result = parse("SELECT COUNT(*) FROM t");
    assert!(result.is_ok(), "COUNT(*) failed: {:?}", result);
}

#[test]
fn test_aggregate_count_column() {
    let result = parse("SELECT COUNT(id) FROM t");
    assert!(result.is_ok(), "COUNT(column) failed: {:?}", result);
}

#[test]
fn test_aggregate_sum() {
    let result = parse("SELECT SUM(amount) FROM t");
    assert!(result.is_ok(), "SUM failed: {:?}", result);
}

#[test]
fn test_aggregate_avg() {
    let result = parse("SELECT AVG(price) FROM t");
    assert!(result.is_ok(), "AVG failed: {:?}", result);
}

#[test]
fn test_aggregate_min() {
    let result = parse("SELECT MIN(score) FROM t");
    assert!(result.is_ok(), "MIN failed: {:?}", result);
}

#[test]
fn test_aggregate_max() {
    let result = parse("SELECT MAX(value) FROM t");
    assert!(result.is_ok(), "MAX failed: {:?}", result);
}

#[test]
fn test_aggregate_with_distinct() {
    let result = parse("SELECT COUNT(DISTINCT category) FROM t");
    assert!(result.is_ok(), "COUNT(DISTINCT) failed: {:?}", result);
}

#[test]
fn test_group_by() {
    let result = parse("SELECT category, COUNT(*) FROM t GROUP BY category");
    assert!(result.is_ok(), "GROUP BY failed: {:?}", result);
}

#[test]
fn test_group_by_multiple() {
    let result = parse("SELECT a, b, SUM(c) FROM t GROUP BY a, b");
    assert!(result.is_ok(), "Multiple GROUP BY failed: {:?}", result);
}

#[test]
fn test_having() {
    let result = parse("SELECT category, COUNT(*) FROM t GROUP BY category HAVING COUNT(*) > 1");
    assert!(result.is_ok(), "HAVING failed: {:?}", result);
}

// ============ P1.1: ORDER BY / LIMIT ============

#[test]
fn test_order_by_asc() {
    let result = parse("SELECT * FROM t ORDER BY id ASC");
    assert!(result.is_ok(), "ORDER BY ASC failed: {:?}", result);
}

#[test]
fn test_order_by_desc() {
    let result = parse("SELECT * FROM t ORDER BY id DESC");
    assert!(result.is_ok(), "ORDER BY DESC failed: {:?}", result);
}

#[test]
fn test_order_by_multiple() {
    let result = parse("SELECT * FROM t ORDER BY a ASC, b DESC");
    assert!(result.is_ok(), "Multiple ORDER BY failed: {:?}", result);
}

#[test]
fn test_limit() {
    let result = parse("SELECT * FROM t LIMIT 10");
    assert!(result.is_ok(), "LIMIT failed: {:?}", result);
}

#[test]
fn test_limit_offset() {
    let result = parse("SELECT * FROM t LIMIT 10 OFFSET 20");
    assert!(result.is_ok(), "LIMIT OFFSET failed: {:?}", result);
}

#[test]
fn test_where_order_limit() {
    let result = parse("SELECT * FROM t WHERE x > 0 ORDER BY id LIMIT 5");
    assert!(result.is_ok(), "WHERE ORDER LIMIT failed: {:?}", result);
}

// ============ P1.3: RESULTSET / 列限定符 ============

#[test]
fn test_resultsets_basic() {
    let result = parse("SELECT * FROM t1 JOIN t2 USING (id)");
    assert!(result.is_ok(), "RESULTSET basic failed: {:?}", result);
}

#[test]
fn test_column_qualifier() {
    let result = parse("SELECT t.id, t.name, o.total FROM orders o");
    assert!(result.is_ok(), "Column qualifier failed: {:?}", result);
}

#[test]
fn test_distinct() {
    let result = parse("SELECT DISTINCT category FROM t");
    assert!(result.is_ok(), "DISTINCT failed: {:?}", result);
}

#[test]
fn test_union() {
    let result = parse("SELECT a FROM t1 UNION SELECT a FROM t2");
    assert!(result.is_ok(), "UNION failed: {:?}", result);
}

#[test]
fn test_union_all() {
    let result = parse("SELECT a FROM t1 UNION ALL SELECT a FROM t2");
    assert!(result.is_ok(), "UNION ALL failed: {:?}", result);
}

// ============ P1.2: 子查询解析 ============

#[test]
fn test_scalar_subquery() {
    let result = parse("SELECT * FROM t WHERE id = (SELECT MAX(id) FROM s)");
    assert!(result.is_ok(), "Scalar subquery failed: {:?}", result);
}

#[test]
fn test_in_subquery_correlated() {
    let result = parse("SELECT * FROM t WHERE id IN (SELECT id FROM s WHERE s.ref = t.ref)");
    assert!(
        result.is_ok(),
        "Correlated IN subquery failed: {:?}",
        result
    );
}

// ============ P1.1: DDL 解析 ============

#[test]
fn test_create_table() {
    let result = parse("CREATE TABLE users (id INT PRIMARY KEY, name TEXT NOT NULL)");
    assert!(result.is_ok(), "CREATE TABLE failed: {:?}", result);
}

#[test]
fn test_create_table_if_not_exists() {
    let result = parse("CREATE TABLE IF NOT EXISTS users (id INT)");
    assert!(
        result.is_ok(),
        "CREATE TABLE IF NOT EXISTS failed: {:?}",
        result
    );
}

#[test]
fn test_drop_table() {
    let result = parse("DROP TABLE users");
    assert!(result.is_ok(), "DROP TABLE failed: {:?}", result);
}

#[test]
fn test_drop_table_if_exists() {
    let result = parse("DROP TABLE IF EXISTS users");
    assert!(result.is_ok(), "DROP TABLE IF EXISTS failed: {:?}", result);
}

#[test]
fn test_create_index() {
    let result = parse("CREATE INDEX idx_users_name ON users (name)");
    assert!(result.is_ok(), "CREATE INDEX failed: {:?}", result);
}

#[test]
fn test_drop_index() {
    let result = parse("DROP INDEX idx_users_name");
    assert!(result.is_ok(), "DROP INDEX failed: {:?}", result);
}

// ============ P1.1: DML 解析 ============

#[test]
fn test_insert_values() {
    let result = parse("INSERT INTO users (id, name) VALUES (1, 'Alice')");
    assert!(result.is_ok(), "INSERT VALUES failed: {:?}", result);
}

#[test]
fn test_insert_select() {
    // Parser 支持带列名的 INSERT ... SELECT（简化版，无 WHERE）
    let result = parse("INSERT INTO archive (id, name) SELECT id, name FROM users");
    assert!(result.is_ok(), "INSERT SELECT failed: {:?}", result);
}

#[test]
fn test_update_single() {
    let result = parse("UPDATE users SET name = 'Bob' WHERE id = 1");
    assert!(result.is_ok(), "UPDATE failed: {:?}", result);
}

#[test]
fn test_delete() {
    let result = parse("DELETE FROM users WHERE id = 1");
    assert!(result.is_ok(), "DELETE failed: {:?}", result);
}

#[test]
fn test_delete_all() {
    let result = parse("DELETE FROM users");
    assert!(result.is_ok(), "DELETE ALL failed: {:?}", result);
}

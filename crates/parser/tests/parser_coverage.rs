use sqlrustgo_parser::parse;
macro_rules! ok {
    ($sql:expr) => {
        assert!(parse($sql).is_ok(), "FAIL: {}", $sql);
    };
}

#[test]
fn t_sel_limit() {
    ok!("SELECT * FROM t LIMIT 10");
}
#[test]
fn t_sel_group_having() {
    ok!("SELECT a, COUNT(*) FROM t GROUP BY a");
}
#[test]
fn t_sel_order() {
    ok!("SELECT * FROM t ORDER BY a ASC, b DESC");
}
#[test]
fn t_sel_where_and_or() {
    ok!("SELECT * FROM t WHERE a = 1 AND b = 2 OR c = 3");
}
#[test]
fn t_sel_in_subq() {
    ok!("SELECT * FROM t WHERE id IN (SELECT id FROM s)");
}
#[test]
fn t_sel_exists() {
    ok!("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s)");
}
#[test]
fn t_sel_between() {
    ok!("SELECT * FROM t WHERE a BETWEEN 1 AND 10");
}
#[test]
fn t_sel_like() {
    ok!("SELECT * FROM t WHERE name LIKE '%x%'");
}
#[test]
fn t_sel_is_null() {
    ok!("SELECT * FROM t WHERE a IS NULL");
}
#[test]
fn t_sel_in_list() {
    ok!("SELECT * FROM t WHERE id IN (1, 2, 3)");
}
#[test]
fn t_sel_union() {
    ok!("SELECT a FROM t1 UNION ALL SELECT a FROM t2");
}
#[test]
fn t_sel_alias() {
    ok!("SELECT a AS col1, b AS col2 FROM t");
}
#[test]
fn t_sel_distinct() {
    ok!("SELECT DISTINCT name FROM t");
}
#[test]
fn t_sel_count() {
    ok!("SELECT COUNT(*) FROM t");
}
#[test]
fn t_join_inner() {
    ok!("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_join_left() {
    ok!("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_join_right() {
    ok!("SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_join_cross() {
    ok!("SELECT * FROM t1 CROSS JOIN t2");
}
#[test]
fn t_join_full() {
    ok!("SELECT * FROM t1 FULL JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_crt_tbl() {
    ok!("CREATE TABLE t (id INT)");
}
#[test]
fn t_crt_tbl_if() {
    ok!("CREATE TABLE IF NOT EXISTS t (id INT)");
}
#[test]
fn t_crt_tbl_pk() {
    ok!("CREATE TABLE t (id INT PRIMARY KEY)");
}
#[test]
fn t_crt_idx() {
    ok!("CREATE INDEX idx ON t (a)");
}
#[test]
fn t_drop_tbl() {
    ok!("DROP TABLE t");
}
#[test]
fn t_drop_idx() {
    ok!("DROP INDEX idx");
}
#[test]
fn t_alt_add() {
    ok!("ALTER TABLE t ADD COLUMN b INT");
}
#[test]
fn t_alt_rename() {
    ok!("ALTER TABLE t RENAME TO t2");
}
#[test]
fn t_del_where() {
    ok!("DELETE FROM t WHERE id = 5");
}
#[test]
fn t_upd_set() {
    ok!("UPDATE t SET name = 'x' WHERE id = 1");
}
#[test]
fn t_begin() {
    ok!("BEGIN");
}
#[test]
fn t_commit() {
    ok!("COMMIT");
}
#[test]
fn t_show() {
    ok!("SHOW TABLES");
}

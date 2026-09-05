// V313-99 / Issue #4692: writable CTE (PostgreSQL 11+, SQLite 3.33+)
// and MATERIALIZED VIEW (PostgreSQL) parser support.

use sqlrustgo_parser::parse;

#[test]
fn cte_with_delete_body_parses() {
    let sql = "WITH cte AS (DELETE FROM t WHERE val < 25 RETURNING *) \
         SELECT * FROM cte";
    assert!(
        parse(sql).is_ok(),
        "CTE with DELETE body must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn cte_with_update_body_parses() {
    let sql = "WITH cte AS (UPDATE t SET val = val * 2 WHERE id = 1 RETURNING *) \
         SELECT * FROM cte";
    assert!(
        parse(sql).is_ok(),
        "CTE with UPDATE body must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn cte_with_insert_body_parses() {
    let sql = "WITH moved AS (INSERT INTO archive SELECT * FROM t WHERE val < 25 RETURNING *) \
         SELECT count(*) FROM moved";
    assert!(
        parse(sql).is_ok(),
        "CTE with INSERT body must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn create_materialized_view_parses() {
    let sql = "CREATE MATERIALIZED VIEW mv AS SELECT * FROM t";
    assert!(
        parse(sql).is_ok(),
        "CREATE MATERIALIZED VIEW must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn create_regular_view_still_parses() {
    let sql = "CREATE VIEW v AS SELECT * FROM t";
    assert!(parse(sql).is_ok(), "plain CREATE VIEW must still parse");
}

#[test]
fn create_materialized_alone_does_not_eat_view() {
    // Bare `CREATE MATERIALIZED` (without VIEW) must NOT silently
    // become a CREATE TABLE — it should reject as an unknown form.
    let sql = "CREATE MATERIALIZED x";
    assert!(
        parse(sql).is_err(),
        "CREATE MATERIALIZED without VIEW must be rejected, got Ok"
    );
}

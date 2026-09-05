// V313-98 / Issue #4701: comprehensive subquery + aggregate + LIMIT + OFFSET
// inside SELECT previously failed with "Parse error: Expected FROM or
// column name" because the column-list loop had no arm for the opening
// `(` of a scalar subquery. The parser now dispatches `LParen` (and any
// other projection-starting token) to `parse_expression()`.

use sqlrustgo_parser::parse;

#[test]
fn insert_select_subquery_with_limit_offset_parses() {
    let sql = "INSERT INTO t \
         SELECT a.id, a.val, \
                (SELECT count(*) FROM b WHERE b.id=a.id) AS matched \
         FROM a \
         ORDER BY a.val DESC LIMIT 2 OFFSET 1";
    assert!(
        parse(sql).is_ok(),
        "issue #4701 example must parse: {:?}",
        parse(sql).err()
    );
}

#[test]
fn select_subquery_with_alias_parses() {
    let sql = "SELECT (SELECT count(*) FROM b WHERE b.id=a.id) AS matched FROM a";
    assert!(
        parse(sql).is_ok(),
        "scalar subquery AS alias: {:?}",
        parse(sql).err()
    );
}

#[test]
fn select_subquery_no_alias_parses() {
    let sql = "SELECT (SELECT 1) FROM a";
    assert!(
        parse(sql).is_ok(),
        "scalar subquery no alias: {:?}",
        parse(sql).err()
    );
}

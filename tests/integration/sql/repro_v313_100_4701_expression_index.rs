// V313-100 / Issue #4701 sub-1: expression index columns in CREATE INDEX.

use sqlrustgo_parser::parse;

#[test]
fn parse_create_index_with_expression() {
    let sql = "CREATE INDEX idx_case ON t(CASE WHEN price > 15 THEN 1 ELSE 0 END)";
    assert!(parse(sql).is_ok(), "expression index must parse: {:?}", parse(sql).err());
}

#[test]
fn parse_create_index_with_expression_and_simple_column() {
    let sql = "CREATE INDEX idx_mixed ON t(name, CASE WHEN price > 15 THEN 1 ELSE 0 END)";
    assert!(parse(sql).is_ok(), "mixed column+expression index must parse: {:?}", parse(sql).err());
}

#[test]
fn parse_create_index_with_simple_column_still_works() {
    let sql = "CREATE INDEX idx_simple ON t(name)";
    assert!(parse(sql).is_ok(), "plain column index must still parse");
}
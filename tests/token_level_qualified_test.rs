// Regression test for Issue #2987 (CTE-01, 10/10 self-reference follow-up):
// `oc.level` (table-qualified `Token::Level` column) must parse without
// `Expected column name, got Level`. The query is the seed+recursive step
// of a typical org-chart recursive CTE used in cte_self_reference corpus.
//
// Verifies the fix in `crates/parser/src/parser.rs` table.col path
// that accepts `Token::Level` as a column name.

use sqlrustgo_parser::parser::parse;

#[test]
fn test_token_level_qualified_column_in_join() {
    // SELECT with table-qualified `level` column joined across two tables.
    // The join ON clause references `oc.id` (table.col) and the SELECT
    // list references `oc.level` (table.col). Both must parse.
    let sql = "SELECT e.id, e.name, e.manager_id, oc.level + 1 \
               FROM employees e \
               JOIN org_chart oc ON e.manager_id = oc.id";
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Expected parse to succeed for `oc.level` qualified column, got: {:?}",
        result.err()
    );
}

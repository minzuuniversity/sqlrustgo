//! Quick unit test - check what compare_values returns for date strings

use sqlrustgo_types::Value;

#[test]
fn test_compare_date_strings() {
    let l = Value::Text("1992-01-01".to_string());
    let r = Value::Text("1994-01-01".to_string());

    let cmp = sqlrustgo::expr_utils::compare_values(&l, &r);
    eprintln!("1992-01-01 vs 1994-01-01: cmp = {}", cmp);
    eprintln!("  expected: < 0 (1992 < 1994)");
    eprintln!("  result: cmp = {}, < 0 = {}, >= 0 = {}", cmp, cmp < 0, cmp >= 0);
    eprintln!("  sql_compare(>=): {}", sqlrustgo::engine_utils::sql_compare(">=", &l, &r));
    eprintln!("  sql_compare(<): {}", sqlrustgo::engine_utils::sql_compare("<", &l, &r));
}

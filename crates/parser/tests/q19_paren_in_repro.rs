//! 复现 #5393 Q19:IN(...) 位于带括号的 OR 组 ⇒ wire 前端解析失败。
use sqlrustgo_parser::parse;

const Q19_SHAPE: &str = "SELECT l_orderkey, sum(l_extendedprice * (1 - l_discount)) AS revenue \
FROM lineitem WHERE ((p_brand = 'Brand#12' AND p_container IN ('SM CASE', 'SM BOX', 'SM PACK', 'SM PKG') \
AND l_quantity >= 1 AND l_quantity <= 1 + 10 AND p_size BETWEEN 1 AND 10) \
OR (p_brand = 'Brand#23' AND p_container IN ('MED BAG', 'MED BOX', 'MED PKG', 'MED PACK') \
AND l_quantity >= 10 AND p_size BETWEEN 10 AND 20))";

#[test]
fn q19_paren_or_group_with_in_list_parses() {
    let r = parse(Q19_SHAPE);
    assert!(r.is_ok(), "parse failed: {:?}", r.err());
}

#[test]
fn minimal_paren_or_in_parses() {
    let r = parse("SELECT * FROM t WHERE ((a = 1 AND b IN (1,2)) OR (c = 2 AND d IN (3,4)))");
    assert!(r.is_ok(), "parse failed: {:?}", r.err());
}

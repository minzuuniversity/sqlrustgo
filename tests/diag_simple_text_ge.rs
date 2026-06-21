//! Compare simple text filter to see if engine works for tiny dataset

use sqlrustgo::ExecutionEngine;

#[test]
fn test_simple_text_ge() {
    let mut engine = ExecutionEngine::with_memory();
    engine.execute("CREATE TABLE t (a TEXT)").expect("create");
    engine
        .execute(
            "INSERT INTO t VALUES ('1992-01-01'), ('1994-01-01'), ('1995-01-01'), ('1996-12-31')",
        )
        .expect("insert");

    // All rows
    let r = engine.execute("SELECT a FROM t").expect("all");
    eprintln!("all 4 rows: {:?}", r.rows);

    // a >= '1994-01-01' should return 3 rows (1994, 1995, 1996)
    let r2 = engine
        .execute("SELECT a FROM t WHERE a >= '1994-01-01'")
        .expect(">=");
    eprintln!("a >= '1994-01-01' (expect 3): {:?}", r2.rows);

    // a < '1995-01-01' should return 2 rows (1992, 1994)
    let r3 = engine
        .execute("SELECT a FROM t WHERE a < '1995-01-01'")
        .expect("<");
    eprintln!("a < '1995-01-01' (expect 2): {:?}", r3.rows);

    // a = '1994-01-01' should return 1
    let r4 = engine
        .execute("SELECT a FROM t WHERE a = '1994-01-01'")
        .expect("=");
    eprintln!("a = '1994-01-01' (expect 1): {:?}", r4.rows);
}

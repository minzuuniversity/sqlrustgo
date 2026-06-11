//! Operator-level regression test for date column comparison with text literal.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 5.
//!
//! Locks in the Q06/Q19 cell-diff pattern:
//! - WHERE date_col >= 'YYYY-MM-DD' (text comparison)
//! - WHERE date_col < 'YYYY-MM-DD' (text comparison)
//! TPC-H stores dates as TEXT, so string comparison must work.
//!
//! Acceptance (per Issue #3283):
//! - Use a minimal fixture (3-5 rows)
//! - Assert cell values, not just row count
//! - Run in < 100ms

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn date_greater_equal_text_literal() {
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderdate TEXT, o_total INTEGER)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1993-07-01', 100)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1993-09-15', 200)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1994-01-15', 300)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1994-08-02', 400)").unwrap();
    // Range: '1993-10-01' to '1994-01-01'
    let r = e.execute(
        "SELECT SUM(o_total) FROM orders \
         WHERE o_orderdate >= '1993-10-01' AND o_orderdate < '1994-01-01'",
    )
    .unwrap();
    // Only 1993-09-15 (200) and 1994-01-15 (300) — wait that's outside range
    // Actually: 1993-10-01 to 1993-12-31
    // Matches: 1994-01-15? No, that's >= 1994-01-01
    // Matches: 1993-09-15? No, that's < 1993-10-01
    // So 0 matches
    assert_eq!(r.rows[0][0].to_string(), "0");
}

#[test]
fn date_range_with_matches() {
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderdate TEXT, o_total INTEGER)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1993-07-01', 100)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1993-09-15', 200)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1994-01-15', 300)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1994-08-02', 400)").unwrap();
    // Range: '1993-07-01' to '1994-01-01' (Q6)
    let r = e.execute(
        "SELECT SUM(o_total) FROM orders \
         WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1994-01-01'",
    )
    .unwrap();
    // Matches: 1993-07-01 (100) + 1993-09-15 (200) = 300
    assert_eq!(r.rows[0][0].to_string(), "300");
}

#[test]
fn date_equality_count() {
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderdate TEXT)").unwrap();
    e.execute("INSERT INTO orders VALUES ('1995-03-15')").unwrap();
    e.execute("INSERT INTO orders VALUES ('1995-03-15')").unwrap();
    e.execute("INSERT INTO orders VALUES ('1995-04-01')").unwrap();
    let r = e.execute(
        "SELECT COUNT(*) FROM orders WHERE o_orderdate < '1995-03-15'",
    )
    .unwrap();
    assert_eq!(r.rows[0][0].to_string(), "0");
}

#[test]
fn date_iso_format_lexicographic_ordering() {
    // ISO date format YYYY-MM-DD sorts correctly as text.
    let mut e = engine();
    e.execute("CREATE TABLE d (d TEXT)").unwrap();
    e.execute("INSERT INTO d VALUES ('1992-01-01')").unwrap();
    e.execute("INSERT INTO d VALUES ('1992-12-31')").unwrap();
    e.execute("INSERT INTO d VALUES ('1993-01-01')").unwrap();
    e.execute("INSERT INTO d VALUES ('1995-03-15')").unwrap();
    // Order ASC: 1992-01-01, 1992-12-31, 1993-01-01, 1995-03-15
    let r = e.execute("SELECT d FROM d ORDER BY d").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "1992-01-01");
    assert_eq!(r.rows[1][0].to_string(), "1992-12-31");
    assert_eq!(r.rows[2][0].to_string(), "1993-01-01");
    assert_eq!(r.rows[3][0].to_string(), "1995-03-15");
}
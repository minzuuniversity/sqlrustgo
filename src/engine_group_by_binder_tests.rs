#![cfg(test)]

//! V4.1.0 — Issue #5462 / #5463: GROUP BY key name resolution.
//!
//! #5462 — `GROUP BY <select-list alias>` was executed as if the alias were a
//! **string constant**: the binder never resolved the alias to its defining
//! expression, `eval_identifier` fell back to `Value::Text(alias)`, every row
//! produced the same group key, and the whole input collapsed into ONE group.
//! No error was raised — a silent wrong answer.
//!
//! #5463 — `GROUP BY <unknown identifier>` was **not rejected by the binder**
//! (the SELECT list had that check, `GROUP BY` did not), so the invalid
//! statement reached the executor, where the row-width assertion panicked and
//! took the whole server down (every connection on it).
//!
//! Both defects share one root cause: the GROUP BY key list was never run
//! through a name-resolution step. The fix lives in one place —
//! `engine_utils::resolve_group_by_keys` — and both symptoms are covered here.
//!
//! The reference behaviour is SQLite / MySQL:
//!   * a GROUP BY key may name a SELECT-list output alias; it is grouped by
//!     the alias's defining expression;
//!   * a GROUP BY key that names neither a column nor an alias is a binder
//!     error (`column '...' not found in schema`).

use crate::execution_engine::ExecutionEngine;
use crate::execution_engine::MemoryExecutionEngine;
use crate::Value;
use parking_lot::RwLock;
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

/// Builds an engine with a small `lineitem`-shaped table.
///
/// Three `l_returnflag` values (A/N/R) and two ship years (1995/1996) — the
/// same shape the TPC-H fixture used by #5462 / #5463 has, small enough to
/// keep the test fast and the expectation readable.
fn engine_with_lineitem() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_returnflag TEXT, l_shipdate TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO lineitem VALUES \
             (1, 'A', '1995-01-01'), \
             (2, 'A', '1995-06-01'), \
             (3, 'N', '1996-01-01'), \
             (4, 'N', '1996-06-01'), \
             (5, 'R', '1996-06-01')",
        )
        .unwrap();
    engine
}

/// Renders result rows as `col,col` strings, sorted — the assertions must not
/// depend on HashMap iteration order.
fn sorted_rows(rows: &[Vec<Value>]) -> Vec<String> {
    let mut out: Vec<String> = rows
        .iter()
        .map(|r| {
            r.iter()
                .map(|v| format!("{v:?}"))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// #5462 — GROUP BY <alias> must group by the alias's defining expression.
// ---------------------------------------------------------------------------

#[test]
fn test_5462_group_by_alias_of_plain_column_groups_by_the_column() {
    let mut engine = engine_with_lineitem();

    // `lf` is a SELECT-list alias for `l_returnflag`. The engine must group
    // by `l_returnflag` (3 groups: A=2, N=2, R=1).
    //
    // Before the fix the alias was executed as the constant "lf", so all 5
    // rows landed in one group and the query returned exactly one row.
    let grouped = engine
        .execute("SELECT l_returnflag AS lf, COUNT(*) FROM lineitem GROUP BY lf")
        .expect("GROUP BY <alias of a plain column> must succeed");

    assert_eq!(
        grouped.rows.len(),
        3,
        "GROUP BY <alias> must produce one group per distinct underlying value, got {:?}",
        sorted_rows(&grouped.rows)
    );
    assert_eq!(
        sorted_rows(&grouped.rows),
        vec![
            "Text(\"A\"),Integer(2)",
            "Text(\"N\"),Integer(2)",
            "Text(\"R\"),Integer(1)",
        ],
        "each group must report the underlying column's value and its own count"
    );
}

#[test]
fn test_5462_group_by_alias_of_expression_groups_by_the_expression() {
    let mut engine = engine_with_lineitem();

    // `ly` is a SELECT-list alias for `EXTRACT(YEAR FROM l_shipdate)`.
    // Grouping by the alias must group by the expression (2 groups: 1995=2,
    // 1996=3).
    //
    // Before the fix the alias was executed as the constant "ly", so all 5
    // rows landed in one group.
    let grouped = engine
        .execute("SELECT EXTRACT(YEAR FROM l_shipdate) AS ly, COUNT(*) FROM lineitem GROUP BY ly")
        .expect("GROUP BY <alias of an expression> must succeed");

    assert_eq!(
        grouped.rows.len(),
        2,
        "GROUP BY <alias of an expression> must produce one group per distinct expression value, got {:?}",
        sorted_rows(&grouped.rows)
    );
    assert_eq!(
        sorted_rows(&grouped.rows),
        vec!["Integer(1995),Integer(2)", "Integer(1996),Integer(3)"],
        "each group must report the expression's value and its own count"
    );
}

#[test]
fn test_5462_group_by_real_column_is_unchanged() {
    let mut engine = engine_with_lineitem();

    // Control: the real column was already correct before the fix and must
    // stay correct after it.
    let grouped = engine
        .execute("SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag")
        .expect("GROUP BY <real column> must succeed");

    assert_eq!(
        sorted_rows(&grouped.rows),
        vec![
            "Text(\"A\"),Integer(2)",
            "Text(\"N\"),Integer(2)",
            "Text(\"R\"),Integer(1)",
        ],
        "GROUP BY <real column> must keep grouping by the column's value"
    );
}

// ---------------------------------------------------------------------------
// #5463 — GROUP BY <unknown identifier> must be a binder error, not a panic.
// ---------------------------------------------------------------------------

#[test]
fn test_5463_group_by_unknown_identifier_is_a_binder_error() {
    let mut engine = engine_with_lineitem();

    let err = engine
        .execute("SELECT COUNT(*) FROM lineitem GROUP BY zzz_bogus_col")
        .expect_err("GROUP BY <unknown column> must be rejected by the binder");

    let msg = err.to_string();
    assert!(
        msg.contains("Binder error: column 'zzz_bogus_col' not found in schema"),
        "the error must name the unresolved column, got: {msg}"
    );
}

#[test]
fn test_5463_group_by_unknown_identifier_does_not_break_later_statements() {
    let mut engine = engine_with_lineitem();

    // The server-panic symptom of #5463: after the bad statement, EVERY
    // statement on that connection failed with
    // "read packet header: failed to fill whole buffer".
    //
    // At the engine level the equivalent invariant is: a rejected GROUP BY
    // must not disturb the engine — the next statement must still run.
    assert!(
        engine
            .execute("SELECT COUNT(*) FROM lineitem GROUP BY zzz_bogus_col")
            .is_err(),
        "the bad statement must be rejected"
    );

    let after = engine
        .execute("SELECT COUNT(*) FROM lineitem")
        .expect("a rejected GROUP BY must not poison the engine");
    assert_eq!(
        after.rows.len(),
        1,
        "the statement after the rejected one must still return its result"
    );
    assert_eq!(after.rows[0][0], Value::Integer(5));
}

#[test]
fn test_5463_group_by_unknown_identifier_in_select_list_is_still_rejected() {
    let mut engine = engine_with_lineitem();

    // Control: the SELECT list already had the binder check (#4041). It must
    // keep rejecting the same unknown column — the GROUP BY fix must not
    // change the SELECT-list behaviour.
    let err = engine
        .execute("SELECT zzz_bogus_col, COUNT(*) FROM lineitem GROUP BY zzz_bogus_col")
        .expect_err("SELECT <unknown column> must be rejected by the binder");
    assert!(
        err.to_string()
            .contains("Binder error: column 'zzz_bogus_col' not found in schema"),
        "got: {err}"
    );
}

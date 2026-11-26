//! Issue #4669 sub-bugs 2 + 3 — the parser must not mis-read the two index
//! forms it used to swallow:
//!
//! - `CREATE INDEX i ON t(UPPER(a))` — the column list entry is a **function
//!   call**, not a column named "UPPER". Before the fix the parser took the
//!   identifier as a bare column, so the executor looked for a column
//!   literally named "UPPER" and failed with "Column not found".
//! - `CREATE INDEX i ON t(a) WHERE a > 200` — the **partial index** predicate
//!   must be parsed. Before the fix the WHERE was never consumed and the
//!   top-level parser tolerated the trailing tokens, so the predicate vanished
//!   and a full index was built silently.

use sqlrustgo_parser::parser::CreateIndexStatement;
use sqlrustgo_parser::{parse, Expression, Statement};

fn create_index(sql: &str) -> CreateIndexStatement {
    match parse(sql).expect("statement must parse") {
        Statement::CreateIndex(idx) => idx,
        other => panic!("expected CreateIndex, got {other:?}"),
    }
}

#[test]
fn function_index_column_is_an_expression_not_a_column() {
    let idx = create_index("CREATE INDEX idx_up ON t(UPPER(a))");
    assert_eq!(idx.columns.len(), 1);
    let spec = &idx.columns[0];
    assert!(
        spec.name.is_none(),
        "the function name must not be taken as a column name, got {:?}",
        spec.name
    );
    match spec.expression.as_ref() {
        Some(Expression::FunctionCall(name, args)) => {
            assert_eq!(name, "UPPER");
            assert_eq!(args.len(), 1, "UPPER takes one argument");
        }
        other => panic!("expected a FunctionCall expression, got {other:?}"),
    }
}

#[test]
fn plain_column_index_stays_a_column() {
    let idx = create_index("CREATE INDEX idx_a ON t(a)");
    assert_eq!(idx.columns.len(), 1);
    assert_eq!(idx.columns[0].name.as_deref(), Some("a"));
    assert!(
        idx.columns[0].expression.is_none(),
        "a bare column must not carry an expression"
    );
}

#[test]
fn partial_index_where_clause_is_parsed() {
    let idx = create_index("CREATE INDEX idx_p ON t(a) WHERE a > 200");
    assert_eq!(idx.columns.len(), 1);
    assert_eq!(idx.columns[0].name.as_deref(), Some("a"));
    assert!(
        idx.where_clause.is_some(),
        "the partial-index predicate must not be dropped"
    );
}

#[test]
fn index_without_where_has_no_predicate() {
    let idx = create_index("CREATE INDEX idx_a ON t(a)");
    assert!(
        idx.where_clause.is_none(),
        "a plain index must not carry a predicate"
    );
}

#[test]
fn multi_column_index_with_expression_keeps_both_forms() {
    let idx = create_index("CREATE INDEX idx_mix ON t(a, LOWER(b))");
    assert_eq!(idx.columns.len(), 2);
    assert_eq!(idx.columns[0].name.as_deref(), Some("a"));
    assert!(idx.columns[0].expression.is_none());
    assert!(
        idx.columns[1].name.is_none(),
        "the function entry must be an expression, got {:?}",
        idx.columns[1].name
    );
    assert!(idx.columns[1].expression.is_some());
}

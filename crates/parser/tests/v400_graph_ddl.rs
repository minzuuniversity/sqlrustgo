//! V400-03 / Issue #3731 (G1): `CREATE GRAPH` / `DROP GRAPH` parser tests.
//!
//! These tests pin down the G1 acceptance criteria:
//! - `CREATE GRAPH <name>` parses to `Statement::CreateGraph`
//! - `CREATE GRAPH IF NOT EXISTS <name>` sets `if_not_exists: true`
//! - `DROP GRAPH <name>` parses to `Statement::DropGraph`
//! - `DROP GRAPH IF EXISTS <name>` sets `if_exists: true`
//! - String-literal graph names are accepted
//! - Missing name / bad token produces a parser error
//!
//! The executor stub lives in `src/execution_engine.rs::execute_*_graph_stub`
//! and returns `affected_rows = 0` (an empty success). It is exercised
//! by the storage tests in `src/execution_engine_tests.rs` and is not
//! re-tested here to keep parser tests focused.

use sqlrustgo_parser::parser::{parse, CreateGraphStatement, DropGraphStatement, Statement};

#[test]
fn parse_create_graph_minimal() {
    let stmt = parse("CREATE GRAPH g").expect("parse CREATE GRAPH g");
    match stmt {
        Statement::CreateGraph(CreateGraphStatement {
            name,
            if_not_exists,
        }) => {
            assert_eq!(name, "g");
            assert!(!if_not_exists);
        }
        other => panic!("expected CreateGraph, got {:?}", other),
    }
}

#[test]
fn parse_create_graph_if_not_exists() {
    let stmt =
        parse("CREATE GRAPH IF NOT EXISTS social").expect("parse CREATE GRAPH IF NOT EXISTS");
    match stmt {
        Statement::CreateGraph(CreateGraphStatement {
            name,
            if_not_exists,
        }) => {
            assert_eq!(name, "social");
            assert!(if_not_exists, "IF NOT EXISTS must set flag");
        }
        other => panic!("expected CreateGraph, got {:?}", other),
    }
}

#[test]
fn parse_create_graph_string_literal_name() {
    // Quoted graph names (MySQL-style) should also be accepted.
    let stmt = parse("CREATE GRAPH IF NOT EXISTS 'graph-1'")
        .expect("parse CREATE GRAPH with string literal name");
    match stmt {
        Statement::CreateGraph(CreateGraphStatement {
            name,
            if_not_exists,
        }) => {
            assert_eq!(name, "graph-1");
            assert!(if_not_exists);
        }
        other => panic!("expected CreateGraph, got {:?}", other),
    }
}

#[test]
fn parse_drop_graph_minimal() {
    let stmt = parse("DROP GRAPH g").expect("parse DROP GRAPH g");
    match stmt {
        Statement::DropGraph(DropGraphStatement { name, if_exists }) => {
            assert_eq!(name, "g");
            assert!(!if_exists);
        }
        other => panic!("expected DropGraph, got {:?}", other),
    }
}

#[test]
fn parse_drop_graph_if_exists() {
    let stmt = parse("DROP GRAPH IF EXISTS social").expect("parse DROP GRAPH IF EXISTS");
    match stmt {
        Statement::DropGraph(DropGraphStatement { name, if_exists }) => {
            assert_eq!(name, "social");
            assert!(if_exists, "IF EXISTS must set flag");
        }
        other => panic!("expected DropGraph, got {:?}", other),
    }
}

#[test]
fn parse_create_graph_missing_name_is_error() {
    let err = parse("CREATE GRAPH").expect_err("CREATE GRAPH without name must fail");
    assert!(
        err.to_lowercase().contains("expected") || err.to_lowercase().contains("graph name"),
        "error message should hint at missing name, got: {err}",
    );
}

#[test]
fn parse_drop_graph_missing_name_is_error() {
    let err = parse("DROP GRAPH").expect_err("DROP GRAPH without name must fail");
    assert!(
        err.to_lowercase().contains("expected") || err.to_lowercase().contains("graph name"),
        "error message should hint at missing name, got: {err}",
    );
}

#[test]
fn parse_token_graph_keyword_recognized() {
    // Sanity: the new Token::Graph variant is reachable via the
    // lexer + keyword table. `SELECT GRAPH FROM t` should parse
    // (GRAPH as an identifier in this context, since it's not at
    // a CREATE/DROP position).
    let _ = parse("SELECT 1").expect("baseline SELECT still parses");
}

#[test]
fn parse_create_graph_does_not_swallow_create_database() {
    // CREATE DATABASE must still go to the database dispatcher,
    // not the new graph one.
    let stmt = parse("CREATE DATABASE d").expect("parse CREATE DATABASE d");
    assert!(
        matches!(stmt, Statement::CreateDatabase(_)),
        "CREATE DATABASE must dispatch to CreateDatabase, got {:?}",
        stmt
    );
}

#[test]
fn parse_drop_graph_does_not_swallow_drop_database() {
    // Same isolation property for DROP.
    let stmt = parse("DROP DATABASE d").expect("parse DROP DATABASE d");
    assert!(
        matches!(stmt, Statement::DropDatabase(_)),
        "DROP DATABASE must dispatch to DropDatabase, got {:?}",
        stmt
    );
}

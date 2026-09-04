//! Reproduction / regression tests for the v312-64a parser-only batch.
//!
//! Closes the parser side of:
//!   - #4651 — `CAST(x AS DATE)` returns parse error
//!   - #4662 — `CREATE TRIGGER … INSTEAD OF …` parse error
//!   - #4663 — `VACUUM` / `REINDEX` / `ANALYZE` (no-table sweep) parse error
//!   - #4655 — `DATE_TRUNC(...)` not registered as a function
//!   - #4665 — `OVER (… ROWS BETWEEN n PRECEDING AND m FOLLOWING)` parse error
//!
//! The executor paths for these features are NOT exercised here; this
//! file asserts that the SQL string is at least accepted by the parser
//! and the corresponding AST node is produced. Executor-side regression
//! tests live in their respective executor-level suites.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_parser::{
    parse, Expression, FrameBound, FrameClause, FrameMode, Statement, WindowSpecification,
};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// ============================================================================
// Issue #4651 — CAST(x AS DATE)
// ============================================================================

#[test]
fn repro_4651_cast_as_date_parses() {
    let stmt = parse("SELECT CAST('2024-01-15' AS DATE)").expect("CAST AS DATE must parse");
    match stmt {
        Statement::Select(s) => {
            // Reach into the SELECT projection columns and assert that
            // the CAST function call is captured with the new Token::Date
            // arm in its argument list.
            let col = s.columns.first().expect("at least one SELECT column");
            match &col.expression {
                Some(Expression::FunctionCall(name, args)) => {
                    assert_eq!(name.to_uppercase(), "CAST");
                    assert!(
                        args.iter()
                            .any(|a| matches!(a, Expression::Literal(s) if s == "DATE")),
                        "CAST AS DATE must carry a 'DATE' literal arg, got: {:?}",
                        args
                    );
                }
                other => panic!("expected FunctionCall CAST, got: {:?}", other),
            }
        }
        other => panic!("expected Statement::Select, got: {:?}", other),
    }
}

// ============================================================================
// Issue #4662 — CREATE TRIGGER … INSTEAD OF …
// ============================================================================

#[test]
fn repro_4662_instead_of_trigger_parses() {
    let stmt = parse(
        "CREATE TRIGGER inst_upd INSTEAD OF UPDATE ON v FOR EACH ROW \
         BEGIN UPDATE t SET val = NEW.val * 10 WHERE id = OLD.id; END",
    )
    .expect("INSTEAD OF trigger must parse");
    match stmt {
        Statement::CreateTrigger(c) => {
            assert_eq!(
                c.timing.to_uppercase(),
                "INSTEAD OF",
                "INSTEAD OF must round-trip as the canonical timing string"
            );
        }
        other => panic!("expected Statement::CreateTrigger, got: {:?}", other),
    }
}

#[test]
fn repro_4662_before_after_still_parse() {
    // Regression: the new INSTEAD OF arm must not have stolen the
    // BEFORE / AFTER keyword paths.
    for timing in &["BEFORE", "AFTER"] {
        let sql = format!(
            "CREATE TRIGGER tr {} INSERT ON t FOR EACH ROW BEGIN SELECT 1; END",
            timing
        );
        let stmt = parse(&sql).unwrap_or_else(|e| panic!("{} must parse: {}", timing, e));
        if let Statement::CreateTrigger(c) = stmt {
            assert_eq!(c.timing.to_uppercase(), *timing);
        } else {
            panic!("expected CreateTrigger");
        }
    }
}

// ============================================================================
// Issue #4663 — VACUUM / REINDEX / ANALYZE (no-table sweep)
// ============================================================================

#[test]
fn repro_4663_vacuum_no_table_parses() {
    let stmt = parse("VACUUM").expect("VACUUM must parse");
    match stmt {
        Statement::Vacuum(v) => {
            assert!(
                v.table_name.is_none(),
                "bare VACUUM should not bind a table"
            );
        }
        other => panic!("expected Statement::Vacuum, got: {:?}", other),
    }
}

#[test]
fn repro_4663_vacuum_with_table_parses() {
    let stmt = parse("VACUUM t").expect("VACUUM t must parse");
    if let Statement::Vacuum(v) = stmt {
        assert_eq!(v.table_name.as_deref(), Some("t"));
    } else {
        panic!("expected Vacuum");
    }
}

#[test]
fn repro_4663_reindex_no_table_parses() {
    let stmt = parse("REINDEX").expect("REINDEX must parse");
    match stmt {
        Statement::Reindex(r) => {
            assert!(r.table_name.is_none());
        }
        other => panic!("expected Statement::Reindex, got: {:?}", other),
    }
}

#[test]
fn repro_4663_reindex_with_table_parses() {
    let stmt = parse("REINDEX idx_t").expect("REINDEX idx_t must parse");
    if let Statement::Reindex(r) = stmt {
        assert_eq!(r.table_name.as_deref(), Some("idx_t"));
    } else {
        panic!("expected Reindex");
    }
}

#[test]
fn repro_4663_analyze_no_table_sweeps_all() {
    // The previous behavior was "expected table name"; the new behavior
    // accepts `ANALYZE` (no table) and returns the count of tables it
    // refreshed statistics for.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t1(id INT)").unwrap();
    x.execute("CREATE TABLE t2(id INT)").unwrap();
    let r = x
        .execute("ANALYZE")
        .expect("ANALYZE without table must run");
    assert_eq!(r.rows.len(), 1, "ANALYZE returns 1 summary row");
    match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => assert_eq!(*n, 2, "sweep should hit both tables"),
        other => panic!("expected Integer, got: {:?}", other),
    }
}

#[test]
fn repro_4663_analyze_with_table_still_works() {
    // Regression: the new no-table sweep must not have broken the
    // single-table form. The single-table form preserves the legacy
    // semantics where rows[0][0] is the analyzed table's row_count
    // (here: 0 because the table is empty).
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT)").unwrap();
    let r = x.execute("ANALYZE t").expect("ANALYZE t must run");
    assert_eq!(r.rows.len(), 1);
    if let sqlrustgo::Value::Integer(n) = &r.rows[0][0] {
        assert_eq!(*n, 0, "ANALYZE t (empty table) reports row_count");
    } else {
        panic!("expected Integer count");
    }
}

#[test]
fn repro_4663_analyze_with_table_returns_row_count() {
    // Cross-check: a non-empty table must report its actual row_count
    // (not the table count) so the legacy test_analyze_table_stats
    // semantics are preserved.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(id INT)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    x.execute("INSERT INTO t VALUES (3)").unwrap();
    let r = x.execute("ANALYZE t").expect("ANALYZE t must run");
    if let sqlrustgo::Value::Integer(n) = &r.rows[0][0] {
        assert_eq!(*n, 3, "ANALYZE t (3 rows) must report row_count=3");
    } else {
        panic!("expected Integer count");
    }
}

// ============================================================================
// Issue #4655 — DATE_TRUNC function registration
// ============================================================================

#[test]
fn repro_4655_date_trunc_year_runs() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-07-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('YEAR', ts) FROM t")
        .expect("DATE_TRUNC must execute");
    assert_eq!(r.rows.len(), 1);
    // The DATE_TRUNC helper returns the calendar-aligned date prefix in
    // `YYYY-MM-DD` form for YEAR/QUARTER/MONTH/DAY units.
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2024-01-01\")");
}

#[test]
fn repro_4655_date_trunc_month_runs() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-07-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('MONTH', ts) FROM t")
        .expect("DATE_TRUNC MONTH must execute");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2024-07-01\")");
}

#[test]
fn repro_4655_date_trunc_quarter_runs() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-08-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('QUARTER', ts) FROM t")
        .expect("DATE_TRUNC QUARTER must execute");
    assert_eq!(r.rows.len(), 1);
    // 2024-08 is Q3 → 2024-07-01
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2024-07-01\")");
}

#[test]
fn repro_4655_date_trunc_day_runs() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-07-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('DAY', ts) FROM t")
        .expect("DATE_TRUNC DAY must execute");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2024-07-15\")");
}

#[test]
fn repro_4655_date_trunc_unknown_unit_returns_null() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-07-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('UNKNOWN', ts) FROM t")
        .expect("DATE_TRUNC must not error on unknown unit");
    assert_eq!(r.rows.len(), 1);
    assert!(
        matches!(r.rows[0][0], sqlrustgo::Value::Null),
        "unknown unit must return NULL, got: {:?}",
        r.rows[0][0]
    );
}

#[test]
fn repro_4765_date_trunc_week_returns_monday() {
    // Issue #4765 — DATE_TRUNC('WEEK', ...) returns NULL. PostgreSQL semantics:
    // the Monday of the ISO week containing the input date.
    // 2024-07-15 is a Monday → unchanged.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-07-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('WEEK', ts) FROM t")
        .expect("DATE_TRUNC WEEK must execute");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2024-07-15\")");
}

#[test]
fn repro_4765_date_trunc_week_thursday_returns_prior_monday() {
    // 2026-01-15 is a Thursday → Monday of that ISO week is 2026-01-12.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2026-01-15 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('WEEK', ts) FROM t")
        .expect("DATE_TRUNC WEEK must execute");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2026-01-12\")");
}

#[test]
fn repro_4765_date_trunc_week_sunday_returns_prior_monday() {
    // 2024-07-21 is a Sunday → Monday of that ISO week is 2024-07-15.
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(ts TEXT)").unwrap();
    x.execute("INSERT INTO t VALUES ('2024-07-21 13:45:30')")
        .unwrap();
    let r = x
        .execute("SELECT DATE_TRUNC('WEEK', ts) FROM t")
        .expect("DATE_TRUNC WEEK must execute");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(format!("{:?}", r.rows[0][0]), "Text(\"2024-07-15\")");
}

// ============================================================================
// Issue #4665 — ROWS BETWEEN frame clause
// ============================================================================

#[test]
fn repro_4665_rows_between_parses() {
    let stmt = parse(
        "SELECT id, SUM(val) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS r FROM t",
    )
    .expect("ROWS BETWEEN must parse");
    if let Statement::Select(s) = stmt {
        // Locate the SUM(...) OVER (...) WindowCall expression.
        let col = s
            .columns
            .iter()
            .find(|c| matches!(&c.expression, Some(Expression::WindowCall(_))))
            .expect("at least one WindowCall column");
        if let Some(Expression::WindowCall(wc)) = &col.expression {
            let frame = wc
                .window_spec
                .frame
                .as_ref()
                .expect("WindowSpecification.frame must be populated");
            assert!(matches!(frame.mode, FrameMode::Rows));
            assert!(matches!(frame.start, FrameBound::Preceding(_)));
            assert!(matches!(frame.end, FrameBound::Following(_)));
            assert!(
                !wc.window_spec.order_by.is_empty(),
                "ORDER BY id must have been parsed too"
            );
        } else {
            panic!("expected WindowCall expression");
        }
    } else {
        panic!("expected SELECT");
    }
}

#[test]
fn repro_4665_range_between_unbounded_parses() {
    let stmt =
        parse("SELECT SUM(v) OVER (ORDER BY x RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)")
            .expect("RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW must parse");
    if let Statement::Select(s) = stmt {
        let col = &s.columns[0];
        if let Some(Expression::WindowCall(wc)) = &col.expression {
            let frame = wc.window_spec.frame.as_ref().unwrap();
            assert!(matches!(frame.mode, FrameMode::Range));
            assert!(matches!(frame.start, FrameBound::UnboundedPreceding));
            assert!(matches!(frame.end, FrameBound::CurrentRow));
        } else {
            panic!("expected WindowCall");
        }
    }
}

#[test]
fn repro_4665_no_frame_backward_compatible() {
    // Regression: omitting the frame clause must still parse and
    // produce WindowSpecification.frame = None so the executor falls
    // back to the legacy UNBOUNDED PRECEDING .. CURRENT ROW behavior.
    let stmt = parse("SELECT SUM(v) OVER (PARTITION BY g ORDER BY x)").unwrap();
    if let Statement::Select(s) = stmt {
        let col = &s.columns[0];
        if let Some(Expression::WindowCall(wc)) = &col.expression {
            assert!(wc.window_spec.frame.is_none());
            assert_eq!(wc.window_spec.partition_by.len(), 1);
            assert_eq!(wc.window_spec.order_by.len(), 1);
        } else {
            panic!("expected WindowCall");
        }
    }
}

#[test]
fn repro_4665_window_spec_struct_has_frame_field() {
    // Compile-time check: WindowSpecification carries a frame field.
    // The compiler will fail this test if the field is removed.
    let ws = WindowSpecification {
        partition_by: vec![],
        order_by: vec![],
        frame: None,
        frame_exclusion: None,
    };
    let _ = ws;
    let _clause: Option<FrameClause> = None;
}

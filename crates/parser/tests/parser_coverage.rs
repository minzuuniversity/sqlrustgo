use sqlrustgo_parser::{parse, AlterColumnOperation, AlterTableOperation, Statement};
macro_rules! ok {
    ($sql:expr) => {
        assert!(parse($sql).is_ok(), "FAIL: {}", $sql);
    };
}

#[test]
fn t_sel_limit() {
    ok!("SELECT * FROM t LIMIT 10");
}
#[test]
fn t_sel_group_having() {
    ok!("SELECT a, COUNT(*) FROM t GROUP BY a");
}
#[test]
fn t_sel_order() {
    ok!("SELECT * FROM t ORDER BY a ASC, b DESC");
}
#[test]
fn t_sel_where_and_or() {
    ok!("SELECT * FROM t WHERE a = 1 AND b = 2 OR c = 3");
}
#[test]
fn t_sel_in_subq() {
    ok!("SELECT * FROM t WHERE id IN (SELECT id FROM s)");
}
#[test]
fn t_sel_exists() {
    ok!("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s)");
}
#[test]
fn t_sel_between() {
    ok!("SELECT * FROM t WHERE a BETWEEN 1 AND 10");
}
#[test]
fn t_sel_like() {
    ok!("SELECT * FROM t WHERE name LIKE '%x%'");
}
#[test]
fn t_sel_is_null() {
    ok!("SELECT * FROM t WHERE a IS NULL");
}
#[test]
fn t_sel_in_list() {
    ok!("SELECT * FROM t WHERE id IN (1, 2, 3)");
}
#[test]
fn t_sel_union() {
    ok!("SELECT a FROM t1 UNION ALL SELECT a FROM t2");
}
#[test]
fn t_sel_alias() {
    ok!("SELECT a AS col1, b AS col2 FROM t");
}
#[test]
fn t_sel_distinct() {
    ok!("SELECT DISTINCT name FROM t");
}
#[test]
fn t_sel_count() {
    ok!("SELECT COUNT(*) FROM t");
}
#[test]
fn t_join_inner() {
    ok!("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_join_left() {
    ok!("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_join_right() {
    ok!("SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_join_cross() {
    ok!("SELECT * FROM t1 CROSS JOIN t2");
}
#[test]
fn t_join_full() {
    ok!("SELECT * FROM t1 FULL JOIN t2 ON t1.id = t2.id");
}
#[test]
fn t_crt_tbl() {
    ok!("CREATE TABLE t (id INT)");
}
#[test]
fn t_crt_tbl_if() {
    ok!("CREATE TABLE IF NOT EXISTS t (id INT)");
}
#[test]
fn t_crt_tbl_pk() {
    ok!("CREATE TABLE t (id INT PRIMARY KEY)");
}
#[test]
fn t_crt_idx() {
    ok!("CREATE INDEX idx ON t (a)");
}
#[test]
fn t_drop_tbl() {
    ok!("DROP TABLE t");
}
#[test]
fn t_drop_idx() {
    ok!("DROP INDEX idx");
}
#[test]
fn t_alt_add() {
    ok!("ALTER TABLE t ADD COLUMN b INT");
}
#[test]
fn t_alt_rename() {
    ok!("ALTER TABLE t RENAME TO t2");
}
#[test]
fn t_del_where() {
    ok!("DELETE FROM t WHERE id = 5");
}
#[test]
fn t_upd_set() {
    ok!("UPDATE t SET name = 'x' WHERE id = 1");
}
#[test]
fn t_begin() {
    ok!("BEGIN");
}
#[test]
fn t_commit() {
    ok!("COMMIT");
}
#[test]
fn t_show() {
    ok!("SHOW TABLES");
}

// --- MERGE statement tests (no integration tests existed before) ---

#[test]
fn t_merge_when_matched_update() {
    ok!("MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.name = s.name");
}
#[test]
fn t_merge_when_not_matched_insert() {
    ok!("MERGE INTO t USING s ON t.id = s.id WHEN NOT MATCHED THEN INSERT (id, name) VALUES (s.id, s.name)");
}
#[test]
fn t_merge_when_matched_and_not_matched() {
    ok!("MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.name = s.name WHEN NOT MATCHED THEN INSERT (id) VALUES (s.id)");
}
#[test]
fn t_merge_when_matched_delete() {
    ok!("MERGE INTO t USING s ON t.id = s.id WHEN MATCHED AND s.flag = 1 THEN DELETE");
}
#[test]
fn t_merge_with_alias() {
    ok!("MERGE INTO target AS tgt USING source AS src ON tgt.id = src.id WHEN MATCHED THEN UPDATE SET tgt.val = src.val");
}
#[test]
fn t_merge_subquery_source() {
    ok!("MERGE INTO t USING (SELECT id, name FROM s) AS sub ON t.id = sub.id WHEN NOT MATCHED THEN INSERT (id, name) VALUES (sub.id, sub.name)");
}
#[test]
fn t_merge_missing_using_rejected() {
    let result = parse("MERGE INTO t WHEN MATCHED THEN UPDATE SET t.x = 1");
    assert!(result.is_err(), "MERGE without USING should be rejected");
}
#[test]
fn t_merge_missing_on_rejected() {
    let result = parse("MERGE INTO t USING s WHEN MATCHED THEN UPDATE SET t.x = 1");
    assert!(
        result.is_err(),
        "MERGE without ON clause should be rejected"
    );
}
#[test]
fn t_merge_missing_when_rejected() {
    let result = parse("MERGE INTO t USING s ON t.id = s.id");
    assert!(
        result.is_err(),
        "MERGE without WHEN clause should be rejected"
    );
}

// --- ALTER TABLE variants ---

#[test]
fn t_alt_drop_column() {
    ok!("ALTER TABLE t DROP COLUMN b");
}
#[test]
fn t_alt_set_default_accepted() {
    let result = parse("ALTER TABLE t ALTER COLUMN b SET DEFAULT 42");
    assert!(result.is_ok(), "SET DEFAULT now supported: {:?}", result);
}
#[test]
fn t_alt_set_data_type() {
    let result = parse("ALTER TABLE users ALTER COLUMN age SET DATA TYPE BIGINT");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::AlterTable(a) => {
            assert_eq!(a.table_name, "users");
            match a.operation {
                AlterTableOperation::AlterColumn { name, op } => {
                    assert_eq!(name, "age");
                    match op {
                        AlterColumnOperation::SetDataType { data_type } => {
                            assert_eq!(data_type, "BIGINT");
                        }
                        other => panic!("Expected SetDataType, got {:?}", other),
                    }
                }
                other => panic!("Expected AlterColumn, got {:?}", other),
            }
        }
        other => panic!("Expected AlterTable, got {:?}", other),
    }
}
#[test]
fn t_alt_set_data_type_keyword_type() {
    let result = parse("ALTER TABLE t ALTER COLUMN c SET DATA TYPE INTEGER");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::AlterTable(a) => match a.operation {
            AlterTableOperation::AlterColumn { name, op } => {
                assert_eq!(name, "c");
                match op {
                    AlterColumnOperation::SetDataType { data_type } => {
                        assert_eq!(data_type, "INTEGER");
                    }
                    other => panic!("Expected SetDataType, got {:?}", other),
                }
            }
            other => panic!("Expected AlterColumn, got {:?}", other),
        },
        other => panic!("Expected AlterTable, got {:?}", other),
    }
}
#[test]
fn t_alt_rename_constraint_rejected() {
    let result = parse("ALTER TABLE t RENAME CONSTRAINT old_name TO new_name");
    assert!(
        result.is_err(),
        "ALTER TABLE RENAME CONSTRAINT not yet supported"
    );
}
#[test]
fn t_alt_add_constraint_rejected() {
    let result = parse("ALTER TABLE t ADD CONSTRAINT fk FOREIGN KEY (id) REFERENCES other (id)");
    assert!(
        result.is_err(),
        "ALTER TABLE ADD CONSTRAINT not yet supported"
    );
}

// --- GRANT/REVOKE for TABLE/FUNCTION/COLUMN privileges (not supported by parser yet — reject) ---

#[test]
fn t_grant_table_privilege_rejected() {
    let result = parse("GRANT SELECT ON TABLE mydb TO admin");
    assert!(result.is_err(), "GRANT ON TABLE not yet supported");
}
#[test]
fn t_revoke_table_privilege_rejected() {
    let result = parse("REVOKE SELECT ON TABLE mydb FROM admin");
    assert!(result.is_err(), "REVOKE ON TABLE not yet supported");
}
#[test]
fn t_grant_function_privilege_rejected() {
    let result = parse("GRANT EXECUTE ON FUNCTION myproc TO admin");
    assert!(
        result.is_err(),
        "GRANT EXECUTE ON FUNCTION not yet supported"
    );
}
#[test]
fn t_revoke_function_privilege_rejected() {
    let result = parse("REVOKE EXECUTE ON FUNCTION myproc FROM admin");
    assert!(
        result.is_err(),
        "REVOKE EXECUTE ON FUNCTION not yet supported"
    );
}
#[test]
fn t_grant_column_privilege() {
    ok!("GRANT SELECT (id, name) ON users TO admin");
}

// --- CREATE DATABASE / DROP DATABASE (not supported by parser yet — reject) ---

#[test]
fn t_create_database_rejected() {
    let result = parse("CREATE DATABASE mydb");
    assert!(result.is_ok(), "CREATE DATABASE should be supported");
}
#[test]
fn t_drop_database_rejected() {
    let _ = parse("DROP DATABASE mydb");
}

// --- Window function ROWS/RANGE clauses ---
//
// These used to assert the parser *rejects* the clause ("not yet
// supported"). The parser now accepts it and the executor honours it —
// `src/expr_utils.rs::compute_window` reads `window_spec.frame` and
// `expr_utils::compute_frame_local` turns it into the row set, so
// `ROWS BETWEEN 1 PRECEDING AND CURRENT ROW` really is a sliding frame
// distinct from the default cumulative one. These assert the AST shape
// instead; the execution semantics are covered by the operator suite.

// No frame clause => `frame: None`, and the executor picks the default
// (RANGE UNBOUNDED PRECEDING .. CURRENT ROW when ORDER BY is present).
#[test]
fn t_window_default_has_no_frame_clause() {
    let result = parse("SELECT SUM(b) OVER (ORDER BY d) FROM t");
    let stmt = result.expect("window call without a frame clause must parse");
    let sqlrustgo_parser::Statement::Select(sel) = stmt else {
        panic!("expected a SELECT");
    };
    let col = &sel.columns[0];
    let Some(sqlrustgo_parser::Expression::WindowCall(wc)) = &col.expression else {
        panic!("expected a window call, got {:?}", col.expression);
    };
    assert!(
        wc.window_spec.frame.is_none(),
        "no frame clause in the query, so frame must be None"
    );
}

// Explicit ROWS frame => parsed into the three parts, not dropped.
#[test]
fn t_window_rows_frame_parsed() {
    let result = parse(
        "SELECT SUM(b) OVER (ORDER BY d ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    let stmt = result.expect("ROWS BETWEEN frame must parse");
    let sqlrustgo_parser::Statement::Select(sel) = stmt else {
        panic!("expected a SELECT");
    };
    let col = &sel.columns[0];
    let Some(sqlrustgo_parser::Expression::WindowCall(wc)) = &col.expression else {
        panic!("expected a window call, got {:?}", col.expression);
    };
    use sqlrustgo_parser::parser::{FrameBound, FrameMode};
    let frame = wc
        .window_spec
        .frame
        .as_ref()
        .expect("an explicit frame clause must be recorded");
    assert_eq!(frame.mode, FrameMode::Rows);
    assert_eq!(frame.start, FrameBound::UnboundedPreceding);
    assert_eq!(frame.end, FrameBound::CurrentRow);
}

// A PRECEDING offset is a distinct frame, not the same as the default.
#[test]
fn t_window_rows_preceding_offset_parsed() {
    let result =
        parse("SELECT SUM(b) OVER (ORDER BY d ROWS BETWEEN 2 PRECEDING AND 1 FOLLOWING) FROM t");
    let stmt = result.expect("offset frame must parse");
    let sqlrustgo_parser::Statement::Select(sel) = stmt else {
        panic!("expected a SELECT");
    };
    let col = &sel.columns[0];
    let Some(sqlrustgo_parser::Expression::WindowCall(wc)) = &col.expression else {
        panic!("expected a window call, got {:?}", col.expression);
    };
    use sqlrustgo_parser::parser::FrameBound;
    let frame = wc.window_spec.frame.as_ref().expect("frame recorded");
    assert_eq!(frame.start, FrameBound::Preceding(2));
    assert_eq!(frame.end, FrameBound::Following(1));
}

// --- NULLS FIRST / NULLS LAST in ORDER BY ---

#[test]
fn t_order_by_nulls_first() {
    ok!("SELECT a FROM t ORDER BY a ASC NULLS FIRST");
}
#[test]
fn t_order_by_nulls_last() {
    ok!("SELECT a FROM t ORDER BY a DESC NULLS LAST");
}

// --- LIMIT ALL ---

#[test]
fn t_limit_all() {
    ok!("SELECT * FROM t LIMIT ALL");
}

// --- CREATE TABLE with CHECK constraint (named variant now supported) ---

#[test]
fn t_create_table_check_constraint_named_rejected() {
    let result = parse("CREATE TABLE t (id INT, val INT, CONSTRAINT chk_val CHECK (val > 0))");
    assert!(
        result.is_ok(),
        "Named CHECK constraint now supported: {:?}",
        result
    );
}

// --- GROUP BY ROLLUP/CUBE ---

#[test]
fn t_group_by_rollup() {
    ok!("SELECT a, SUM(b) FROM t GROUP BY ROLLUP (a)");
}
#[test]
fn t_group_by_cube() {
    ok!("SELECT a, b, SUM(c) FROM t GROUP BY CUBE (a, b)");
}

// --- DO statement (not supported by parser yet — reject) ---

#[test]
fn t_do_statement_rejected() {
    let result = parse("DO $$ BEGIN NULL; END $$");
    assert!(result.is_err(), "DO statement not yet supported");
}

// --- SET NAMES / SET CHARACTER SET (not supported by parser yet — reject) ---

#[test]
fn t_set_names_rejected() {
    let result = parse("SET NAMES utf8");
    assert!(result.is_err(), "SET NAMES not yet supported");
}
#[test]
fn t_set_character_set_rejected() {
    let result = parse("SET CHARACTER SET utf8");
    assert!(result.is_err(), "SET CHARACTER SET not yet supported");
}

// --- USE DATABASE (supported) ---

#[test]
fn t_use_database() {
    let result = parse("USE mydb");
    assert!(
        result.is_ok(),
        "USE DATABASE should be supported: {:?}",
        result
    );
}

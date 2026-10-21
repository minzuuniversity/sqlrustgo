//! #5406：差分对照语料（36 例）—— 曾经是「假绿」
//!
//! ## 曾经的现象
//!
//! ```
//! Running tests/integration/compat/differential_corpus_test.rs
//! test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
//!              finished in 0.00s
//! ```
//!
//! **`finished in 0.00s` 是关键**：36 个用例「通过」，耗时 0.00 秒 ——
//! **一次差分比对都没执行**。
//!
//! 成因：两侧都用**外部二进制**做对照，路径是硬编码的 macOS 路径：
//!
//! ```rust
//! sqlrustgo_bin: std::env::var("SQLRUSTGO_BIN")
//!     .unwrap_or_else(|_| "<某人的家目录>/dev/sqlrustgo/target/debug/sqlrustgo".to_string()),
//! ```
//!
//! CI 的 runner 是 Linux 容器 ⇒ 该路径不存在 ⇒ 引擎起不来 ⇒
//! [`run_sql`] 返回 `None` ⇒ 测试走
//!
//! ```rust
//! None => { println!("  SKIP: {} (sqlrustgo not available)", test_name); return; }
//! ```
//!
//! ⇒ **报告「通过」，但对引擎行为零验证**。与 #5379 要消灭的
//! 「fixture 悬空 → 目标跑空表 → 断言是空的」是同一类缺陷，
//! 只是断口从「数据」换成了「引擎二进制」。
//!
//! ## 现在的做法
//!
//! 两侧都改成**进程内**，于是这个文件**没有任何外部依赖**：
//!
//! * 引擎侧：`ExecutionEngine<MemoryStorage>`（root 包的公开 API）；
//! * oracle 侧：`rusqlite`（root 包的依赖，`bundled` feature ⇒ SQLite 的
//!   C 源码随仓库编译，CI 不需要额外二进制、不需要网络）；
//! * SQLite 独有的 `MOD` / `CHAR_LENGTH` 由共享的
//!   [`sqlite_dialect`](../../common/sqlite_dialect.rs) 做**显式**改写。
//!
//! ⇒ 没有硬编码路径，没有「环境缺失就当通过」的降级路径，
//! 也就**不可能假绿**。
//!
//! 语料本身（36 条 SQL）未做任何改动。

#[path = "../../common/sqlite_dialect.rs"]
mod sqlite_dialect;

use rusqlite::types::ValueRef;
use rusqlite::Connection;
use sqlite_dialect::{adapt_for_sqlite, cells_equal, format_real, NULL_CELL};
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};

/// 把脚本切成一条条语句。
fn split(script: &str) -> Vec<String> {
    sqlrustgo_parser::split_sql_statements(script)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 引擎侧：跑完整个脚本，取**最后一条语句**的结果行。
fn engine_rows(script: &str) -> Result<Vec<Vec<String>>, String> {
    let storage = std::sync::Arc::new(parking_lot::RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    let stmts = split(script);
    let Some((last, setup)) = stmts.split_last() else {
        return Err("脚本里没有语句".to_string());
    };
    for stmt in setup {
        engine
            .execute(stmt)
            .map_err(|e| format!("engine 执行 `{}` 失败: {}", stmt, e))?;
    }
    let res = engine
        .execute(last)
        .map_err(|e| format!("engine 执行 `{}` 失败: {}", last, e))?;
    Ok(res
        .rows
        .iter()
        .map(|row| row.iter().map(engine_cell).collect())
        .collect())
}

/// SQLite 侧：跑完整个脚本，取**最后一条语句**的结果行。
fn sqlite_rows(script: &str) -> Result<Vec<Vec<String>>, String> {
    let conn = Connection::open_in_memory().map_err(|e| format!("打开内存 SQLite 失败: {}", e))?;

    let stmts = split(script);
    let Some((last, setup)) = stmts.split_last() else {
        return Err("脚本里没有语句".to_string());
    };
    let setup_sql = setup.join(";\n");
    if !setup_sql.trim().is_empty() {
        let (sqlite_setup, notes) = adapt_for_sqlite(&setup_sql);
        for note in &notes {
            eprintln!("[SQLite 方言改写] {}", note);
        }
        conn.execute_batch(&sqlite_setup)
            .map_err(|e| format!("SQLite 执行准备语句失败: {}", e))?;
    }

    // oracle 侧的方言改写 —— 改写内容必须可见。
    let (sqlite_last, notes) = adapt_for_sqlite(last);
    for note in &notes {
        eprintln!("[SQLite 方言改写] {}", note);
    }

    let mut stmt = conn
        .prepare(&sqlite_last)
        .map_err(|e| format!("SQLite 准备 `{}` 失败: {}", sqlite_last, e))?;
    let ncols = stmt.column_count();
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("SQLite 执行 `{}` 失败: {}", sqlite_last, e))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("读行失败: {}", e))? {
        let mut cells = Vec::new();
        for idx in 0..ncols {
            let v = row
                .get_ref(idx)
                .map_err(|e| format!("读第 {} 列失败: {}", idx, e))?;
            cells.push(sqlite_cell(v));
        }
        out.push(cells);
    }
    Ok(out)
}

/// 把引擎的 `Value` 渲染成可比较的字符串。
fn engine_cell(v: &Value) -> String {
    match v {
        Value::Null => NULL_CELL.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format_real(*f),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => b.iter().map(|x| format!("{:02x}", x)).collect(),
        Value::Point(x, y) => format!("({},{})", x, y),
        Value::Json(j) => j.to_string(),
    }
}

/// 把 SQLite 的单元格渲染成可比较的字符串。
fn sqlite_cell(v: ValueRef<'_>) -> String {
    match v {
        ValueRef::Null => NULL_CELL.to_string(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => format_real(f),
        ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
        ValueRef::Blob(b) => b.iter().map(|x| format!("{:02x}", x)).collect(),
    }
}

/// 断言两侧结果一致，不一致时把第一处差异打印出来。
fn assert_match(test_name: &str, script: &str) {
    let ours = engine_rows(script).unwrap_or_else(|e| panic!("{}: {}", test_name, e));
    let theirs = sqlite_rows(script).unwrap_or_else(|e| panic!("{}: {}", test_name, e));

    if ours.len() != theirs.len() {
        panic!(
            "差分测试失败: {}\n  SQL: {}\n  引擎 {} 行，SQLite {} 行\n  引擎: {:?}\n  SQLite: {:?}",
            test_name,
            script,
            ours.len(),
            theirs.len(),
            ours,
            theirs
        );
    }
    for (i, (o, t)) in ours.iter().zip(theirs.iter()).enumerate() {
        if o.len() != t.len() {
            panic!(
                "差分测试失败: {}\n  SQL: {}\n  第 {} 行列数不同: 引擎={} SQLite={}\n  引擎: {:?}\n  SQLite: {:?}",
                test_name, script, i, o.len(), t.len(), ours, theirs
            );
        }
        for (j, (oc, tc)) in o.iter().zip(t.iter()).enumerate() {
            if !cells_equal(oc, tc) {
                panic!(
                    "差分测试失败: {}\n  SQL: {}\n  第 {} 行第 {} 列不同: 引擎={:?} SQLite={:?}\n  引擎: {:?}\n  SQLite: {:?}",
                    test_name, script, i, j, oc, tc, ours, theirs
                );
            }
        }
    }
}

#[test]
fn test_p3_math_001_mod_integer() {
    assert_match(
        "P3-MATH-001: MOD returns Integer",
        "CREATE TABLE t(a INT, b INT); INSERT INTO t VALUES (10, 3); SELECT MOD(a, b) FROM t;",
    );
}

#[test]
fn test_p3_agg_002_count_sum() {
    assert_match(
        "P3-AGG-002: COUNT vs SUM",
        "CREATE TABLE t(val INT); INSERT INTO t VALUES (1), (2), (3), (NULL); SELECT COUNT(val), SUM(val) FROM t;",
    );
}

#[test]
fn test_p3_set_001_union_all() {
    assert_match(
        "P3-SET-001: UNION ALL",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT * FROM t1 UNION ALL SELECT * FROM t2 ORDER BY id;",
    );
}

#[test]
fn test_p3_set_002_union_distinct() {
    assert_match(
        "P3-SET-002: UNION DISTINCT",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT * FROM t1 UNION SELECT * FROM t2 ORDER BY id;",
    );
}

#[test]
fn test_p3_null_001_null_comparison() {
    assert_match(
        "P3-NULL-001: NULL in WHERE",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, NULL), (2, 10); SELECT * FROM t WHERE val IS NULL;",
    );
}

#[test]
fn test_p3_char_001_char_length() {
    assert_match("P3-CHAR-001: CHAR_LENGTH", "SELECT CHAR_LENGTH('hello');");
}

#[test]
fn test_p3_agg_001_sum_empty() {
    assert_match(
        "P3-AGG-001: SUM empty",
        "CREATE TABLE t(val INT); SELECT SUM(val) FROM t;",
    );
}

#[test]
fn test_p3_tx_001_rollback() {
    assert_match(
        "P3-TX-001: Transaction rollback",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10); BEGIN; UPDATE t SET val = 20 WHERE id = 1; ROLLBACK; SELECT val FROM t WHERE id = 1;",
    );
}

#[test]
fn test_p3_update_001_expression() {
    assert_match(
        "P3-UPDATE-001: UPDATE expression",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10); UPDATE t SET val = val * 2 WHERE id = 1; SELECT val FROM t WHERE id = 1;",
    );
}

#[test]
fn test_p3_schema_001_sqlite_master() {
    assert_match(
        "P3-SCHEMA-001: sqlite_master",
        "CREATE TABLE t(id INT); SELECT type, name FROM sqlite_master WHERE name = 't';",
    );
}

#[test]
fn test_p3_agg_003_avg_min_max() {
    assert_match(
        "P3-AGG-003: AVG/MIN/MAX",
        "CREATE TABLE t(val INT); INSERT INTO t VALUES (1), (2), (3), (4), (5); SELECT AVG(val), MIN(val), MAX(val) FROM t;",
    );
}

#[test]
fn test_p3_agg_004_group_by() {
    assert_match(
        "P3-AGG-004: GROUP BY",
        "CREATE TABLE t(dept TEXT, salary INT); INSERT INTO t VALUES ('a', 100), ('a', 200), ('b', 150); SELECT dept, SUM(salary) FROM t GROUP BY dept ORDER BY dept;",
    );
}

#[test]
fn test_p3_agg_005_having() {
    assert_match(
        "P3-AGG-005: HAVING",
        "CREATE TABLE t(dept TEXT, salary INT); INSERT INTO t VALUES ('a', 100), ('a', 200), ('b', 150); SELECT dept, SUM(salary) FROM t GROUP BY dept HAVING SUM(salary) > 200 ORDER BY dept;",
    );
}

#[test]
fn test_p3_join_001_inner_join() {
    assert_match(
        "P3-JOIN-001: INNER JOIN",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT t1.id, t2.id FROM t1 INNER JOIN t2 ON t1.id = t2.id ORDER BY t1.id;",
    );
}

#[test]
fn test_p3_join_002_left_join() {
    assert_match(
        "P3-JOIN-002: LEFT JOIN",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2), (3); SELECT t1.id, t2.id FROM t1 LEFT JOIN t2 ON t1.id = t2.id ORDER BY t1.id;",
    );
}

#[test]
fn test_p3_sub_001_exists() {
    assert_match(
        "P3-SUB-001: EXISTS",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2); INSERT INTO t2 VALUES (2); SELECT * FROM t1 WHERE EXISTS (SELECT 1 FROM t2 WHERE t2.id = t1.id) ORDER BY id;",
    );
}

#[test]
fn test_p3_sub_002_in() {
    assert_match(
        "P3-SUB-002: IN subquery",
        "CREATE TABLE t1(id INT); CREATE TABLE t2(id INT); INSERT INTO t1 VALUES (1), (2), (3); INSERT INTO t2 VALUES (2), (4); SELECT * FROM t1 WHERE id IN (SELECT id FROM t2) ORDER BY id;",
    );
}

#[test]
fn test_p3_limit_001() {
    assert_match(
        "P3-LIMIT-001: LIMIT",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3), (4), (5); SELECT id FROM t ORDER BY id LIMIT 3;",
    );
}

#[test]
fn test_p3_limit_002_with_offset() {
    assert_match(
        "P3-LIMIT-002: LIMIT with OFFSET",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3), (4), (5); SELECT id FROM t ORDER BY id LIMIT 2 OFFSET 2;",
    );
}

#[test]
fn test_p3_delete_001() {
    assert_match(
        "P3-DELETE-001: DELETE",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3); DELETE FROM t WHERE id = 2; SELECT * FROM t ORDER BY id;",
    );
}

#[test]
fn test_p3_alter_001_add_column() {
    assert_match(
        "P3-ALTER-001: ALTER TABLE ADD COLUMN",
        "CREATE TABLE t(id INT); ALTER TABLE t ADD COLUMN name TEXT; INSERT INTO t VALUES (1, 'a'); SELECT * FROM t;",
    );
}

#[test]
fn test_p3_string_001_upper_lower() {
    assert_match(
        "P3-STRING-001: UPPER/LOWER",
        "SELECT UPPER('hello'), LOWER('WORLD');",
    );
}

#[test]
fn test_p3_string_002_substr() {
    assert_match(
        "P3-STRING-002: SUBSTR",
        "SELECT SUBSTR('hello world', 1, 5);",
    );
}

#[test]
fn test_p3_string_003_replace() {
    assert_match(
        "P3-STRING-003: REPLACE",
        "SELECT REPLACE('hello world', 'world', 'rust');",
    );
}

#[test]
fn test_p3_string_004_trim() {
    assert_match("P3-STRING-004: TRIM", "SELECT TRIM('  hello  ');");
}

#[test]
fn test_p3_math_002_abs() {
    assert_match("P3-MATH-002: ABS", "SELECT ABS(-42);");
}

#[test]
fn test_p3_math_003_round() {
    assert_match("P3-MATH-003: ROUND", "SELECT ROUND(3.7);");
}

#[test]
fn test_p3_null_002_coalesce() {
    assert_match("P3-NULL-002: COALESCE", "SELECT COALESCE(NULL, 'default');");
}

#[test]
fn test_p3_null_003_ifnull() {
    assert_match("P3-NULL-003: IFNULL", "SELECT IFNULL(NULL, 'default');");
}

#[test]
fn test_p3_tx_002_commit() {
    assert_match(
        "P3-TX-002: COMMIT",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10); BEGIN; UPDATE t SET val = 20 WHERE id = 1; COMMIT; SELECT val FROM t WHERE id = 1;",
    );
}

#[test]
fn test_p3_cte_001_simple() {
    assert_match(
        "P3-CTE-001: Simple CTE",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3); WITH cte AS (SELECT id FROM t WHERE id > 1) SELECT * FROM cte ORDER BY id;",
    );
}

#[test]
fn test_p3_view_001_create_view() {
    assert_match(
        "P3-VIEW-001: CREATE VIEW (may not be implemented)",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2); CREATE VIEW v AS SELECT id FROM t;",
    );
}

#[test]
fn test_p3_index_001_drop_index() {
    assert_match(
        "P3-INDEX-001: DROP INDEX (may not be implemented)",
        "CREATE TABLE t(id INT, name TEXT); CREATE INDEX idx_t_name ON t(name); DROP INDEX idx_t_name;",
    );
}

#[test]
fn test_p3_window_001_row_number() {
    assert_match(
        "P3-WINDOW-001: ROW_NUMBER",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3); SELECT id, ROW_NUMBER() OVER (ORDER BY id) AS rn FROM t ORDER BY id;",
    );
}

#[test]
fn test_p3_window_002_rank() {
    assert_match(
        "P3-WINDOW-002: RANK",
        "CREATE TABLE t(id INT, val INT); INSERT INTO t VALUES (1, 10), (2, 10), (3, 20); SELECT id, RANK() OVER (ORDER BY val) AS rk FROM t ORDER BY id;",
    );
}

#[test]
fn test_p3_window_003_ntile() {
    assert_match(
        "P3-WINDOW-003: NTILE (not implemented)",
        "CREATE TABLE t(id INT); INSERT INTO t VALUES (1), (2), (3), (4), (5), (6), (7); SELECT id, NTILE(3) OVER (ORDER BY id) AS bucket FROM t ORDER BY id;",
    );
}

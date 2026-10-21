//! bustubx P3 差分测试 —— SQLRustGo vs SQLite
//!
//! #5449：这个文件此前**从未被编译过** —— 它在 `tests/compat/` 下（cargo 不会
//! 自动发现子目录），也没有 `[[test]]` 声明。把它变成可执行目标后，第一次编译
//! 就暴露了两处硬伤：
//!
//! 1. `make_db_dir()` 被调用但**从未定义** ⇒ 编译失败；
//! 2. 引擎二进制路径硬编码为
//!    **开发机上的绝对路径**（形如 `<某人的家目录>/dev/sqlrustgo/target/debug/
//!    sqlrustgo`）—— 既忽略 `CARGO_TARGET_DIR`，也在任何 CI 上都不存在。
//!    这正是 #5477 刚清掉的同一类反模式。
//!
//! 修法不是「把路径改成另一个路径」，而是**去掉这个外部依赖**：
//!
//! * 「我们这边」用**进程内**的 `ExecutionEngine`（root 包的公开 API）；
//! * 「他们那边」用**进程内**的 SQLite —— `rusqlite` 是 root 包的依赖且带
//!   `bundled` feature，SQLite 的 C 源码随仓库编译，CI 不需要额外二进制、
//!   也不需要网络。仓库里 `tests/common/sqlite_oracle.rs` 用的就是同一套做法。
//!
//! 于是这个差分测试既没有硬编码路径，也没有「环境没装 sqlite3」这种说不清的
//! 失败模式：两个引擎都在同一个进程里，随时可跑。
//!
//! 比较规则：**逐单元格、数值感知**。`17` 与 `17.0` 是同一个数；NULL 用一个
//! 不可能与真实文本撞上的哨兵表示。

use std::sync::Arc;

use parking_lot::RwLock;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};

// #5406：方言适配与数值感知比较已抽到共享模块，
// `tests/integration/compat/differential_corpus_test.rs` 用的是同一份。
#[path = "../common/sqlite_dialect.rs"]
mod sqlite_dialect;

use sqlite_dialect::{adapt_for_sqlite, cells_equal, format_real, NULL_CELL};

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
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
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
        // 准备语句同样要走方言改写（否则 `INSERT ... MOD(...)` 会在 oracle 侧
        // 直接 "no such function"）。
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

    if ours == theirs {
        return;
    }
    // 逐单元格再比一遍：整数值不等但数值相等（17 vs 17.0）算一致。
    if ours.len() == theirs.len() {
        let mut diff = None;
        for (i, (o, t)) in ours.iter().zip(theirs.iter()).enumerate() {
            if o.len() != t.len() {
                diff = Some(format!(
                    "第 {} 行列数不同: 引擎={} SQLite={}",
                    i,
                    o.len(),
                    t.len()
                ));
                break;
            }
            for (j, (oc, tc)) in o.iter().zip(t.iter()).enumerate() {
                if !cells_equal(oc, tc) {
                    diff = Some(format!(
                        "第 {} 行第 {} 列不同: 引擎={:?} SQLite={:?}",
                        i, j, oc, tc
                    ));
                    break;
                }
            }
            if diff.is_some() {
                break;
            }
        }
        if diff.is_none() {
            // 差异只在数值表示上（如 17 vs 17.0）⇒ 视为一致。
            return;
        }
    }

    panic!(
        "差分测试失败: {}\n  SQL: {}\n  引擎结果 ({} 行): {:?}\n  SQLite 结果 ({} 行): {:?}",
        test_name,
        script,
        ours.len(),
        ours,
        theirs.len(),
        theirs
    );
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

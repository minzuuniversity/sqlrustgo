//! bustubx P3 差分测试 —— SQLRustGo vs SQLite
//!
//! #5449：这个文件此前**从未被编译过** —— 它在 `tests/compat/` 下（cargo 不会
//! 自动发现子目录），也没有 `[[test]]` 声明。把它变成可执行目标后，第一次编译
//! 就暴露了两处硬伤：
//!
//! 1. `make_db_dir()` 被调用但**从未定义** ⇒ 编译失败；
//! 2. 引擎二进制路径硬编码为
//!    `/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo` —— 一台个人机器的
//!    绝对路径，既忽略 `CARGO_TARGET_DIR`，也在任何 CI 上都不存在。
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

/// NULL 的哨兵表示 —— 不可能与真实文本 `"NULL"` 撞上。
const NULL_CELL: &str = "\u{0}NULL";

/// 浮点比较的相对容差。
const FLOAT_REL_TOLERANCE: f64 = 1e-9;

/// 把脚本切成一条条语句。
fn split(script: &str) -> Vec<String> {
    sqlrustgo_parser::split_sql_statements(script)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 找到 `needle` 在 `haystack` 中的词边界位置（前后都不能是标识符字符）。
fn find_keyword(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(needle) {
        let abs = start + rel;
        let before_ok = abs == 0
            || !bytes[abs - 1].to_ascii_lowercase().is_ascii_alphanumeric()
                && bytes[abs - 1] != b'_';
        let after = abs + needle.len();
        let after_ok = after >= bytes.len()
            || (!bytes[after].to_ascii_lowercase().is_ascii_alphanumeric() && bytes[after] != b'_');
        if before_ok && after_ok {
            return Some(abs);
        }
        start = abs + 1;
        if start >= haystack.len() {
            break;
        }
    }
    None
}

/// 找到 `(` 在 `open_idx` 处对应的 `)`。
fn matching_paren(s: &str, open_idx: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    for (i, b) in bytes.iter().enumerate().skip(open_idx) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// 在括号深度 0 处按逗号切分参数。
fn split_top_level(args: &str) -> Option<(String, String)> {
    let bytes = args.as_bytes();
    let mut depth = 0i32;
    let mut split_at = None;
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                split_at = Some(i);
                break;
            }
            _ => {}
        }
    }
    let at = split_at?;
    Some((
        args[..at].trim().to_string(),
        args[at + 1..].trim().to_string(),
    ))
}

/// 把引擎的函数名改写成 SQLite 的等价写法。
///
/// 只收**语义完全等价**的映射，而且**每一条改写都会被打印出来** ——
/// 静默改写会让差分测试拿引擎去和**另一个查询**比较，什么也证明不了
/// （`tests/common/sqlite_oracle.rs` 的 `adapt_sql` 守的是同一条规矩）。
///
/// * `CHAR_LENGTH(x)` → `LENGTH(x)` —— SQLite 的 `LENGTH` 就是字符数；
/// * `MOD(a, b)` → `((a) % (b))` —— SQLite 的 `%` 就是取模。
///
/// SQLite **没有** `MOD` 与 `CHAR_LENGTH` 这两个函数名，所以不改写就
/// `no such function` —— 那种失败不是引擎的缺陷，是 oracle 的方言差异。
fn adapt_for_sqlite(sql: &str) -> (String, Vec<String>) {
    let mut out = sql.to_string();
    let mut notes: Vec<String> = Vec::new();

    while let Some(pos) = find_keyword(&out, "CHAR_LENGTH") {
        let open = pos + "CHAR_LENGTH".len();
        let Some(close) = matching_paren(&out, open) else {
            break;
        };
        let inner = out[open + 1..close].to_string();
        out = format!("{}LENGTH({}){}", &out[..pos], inner, &out[close + 1..]);
        notes.push(format!("CHAR_LENGTH({}) → LENGTH({})", inner, inner));
    }

    while let Some(pos) = find_keyword(&out, "MOD") {
        let open = pos + "MOD".len();
        let Some(close) = matching_paren(&out, open) else {
            break;
        };
        let inner = out[open + 1..close].to_string();
        let Some((a, b)) = split_top_level(&inner) else {
            break;
        };
        out = format!("{}(({}) % ({})){}", &out[..pos], a, b, &out[close + 1..]);
        notes.push(format!("MOD({}, {}) → (({}) % ({}))", a, b, a, b));
    }

    (out, notes)
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

/// 整数值浮点不带 `.0` 后缀，这样 `17` 与 `17.0` 渲染一致。
fn format_real(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.007_199_254_740_992e15 {
        format!("{}", f as i64)
    } else {
        format!("{}", f)
    }
}

/// 两个单元格是否代表同一个值。
fn cells_equal(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(x), Ok(y)) => {
            if x == y {
                return true;
            }
            let denom = x.abs().max(y.abs());
            if denom == 0.0 {
                return x == y;
            }
            (x - y).abs() / denom <= FLOAT_REL_TOLERANCE
        }
        _ => false,
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

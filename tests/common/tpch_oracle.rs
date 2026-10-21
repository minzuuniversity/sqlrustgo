//! TPC-H fixture 的双侧装载与差分比较 —— 全部在进程内。
//!
//! #5404：`tests/integration/sql/_q15_q16_check.rs` 与
//! `tests/integration/sql/four_way_cell_diff_test.rs` 这两个目标**零断言**
//! —— 它们的测试函数只 `println!` 结果，连错误分支也只是打印，所以引擎
//! 返回错误、返回 0 行、返回完全错误的数据，目标都���「通过」。
//!
//! #5391 记录了同一族的另外 19 个文件（`tests/integration/oracle/`）。
//! 两个 issue 的共同点是：这些目标需要**一个能给出真值的东西**。
//!
//! ## 真值从哪来
//!
//! 外部数据库（PostgreSQL / MariaDB）在 CI 上不可得 ⇒ 用「不可得就不跑」
//! 的方式处理会退化回「跳过即通过」。所以真值改成**进程内 SQLite**：
//!
//! * SQLite 是与 sqlrustgo **没有共享代码路径**的独立实现，因此它给出的
//!   结果是**独立证据**，而不是「用被测代码算出的期望值」；
//! * `rusqlite` 用 `bundled` feature ⇒ SQLite 的 C 源码随仓库编译，
//!   CI 不需要额外二进制、不需要网络、不需要 PATH 查找。
//!
//! 批次 7 已经把这条路子走过一遍（`tests/integration/compat/differential_corpus_test.rs`），
//! 本模块把「装载 fixture + 渲染两侧单元格 + 逐格比较」抽成共享件，
//! 让后续同类目标不再各写一份。
//!
//! ## 比较规则
//!
//! 逐单元格、**数值感知**：`17` 与 `17.0` 是同一个数；NULL 用一个不可能与
//! 真实文本撞上的哨兵表示。见 [`sqlite_dialect`](sqlite_dialect.rs)。

#![allow(dead_code)]

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine, Value};
use sqlrustgo_types::Value as SqlValue;
use std::sync::Arc;

#[path = "sqlite_dialect.rs"]
mod sqlite_dialect;

pub use sqlite_dialect::{adapt_for_sqlite, cells_equal, format_real, NULL_CELL};

/// The TPC-H tables, with their column counts and column kinds.
///
/// The kinds drive the typing of the values loaded into the oracle:
/// `int` columns become SQLite INTEGER, `float` columns REAL, `text`
/// columns TEXT. Getting this wrong would make the oracle compute a
/// different query (e.g. `SUM` over TEXT is 0 in SQLite), so the kinds are
/// declared **once**, here.
pub const TPC_H_TABLE_SPECS: [(&str, usize, &[&str]); 8] = [
    ("region", 3, &["int", "text", "text"]),
    ("nation", 4, &["int", "text", "int", "text"]),
    (
        "supplier",
        7,
        &["int", "text", "text", "int", "text", "float", "text"],
    ),
    (
        "customer",
        8,
        &[
            "int", "text", "text", "int", "text", "float", "text", "text",
        ],
    ),
    (
        "part",
        9,
        &[
            "int", "text", "text", "text", "text", "int", "text", "float", "text",
        ],
    ),
    ("partsupp", 5, &["int", "int", "int", "float", "text"]),
    (
        "orders",
        9,
        &[
            "int", "int", "text", "float", "text", "text", "text", "int", "text",
        ],
    ),
    (
        "lineitem",
        16,
        &[
            "int", "int", "int", "int", "float", "float", "float", "float", "text", "text", "text",
            "text", "text", "text", "text", "text",
        ],
    ),
];

/// Look up the spec of one table.
pub fn table_spec(table: &str) -> Option<(&'static str, usize, &'static [&'static str])> {
    TPC_H_TABLE_SPECS
        .iter()
        .find(|(name, _, _)| *name == table)
        .map(|(name, cols, kinds)| (*name, *cols, *kinds))
}

/// The DDL for the TPC-H tables, as SQLite accepts it.
pub fn sqlite_ddl(tables: &[&str]) -> String {
    let mut ddl = String::new();
    for table in tables {
        let Some((_, _, kinds)) = table_spec(table) else {
            panic!("unknown TPC-H table `{table}`");
        };
        let defs: Vec<String> = kinds
            .iter()
            .zip(sqlite_column_names(table).iter())
            .map(|(kind, name)| match *kind {
                "int" => format!("{name} INTEGER"),
                "float" => format!("{name} REAL"),
                _ => format!("{name} TEXT"),
            })
            .collect();
        ddl.push_str(&format!("CREATE TABLE {table} ({});\n", defs.join(", ")));
    }
    ddl
}

/// The column names of one TPC-H table, in schema order.
pub fn sqlite_column_names(table: &str) -> &'static [&'static str] {
    match table {
        "region" => &["r_regionkey", "r_name", "r_comment"],
        "nation" => &["n_nationkey", "n_name", "n_regionkey", "n_comment"],
        "supplier" => &[
            "s_suppkey",
            "s_name",
            "s_address",
            "s_nationkey",
            "s_phone",
            "s_acctbal",
            "s_comment",
        ],
        "customer" => &[
            "c_custkey",
            "c_name",
            "c_address",
            "c_nationkey",
            "c_phone",
            "c_acctbal",
            "c_mktsegment",
            "c_comment",
        ],
        "part" => &[
            "p_partkey",
            "p_name",
            "p_mfgr",
            "p_brand",
            "p_type",
            "p_size",
            "p_container",
            "p_retailprice",
            "p_comment",
        ],
        "partsupp" => &[
            "ps_partkey",
            "ps_suppkey",
            "ps_availqty",
            "ps_supplycost",
            "ps_comment",
        ],
        "orders" => &[
            "o_orderkey",
            "o_custkey",
            "o_orderstatus",
            "o_totalprice",
            "o_orderdate",
            "o_orderpriority",
            "o_clerk",
            "o_shippriority",
            "o_comment",
        ],
        "lineitem" => &[
            "l_orderkey",
            "l_partkey",
            "l_suppkey",
            "l_linenumber",
            "l_quantity",
            "l_extendedprice",
            "l_discount",
            "l_tax",
            "l_returnflag",
            "l_linestatus",
            "l_shipdate",
            "l_commitdate",
            "l_receiptdate",
            "l_shipinstruct",
            "l_shipmode",
            "l_comment",
        ],
        other => panic!("unknown TPC-H table `{other}`"),
    }
}

/// The DDL for the TPC-H tables, as sqlrustgo accepts it.
pub fn engine_ddl(tables: &[&str]) -> Vec<String> {
    tables
        .iter()
        .map(|table| {
            let Some((_, _, kinds)) = table_spec(table) else {
                panic!("unknown TPC-H table `{table}`");
            };
            let defs: Vec<String> = sqlite_column_names(table)
                .iter()
                .zip(kinds.iter())
                .map(|(name, kind)| match *kind {
                    "int" => format!("{name} INTEGER"),
                    "float" => format!("{name} REAL"),
                    _ => format!("{name} TEXT"),
                })
                .collect();
            format!("CREATE TABLE {table} ({})", defs.join(", "))
        })
        .collect()
}

/// Parse a TBL document into rows of trimmed fields.
pub fn parse_tbl(content: &str) -> Vec<Vec<String>> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.split('|').map(|f| f.trim().to_string()).collect())
        .collect()
}

/// Load the given tables from a fixture directory into an in-memory engine.
///
/// Returns the number of rows loaded per table.
pub fn load_into_engine(
    fixture_dir: &std::path::Path,
    tables: &[&str],
) -> Result<Vec<(String, usize)>, String> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for ddl in engine_ddl(tables) {
        engine
            .execute(&ddl)
            .map_err(|e| format!("creating the table failed: {e}"))?;
    }
    let mut loaded = Vec::new();
    for table in tables {
        let Some((_, cols, _)) = table_spec(table) else {
            return Err(format!("unknown TPC-H table `{table}`"));
        };
        let path = fixture_dir.join(format!("{table}.tbl"));
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let rows = parse_tbl(&content);
        let n = load_rows_into_storage(&storage, &rows, table, cols)?;
        loaded.push((table.to_string(), n));
    }
    let _ = &mut engine;
    Ok(loaded)
}

/// Load already-parsed rows into an in-memory storage under `table`.
///
/// The insert error is **not** swallowed: a failed batch insert leaves the
/// table empty, and every assertion of the calling test would then run
/// against an empty table and pass vacuously.
///
/// The rows are inserted in batches of 10 000, exactly like
/// `tests/common/tpch_fixture.rs::load_rows_into_storage`, so the engine
/// sees the same write pattern in both loaders.
pub fn load_rows_into_storage(
    storage: &Arc<RwLock<MemoryStorage>>,
    rows: &[Vec<String>],
    table: &str,
    ncols: usize,
) -> Result<usize, String> {
    const BATCH_SIZE: usize = 10_000;
    let kinds = table_spec(table).map(|(_, _, k)| k).unwrap_or(&[]);
    let mut batch: Vec<Vec<SqlValue>> = Vec::with_capacity(BATCH_SIZE);
    let mut count = 0usize;

    let mut flush = |storage: &Arc<RwLock<MemoryStorage>>,
                     batch: &mut Vec<Vec<SqlValue>>|
     -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }
        let n = batch.len();
        let data = std::mem::replace(batch, Vec::with_capacity(BATCH_SIZE));
        {
            let mut guard = storage.write();
            guard
                .insert(table, data)
                .map_err(|e| format!("inserting into `{table}` failed: {e}"))?;
        }
        count += n;
        Ok(())
    };

    for row in rows {
        if row.len() < ncols {
            continue;
        }
        let record: Vec<SqlValue> = (0..ncols)
            .map(|i| {
                let raw = row[i].as_str();
                match kinds.get(i).copied().unwrap_or("text") {
                    "int" => match raw.parse::<i64>() {
                        Ok(v) => SqlValue::Integer(v),
                        Err(_) => SqlValue::Null,
                    },
                    "float" => match raw.parse::<f64>() {
                        Ok(v) => SqlValue::Float(v),
                        Err(_) => SqlValue::Null,
                    },
                    _ => SqlValue::Text(raw.to_string()),
                }
            })
            .collect();
        batch.push(record);
        if batch.len() >= BATCH_SIZE {
            flush(storage, &mut batch)?;
        }
    }
    flush(storage, &mut batch)?;
    Ok(count)
}

/// Load the given tables from a fixture directory into an in-memory SQLite.
pub fn load_into_sqlite(
    fixture_dir: &std::path::Path,
    tables: &[&str],
) -> Result<rusqlite::Connection, String> {
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|e| format!("opening the in-memory SQLite failed: {e}"))?;
    conn.execute_batch(&sqlite_ddl(tables))
        .map_err(|e| format!("creating the tables failed: {e}"))?;
    for table in tables {
        let Some((_, cols, kinds)) = table_spec(table) else {
            return Err(format!("unknown TPC-H table `{table}`"));
        };
        let path = fixture_dir.join(format!("{table}.tbl"));
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let rows = parse_tbl(&content);
        let placeholders: Vec<String> = (1..=cols).map(|i| format!("?{i}")).collect();
        let sql = format!("INSERT INTO {table} VALUES ({})", placeholders.join(", "));
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("preparing `{sql}` failed: {e}"))?;
        for row in &rows {
            let values: Vec<rusqlite::types::Value> = (0..cols)
                .map(|i| {
                    let raw = row.get(i).map(|s: &String| s.as_str()).unwrap_or("");
                    match kinds[i] {
                        "int" => rusqlite::types::Value::Integer(raw.parse().unwrap_or(0)),
                        "float" => rusqlite::types::Value::Real(raw.parse().unwrap_or(0.0)),
                        _ => rusqlite::types::Value::Text(raw.to_string()),
                    }
                })
                .collect();
            stmt.execute(rusqlite::params_from_iter(values.iter()))
                .map_err(|e| format!("inserting into `{table}` failed: {e}"))?;
        }
    }
    Ok(conn)
}

/// Render the engine's `Value` into a comparable string.
pub fn engine_cell(v: &Value) -> String {
    match v {
        Value::Null => NULL_CELL.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format_real(*f),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => b.iter().map(|x| format!("{x:02x}")).collect(),
        Value::Point(x, y) => format!("({x},{y})"),
        Value::Json(j) => j.to_string(),
    }
}

/// Render a SQLite cell into a comparable string.
pub fn sqlite_cell(v: rusqlite::types::ValueRef<'_>) -> String {
    use rusqlite::types::ValueRef;
    match v {
        ValueRef::Null => NULL_CELL.to_string(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => format_real(f),
        ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
        ValueRef::Blob(b) => b.iter().map(|x| format!("{x:02x}")).collect(),
    }
}

/// Run a statement on the engine and return the rendered rows.
pub fn engine_rows(
    engine: &mut ExecutionEngine<MemoryStorage>,
    sql: &str,
) -> Result<Vec<Vec<String>>, String> {
    let res = engine
        .execute(sql)
        .map_err(|e| format!("the engine failed to execute the statement: {e}"))?;
    Ok(res
        .rows
        .iter()
        .map(|row| row.iter().map(engine_cell).collect())
        .collect())
}

/// Run a statement on the SQLite oracle and return the rendered rows.
///
/// Any dialect rewrite is **printed** — a silent rewrite would compare the
/// engine against a *different query* and prove nothing.
pub fn sqlite_rows(conn: &rusqlite::Connection, sql: &str) -> Result<Vec<Vec<String>>, String> {
    let (sqlite_sql, notes) = adapt_for_sqlite(sql);
    for note in &notes {
        eprintln!("[SQLite dialect rewrite] {note}");
    }
    let mut stmt = conn
        .prepare(&sqlite_sql)
        .map_err(|e| format!("SQLite failed to prepare the statement: {e}"))?;
    let ncols = stmt.column_count();
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("SQLite failed to execute the statement: {e}"))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("reading a row failed: {e}"))?
    {
        let mut cells = Vec::with_capacity(ncols);
        for idx in 0..ncols {
            let v = row
                .get_ref(idx)
                .map_err(|e| format!("reading column {idx} failed: {e}"))?;
            cells.push(sqlite_cell(v));
        }
        out.push(cells);
    }
    Ok(out)
}

/// Assert that the engine and the oracle produced the same rows.
///
/// The failure message names the first differing cell, and both sides are
/// printed in full so the difference can be inspected.
pub fn assert_rows_match(label: &str, ours: &[Vec<String>], theirs: &[Vec<String>]) {
    if ours.len() != theirs.len() {
        panic!(
            "{label}: the engine returned {} rows, the SQLite oracle returned {} rows\n\
             engine: {ours:?}\n\
             oracle: {theirs:?}",
            ours.len(),
            theirs.len()
        );
    }
    for (i, (o, t)) in ours.iter().zip(theirs.iter()).enumerate() {
        if o.len() != t.len() {
            panic!(
                "{label}: row {i} has a different number of columns: \
                 engine={o:?} oracle={t:?}"
            );
        }
        for (j, (oc, tc)) in o.iter().zip(t.iter()).enumerate() {
            if !cells_equal(oc, tc) {
                panic!(
                    "{label}: row {i}, column {j} differs: engine={oc:?} oracle={tc:?}\n\
                     engine: {ours:?}\n\
                     oracle: {theirs:?}"
                );
            }
        }
    }
}

/// Whether two cells represent the same value, **ignoring padding**.
///
/// The generated fixture and a real TPC-H database render the same value
/// differently: PostgreSQL's `CHAR(n)` pads with spaces
/// (`"1-URGENT       "`) and its `DECIMAL(15,2)` renders two decimals
/// (`"142768.00"`), while the engine and SQLite render
/// `"1-URGENT"` and `"142768"`. Those are **rendering** differences, not
/// semantic ones.
///
/// A comparison that does not account for them reports every one of them as
/// a defect, and a defect report that cries wolf is not a defect report.
/// So the cell comparison normalizes: surrounding whitespace is removed
/// and the two sides are then compared numerically when they look numeric
/// (see [`cells_equal`]), textually otherwise.
pub fn cells_equal_normalized(a: &str, b: &str) -> bool {
    let a = a.trim();
    let b = b.trim();
    if a == b {
        return true;
    }
    // A numeric cell may render as "142768" on one side and "142768.00" on
    // the other: compare them as numbers rather than as text.
    if let (Ok(x), Ok(y)) = (a.parse::<f64>(), b.parse::<f64>()) {
        if x == y {
            return true;
        }
        let denom = x.abs().max(y.abs());
        if denom == 0.0 {
            return x == y;
        }
        return (x - y).abs() / denom <= sqlite_dialect::FLOAT_REL_TOLERANCE;
    }
    false
}

/// The first difference between two result sets, ignoring padding and
/// numeric rendering.
///
/// Returns `None` when the two sides agree.
pub fn first_difference_normalized(ours: &[Vec<String>], theirs: &[Vec<String>]) -> Option<String> {
    if ours.len() != theirs.len() {
        return Some(format!(
            "row count differs: engine={} oracle={}",
            ours.len(),
            theirs.len()
        ));
    }
    for (i, (o, t)) in ours.iter().zip(theirs.iter()).enumerate() {
        if o.len() != t.len() {
            return Some(format!(
                "row {i}: column count differs: engine={} oracle={}",
                o.len(),
                t.len()
            ));
        }
        for (j, (oc, tc)) in o.iter().zip(t.iter()).enumerate() {
            if !cells_equal_normalized(oc, tc) {
                return Some(format!("row {i}, column {j}: engine={oc:?} oracle={tc:?}"));
            }
        }
    }
    None
}

/// The eight TPC-H tables, in dependency order.
pub const ALL_TABLES: [&str; 8] = [
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Assert that the engine and the oracle produced the same rows, ignoring
/// padding and numeric rendering.
pub fn assert_rows_match_normalized(label: &str, ours: &[Vec<String>], theirs: &[Vec<String>]) {
    if ours.len() != theirs.len() {
        panic!(
            "{label}: the engine returned {} rows, the SQLite oracle returned {} rows\n\
             engine: {ours:?}\n\
             oracle: {theirs:?}",
            ours.len(),
            theirs.len()
        );
    }
    for (i, (o, t)) in ours.iter().zip(theirs.iter()).enumerate() {
        if o.len() != t.len() {
            panic!(
                "{label}: row {i} has a different number of columns: \
                 engine={o:?} oracle={t:?}"
            );
        }
        for (j, (oc, tc)) in o.iter().zip(t.iter()).enumerate() {
            if !cells_equal_normalized(oc, tc) {
                panic!(
                    "{label}: row {i}, column {j} differs: engine={oc:?} oracle={tc:?}\n\
                     engine: {ours:?}\n\
                     oracle: {theirs:?}"
                );
            }
        }
    }
}

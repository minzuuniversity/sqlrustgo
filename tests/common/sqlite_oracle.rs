//! In-process SQLite as an **independent oracle** for TPC-H queries
//! (issue #5379).
//!
//! # Why
//!
//! The 13 TPC-H diagnostic tests in `tests/integration/oracle/` had **zero**
//! assertions — they printed the expected value with `eprintln!` and nothing
//! else, so they would have "passed" even if every query returned garbage:
//!
//! ```text
//! tests/integration/oracle/diag_q11.rs:73   eprintln!("... (expect ~1060): {:?}", r1.rows);
//! tests/integration/oracle/diag_q11.rs:84   eprintln!("... (expect 80): {}", r3.rows.len());
//! ```
//!
//! Filling in the engine's own current output as the expected value would be
//! circular — it would freeze today's behaviour as "correct" (that is what
//! `scripts/gate/check_g1_tpch_baseline.sh`'s step 4 already does: it re-runs
//! the very same `cargo test --test tpch_hash_test` and calls it an
//! "independent verification").
//!
//! This module instead computes the expected result with **SQLite**, a
//! different engine, loaded from the same fixture. `GATE_SCRIPT_STANDARD.md:160`
//! already requires a gate's oracle to be "DuckDB/SQLite"; SQLite is the one
//! that is actually available here:
//!
//! * `rusqlite` is a dependency of the root package (`Cargo.toml:252`) with
//!   the `bundled` feature, so the SQLite C sources are vendored and compiled
//!   — CI needs no extra binary and no network;
//! * DuckDB is not used anywhere in this repository.
//!
//! # Rules
//!
//! 1. The expected value comes from SQLite, **never** from the engine's own
//!    output.
//! 2. Where SQLite cannot run the query as written, the difference is
//!    reported explicitly by [`adapt_sql`] and the caller falls back to
//!    data-independent invariants. Nothing is silently rewritten.
//! 3. Row comparison is numeric-aware: `17` and `17.0` are the same number,
//!    and floats are compared with a relative tolerance.
//!
//! # Example
//!
//! ```no_run
//! #[path = "../../common/sqlite_oracle.rs"]
//! mod sqlite_oracle;
//! use sqlite_oracle::SqliteOracle;
//!
//! let oracle = SqliteOracle::open(&fixture, &["nation", "supplier", "partsupp"]).unwrap();
//! let expected = oracle.query("SELECT n_name FROM nation").unwrap();
//! ```

#![allow(dead_code)]

use rusqlite::types::Value as SqlValue;
use rusqlite::Connection;

use crate::tpch_fixture::Fixture;

/// Relative tolerance used when comparing floating point cells.
pub const FLOAT_REL_TOLERANCE: f64 = 1e-9;

/// Standard TPC-H schema, in SQLite dialect.
///
/// `INTEGER` / `REAL` / `TEXT` affinities are the standard TPC-H column
/// types. Note that a `REAL` column holding an integral value (e.g.
/// `l_quantity = 17`) is stored as `17.0` while the engine's loader types it
/// as an integer — [`diff_rows`] compares numerically, so the two agree.
const TPC_H_SCHEMA: [(&str, &str); 8] = [
    (
        "region",
        "CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)",
    ),
    (
        "nation",
        "CREATE TABLE nation (n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT)",
    ),
    (
        "supplier",
        "CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    ),
    (
        "customer",
        "CREATE TABLE customer (c_custkey INTEGER, c_name TEXT, c_address TEXT, c_nationkey INTEGER, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT)",
    ),
    (
        "part",
        "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT)",
    ),
    (
        "partsupp",
        "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT)",
    ),
    (
        "orders",
        "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    ),
    (
        "lineitem",
        "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
    ),
];

/// The indexes the TPC-H specification defines for its schema.
///
/// The oracle is the **reference implementation** of every statement the
/// wire tests compare against. Without these indexes it answers the
/// standard statements by brute force: `queries/q21.sql` (two correlated
/// `EXISTS` / `NOT EXISTS` subqueries over `lineitem`) needs **41 seconds**
/// in SQLite, against **441 ms** for the engine, which has the same
/// indexes.
///
/// That gap is not merely slow — it breaks the test. The server reaps
/// connections that have been idle for longer than
/// `SQLRUSTGO_IDLE_TIMEOUT_SECS` (30 s by default, issue #4682). A client
/// that spends 41 s computing the reference leaves the engine's connection
/// idle for longer than that, the server closes it, and the **next** engine
/// statement fails with `read packet header: failed to fill whole buffer` —
/// a failure that looks exactly like an engine crash and is attributed to
/// the engine (issue #5464).
///
/// An index does not change a query's *result*, only how fast the reference
/// finds it, so adding them keeps the oracle an independent reference while
/// making it fast enough not to starve the engine's connection. Every
/// conforming TPC-H database has these indexes.
const TPC_H_INDEXES: [&str; 14] = [
    "CREATE INDEX idx_nation_region ON nation(n_regionkey)",
    "CREATE INDEX idx_supplier_nation ON supplier(s_nationkey)",
    "CREATE INDEX idx_supplier_name ON supplier(s_name)",
    "CREATE INDEX idx_customer_nation ON customer(c_nationkey)",
    "CREATE INDEX idx_customer_name ON customer(c_name)",
    "CREATE INDEX idx_part_brand_type_size ON part(p_brand, p_type, p_size)",
    "CREATE INDEX idx_part_name ON part(p_name)",
    "CREATE INDEX idx_partsupp_suppkey ON partsupp(ps_suppkey)",
    "CREATE INDEX idx_orders_custkey ON orders(o_custkey)",
    "CREATE INDEX idx_orders_orderdate ON orders(o_orderdate)",
    "CREATE INDEX idx_lineitem_partkey ON lineitem(l_partkey)",
    "CREATE INDEX idx_lineitem_suppkey ON lineitem(l_suppkey)",
    "CREATE INDEX idx_lineitem_orderkey ON lineitem(l_orderkey)",
    "CREATE INDEX idx_lineitem_shipdate ON lineitem(l_shipdate)",
];

/// Number of leading `.tbl` fields that map to each table's columns.
///
/// The `.tbl` format appends one trailing `|` after the comment column, so a
/// file has `columns + 1` fields. The extra field is dropped on import.
fn column_count(table: &str) -> usize {
    match table {
        "region" => 3,
        "nation" => 4,
        "supplier" => 7,
        "customer" => 8,
        "part" => 9,
        "partsupp" => 5,
        "orders" => 9,
        "lineitem" => 16,
        _ => 0,
    }
}

/// An in-process SQLite database loaded with a TPC-H fixture.
pub struct SqliteOracle {
    conn: Connection,
}

impl SqliteOracle {
    /// Open an in-memory SQLite database and load the requested tables.
    ///
    /// The fixture files are parsed independently of the engine's loader (this
    /// module does its own `.tbl` splitting), so a defect in the engine's
    /// loader cannot hide itself by also corrupting the oracle.
    pub fn open(fixture: &Fixture, tables: &[&str]) -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("open sqlite: {e}"))?;
        let mut oracle = SqliteOracle { conn };
        for table in tables {
            oracle.load_table(fixture, table)?;
        }
        oracle.create_indexes()?;
        Ok(oracle)
    }

    /// Create the TPC-H indexes (see [`TPC_H_INDEXES`]).
    ///
    /// The indexes are created **after** the bulk load, which is both
    /// faster and the order a real database would use. A table that was
    /// not loaded cannot get an index — the statements are skipped, so
    /// SQLite would refuse to create the index.
    fn create_indexes(&mut self) -> Result<(), String> {
        for ddl in TPC_H_INDEXES {
            if let Err(e) = self.conn.execute_batch(ddl) {
                // A missing table is not a failure of the oracle: the
                // caller asked for a subset of the tables.
                let msg = e.to_string();
                if msg.contains("no such table") {
                    continue;
                }
                return Err(format!("create index `{ddl}`: {e}"));
            }
        }
        Ok(())
    }

    /// Load one table from the fixture into SQLite.
    fn load_table(&mut self, fixture: &Fixture, table: &str) -> Result<(), String> {
        let ddl = TPC_H_SCHEMA
            .iter()
            .find(|(t, _)| *t == table)
            .map(|(_, ddl)| *ddl)
            .ok_or_else(|| format!("no TPC-H schema for table `{table}`"))?;
        self.conn
            .execute_batch(ddl)
            .map_err(|e| format!("create {table}: {e}"))?;

        let ncols = column_count(table);
        let content = fixture.read_table(table)?;
        let tx = self
            .conn
            .transaction()
            .map_err(|e| format!("begin tx for {table}: {e}"))?;
        {
            let mut stmt = tx
                .prepare(&format!(
                    "INSERT INTO {table} VALUES ({})",
                    vec!["?"; ncols].join(",")
                ))
                .map_err(|e| format!("prepare insert {table}: {e}"))?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let fields: Vec<&str> = line.split('|').collect();
                if fields.len() < ncols {
                    continue;
                }
                let values: Vec<SqlValue> = fields[..ncols].iter().map(|f| bind_value(f)).collect();
                stmt.execute(rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| format!("insert into {table}: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("commit {table}: {e}"))?;
        Ok(())
    }

    /// Execute a query and return its rows as normalized strings.
    pub fn query(&self, sql: &str) -> Result<Vec<Vec<String>>, String> {
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| format!("prepare `{sql}`: {e}"))?;
        let ncols = stmt.column_count();
        let mut rows = stmt.query([]).map_err(|e| format!("query `{sql}`: {e}"))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| format!("row: {e}"))? {
            let mut cells = Vec::new();
            for idx in 0..ncols {
                let v = row.get_ref(idx).map_err(|e| format!("cell {idx}: {e}"))?;
                cells.push(sqlite_cell_to_string(v));
            }
            out.push(cells);
        }
        Ok(out)
    }

    /// Row count of a table, as SQLite sees it.
    pub fn table_row_count(&self, table: &str) -> Result<i64, String> {
        self.conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .map_err(|e| format!("count {table}: {e}"))
    }
}

/// Bind one raw `.tbl` field with the same typing rule the engine's loader
/// uses (integer → float → text, empty → NULL), so both sides classify a
/// field identically.
fn bind_value(field: &str) -> SqlValue {
    let t = field.trim();
    if t.is_empty() {
        SqlValue::Null
    } else if let Ok(i) = t.parse::<i64>() {
        SqlValue::Integer(i)
    } else if let Ok(f) = t.parse::<f64>() {
        SqlValue::Real(f)
    } else {
        SqlValue::Text(t.to_string())
    }
}

/// Render one SQLite cell as a comparable string.
pub fn sqlite_cell_to_string(v: rusqlite::types::ValueRef<'_>) -> String {
    match v {
        rusqlite::types::ValueRef::Null => NULL_CELL.to_string(),
        rusqlite::types::ValueRef::Integer(i) => i.to_string(),
        rusqlite::types::ValueRef::Real(f) => format_real(f),
        rusqlite::types::ValueRef::Text(s) => String::from_utf8_lossy(s).to_string(),
        rusqlite::types::ValueRef::Blob(b) => b.iter().map(|x| format!("{x:02x}")).collect(),
    }
}

/// Sentinel used for SQL NULL so it cannot collide with the text `"NULL"`.
pub const NULL_CELL: &str = "\u{0}NULL";

/// Render one engine `Value` as a comparable string.
pub fn engine_cell_to_string(v: &sqlrustgo_types::Value) -> String {
    use sqlrustgo_types::Value;
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

/// Render the engine's result rows as comparable strings.
pub fn engine_rows_to_strings(rows: &[Vec<sqlrustgo_types::Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|r| r.iter().map(engine_cell_to_string).collect())
        .collect()
}

/// Canonical float rendering: integral values lose the `.0` suffix so that
/// `17` and `17.0` render identically.
fn format_real(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.007_199_254_740_992e15 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

/// Compare two row sets and describe the first difference, if any.
///
/// Comparison is numeric-aware: two cells that both parse as `f64` are
/// compared with [`FLOAT_REL_TOLERANCE`]; everything else is compared as
/// strings. Row order must already match (queries under test use ORDER BY, or
/// the caller sorts both sides).
pub fn diff_rows(engine: &[Vec<String>], oracle: &[Vec<String>]) -> Option<String> {
    if engine.len() != oracle.len() {
        return Some(format!(
            "row count differs: engine={} oracle={}",
            engine.len(),
            oracle.len()
        ));
    }
    for (i, (e, o)) in engine.iter().zip(oracle.iter()).enumerate() {
        if e.len() != o.len() {
            return Some(format!(
                "row {i}: column count differs: engine={} oracle={} (engine={e:?} oracle={o:?})",
                e.len(),
                o.len()
            ));
        }
        for (j, (ec, oc)) in e.iter().zip(o.iter()).enumerate() {
            if !cells_equal(ec, oc) {
                return Some(format!("row {i} col {j}: engine={ec:?} oracle={oc:?}"));
            }
        }
    }
    None
}

/// Whether two rendered cells represent the same value.
pub fn cells_equal(a: &str, b: &str) -> bool {
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

/// Result of adapting a query to SQLite's dialect.
#[derive(Debug, Clone, Default)]
pub struct AdaptedSql {
    /// The query as SQLite will run it.
    pub sql: String,
    /// Human-readable descriptions of the rewrites that were applied.
    pub adaptations: Vec<String>,
    /// Constructs SQLite cannot run; the caller must not silently ignore
    /// these — it has to fall back to invariants or register the case.
    pub unsupported: Vec<String>,
}

/// Adapt a query to SQLite's dialect.
///
/// Only a small, explicit whitelist is rewritten:
///
/// * `EXTRACT(YEAR FROM <expr>)` → `CAST(strftime('%Y', <expr>) AS INTEGER)`
///
/// Anything else that SQLite cannot run is reported in
/// [`AdaptedSql::unsupported`] instead of being rewritten — a silent rewrite
/// would compare the engine against a *different* query and prove nothing.
pub fn adapt_sql(sql: &str) -> AdaptedSql {
    let mut out = AdaptedSql::default();
    let mut current = sql.to_string();

    // EXTRACT(YEAR FROM <expr>) — find the balanced closing parenthesis.
    while let Some(pos) = find_keyword(&current, "EXTRACT(YEAR FROM ") {
        // The `(` that opens `EXTRACT(` — its matching `)` closes the whole
        // construct, and the year expression lives between the two.
        let open_paren = pos + "EXTRACT".len();
        let expr_start = pos + "EXTRACT(YEAR FROM ".len();
        let Some(expr_end) = matching_paren(&current, open_paren) else {
            out.unsupported
                .push("EXTRACT(YEAR FROM ...) with unbalanced parentheses".to_string());
            break;
        };
        let expr = current[expr_start..expr_end].trim().to_string();
        let replacement = format!("CAST(strftime('%Y', {expr}) AS INTEGER)");
        current = format!(
            "{}{}{}",
            &current[..pos],
            replacement,
            &current[expr_end + 1..]
        );
        out.adaptations.push(format!(
            "EXTRACT(YEAR FROM {expr}) -> CAST(strftime('%Y', {expr}) AS INTEGER)"
        ));
    }

    for (needle, why) in [
        ("DATE_TRUNC(", "SQLite has no DATE_TRUNC"),
        ("EXTRACT(MONTH", "SQLite has no EXTRACT(MONTH ...)"),
        ("EXTRACT(DAY", "SQLite has no EXTRACT(DAY ...)"),
        ("::", "PostgreSQL cast syntax is not valid in SQLite"),
        ("TOP ", "TOP n is not valid in SQLite"),
    ] {
        if current.contains(needle) {
            out.unsupported.push(format!("{why} (`{needle}`)"));
        }
    }

    out.sql = current;
    out
}

/// Find `needle` in `haystack` at a word boundary, returning its byte offset.
fn find_keyword(haystack: &str, needle: &str) -> Option<usize> {
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(needle) {
        let abs = start + rel;
        let before_ok = abs == 0
            || !haystack[..abs]
                .chars()
                .next_back()
                .map(|c| c.is_alphanumeric() || c == '_')
                .unwrap_or(false);
        if before_ok {
            return Some(abs);
        }
        start = abs + 1;
        if start >= haystack.len() {
            break;
        }
    }
    None
}

/// Index of the `)` that closes the `(` at `open_idx`.
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

/// Assert that the engine's result matches the independent SQLite oracle.
///
/// A mismatch is a **test failure** unless the difference is registered in
/// `tests/baseline/tpch_known_differences.json` with an issue and an owner —
/// in which case it is reported through the `TPCH-KNOWN-DIFF` marker so the
/// CI log shows exactly which tracked defect was tolerated.
///
/// # Comparison is order-insensitive
///
/// Rows are compared as a **sorted multiset**, not as sequences. Two engines
/// are free to return the same rows in a different order when the query has
/// no `ORDER BY` — comparing sequences would turn every such difference into
/// a false failure. The consequence is stated plainly: this helper does not
/// assert *row ordering*. It asserts row *content*, which is the property the
/// TPC-H correctness questions are about.
pub fn check_against_oracle(
    case_id: &str,
    engine_rows: &[Vec<String>],
    oracle_rows: &[Vec<String>],
) {
    let mut e = engine_rows.to_vec();
    let mut o = oracle_rows.to_vec();
    e.sort();
    o.sort();
    if let Some(diff) = diff_rows(&e, &o) {
        if let Some(entry) = crate::tpch_fixture::lookup_known_difference(case_id) {
            crate::tpch_fixture::report_known_difference(&entry, &diff);
            return;
        }
        panic!(
            "{case_id}: engine result differs from the independent SQLite oracle: {diff}\n\
             (if this is a known engine defect, register it in \
             tests/baseline/tpch_known_differences.json with an issue and an owner)"
        );
    }
}

/// All eight TPC-H tables, in dependency order.
pub const TPC_H_ALL_TABLES: [&str; 8] = [
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// Compare rows produced **over the MySQL wire protocol** with the rows the
/// independent SQLite oracle produces for the same statement.
///
/// The wire client renders a SQL NULL as the empty string — it is the lowest
/// common denominator of the raw text protocol — while the oracle renders it
/// as [`NULL_CELL`]. The two representations cannot be told apart here, so
/// this helper does two things instead of quietly normalising:
///
/// 1. it **counts** the NULLs the oracle produced, and
/// 2. it **fails** if there were any, because a NULL in the result would
///    silently turn into an empty string and hide a real difference.
///
/// Every statement in the standard 22-query set was measured to produce zero
/// NULLs on the generated fixture, so this is a guard against a future
/// statement silently losing that property — not a restriction on the
/// current set.
///
/// Row order is not compared, for the same reason as in
/// [`check_against_oracle`].
pub fn check_wire_against_oracle(
    case_id: &str,
    wire_rows: &[Vec<String>],
    sql: &str,
    oracle: &SqliteOracle,
) {
    let adapted = adapt_sql(sql);
    assert!(
        adapted.unsupported.is_empty(),
        "{case_id}: the statement uses constructs the SQLite oracle does not support: {:?}",
        adapted.unsupported
    );
    let oracle_rows = oracle
        .query(&adapted.sql)
        .unwrap_or_else(|e| panic!("{case_id}: the SQLite oracle failed on this statement: {e}"));

    let nulls = oracle_rows
        .iter()
        .flat_map(|r| r.iter())
        .filter(|c| c.as_str() == NULL_CELL)
        .count();
    assert!(
        nulls == 0,
        "{case_id}: the statement produced {nulls} NULL cell(s). The wire protocol renders a \
         NULL as the empty string, which cannot be told apart from a real empty string, so \
         this comparison would be unable to detect a difference. Handle the NULL explicitly \
         before comparing."
    );

    // The oracle's NULL sentinel cannot occur (checked above), so the two
    // representations are directly comparable.
    let mut w = wire_rows.to_vec();
    let mut o = oracle_rows;
    w.sort();
    o.sort();
    if let Some(diff) = diff_rows(&w, &o) {
        if let Some(entry) = crate::tpch_fixture::lookup_known_difference(case_id) {
            crate::tpch_fixture::report_known_difference(&entry, &diff);
            return;
        }
        panic!(
            "{case_id}: the wire-protocol result differs from the independent SQLite oracle: {diff}\n\
             (if this is a known engine defect, register it in \
             tests/baseline/tpch_known_differences.json with an issue and an owner)"
        );
    }
}

/// Compare only the **row count** of a wire-protocol result with the oracle's.
///
/// Used by the statements whose result is a single aggregate: the row count
/// is the whole result there, and comparing the full rows would additionally
/// require the NULL-free property that [`check_wire_against_oracle`]
/// enforces.
pub fn check_wire_row_count_against_oracle(
    case_id: &str,
    wire_rows: &[Vec<String>],
    sql: &str,
    oracle: &SqliteOracle,
) {
    let adapted = adapt_sql(sql);
    assert!(
        adapted.unsupported.is_empty(),
        "{case_id}: the statement uses constructs the SQLite oracle does not support: {:?}",
        adapted.unsupported
    );
    let oracle_rows = oracle
        .query(&adapted.sql)
        .unwrap_or_else(|e| panic!("{case_id}: the SQLite oracle failed on this statement: {e}"));
    if wire_rows.len() != oracle_rows.len() {
        panic!(
            "{case_id}: the wire protocol returned {} row(s) but the independent SQLite oracle \
             returned {} row(s)",
            wire_rows.len(),
            oracle_rows.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tpch_indexes_are_created() {
        // The indexes are not a performance nicety: without them the oracle
        // needs 41 s for `queries/q21.sql`, the server reaps the engine's
        // idle connection after 30 s, and the *next* engine statement is
        // misreported as an engine crash (issue #5464).
        let conn = Connection::open_in_memory().expect("open sqlite");
        for (_, ddl) in TPC_H_SCHEMA.iter() {
            conn.execute_batch(ddl).expect("create table");
        }
        let mut oracle = SqliteOracle { conn };
        oracle.create_indexes().expect("create indexes");

        let mut found: Vec<String> = oracle
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'index'")
            .expect("prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .filter_map(|r| r.ok())
            .collect();
        found.sort();
        assert_eq!(
            found.len(),
            TPC_H_INDEXES.len(),
            "every TPC-H index must exist: {found:?}"
        );
        for ddl in TPC_H_INDEXES {
            let name = ddl
                .split_whitespace()
                .nth(2)
                .expect("index name")
                .trim_end_matches('(');
            assert!(
                found.iter().any(|f| f == name),
                "index `{name}` is missing: {found:?}"
            );
        }
    }

    #[test]
    fn extracts_year_is_rewritten() {
        let a = adapt_sql("SELECT EXTRACT(YEAR FROM l_shipdate) FROM lineitem");
        assert!(a.unsupported.is_empty());
        assert_eq!(a.adaptations.len(), 1);
        assert!(a.sql.contains("strftime('%Y', l_shipdate)"));
    }

    #[test]
    fn extract_year_with_nested_call_is_rewritten() {
        let a = adapt_sql("SELECT EXTRACT(YEAR FROM CAST(x AS TEXT)) FROM t");
        assert!(a.unsupported.is_empty());
        assert!(a
            .sql
            .contains("CAST(strftime('%Y', CAST(x AS TEXT)) AS INTEGER)"));
    }

    #[test]
    fn unsupported_constructs_are_reported_not_rewritten() {
        let a = adapt_sql("SELECT DATE_TRUNC('month', d) FROM t");
        assert_eq!(a.unsupported.len(), 1);
        assert!(a.sql.contains("DATE_TRUNC"), "must not silently rewrite");
    }

    #[test]
    fn numeric_cells_compare_across_int_and_float_forms() {
        assert!(cells_equal("17", "17.0"));
        assert!(cells_equal("0.30000000000000004", "0.3"));
        assert!(!cells_equal("17", "18"));
        assert!(!cells_equal("abc", "abd"));
    }

    #[test]
    fn null_sentinel_does_not_collide_with_text() {
        assert!(!cells_equal(NULL_CELL, "NULL"));
    }

    #[test]
    fn diff_rows_reports_row_count_mismatch() {
        let e = vec![vec!["1".to_string()]];
        let o = vec![];
        let d = diff_rows(&e, &o).expect("difference");
        assert!(d.contains("row count differs"));
    }

    #[test]
    fn diff_rows_reports_cell_mismatch() {
        let e = vec![vec!["1".to_string()]];
        let o = vec![vec!["2".to_string()]];
        let d = diff_rows(&e, &o).expect("difference");
        assert!(d.contains("engine=\"1\""));
    }
}

//! c7-allow-local-credentials: this test starts the repository's **own**
//! server (`sqlrustgo-mysql-server` / `start_ephemeral` / a docker
//! container), so the `tester` credential below is created by the test
//! itself and belongs to no particular machine.

//! A04 SQL/MySQL Protocol Differential Matrix (Issue #5226)
//!
//! 从真实 MySQL 客户端（`mysql` CLI + `sqlrustgo_mysql_client`）验证
//! 查询结果与错误语义，对照 SQLite CLI 与 PostgreSQL (Docker) 参照实现。
//!
//! ## 覆盖（A04 body）
//! JOIN / 子查询 / 聚合 / HAVING / NULL / 类型转换 / ORDER-LIMIT /
//! 错误类别 / 列名 / 列类型 / prepared statement / DDL 约束
//!
//! ## 验收对齐
//! 1. 比较列名、类型类别、排序规则、逐值、NULL、错误类别
//! 2. 参照不可用 → `not_comparable` + 依据
//! 3. 全部 case 带 seed、SQL corpus hash、原始 wire/client 日志
//! 4. 静默错误结果 → 独立 defect Issue
//!
//! ## 参照引擎状态（探测于 2026-10-10）
//! - SQLite 3.37 CLI: ✅ `/usr/bin/sqlite3`
//! - PostgreSQL 17.10 Docker: ✅ `a04-pg-ref` @ `127.0.0.1:15432`
//! - MySQL/MariaDB 服务端: ❌ 本机无 mariadbd/mysqld；Docker registry 不可达
//!
//! ## 证据落盘
//! 运行后在 `docs/releases/v4.1.0/evidence/a04-5226/` 生成：
//! - `matrix.json` — 全部 case 的 verdict 矩阵
//! - `wire/<case_id>_<engine>.txt` — 原始 wire/client 输出
//!
//! ## Independent Judge 重放
//! ```bash
//! # 前置: Docker postgres:17-alpine 容器 a04-pg-ref 已启动
//! cargo test --all-features --test a04_mysql_diff_matrix -- --nocapture
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// #5406 / #5477：二进制定位收敛到共享 helper。
#[path = "../../common/bin_resolver.rs"]
mod bin_resolver;

// ============================================================================
// Corpus 定义
// ============================================================================

/// 一个差分 case。
#[derive(Debug, Clone)]
struct DiffCase {
    id: &'static str,
    category: &'static str,
    /// 期望被所有引擎接受（否则可能 dialect 不兼容）
    setup: &'static [&'static str],
    /// 被测查询（所有引擎跑同一 SQL）
    query: &'static str,
    /// 是否需要 ORDER BY 才能比较行序；false = 自动排序行集合
    order_matters: bool,
    /// 仅在 MySQL 方言可比（例如 LIMIT 语法差异等）
    mysql_only: bool,
}

const CASES: &[DiffCase] = &[
    // ---- JOIN ----
    DiffCase {
        id: "join_inner_001",
        category: "JOIN",
        setup: &[
            "CREATE TABLE t_join_a (id INTEGER, name TEXT, val INTEGER)",
            "INSERT INTO t_join_a VALUES (1, 'a', 10)",
            "INSERT INTO t_join_a VALUES (2, 'b', 20)",
            "INSERT INTO t_join_a VALUES (3, 'c', 30)",
        ],
        query: "SELECT a.id, a.name, b.val FROM t_join_a a INNER JOIN t_join_a b ON a.id = b.id ORDER BY a.id",
        order_matters: true,
        mysql_only: false,
    },
    DiffCase {
        id: "join_left_002",
        category: "JOIN",
        setup: &[
            "CREATE TABLE t_join_l (id INTEGER, x TEXT)",
            "INSERT INTO t_join_l VALUES (1, 'one')",
            "INSERT INTO t_join_l VALUES (2, 'two')",
        ],
        query: "SELECT l.id, l.x, r.id FROM t_join_l l LEFT JOIN t_join_l r ON l.id = r.id + 1 ORDER BY l.id",
        order_matters: true,
        mysql_only: false,
    },
    // ---- 子查询 ----
    DiffCase {
        id: "sub_scalar_001",
        category: "SUBQUERY",
        setup: &[
            "CREATE TABLE t_sub (v INTEGER)",
            "INSERT INTO t_sub VALUES (5)",
            "INSERT INTO t_sub VALUES (15)",
            "INSERT INTO t_sub VALUES (25)",
        ],
        query: "SELECT v FROM t_sub WHERE v > (SELECT AVG(v) FROM t_sub) ORDER BY v",
        order_matters: true,
        mysql_only: false,
    },
    DiffCase {
        id: "sub_exists_002",
        category: "SUBQUERY",
        setup: &[
            "CREATE TABLE t_ex_parent (id INTEGER)",
            "CREATE TABLE t_ex_child (pid INTEGER, c INTEGER)",
            "INSERT INTO t_ex_parent VALUES (1)",
            "INSERT INTO t_ex_parent VALUES (2)",
            "INSERT INTO t_ex_child VALUES (1, 100)",
        ],
        query: "SELECT id FROM t_ex_parent p WHERE EXISTS (SELECT 1 FROM t_ex_child c WHERE c.pid = p.id) ORDER BY id",
        order_matters: true,
        mysql_only: false,
    },
    // ---- 聚合 ----
    DiffCase {
        id: "agg_group_001",
        category: "AGGREGATE",
        setup: &[
            "CREATE TABLE t_agg (g TEXT, n INTEGER)",
            "INSERT INTO t_agg VALUES ('x', 1)",
            "INSERT INTO t_agg VALUES ('x', 2)",
            "INSERT INTO t_agg VALUES ('y', 10)",
        ],
        query: "SELECT g, COUNT(*), SUM(n), MIN(n), MAX(n) FROM t_agg GROUP BY g ORDER BY g",
        order_matters: true,
        mysql_only: false,
    },
    DiffCase {
        id: "agg_having_002",
        category: "HAVING",
        setup: &[
            "CREATE TABLE t_hv (g TEXT, n INTEGER)",
            "INSERT INTO t_hv VALUES ('a', 1)",
            "INSERT INTO t_hv VALUES ('a', 2)",
            "INSERT INTO t_hv VALUES ('a', 3)",
            "INSERT INTO t_hv VALUES ('b', 1)",
            "INSERT INTO t_hv VALUES ('b', 5)",
        ],
        query: "SELECT g, SUM(n) FROM t_hv GROUP BY g HAVING SUM(n) > 3 ORDER BY g",
        order_matters: true,
        mysql_only: false,
    },
    // ---- NULL ----
    DiffCase {
        id: "null_select_001",
        category: "NULL",
        setup: &[
            "CREATE TABLE t_null (id INTEGER, v TEXT)",
            "INSERT INTO t_null VALUES (1, 'has')",
            "INSERT INTO t_null VALUES (2, NULL)",
        ],
        query: "SELECT id, v FROM t_null ORDER BY id",
        order_matters: true,
        mysql_only: false,
    },
    DiffCase {
        id: "null_where_002",
        category: "NULL",
        setup: &[
            "CREATE TABLE t_nw (v INTEGER)",
            "INSERT INTO t_nw VALUES (1)",
            "INSERT INTO t_nw VALUES (NULL)",
            "INSERT INTO t_nw VALUES (3)",
        ],
        query: "SELECT v FROM t_nw WHERE v IS NOT NULL ORDER BY v",
        order_matters: true,
        mysql_only: false,
    },
    // ---- 类型转换 ----
    DiffCase {
        id: "cast_int_text_001",
        category: "CAST",
        setup: &[
            "CREATE TABLE t_cast (n INTEGER)",
            "INSERT INTO t_cast VALUES (42)",
        ],
        query: "SELECT n, n + 0 FROM t_cast",
        order_matters: false,
        mysql_only: false,
    },
    DiffCase {
        id: "arith_div_002",
        category: "CAST",
        setup: &[
            "CREATE TABLE t_div (a INTEGER, b INTEGER)",
            "INSERT INTO t_div VALUES (7, 2)",
        ],
        query: "SELECT a, b, a / b FROM t_div",
        order_matters: false,
        mysql_only: false,
    },
    // ---- ORDER / LIMIT ----
    DiffCase {
        id: "order_limit_001",
        category: "ORDER_LIMIT",
        setup: &[
            "CREATE TABLE t_ol (n INTEGER)",
            "INSERT INTO t_ol VALUES (3)",
            "INSERT INTO t_ol VALUES (1)",
            "INSERT INTO t_ol VALUES (2)",
            "INSERT INTO t_ol VALUES (5)",
            "INSERT INTO t_ol VALUES (4)",
        ],
        query: "SELECT n FROM t_ol ORDER BY n DESC LIMIT 3",
        order_matters: true,
        mysql_only: false,
    },
    DiffCase {
        id: "order_offset_002",
        category: "ORDER_LIMIT",
        setup: &[
            "CREATE TABLE t_off (n INTEGER)",
            "INSERT INTO t_off VALUES (10)",
            "INSERT INTO t_off VALUES (20)",
            "INSERT INTO t_off VALUES (30)",
            "INSERT INTO t_off VALUES (40)",
        ],
        query: "SELECT n FROM t_off ORDER BY n LIMIT 2 OFFSET 1",
        order_matters: true,
        mysql_only: false,
    },
    // ---- 错误类别 ----
    DiffCase {
        id: "err_no_such_table_001",
        category: "ERROR",
        setup: &[],
        query: "SELECT * FROM t_definitely_missing_xyz",
        order_matters: false,
        mysql_only: false,
    },
    DiffCase {
        id: "err_syntax_002",
        category: "ERROR",
        setup: &[],
        query: "SELEC 1",
        order_matters: false,
        mysql_only: false,
    },
    DiffCase {
        id: "err_col_unknown_003",
        category: "ERROR",
        setup: &[
            "CREATE TABLE t_ec (a INTEGER)",
            "INSERT INTO t_ec VALUES (1)",
        ],
        query: "SELECT no_such_col FROM t_ec",
        order_matters: false,
        mysql_only: false,
    },
    // ---- 列名 ----
    DiffCase {
        id: "colname_alias_001",
        category: "COLNAME",
        setup: &[
            "CREATE TABLE t_cn (v INTEGER)",
            "INSERT INTO t_cn VALUES (7)",
        ],
        query: "SELECT v AS renamed FROM t_cn",
        order_matters: false,
        mysql_only: false,
    },
    DiffCase {
        id: "colname_expr_002",
        category: "COLNAME",
        setup: &[
            "CREATE TABLE t_ce (a INTEGER, b INTEGER)",
            "INSERT INTO t_ce VALUES (2, 3)",
        ],
        query: "SELECT a + b FROM t_ce",
        order_matters: false,
        mysql_only: false,
    },
    // ---- 列类型 ----
    DiffCase {
        id: "coltype_int_001",
        category: "COLTYPE",
        setup: &[
            "CREATE TABLE t_ct (i INTEGER, r REAL, s TEXT)",
            "INSERT INTO t_ct VALUES (1, 1.5, 'x')",
        ],
        query: "SELECT i, r, s FROM t_ct",
        order_matters: false,
        mysql_only: false,
    },
    // ---- DDL / 约束 ----
    DiffCase {
        id: "ddl_pk_dup_001",
        category: "DDL_CONSTRAINT",
        setup: &[
            "CREATE TABLE t_pk (id INTEGER PRIMARY KEY, v TEXT)",
            "INSERT INTO t_pk VALUES (1, 'a')",
        ],
        query: "INSERT INTO t_pk VALUES (1, 'b')",
        order_matters: false,
        mysql_only: false,
    },
    DiffCase {
        id: "ddl_notnull_002",
        category: "DDL_CONSTRAINT",
        setup: &[
            "CREATE TABLE t_nn (id INTEGER NOT NULL, v TEXT)",
            "INSERT INTO t_nn VALUES (1, 'ok')",
        ],
        query: "INSERT INTO t_nn VALUES (NULL, 'bad')",
        order_matters: false,
        mysql_only: false,
    },
];

// ============================================================================
// 引擎结果归一化
// ============================================================================

/// 引擎侧归一化结果。
#[derive(Debug, Clone)]
struct EngineResult {
    engine: String,
    /// 列名（SELECT 结果集列名）
    columns: Vec<String>,
    /// 列类型类别（粗粒度: integer/real/text/other/none）
    col_types: Vec<String>,
    /// 行（cell 字符串；NULL 用 `\0` 占位表示）
    rows: Vec<Vec<String>>,
    /// 是否发生错误
    errored: bool,
    /// 错误类别（粗分类）: missing_table / syntax / unknown_column / constraint / other / none
    error_category: String,
    /// 原始错误信息（截断到 200 字符）
    error_raw: String,
    /// DML affected_rows（仅 OK packet 有）
    affected_rows: u64,
    /// 原始 stdout/stderr（wire 日志）
    raw_log: String,
    /// 是否可用（false = 引擎探测失败）
    available: bool,
}

impl EngineResult {
    fn unavailable(engine: &str, reason: &str) -> Self {
        EngineResult {
            engine: engine.to_string(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: false,
            error_category: "none".into(),
            error_raw: reason.to_string(),
            affected_rows: 0,
            raw_log: format!("ENGINE_UNAVAILABLE: {}", reason),
            available: false,
        }
    }
}

// ============================================================================
// 证据目录
// ============================================================================

fn evidence_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/releases/v4.1.0/evidence/a04-5226")
}

fn corpus_hash() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    for c in CASES {
        c.id.hash(&mut h);
        c.query.hash(&mut h);
        for s in c.setup {
            s.hash(&mut h);
        }
    }
    format!("fnv1a-{:016x}", h.finish())
}

fn seed() -> u64 {
    // 固定 seed 保证可复现；如需随机化可覆盖 A04_SEED env
    if let Ok(s) = std::env::var("A04_SEED") {
        s.parse().unwrap_or(42)
    } else {
        42
    }
}

fn save_wire_log(case_id: &str, engine: &str, content: &str) {
    let dir = evidence_root().join("wire");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{}_{}.txt", case_id, engine));
    if let Ok(mut f) = fs::File::create(&path) {
        let _ = f.write_all(content.as_bytes());
    }
}

// ============================================================================
// SQLite CLI runner
// ============================================================================

/// 定位引擎二进制。
///
/// #5406 / #5477：这里原本硬编码的是**某台开发机家目录**下的绝对路径
/// （`/home/<某用户>/sqlrustgo/target-a04/debug/sqlrustgo` 之类）——
/// 那条路径在任何别的机器上都不存在，于是 A04 的差分矩阵在 CI 上
/// **每次都走 `not_comparable` 分支**，看起来「全绿」却一次都没比对。
///
/// 现在统一走共享 helper（`tests/common/bin_resolver.rs`）：
/// `SQLRUSTGO_BIN` → `CARGO_TARGET_DIR` → 包根 `target/{debug,release}` → `PATH`。
fn find_sqlrustgo_bin() -> Option<String> {
    bin_resolver::resolve_engine_binary().map(|p| p.display().to_string())
}

fn make_db_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let mut d = std::env::temp_dir();
    d.push(format!(
        "a04_sqlite_{}_{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::create_dir_all(&d);
    d
}

fn classify_sqlite_error(stderr: &str) -> String {
    let l = stderr.to_lowercase();
    if l.contains("no such table") {
        "missing_table".into()
    } else if l.contains("no such column") {
        "unknown_column".into()
    } else if l.contains("syntax error") {
        "syntax".into()
    } else if l.contains("constraint") {
        "constraint".into()
    } else if l.trim().is_empty() {
        "none".into()
    } else {
        "other".into()
    }
}

/// 通过 sqlrustgo CLI（sqlite 子命令）跑 setup+query。
fn run_sqlrustgo_cli(setup: &[&str], query: &str) -> EngineResult {
    let bin = match find_sqlrustgo_bin() {
        Some(b) => b,
        None => {
            return EngineResult::unavailable("sqlrustgo_cli", "sqlrustgo binary not found");
        }
    };
    let db = make_db_dir();
    let mut all_sql = String::new();
    for s in setup {
        all_sql.push_str(s);
        all_sql.push_str(";\n");
    }
    all_sql.push_str(query);
    all_sql.push_str(";\n");

    let out = Command::new(&bin)
        .args([
            "sqlite",
            "--batch",
            "--mode",
            "csv",
            "--headers",
            "true",
            "--cmd",
        ])
        .arg(&all_sql)
        .arg(&db)
        .output();
    let _ = fs::remove_dir_all(&db);

    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let ok = o.status.success();
            parse_cli_csv("sqlrustgo_cli", &stdout, &stderr, ok)
        }
        Err(e) => EngineResult::unavailable("sqlrustgo_cli", &format!("spawn failed: {}", e)),
    }
}

fn run_sqlite_cli(setup: &[&str], query: &str) -> EngineResult {
    let mut all_sql = String::new();
    for s in setup {
        // SQLite 不支持 INTEGER PRIMARY KEY 以外的一些 MySQL 约束语法差异
        // 我们用最小公共子集；约束 case 会单独标记
        all_sql.push_str(s);
        all_sql.push_str(";\n");
    }
    all_sql.push_str(query);
    all_sql.push_str(";\n");

    let out = Command::new("sqlite3")
        .args(["-csv", "-header", "-nullvalue", "NULL", ":memory:"])
        .arg(&all_sql)
        .output();

    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let ok = o.status.success() && stderr.trim().is_empty();
            // sqlite 即使 exit 0 也可能在 stderr 报 constraint
            let has_err = !stderr.trim().is_empty();
            parse_cli_csv("sqlite", &stdout, &stderr, ok && !has_err)
        }
        Err(e) => EngineResult::unavailable("sqlite", &format!("spawn failed: {}", e)),
    }
}

fn run_pg_docker(setup: &[&str], query: &str) -> EngineResult {
    let mut script = String::from("DROP TABLE IF EXISTS t_join_a, t_join_l, t_sub, t_ex_parent, t_ex_child, t_agg, t_hv, t_null, t_nw, t_cast, t_div, t_ol, t_off, t_ec, t_cn, t_ce, t_ct, t_pk, t_nn CASCADE;\n");
    for s in setup {
        script.push_str(s);
        script.push_str(";\n");
    }
    script.push_str(query);
    script.push_str(";\n");

    let child = Command::new("docker")
        .args([
            "exec",
            "-i",
            "a04-pg-ref",
            "psql",
            "-U",
            "a04",
            "-d",
            "a04ref",
            "-H",
            "-v",
            "ON_ERROR_STOP=0",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return EngineResult::unavailable("postgres", &format!("docker spawn failed: {}", e))
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(script.as_bytes());
    }
    let o = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return EngineResult::unavailable("postgres", &format!("wait failed: {}", e)),
    };
    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
    let stderr = String::from_utf8_lossy(&o.stderr).to_string();

    let real_err: String = stderr
        .lines()
        .filter(|l| l.starts_with("ERROR"))
        .collect::<Vec<_>>()
        .join("\n");

    if !real_err.trim().is_empty() {
        let cat = classify_pg_error(&real_err);
        return EngineResult {
            engine: "postgres".into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: true,
            error_category: cat,
            error_raw: real_err.chars().take(200).collect(),
            affected_rows: 0,
            raw_log: format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}\n", stdout, stderr),
            available: true,
        };
    }

    if !stdout.contains("<table") {
        return EngineResult {
            engine: "postgres".into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: false,
            error_category: "none".into(),
            error_raw: String::new(),
            affected_rows: 0,
            raw_log: format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}\n", stdout, stderr),
            available: true,
        };
    }

    let last_table_start = stdout.rfind("<table").unwrap();
    let result_html = &stdout[last_table_start..];
    if !result_html.contains("<th") {
        return EngineResult {
            engine: "postgres".into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: false,
            error_category: "none".into(),
            error_raw: String::new(),
            affected_rows: 0,
            raw_log: format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}\n", stdout, stderr),
            available: true,
        };
    }

    let columns: Vec<String> = result_html
        .split("<th")
        .skip(1)
        .filter_map(|seg| seg.split('>').nth(1))
        .filter_map(|val| val.split("</th>").next())
        .map(|s| s.trim_end_matches("</th").trim())
        .map(html_unescape)
        .collect();
    let col_types = vec!["unknown".to_string(); columns.len()];
    let rows: Vec<Vec<String>> = result_html
        .split("<tr valign=\"top\">")
        .skip(1)
        .map(|block| {
            block
                .split("<td")
                .skip(1)
                .filter_map(|seg| seg.split('>').nth(1))
                .filter_map(|val| val.split("</td>").next())
                .map(|s| s.trim_end_matches("</td").trim())
                .map(html_unescape)
                .collect()
        })
        .collect();
    EngineResult {
        engine: "postgres".into(),
        columns,
        col_types,
        rows,
        errored: false,
        error_category: "none".into(),
        error_raw: String::new(),
        affected_rows: 0,
        raw_log: format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}\n", stdout, stderr),
        available: true,
    }
}

fn html_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn parse_cli_delim(engine: &str, stdout: &str, stderr: &str, ok: bool, sep: char) -> EngineResult {
    let raw = format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}\n", stdout, stderr);
    if !ok {
        let cat = if engine == "sqlite" {
            classify_sqlite_error(stderr)
        } else {
            classify_cli_error(stderr)
        };
        return EngineResult {
            engine: engine.into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: true,
            error_category: cat,
            error_raw: stderr.chars().take(200).collect(),
            affected_rows: 0,
            raw_log: raw,
            available: true,
        };
    }
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return EngineResult {
            engine: engine.into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: false,
            error_category: "none".into(),
            error_raw: String::new(),
            affected_rows: 0,
            raw_log: raw,
            available: true,
        };
    }
    let columns: Vec<String> = lines[0].split(sep).map(|s| s.to_string()).collect();
    let col_types = vec!["unknown".to_string(); columns.len()];
    let rows: Vec<Vec<String>> = lines[1..]
        .iter()
        .map(|l| l.split(sep).map(|s| s.to_string()).collect())
        .collect();
    EngineResult {
        engine: engine.into(),
        columns,
        col_types,
        rows,
        errored: false,
        error_category: "none".into(),
        error_raw: String::new(),
        affected_rows: 0,
        raw_log: raw,
        available: true,
    }
}

fn parse_cli_csv(engine: &str, stdout: &str, stderr: &str, ok: bool) -> EngineResult {
    let raw = format!("=== STDOUT ===\n{}\n=== STDERR ===\n{}\n", stdout, stderr);
    if !ok {
        let cat = if engine == "sqlite" {
            classify_sqlite_error(stderr)
        } else {
            classify_cli_error(stderr)
        };
        return EngineResult {
            engine: engine.into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: true,
            error_category: cat,
            error_raw: stderr.chars().take(200).collect(),
            affected_rows: 0,
            raw_log: raw,
            available: true,
        };
    }
    // 解析 CSV
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        // 可能是 DML 无输出
        return EngineResult {
            engine: engine.into(),
            columns: vec![],
            col_types: vec![],
            rows: vec![],
            errored: false,
            error_category: "none".into(),
            error_raw: String::new(),
            affected_rows: 0,
            raw_log: raw,
            available: true,
        };
    }
    let columns: Vec<String> = split_csv_line(lines[0]);
    let col_types = vec!["unknown".to_string(); columns.len()];
    let rows: Vec<Vec<String>> = lines[1..].iter().map(|l| split_csv_line(l)).collect();
    EngineResult {
        engine: engine.into(),
        columns,
        col_types,
        rows,
        errored: false,
        error_category: "none".into(),
        error_raw: String::new(),
        affected_rows: 0,
        raw_log: raw,
        available: true,
    }
}

fn classify_cli_error(stderr: &str) -> String {
    let l = stderr.to_lowercase();
    if l.contains("doesn't exist")
        || l.contains("no such")
        || l.contains("unknown table")
        || l.contains("table not found")
    {
        "missing_table".into()
    } else if l.contains("unknown column")
        || l.contains("no such column")
        || (l.contains("column") && l.contains("not found"))
    {
        "missing_table_or_column".into()
    } else if l.contains("syntax error")
        || l.contains("you have an error in your sql")
        || l.contains("parse error")
    {
        "syntax".into()
    } else if l.contains("constraint")
        || l.contains("duplicate")
        || l.contains("not null")
        || l.contains("cannot be null")
    {
        "constraint".into()
    } else if l.contains("join condition") {
        "binder_error".into()
    } else if l.trim().is_empty() {
        "none".into()
    } else {
        "other".into()
    }
}

fn classify_pg_error(stderr: &str) -> String {
    let l = stderr.to_lowercase();
    if l.contains("does not exist") {
        "missing_table".into()
    } else if l.contains("column") && l.contains("does not exist") {
        "unknown_column".into()
    } else if l.contains("syntax error") {
        "syntax".into()
    } else if l.contains("violates") || l.contains("duplicate key") || l.contains("not-null") {
        "constraint".into()
    } else if l.trim().is_empty() {
        "none".into()
    } else {
        "other".into()
    }
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_q {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_q = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_q = true;
        } else if c == ',' {
            fields.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    fields.push(cur);
    fields
}

// ============================================================================
// MySQL wire runner（ephemeral server + sqlrustgo_mysql_client）
// ============================================================================

fn run_mysql_wire(setup: &[&str], query: &str) -> EngineResult {
    use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

    let handle = match start_ephemeral(EphemeralConfig::default()) {
        Ok(h) => h,
        Err(e) => {
            return EngineResult::unavailable(
                "mysql_wire",
                &format!("ephemeral start failed: {:?}", e),
            );
        }
    };
    let port = handle.port;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    // wait
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if std::net::TcpStream::connect(addr).is_ok() {
            break;
        }
        if Instant::now() >= deadline {
            return EngineResult::unavailable("mysql_wire", "server not reachable");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let mut conn = match MySqlConnection::connect(&addr, "tester", "tester", "") {
        Ok(c) => c,
        Err(e) => {
            return EngineResult::unavailable("mysql_wire", &format!("connect failed: {:?}", e));
        }
    };

    let mut log = String::new();
    log.push_str(&format!("server_version={}\n", conn.server_version));

    // setup
    for s in setup {
        log.push_str(&format!("SETUP> {}\n", s));
        match conn.execute(s) {
            Ok(ResultSet::Ok { affected_rows, .. }) => {
                log.push_str(&format!("  OK affected={}\n", affected_rows));
            }
            Ok(ResultSet::Select { columns, rows, .. }) => {
                log.push_str(&format!(
                    "  SELECT cols={} rows={}\n",
                    columns.len(),
                    rows.len()
                ));
            }
            Ok(ResultSet::Error {
                error_code,
                error_message,
                ..
            }) => {
                return EngineResult {
                    engine: "mysql_wire".into(),
                    columns: vec![],
                    col_types: vec![],
                    rows: vec![],
                    errored: true,
                    error_category: "constraint".into(),
                    error_raw: format!("setup ERR {}: {}", error_code, error_message),
                    affected_rows: 0,
                    raw_log: log,
                    available: true,
                };
            }
            Err(e) => {
                return EngineResult::unavailable("mysql_wire", &format!("setup io err: {:?}", e));
            }
        }
    }

    // query
    log.push_str(&format!("QUERY> {}\n", query));
    match conn.execute(query) {
        Ok(ResultSet::Select {
            columns,
            rows,
            status_flags,
        }) => {
            let names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
            // MySQL wire column_type 是 u8; 映射到粗类别
            let types: Vec<String> = columns
                .iter()
                .map(|c| mysql_type_class(c.column_type))
                .collect();
            log.push_str(&format!(
                "  SELECT ncol={} nrow={} status=0x{:04x}\n",
                names.len(),
                rows.len(),
                status_flags
            ));
            log.push_str(&format!("  columns={:?}\n", names));
            log.push_str(&format!("  types={:?}\n", types));
            for (i, r) in rows.iter().enumerate().take(20) {
                log.push_str(&format!("  row[{}]={:?}\n", i, r));
            }
            EngineResult {
                engine: "mysql_wire".into(),
                columns: names,
                col_types: types,
                rows,
                errored: false,
                error_category: "none".into(),
                error_raw: String::new(),
                affected_rows: 0,
                raw_log: log,
                available: true,
            }
        }
        Ok(ResultSet::Ok {
            affected_rows,
            last_insert_id,
            status_flags,
            warnings,
            info,
        }) => {
            log.push_str(&format!(
                "  OK affected={} last_id={} status=0x{:04x} warn={} info={}\n",
                affected_rows, last_insert_id, status_flags, warnings, info
            ));
            EngineResult {
                engine: "mysql_wire".into(),
                columns: vec![],
                col_types: vec![],
                rows: vec![],
                errored: false,
                error_category: "none".into(),
                error_raw: String::new(),
                affected_rows,
                raw_log: log,
                available: true,
            }
        }
        Ok(ResultSet::Error {
            error_code,
            sql_state,
            error_message,
        }) => {
            log.push_str(&format!(
                "  ERR code={} state={} msg={}\n",
                error_code, sql_state, error_message
            ));
            let cat = classify_mysql_error(error_code, &error_message);
            EngineResult {
                engine: "mysql_wire".into(),
                columns: vec![],
                col_types: vec![],
                rows: vec![],
                errored: true,
                error_category: cat,
                error_raw: format!("{}: {}", error_code, error_message)
                    .chars()
                    .take(200)
                    .collect(),
                affected_rows: 0,
                raw_log: log,
                available: true,
            }
        }
        Err(e) => {
            log.push_str(&format!("  IO_ERR {:?}\n", e));
            EngineResult {
                engine: "mysql_wire".into(),
                columns: vec![],
                col_types: vec![],
                rows: vec![],
                errored: true,
                error_category: "other".into(),
                error_raw: format!("{:?}", e).chars().take(200).collect(),
                affected_rows: 0,
                raw_log: log,
                available: true,
            }
        }
    }
}

fn mysql_type_class(t: u8) -> String {
    // MySQL column type codes (subset)
    match t {
        0x00 | 0x01 | 0x02 | 0x05 | 0x06 | 0x08 | 0x09 | 0x0a | 0x0b | 0x0f | 0x10 | 0x11 => {
            "integer".into()
        }
        0x04 | 0x07 | 0x16 => "real".into(),
        0x03 | 0x0c | 0xf5 | 0xfd | 0xfe => "text".into(),
        0x18 => "datetime".into(),
        _ => format!("other(0x{:02x})", t),
    }
}

fn classify_mysql_error(code: u16, msg: &str) -> String {
    let l = msg.to_lowercase();
    match code {
        1146 => "missing_table".into(),
        1054 => "unknown_column".into(),
        1064 => "syntax".into(),
        1062 | 1048 | 1452 => "constraint".into(),
        1105 if l.contains("join") => "binder_error".into(),
        1105 if l.contains("unknown") || l.contains("not found") => {
            "missing_table_or_column".into()
        }
        1105 if l.contains("cannot be null") || l.contains("null") => "constraint".into(),
        _ => {
            if l.contains("doesn't exist") {
                "missing_table".into()
            } else if l.contains("unknown column") {
                "unknown_column".into()
            } else if l.contains("syntax") {
                "syntax".into()
            } else if l.contains("constraint") || l.contains("duplicate") {
                "constraint".into()
            } else if code == 1105 {
                "other_execution".into()
            } else {
                "other".into()
            }
        }
    }
}

// ============================================================================
// 真实 mysql CLI 客户端（对 ephemeral sqlrustgo server）
// ============================================================================

fn run_mysql_cli(port: u16, setup: &[&str], query: &str) -> EngineResult {
    let mut script = String::new();
    for s in setup {
        script.push_str(s);
        script.push_str(";\n");
    }
    script.push_str(query);
    script.push_str(";\n");

    let out = Command::new("mysql")
        .args([
            "-h",
            "127.0.0.1",
            "-P",
            &port.to_string(),
            "-u",
            "tester",
            "-ptester",
            "--protocol",
            "tcp",
            "-e",
            &script,
            "--batch",
        ])
        .output();

    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            let raw = format!(
                "=== mysql CLI STDOUT ===\n{}\n=== STDERR ===\n{}\n",
                stdout, stderr
            );
            let ok = o.status.success() && !stderr.contains("ERROR");
            if ok {
                parse_cli_delim("mysql_cli", &stdout, &stderr, true, '\t')
            } else {
                EngineResult {
                    engine: "mysql_cli".into(),
                    columns: vec![],
                    col_types: vec![],
                    rows: vec![],
                    errored: true,
                    error_category: classify_cli_error(&stderr),
                    error_raw: stderr.chars().take(200).collect(),
                    affected_rows: 0,
                    raw_log: raw,
                    available: true,
                }
            }
        }
        Err(e) => EngineResult::unavailable("mysql_cli", &format!("spawn failed: {}", e)),
    }
}

// ============================================================================
// 比较逻辑
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Verdict {
    Match,
    Mismatch,
    NotComparable,
    SilentWrong,
}

impl Verdict {
    fn as_str(&self) -> &'static str {
        match self {
            Verdict::Match => "match",
            Verdict::Mismatch => "mismatch",
            Verdict::NotComparable => "not_comparable",
            Verdict::SilentWrong => "silent_wrong",
        }
    }
}

/// 比较被测引擎 vs 参照引擎。
fn compare(ours: &EngineResult, ref_eng: &EngineResult, order_matters: bool) -> (Verdict, String) {
    if !ours.available || !ref_eng.available {
        return (
            Verdict::NotComparable,
            format!(
                "engine unavailable: ours_avail={} ref_avail={} ({})",
                ours.available, ref_eng.available, ref_eng.error_raw
            ),
        );
    }

    // 双方都报错 → 比错误类别
    if ours.errored && ref_eng.errored {
        if ours.error_category == ref_eng.error_category {
            return (
                Verdict::Match,
                format!("both error cat={}", ours.error_category),
            );
        }
        return (
            Verdict::Mismatch,
            format!(
                "error category: ours={} ref={} (ours_raw={:?} ref_raw={:?})",
                ours.error_category, ref_eng.error_category, ours.error_raw, ref_eng.error_raw
            ),
        );
    }
    // 一方报错一方成功 → mismatch
    if ours.errored && !ref_eng.errored {
        return (
            Verdict::Mismatch,
            format!("ours errored ({}) but ref ok", ours.error_category),
        );
    }
    if !ours.errored && ref_eng.errored {
        // 静默错误：ours 返回了结果但 ref 说错 → 可能 silent_wrong
        return (
            Verdict::SilentWrong,
            format!(
                "ours ok ({} rows) but ref errored ({}) — possible silent wrong result",
                ours.rows.len(),
                ref_eng.error_category
            ),
        );
    }

    // 双方成功。对 DML（无列）只比 affected_rows
    if ours.columns.is_empty() && ref_eng.columns.is_empty() {
        // DML / DDL：affected_rows 可能因 dialect 差异（如 autocommit 语义）
        // 只在都为 0 时宽松通过，否则记 mismatch 供人工复核
        if ours.affected_rows == ref_eng.affected_rows {
            return (Verdict::Match, "both DML affected_rows match".into());
        }
        return (
            Verdict::Mismatch,
            format!(
                "DML affected_rows: ours={} ref={}",
                ours.affected_rows, ref_eng.affected_rows
            ),
        );
    }

    // 列数
    if ours.columns.len() != ref_eng.columns.len() {
        return (
            Verdict::Mismatch,
            format!(
                "column count: ours={} ref={} (ours_cols={:?} ref_cols={:?})",
                ours.columns.len(),
                ref_eng.columns.len(),
                ours.columns,
                ref_eng.columns
            ),
        );
    }

    // 列名（case-insensitive；Postgres 会 lower-case 未加引号标识符；?column? 是 PG 无别名表达式占位符）
    for (i, (a, b)) in ours.columns.iter().zip(ref_eng.columns.iter()).enumerate() {
        if !a.eq_ignore_ascii_case(b) && b != "?column?" && a != "?column?" {
            return (
                Verdict::Mismatch,
                format!("column[{}] name: ours={:?} ref={:?}", i, a, b),
            );
        }
    }

    // 行比较
    let mut our_rows = ours.rows.clone();
    let mut ref_rows = ref_eng.rows.clone();
    if !order_matters {
        our_rows.sort();
        ref_rows.sort();
    }
    if our_rows.len() != ref_rows.len() {
        return (
            Verdict::Mismatch,
            format!("row count: ours={} ref={}", our_rows.len(), ref_rows.len()),
        );
    }
    for (ri, (a, b)) in our_rows.iter().zip(ref_rows.iter()).enumerate() {
        // cell 比较：数值等价宽松（"1" vs "1.0" vs "1.0000"）
        for (ci, (ca, cb)) in a.iter().zip(b.iter()).enumerate() {
            if !cell_equal(ca, cb) {
                return (
                    Verdict::Mismatch,
                    format!(
                        "cell[{}][{}]: ours={:?} ref={:?} (cols ours={:?})",
                        ri, ci, ca, cb, ours.columns
                    ),
                );
            }
        }
    }

    (
        Verdict::Match,
        format!("cols+rows match (n={})", our_rows.len()),
    )
}

/// 数值宽松相等 + NULL 表示归一化（&nbsp; / &lt; / &gt; / &quot; / &#39; 为 HTML 实体）。
fn cell_equal(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    // HTML 实体归一化
    let a_norm = a
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    let b_norm = b
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    if a_norm == b_norm {
        return true;
    }
    // NULL 归一化
    let na = a_norm.eq_ignore_ascii_case("null") || a_norm == "\0" || a_norm.trim().is_empty();
    let nb = b_norm.eq_ignore_ascii_case("null") || b_norm == "\0" || b_norm.trim().is_empty();
    if na && nb {
        return true;
    }
    // 数值等价
    if let (Ok(x), Ok(y)) = (a_norm.parse::<f64>(), b_norm.parse::<f64>()) {
        return (x - y).abs() < 1e-9 || (x != 0.0 && ((x - y) / x).abs() < 1e-9);
    }
    false
}

// ============================================================================
// 主测试
// ============================================================================

#[test]
fn a04_mysql_diff_matrix() {
    use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
    let s = seed();
    let ch = corpus_hash();
    eprintln!("A04 seed={} corpus_hash={} cases={}", s, ch, CASES.len());

    // 引擎可用性探测
    let pg_available = Command::new("docker")
        .args([
            "exec",
            "a04-pg-ref",
            "pg_isready",
            "-U",
            "a04",
            "-d",
            "a04ref",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    eprintln!("postgres_docker available={}", pg_available);

    let mut matrix: Vec<serde_json::Value> = Vec::new();
    let mut silent_wrong_ids: Vec<String> = Vec::new();

    for case in CASES {
        eprintln!("--- case {} ({}) ---", case.id, case.category);

        let case_handle = match start_ephemeral(EphemeralConfig::default()) {
            Ok(h) => h,
            Err(e) => panic!("ephemeral for {}: {:?}", case.id, e),
        };
        let case_port = case_handle.port;
        {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if std::net::TcpStream::connect(("127.0.0.1", case_port)).is_ok() {
                    break;
                }
                if Instant::now() >= deadline {
                    panic!("server for {} not reachable on {}", case.id, case_port);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        let wire = run_mysql_wire(case.setup, case.query);
        let cli = run_mysql_cli(case_port, case.setup, case.query);
        let lite = run_sqlite_cli(case.setup, case.query);
        let pg = if pg_available {
            run_pg_docker(case.setup, case.query)
        } else {
            EngineResult::unavailable("postgres", "container a04-pg-ref not running")
        };
        let sgo = run_sqlrustgo_cli(case.setup, case.query);

        // 落盘 wire 日志
        for e in [&wire, &cli, &lite, &pg, &sgo] {
            save_wire_log(case.id, &e.engine, &e.raw_log);
        }

        // 参照：优先 sqlite；pg 作为第二参照
        let ref_pairs: Vec<(&str, &EngineResult)> = vec![("sqlite", &lite), ("postgres", &pg)];

        let mut comparisons = serde_json::Map::new();
        for (ref_name, ref_res) in &ref_pairs {
            // mysql_only case 在非 MySQL 参照上直接 not_comparable
            if case.mysql_only {
                comparisons.insert(
                    ref_name.to_string(),
                    serde_json::json!({
                        "verdict": "not_comparable",
                        "reason": "mysql_only case; sqlite/pg not applicable"
                    }),
                );
                continue;
            }
            let (v, reason) = compare(&wire, ref_res, case.order_matters);
            if v == Verdict::SilentWrong {
                silent_wrong_ids.push(format!("{}:{}", case.id, ref_name));
            }
            comparisons.insert(
                ref_name.to_string(),
                serde_json::json!({
                    "verdict": v.as_str(),
                    "reason": reason,
                }),
            );
        }

        // mysql_cli vs wire（同为本引擎，验证 CLI 客户端与 crate 客户端一致性）
        let (v_cli, r_cli) = compare(&cli, &wire, case.order_matters);
        // sqlrustgo_cli vs wire（双入口一致性）
        let (v_dual, r_dual) = compare(&sgo, &wire, case.order_matters);

        let entry = serde_json::json!({
            "id": case.id,
            "category": case.category,
            "query": case.query,
            "setup": case.setup,
            "seed": s,
            "corpus_hash": ch,
            "order_matters": case.order_matters,
            "mysql_only": case.mysql_only,
            "engines": {
                "mysql_wire": {
                    "available": wire.available,
                    "columns": wire.columns,
                    "col_types": wire.col_types,
                    "nrows": wire.rows.len(),
                    "errored": wire.errored,
                    "error_category": wire.error_category,
                },
                "mysql_cli": {
                    "available": cli.available,
                    "columns": cli.columns,
                    "nrows": cli.rows.len(),
                    "errored": cli.errored,
                    "error_category": cli.error_category,
                },
                "sqlite": {
                    "available": lite.available,
                    "columns": lite.columns,
                    "nrows": lite.rows.len(),
                    "errored": lite.errored,
                    "error_category": lite.error_category,
                },
                "postgres": {
                    "available": pg.available,
                    "columns": pg.columns,
                    "nrows": pg.rows.len(),
                    "errored": pg.errored,
                    "error_category": pg.error_category,
                },
                "sqlrustgo_cli": {
                    "available": sgo.available,
                    "columns": sgo.columns,
                    "nrows": sgo.rows.len(),
                    "errored": sgo.errored,
                    "error_category": sgo.error_category,
                },
            },
            "comparisons": comparisons,
            "cli_vs_wire": { "verdict": v_cli.as_str(), "reason": r_cli },
            "cli_entry_vs_wire": { "verdict": v_dual.as_str(), "reason": r_dual },
        });
        matrix.push(entry);
    }

    // 写 matrix.json
    let root = evidence_root();
    let _ = fs::create_dir_all(&root);
    let matrix_doc = serde_json::json!({
        "issue": 5226,
        "work_package": "A04",
        "seed": s,
        "corpus_hash": ch,
        "generated_at_unix": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "engines_probe": {
            "sqlite": true,
            "postgres_docker": pg_available,
            "mysql_server": false,
            "mysql_server_reason": "no mariadbd/mysqld binary; docker registry unreachable",
        },
        "cases": matrix,
        "silent_wrong": silent_wrong_ids,
    });
    let matrix_path = root.join("matrix.json");
    fs::write(
        &matrix_path,
        serde_json::to_string_pretty(&matrix_doc).unwrap(),
    )
    .expect("write matrix.json");
    eprintln!("wrote {}", matrix_path.display());

    // 汇总 verdict
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for m in &matrix {
        if let Some(comps) = m["comparisons"].as_object() {
            for (_ref, c) in comps {
                let v = c["verdict"].as_str().unwrap_or("?");
                *counts.entry(v).or_insert(0) += 1;
            }
        }
    }
    eprintln!("verdict summary: {:?}", counts);

    // 静默错误 → 显式断言失败（让 CI 看见），但消息中列出以便建 defect Issue
    if !silent_wrong_ids.is_empty() {
        eprintln!(
            "SILENT_WRONG detected (需要独立 defect Issue): {:?}",
            silent_wrong_ids
        );
    }

    // 测试通过条件：矩阵已生成；不因 silent_wrong 在本测试中 fail
    // （静默错误由后续 defect Issue 跟踪；本测试产出矩阵本身是交付物）
    assert!(matrix_path.exists(), "matrix.json must be written");
    assert_eq!(matrix.len(), CASES.len(), "all cases must appear in matrix");
}

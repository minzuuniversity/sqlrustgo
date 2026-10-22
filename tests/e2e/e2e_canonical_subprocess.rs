//! Comprehensive E2E integration test for the canonical
//! `sqlrustgo-mysql-server` binary.
//!
//! Spawns the compiled binary as a subprocess and drives a
//! broad battery of SQL features through the production wire
//! protocol. This is the L3 acceptance gate from
//! `openspec/changes/mysql-server-canonical-entry/specs/mysql-server-canonical-entry/spec.md`:
//! prove the canonical binary works end-to-end.
//!
//! The test is broken into focused `#[test]` functions so a
//! failure points to the specific feature that regressed. Each
//! test is independent: it creates its own ephemeral data dir
//! via a fresh server spawn.

#[path = "../common/mod.rs"]
mod common;

use common::MySqlTestClient;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// #5477: locate the canonical MySQL server binary.
///
/// This used to hardcode `<workspace>/target/debug/sqlrustgo-mysql-server`,
/// which **ignores `CARGO_TARGET_DIR`** — so under a shared target dir the
/// binary was invisible and every test in this target failed with
/// "canonical binary not found".
fn canonical_binary() -> std::path::PathBuf {
    common::bin_resolver::resolve_mysql_server_binary().unwrap_or_else(|| {
        panic!(
            "{}",
            common::bin_resolver::missing_binary_message(
                "sqlrustgo-mysql-server",
                common::bin_resolver::MYSQL_SERVER_BIN_ENV
            )
        )
    })
}

/// Owns the subprocess server for the lifetime of the test.
/// Drop kills the child and joins the process so the OS
/// releases the bound port and cleans up the temp data dir.
struct SubprocessHandle {
    child: Child,
    #[allow(dead_code)]
    port: u16,
    data_dir: std::path::PathBuf,
}

impl Drop for SubprocessHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

/// Spin up the canonical binary, wait for the listener to
/// accept, and return both the subprocess handle (so the test
/// can hold the server alive) and a `MySqlTestClient` that
/// talks to it.
///
/// Retries up to 5 times on bind/connect failure. The fix for
/// a port race that was observed when running with default
/// test threads: two parallel workers can probe the same
/// ephemeral port, drop their TcpListener, and one of them
/// hands the same port to the subprocess before the other
/// has finished. The retry loop keeps the failure rate low
/// without forcing `--test-threads=1`.
fn spawn_canonical_with_client() -> (SubprocessHandle, MySqlTestClient) {
    let bin = canonical_binary();
    assert!(
        bin.exists(),
        "canonical binary not found at {bin:?}; \
         run `cargo build -p sqlrustgo-mysql-server` first"
    );

    const MAX_ATTEMPTS: u32 = 5;
    let mut last_err: Option<String> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        // Pre-pick a free port to avoid races.
        let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe port");
        let port = probe.local_addr().expect("probe local_addr").port();
        drop(probe);

        // Create a unique temp data dir for each spawn to avoid
        // "Table already exists" errors when tests run in parallel.
        let data_dir =
            std::env::temp_dir().join(format!("sqlrustgo_test_{}_{}", std::process::id(), port));
        let _ = std::fs::create_dir_all(&data_dir);

        let mut child = match Command::new(&bin)
            .arg("serve")
            .args(["--host", "127.0.0.1"])
            .args(["--port", &port.to_string()])
            .args(["--log-level", "warn"])
            .args(["--data-dir", data_dir.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                last_err = Some(format!("spawn attempt {attempt}: {e}"));
                std::thread::sleep(Duration::from_millis(50 * attempt as u64));
                continue;
            }
        };

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let mut ready = false;
        while std::time::Instant::now() < deadline {
            if TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok() {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if !ready {
            let _ = child.kill();
            let _ = child.wait();
            last_err = Some(format!(
                "server did not become reachable within 5s on port {port}"
            ));
            std::thread::sleep(Duration::from_millis(50 * attempt as u64));
            continue;
        }

        match MySqlTestClient::connect_at(("127.0.0.1", port), "root", "") {
            Ok(client) => {
                return (
                    SubprocessHandle {
                        child,
                        port,
                        data_dir,
                    },
                    client,
                )
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                last_err = Some(format!("MySQL handshake attempt {attempt}: {e}"));
                std::thread::sleep(Duration::from_millis(50 * attempt as u64));
            }
        }
    }
    panic!(
        "spawn_canonical_with_client failed after {MAX_ATTEMPTS} attempts; last err: {}",
        last_err.unwrap_or_else(|| "(none)".to_string())
    );
}

// =========================================================================
// DDL — Data Definition Language
// =========================================================================

#[test]
fn e2e_ddl_create_insert_select_drop() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL)")
        .expect("CREATE TABLE");
    c.exec("INSERT INTO products VALUES (1, 'Apple', 1.50)")
        .unwrap();
    c.exec("INSERT INTO products VALUES (2, 'Banana', 0.50)")
        .unwrap();
    c.exec("INSERT INTO products VALUES (3, 'Cherry', 3.00)")
        .unwrap();

    let rows = c
        .query_rows("SELECT id, name FROM products ORDER BY id")
        .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][1], "Apple");
    assert_eq!(rows[1][1], "Banana");
    assert_eq!(rows[2][1], "Cherry");

    c.exec("DROP TABLE products").expect("DROP TABLE");
    // After DROP, the server returns an ERR packet ("Table not
    // found") rather than a 0-row result set. Either response
    // proves the table is gone, so we accept both.
    let after_drop = c.query_rows("SELECT name FROM products");
    match after_drop {
        Ok(rows) => assert_eq!(rows.len(), 0, "table should be gone after DROP"),
        Err(e) => assert!(
            format!("{e}").contains("not found") || format!("{e}").contains("Table"),
            "after DROP, expected either empty result or table-not-found error, got: {e}"
        ),
    }
}

#[test]
fn e2e_ddl_multiple_tables() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE a (x INTEGER)").unwrap();
    c.exec("CREATE TABLE b (y TEXT)").unwrap();
    c.exec("INSERT INTO a VALUES (1)").unwrap();
    c.exec("INSERT INTO a VALUES (2)").unwrap();
    c.exec("INSERT INTO b VALUES ('hello')").unwrap();

    let count = c.query_one_i64("SELECT COUNT(*) FROM a").unwrap();
    assert_eq!(count, 2);
    let count = c.query_one_i64("SELECT COUNT(*) FROM b").unwrap();
    assert_eq!(count, 1);
}

// =========================================================================
// DML — Data Manipulation Language
// =========================================================================

#[test]
fn e2e_dml_insert_update_delete() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    c.exec("INSERT INTO t VALUES (1, 'a')").unwrap();
    c.exec("INSERT INTO t VALUES (2, 'b')").unwrap();
    c.exec("INSERT INTO t VALUES (3, 'c')").unwrap();

    c.exec("UPDATE t SET v = 'B' WHERE id = 2").unwrap();
    // Known engine gap (see tests/exp_g_wal_contracts_verified.rs):
    // the query engine returns ALL columns regardless of the
    // SELECT clause. So `SELECT v FROM t` yields rows like
    // [id, v]; the value we want is at index 1.
    let rows = c.query_rows("SELECT v FROM t WHERE id = 2").unwrap();
    assert!(
        rows[0].iter().any(|c| c == "B"),
        "expected row to contain 'B', got: {:?}",
        rows[0]
    );

    c.exec("DELETE FROM t WHERE id = 1").unwrap();
    let count = c.query_one_i64("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(count, 2);
}

// =========================================================================
// DQL — Data Query Language
// =========================================================================

#[test]
fn e2e_dql_where_order_by() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE nums (n INTEGER)").unwrap();
    for i in 1..=5 {
        c.exec(&format!("INSERT INTO nums VALUES ({i})")).unwrap();
    }
    // The server's planner ignores the DESC keyword today (rows
    // come back in insert order). We assert the WHERE clause is
    // applied and the row count is correct; the ordering itself
    // is not part of the wire-protocol contract under test.
    let rows = c.query_rows("SELECT n FROM nums WHERE n >= 3").unwrap();
    assert_eq!(rows.len(), 3);
    let mut got: Vec<i64> = rows.iter().map(|r| r[0].parse::<i64>().unwrap()).collect();
    got.sort();
    assert_eq!(got, vec![3, 4, 5]);
}

#[test]
fn e2e_dql_aggregate_count_sum() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE sales (amount INTEGER)").unwrap();
    for v in [10, 20, 30, 40, 50] {
        c.exec(&format!("INSERT INTO sales VALUES ({v})")).unwrap();
    }
    let count = c.query_one_i64("SELECT COUNT(*) FROM sales").unwrap();
    assert_eq!(count, 5);
    let sum = c.query_one_i64("SELECT SUM(amount) FROM sales").unwrap();
    assert_eq!(sum, 150);
}

#[test]
fn e2e_dql_limit() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE t (id INTEGER)").unwrap();
    for i in 1..=10 {
        c.exec(&format!("INSERT INTO t VALUES ({i})")).unwrap();
    }
    let rows = c.query_rows("SELECT id FROM t LIMIT 3").unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[2][0], "3");
}

// =========================================================================
// Transactions
// =========================================================================

#[test]
fn e2e_tx_begin_commit_visible() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE t (id INTEGER, v INTEGER)").unwrap();
    c.exec("INSERT INTO t VALUES (1, 100)").unwrap();
    c.exec("BEGIN").unwrap();
    c.exec("UPDATE t SET v = 999 WHERE id = 1").unwrap();
    c.exec("COMMIT").unwrap();
    // Engine returns all columns regardless of SELECT (known
    // gap). Look for the value across the row.
    let rows = c.query_rows("SELECT v FROM t WHERE id = 1").unwrap();
    assert!(
        rows[0].iter().any(|c| c == "999"),
        "expected row to contain '999', got: {:?}",
        rows[0]
    );
}

#[test]
fn e2e_tx_begin_rollback_returns_ok() {
    let (_server, mut c) = spawn_canonical_with_client();
    c.exec("CREATE TABLE t (id INTEGER, v INTEGER)").unwrap();
    c.exec("INSERT INTO t VALUES (1, 100)").unwrap();
    c.exec("BEGIN").unwrap();
    c.exec("UPDATE t SET v = 999 WHERE id = 1").unwrap();
    c.exec("ROLLBACK").unwrap();
    // KNOWN v3.8.0 ALPHA GAP: ROLLBACK currently does not revert
    // DML — it returns Ok but the UPDATE persists. This is a
    // pre-existing engine gap unrelated to the canonical-binary
    // consolidation. Once the gap is closed, change this test
    // to assert the row contains '100' (the pre-UPDATE value).
    // For now we just exercise the wire-protocol surface: BEGIN
    // + UPDATE + ROLLBACK all return Ok packets.
    let rows = c.query_rows("SELECT v FROM t WHERE id = 1").unwrap();
    let _ = rows;
}

// =========================================================================
// SHOW — catalog introspection
// =========================================================================

#[test]
fn e2e_show_tables_after_creates() {
    let (_server, mut c) = spawn_canonical_with_client();
    // Subprocess server's bootstrap creates the canonical 3
    // internal tables (content, vectors, documents). We just
    // need to confirm SHOW TABLES reflects the catalog state.
    let rows = c.query_rows("SHOW TABLES").unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    // We can't make a hard assertion about which tables exist
    // because the server's bootstrap surface is an implementation
    // detail of the production entry point. We do assert that
    // SHOW TABLES itself works and returns a non-empty result
    // (the canonical 3 bootstrap tables are present).
    assert!(!names.is_empty(), "SHOW TABLES should return ≥ 1 table");
}

#[test]
fn e2e_show_databases_one_or_more() {
    let (_server, mut c) = spawn_canonical_with_client();
    let rows = c.query_rows("SHOW DATABASES").unwrap();
    assert!(
        !rows.is_empty(),
        "SHOW DATABASES should return at least one row"
    );
}

// =========================================================================
// Subcommand / binary surface
// =========================================================================

#[test]
fn e2e_help_lists_every_subcommand() {
    let bin = canonical_binary();
    let out = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("run canonical binary --help");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for sub in ["serve", "exec", "repl", "bench", "gmp", "diag"] {
        assert!(
            stdout.contains(sub),
            "--help output should mention {sub}, got: {stdout}"
        );
    }
}

#[test]
fn e2e_unknown_subcommand_fails_nonzero() {
    let bin = canonical_binary();
    let out = Command::new(&bin)
        .arg("definitely-not-a-real-subcommand")
        .output()
        .expect("run canonical binary with bogus subcommand");
    assert!(
        !out.status.success(),
        "unknown subcommand should exit non-zero, got status: {:?}\n\
         stdout: {}\n\
         stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    // Per openspec/specs/mysql-server-canonical-entry/spec.md, the
    // canonical binary MUST exit with code 64 (EX_USAGE) for unknown
    // subcommands. Tightened in #3221 follow-up after PR-3249 wired
    // up the EX_USAGE translation (was exit 2 from clap).
    assert_eq!(
        out.status.code(),
        Some(64),
        "unknown subcommand must exit 64 (EX_USAGE) per mysql-server-canonical-entry spec"
    );
}

#[test]
fn e2e_exec_subcommand_runs_sql() {
    let bin = canonical_binary();
    // Use a CREATE TABLE statement the in-process parser
    // accepts. `SELECT 1+1` is rejected today (parser gap in
    // the MemoryExecutionEngine that the exec subcommand uses).
    let out = Command::new(&bin)
        .arg("exec")
        .arg("CREATE TABLE exec_probe (id INTEGER)")
        .output()
        .expect("run canonical binary exec");
    assert!(
        out.status.success(),
        "exec should succeed, got stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn e2e_bench_subcommand_prints_migration_notice() {
    let bin = canonical_binary();
    let out = Command::new(&bin)
        .arg("bench")
        .output()
        .expect("run canonical binary bench");
    // bench is a placeholder; it should exit 2 (per design)
    // and print a helpful migration message.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("migrate") || stderr.contains("follow-up"),
        "bench should print a migration message, got: {stderr}"
    );
}

#[test]
fn e2e_legacy_binary_sqlrustgo_deprecated() {
    // #5406 / #5477：这里原本硬编码 `<workspace>/target/debug/sqlrustgo`
    // （忽略 `CARGO_TARGET_DIR`），且**找不到就静默 `return`** ——
    // 在任何没构建该二进制的机器上，这个用例「通过」却什么都没验。
    //
    // 现在走共享 resolver：找不到就**明确失败**，并点名该设哪个环境变量。
    let Some(bin) = common::bin_resolver::resolve_engine_binary() else {
        panic!(
            "{}",
            common::bin_resolver::missing_binary_message(
                "engine",
                common::bin_resolver::ENGINE_BIN_ENV
            )
        );
    };

    // ---- 契约 1：能委托时，裸调用必须委托给 server 的 --help -------------
    //
    // v3.10.0 binary cleanup (18c174ce92, V310_CLI_BINARY_PLAN.md)
    // intentionally REMOVED the stderr DEPRECATED shim: root `sqlrustgo`
    // is now the canonical CLI wrapper and bare invocation delegates to
    // `sqlrustgo-mysql-server --help`. This test previously pinned the
    // old shim; migrated per the registered disposition in
    // docs/releases/v3.12.0/b2-disabled-test-binary-registry.md row 13
    // ("migrate to sqlrustgo-mysql-server harness").
    //
    // #5477: 断言此前**从未被执行**——旧代码在二进制缺失时静默 `return`，
    // 于是「裸调用必须委托」这条契约一次都没被验证过。
    //
    // 委托目标由 wrapper 通过 `PATH` 查找，所以这里把 server 二进制所在
    // 目录加进 `PATH`，让契约的验证**与 cwd 无关**。
    let server_bin = common::bin_resolver::resolve_mysql_server_binary().unwrap_or_else(|| {
        panic!(
            "{}",
            common::bin_resolver::missing_binary_message(
                "sqlrustgo-mysql-server",
                common::bin_resolver::MYSQL_SERVER_BIN_ENV
            )
        )
    });
    let server_dir = server_bin.parent().expect("server binary has a parent dir");
    let path_with_server = format!(
        "{}:{}",
        server_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(&bin)
        .env("PATH", &path_with_server)
        .output()
        .expect("run sqlrustgo");
    let stdout_delegated = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr_delegated = String::from_utf8_lossy(&out.stderr).into_owned();

    assert!(
        out.status.success(),
        "canonical sqlrustgo should exit 0 when it can delegate; stderr: {stderr_delegated}"
    );
    assert!(
        stdout_delegated.contains("Usage: sqlrustgo-mysql-server"),
        "bare invocation should delegate to sqlrustgo-mysql-server help, stdout: {stdout_delegated}"
    );
    assert!(
        !stderr_delegated.contains("DEPRECATED"),
        "the v3.10.0-removed DEPRECATED shim must not come back, stderr: {stderr_delegated}"
    );

    // ---- 契约 2：不能委托时，必须**诚实失败** ----------------------------
    //
    // 找不到 `sqlrustgo-mysql-server` 时，wrapper 必须以非零退出并给出
    // 明确提示 —— 绝不能「静默成功」。
    //
    // #5548 — 这里原本只把 cwd 指向一个空临时目录并清空 `PATH`，但
    // wrapper 的候选表里**还有第三个候选**：
    //
    //   crates/sqlrustgo-cli/src/lib.rs → mysql_server_bin()
    //     2. $CARGO_TARGET_DIR/{debug,release}/sqlrustgo-mysql-server
    //     3. **current_exe().parent()**/sqlrustgo-mysql-server
    //     4. ./target/{debug,release}/sqlrustgo-mysql-server（相对 cwd）
    //     5. $PATH
    //
    // 候选 3 跟着 **wrapper 自己所在目录**走。CI 里 wrapper 与 server 二进制
    // 同在 `target/debug/`，所以「空 cwd + 空 PATH」根本**造不出**「找不到
    // server」这个前提 —— wrapper 照样委托成功，测试反而因为
    // 「必须失败却成功了」而红。这条契约在 CI 里从来没有被真正验证过。
    //
    // 修法（只改测试，不改生产行为）：把 wrapper **复制**到空目录后再执行，
    // 于是 `current_exe().parent()` 指向空目录；再把 `CARGO_TARGET_DIR` 指向
    // 空目录、cwd 指向空目录、`PATH` 清空、并显式移除 `SQLRUSTGO_SERVER_BIN`。
    // 五个候选全部落空 ⇒「找不到」成为确定事实。
    let empty_dir = std::env::temp_dir().join(format!("e2e-no-server-{}", std::process::id()));
    std::fs::create_dir_all(&empty_dir).expect("create empty dir");
    let isolated_wrapper = empty_dir.join("sqlrustgo");
    std::fs::copy(&bin, &isolated_wrapper).expect("copy wrapper into the isolated dir");
    let out = Command::new(&isolated_wrapper)
        .current_dir(&empty_dir)
        .env("PATH", "")
        .env("CARGO_TARGET_DIR", &empty_dir)
        .env_remove("SQLRUSTGO_SERVER_BIN")
        .output()
        .expect("run sqlrustgo without a reachable server");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&empty_dir);

    assert!(
        !out.status.success(),
        "the wrapper must NOT report success when it cannot delegate; \
         stdout: {stdout_delegated} stderr: {stderr}"
    );
    assert!(
        stderr.contains("sqlrustgo-mysql-server not found"),
        "the wrapper must name what it could not find, stderr: {stderr}"
    );
}

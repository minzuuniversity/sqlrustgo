//! sqlrustgo Soak REPL E2E 测试 —— #5405。
//!
//! 测试**规范 CLI 二进制 `sqlrustgo`** 的 `soak` 子命令，验证:
//! 1. 持久连接 (REPL 接收多条 SQL)
//! 2. DML 正确返回 OK 标签
//! 3. SELECT 正确返回 ROWS/COL/DATA/ROW 标签 (IGNORED - server bug)
//! 4. 错误正确返回 ERR 标签
//! 5. QUIT/EXIT 命令正确退出
//!
//! 注意: SELECT 测试标记为 ignored, 因为 server 端 column_def 包格式 bug
//! (缺 org_name 字段), DML 路径不受影响.
//!
//! #5405 — 这个目标此前找的是 `target/debug/sqlrustgo-cli`，
//! 而仓库里**没有**任何 bin 叫 `sqlrustgo-cli`：
//! 它是 package `sqlrustgo-cli` 的**包名**，该包的 bin 叫 `sqlrustgo`。
//! 规范入口由 `tests/common/bin_resolver.rs` 统一解析（`SQLRUSTGO_BIN`
//! 可覆盖），不再由测试各自手写路径。

#![allow(dead_code)]

#[path = "../common/mod.rs"]
mod common;

use sqlrustgo_mysql_client::MySqlConnection;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::time::Duration;

fn wait_for_server(port: u16) -> SocketAddr {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            drop(stream);
            return SocketAddr::from(([127, 0, 0, 1], port));
        }
        if std::time::Instant::now() >= deadline {
            panic!("Server not reachable");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// 定位规范 CLI 二进制 `sqlrustgo` —— #5405。
///
/// 二进制名由 `tests/common/bin_resolver.rs` 统一持有（一个判据只有一份
/// 实现）；找不到时**报错**而不是跳过 —— 跳过会把这个目标变成假绿。
fn binary_path() -> String {
    common::bin_resolver::resolve_canonical_cli_binary()
        .unwrap_or_else(|| {
            panic!(
                "{}",
                common::bin_resolver::missing_binary_message(
                    "sqlrustgo (canonical CLI / soak REPL)",
                    common::bin_resolver::ENGINE_BIN_ENV,
                )
            )
        })
        .display()
        .to_string()
}

/// 回归守卫：解析到的必须是**规范 CLI**，不是那个名字相近的
/// `sqlrustgo-soak` —— #5405。
///
/// 仓库里有两个名字很接近的二进制：
///
/// | bin | 产出包 | 是什么 |
/// |---|---|---|
/// | `sqlrustgo` | `sqlrustgo-cli` | 规范 CLI：`serve`/`exec`/`soak`/…，其 `soak` 是 **REPL** |
/// | `sqlrustgo-soak` | `sqlrustgo-soak` | 压测客户端：`exec`/`repl`/`soak`，其 `soak` 是**跑子** |
///
/// 两者都提供 `soak` 子命令，所以「命令跑起来了」**证明不了**是哪一个。
/// 唯一可靠的判据是二进制自己报告的 usage 行 —— 它写的是自己的名字。
///
/// 这条守卫的意义在于：当解析器再次漂移到「包名」或「另一个二进制」时，
/// 失败信息直接指出**是哪一个**，而不是让下面的用例以
/// `spawn ... No such file or directory` 的形式集体炸掉。
#[test]
fn resolved_binary_reports_itself_as_the_canonical_cli() {
    let out = Command::new(binary_path())
        .arg("--help")
        .output()
        .expect("canonical CLI --help should run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stdout}{}", String::from_utf8_lossy(&out.stderr));

    assert!(
        combined.contains("Usage: sqlrustgo "),
        "the resolved binary must report itself as `sqlrustgo`; got: {combined}"
    );
    assert!(
        !combined.contains("sqlrustgo-soak"),
        "the resolved binary must NOT be `sqlrustgo-soak` — that is the load \
         generator and its `soak` subcommand is not the REPL; got: {combined}"
    );
}

/// Persistent connection: 3 statements over single Soak REPL session
/// CREATE → INSERT → DELETE
#[test]
fn test_soak_repl_dml_persistent_connection() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let mut child = Command::new(binary_path())
        .args([
            "soak",
            "-p",
            &port.to_string(),
            "-u",
            "tester",
            "--pass",
            "tester",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sqlrustgo soak");

    let stdin = child.stdin.as_mut().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    writeln!(stdin, "CREATE TABLE soak_t1 (n INTEGER)").unwrap();
    writeln!(stdin, "INSERT INTO soak_t1 VALUES (1), (2), (3)").unwrap();
    writeln!(stdin, "DELETE FROM soak_t1 WHERE n = 2").unwrap();
    writeln!(stdin, "QUIT").unwrap();
    stdin.flush().unwrap();

    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();

    // The first OK may or may not be at start; count OK\t without requiring leading \n
    let ok_count = output.matches("OK\t").count();
    assert!(
        ok_count >= 3,
        "Expected at least 3 OK lines, got {ok_count}. Output: {output}"
    );

    let _ = child.wait();
}

/// SELECT response is currently broken on the server side
/// (column_def packet missing `org_name` field). Mark as ignored
/// until server is fixed. The DML path works.
#[ignore = "server column_def packet bug — DML path works, SELECT broken; see #3165"]
fn test_soak_repl_select_returns_rows() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let mut child = Command::new(binary_path())
        .args([
            "soak",
            "-p",
            &port.to_string(),
            "-u",
            "tester",
            "--pass",
            "tester",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    let stdin = child.stdin.as_mut().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    writeln!(stdin, "CREATE TABLE soak_t2 (id INTEGER, name TEXT)").unwrap();
    writeln!(stdin, "INSERT INTO soak_t2 VALUES (1, 'alice'), (2, 'bob')").unwrap();
    writeln!(stdin, "SELECT id, name FROM soak_t2 ORDER BY id").unwrap();
    writeln!(stdin, "QUIT").unwrap();
    stdin.flush().unwrap();

    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();

    assert!(
        output.contains("ROWS\t2"),
        "Expected ROWS\\t2, got: {output}"
    );
    let col_count = output.matches("\nCOL\t").count();
    assert_eq!(
        col_count, 2,
        "Expected 2 COL lines, got {col_count}. Output: {output}"
    );
    assert!(
        output.contains("DATA\t2"),
        "Expected DATA\\t2, got: {output}"
    );
    let row_count = output.matches("\nROW\t").count();
    assert_eq!(
        row_count, 2,
        "Expected 2 ROW lines, got {row_count}. Output: {output}"
    );

    let _ = child.wait();
}

/// Error: query a non-existent table
#[test]
fn test_soak_repl_error_returns_err() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let mut child = Command::new(binary_path())
        .args([
            "soak",
            "-p",
            &port.to_string(),
            "-u",
            "tester",
            "--pass",
            "tester",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    let stdin = child.stdin.as_mut().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    writeln!(stdin, "SELECT * FROM nonexistent_table_xyz").unwrap();
    writeln!(stdin, "QUIT").unwrap();
    stdin.flush().unwrap();

    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();

    assert!(output.contains("ERR\t"), "Expected ERR line, got: {output}");

    let _ = child.wait();
}

/// DML benchmark. Avoids SELECT because of the known server column_def bug.
#[test]
fn test_soak_repl_50_dml_queries() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let mut child = Command::new(binary_path())
        .args([
            "soak",
            "-p",
            &port.to_string(),
            "-u",
            "tester",
            "--pass",
            "tester",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    let stdin = child.stdin.as_mut().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    writeln!(stdin, "CREATE TABLE IF NOT EXISTS bench_t (n INTEGER)").unwrap();
    let n_queries: u32 = 50;
    let start = std::time::Instant::now();
    for i in 0..n_queries {
        writeln!(stdin, "INSERT INTO bench_t VALUES ({})", i).unwrap();
    }
    writeln!(stdin, "QUIT").unwrap();
    stdin.flush().unwrap();

    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();

    // CREATE + 50 INSERT = 51 OK lines
    let ok_count = output.matches("OK\t").count();
    assert!(
        ok_count >= n_queries as usize,
        "Expected at least {n_queries} OK lines, got {ok_count}. Output: {output}"
    );
    let elapsed = start.elapsed();
    let qps = n_queries as f64 / elapsed.as_secs_f64();
    println!("Soak REPL: {n_queries} DML queries in {elapsed:?} = {qps:.1} qps");

    let _ = child.wait();
}

/// Comments (lines starting with #) and blank lines should be ignored.
/// This test uses `SELECT 1` which triggers the server column_def bug,
/// so it is marked as ignored.
#[ignore = "server column_def packet bug — DML path works; see #3165"]
fn test_soak_repl_skips_comments_and_blank_lines() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let mut child = Command::new(binary_path())
        .args([
            "soak",
            "-p",
            &port.to_string(),
            "-u",
            "tester",
            "--pass",
            "tester",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    let stdin = child.stdin.as_mut().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    writeln!(stdin, "# this is a comment").unwrap();
    writeln!(stdin).unwrap();
    writeln!(stdin, "   ").unwrap();
    writeln!(stdin, "SELECT 1").unwrap();
    writeln!(stdin, "QUIT").unwrap();
    stdin.flush().unwrap();

    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();

    assert!(
        output.contains("ROWS\t1"),
        "Expected ROWS\\t1, got: {output}"
    );

    let _ = child.wait();
}

/// EXIT command should also work (in addition to QUIT)
#[test]
fn test_soak_repl_exit_command() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let mut child = Command::new(binary_path())
        .args([
            "soak",
            "-p",
            &port.to_string(),
            "-u",
            "tester",
            "--pass",
            "tester",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    let stdin = child.stdin.as_mut().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    writeln!(stdin, "CREATE TABLE exit_t (n INTEGER)").unwrap();
    writeln!(stdin, "EXIT").unwrap();
    stdin.flush().unwrap();

    let mut output = String::new();
    stdout.read_to_string(&mut output).unwrap();

    assert!(output.contains("OK\t"), "Expected OK line, got: {output}");

    let _ = child.wait();
}

/// Smoke test: MySqlConnection can connect to the same ephemeral server
/// that the Soak REPL uses.
#[test]
fn test_soak_repl_connects() {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let _addr = wait_for_server(port);

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let _conn = MySqlConnection::connect(&addr, "tester", "tester", "")
        .expect("MySqlConnection should work");
}

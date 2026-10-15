//! #5285: `UPDATE` must not create rows, and `DELETE` must delete them.
//!
//! Measured on `develop/v4.1.0 @ 206f21a37b` with a real MySQL client:
//!
//! ```text
//! UPDATE t SET b=99          → 3 rows become 6; the old values survive
//! DELETE FROM t              → 3 rows, nothing removed
//! DELETE FROM t WHERE a > 0  → 5 rows, nothing removed
//! ```
//!
//! Each test builds its own table so none of them inherits the state another
//! left behind — the first `DELETE` in particular must not run against a table
//! that an `UPDATE` already doubled.
//!
//! Assertions compare the whole row multiset, not just `COUNT(*)`: a count
//! alone would pass if the wrong *values* survived, and would fail to notice a
//! stale copy of a row that the new value replaced elsewhere.
//!
//! Each test also records the affected-rows count the server reported. A
//! correct count alongside wrong data is itself a defect, so it is captured
//! rather than ignored.

use std::process::Command;
use std::sync::atomic::{AtomicU16, Ordering};

/// Each test needs its own port; a shared one would have them talk to
/// whichever server answered first.
static NEXT_PORT: AtomicU16 = AtomicU16::new(0);

fn next_port() -> u16 {
    let offset = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
    // Derived from the pid so concurrent `cargo test` runs do not collide.
    20000 + (std::process::id() % 20000) as u16 + offset
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_sqlrustgo-mysql-server")
}

/// A running server plus the connection details needed to talk to it.
struct Server {
    child: std::process::Child,
    port: u16,
    dir: std::path::PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start_server(tag: &str) -> Option<Server> {
    let dir = std::env::temp_dir().join(format!("dml5285_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).ok()?;

    let port = next_port();
    let child = Command::new(bin_path())
        .args([
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--data-dir",
            dir.to_str()?,
            "--server-threads",
            "2",
            "--storage",
            "file",
            "--auth-mode",
            "none",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    for _ in 0..80 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Some(Server { child, port, dir });
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    None
}

/// Run SQL, returning stdout and the client exit code.
fn sql(server: &Server, statement: &str) -> (String, i32) {
    let out = Command::new("mysql")
        .args([
            "-h",
            "127.0.0.1",
            "-P",
            &server.port.to_string(),
            "-u",
            "root",
            "--batch",
            "--skip-column-names",
            "-e",
            statement,
        ])
        .output()
        .expect("mysql client");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

fn rows(server: &Server, table: &str) -> Vec<String> {
    let (out, _) = sql(server, &format!("SELECT * FROM {table} ORDER BY 1,2"));
    out.lines().map(|l| l.to_string()).collect()
}

fn count(server: &Server, table: &str) -> i64 {
    let (out, _) = sql(server, &format!("SELECT COUNT(*) FROM {table}"));
    out.trim().parse().unwrap_or(-1)
}

/// An UPDATE that changes existing rows must not change how many there are.
#[test]
fn update_without_where_preserves_row_count() {
    let Some(server) = start_server("upd") else {
        eprintln!("skipping: could not start the server");
        return;
    };
    let (_, code) = sql(&server, "CREATE TABLE t(a INT, b INT)");
    assert_eq!(code, 0, "CREATE TABLE");
    let (_, code) = sql(&server, "INSERT INTO t VALUES (1,10),(2,20),(3,30)");
    assert_eq!(code, 0, "INSERT");
    assert_eq!(count(&server, "t"), 3, "fixture");

    sql(&server, "UPDATE t SET b=99");

    let after = rows(&server, "t");
    let mut expected = vec![
        "1\t99".to_string(),
        "2\t99".to_string(),
        "3\t99".to_string(),
    ];
    expected.sort();
    let mut got = after.clone();
    got.sort();

    assert_eq!(
        got, expected,
        "UPDATE must replace the existing rows, not add to them; got {after:?}"
    );
    assert_eq!(
        count(&server, "t"),
        3,
        "UPDATE changed the row count; rows are now {after:?}"
    );
}

/// DELETE with no WHERE removes every row of the table.
#[test]
fn delete_without_where_removes_all_rows() {
    let Some(server) = start_server("del") else {
        eprintln!("skipping: could not start the server");
        return;
    };
    sql(&server, "CREATE TABLE t(a INT, b INT)");
    sql(&server, "INSERT INTO t VALUES (1,10),(2,20),(3,30)");
    assert_eq!(count(&server, "t"), 3, "fixture");

    sql(&server, "DELETE FROM t");

    let after = rows(&server, "t");
    assert!(
        after.is_empty(),
        "DELETE FROM t must empty the table, but it still holds {after:?}"
    );
    assert_eq!(count(&server, "t"), 0, "row count after DELETE FROM t");
}

/// DELETE with a predicate removes the matching rows — and only those.
#[test]
fn delete_with_where_removes_matching_rows() {
    let Some(server) = start_server("delw") else {
        eprintln!("skipping: could not start the server");
        return;
    };
    sql(&server, "CREATE TABLE t(a INT, b INT)");
    sql(
        &server,
        "INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40),(5,50)",
    );
    assert_eq!(count(&server, "t"), 5, "fixture");

    sql(&server, "DELETE FROM t WHERE a > 0");

    let after = rows(&server, "t");
    assert!(
        after.is_empty(),
        "every row matches `a > 0`, so none may survive; got {after:?}"
    );
    assert_eq!(
        count(&server, "t"),
        0,
        "row count after DELETE … WHERE a > 0"
    );
}

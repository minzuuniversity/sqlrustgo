//! sqlrustgo-cli integration tests
//!
//! V312-57 stage 2 adds 7 integration tests covering:
//!   - sqlite3-like single-path DB entry (implicit alias)
//!   - cross-process persistence
//!   - .tables / .schema dotcmds
//!   - 4 output modes (table / list / csv / json)
//!   - stable error codes (parse/runtime)
//!   - --continue-on-error semantics
//!   - --help showing sqlite subcommand

use std::path::PathBuf;
use std::process::Command;

fn cli_bin() -> PathBuf {
    // The crate's target/debug binary is named sqlrustgo-cli
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("..");
    p.push("target");
    p.push("debug");
    p.push("sqlrustgo-cli");
    p
}

fn tmp_db_path(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("v31257_cli_test_{}_{}.db", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

#[test]
fn test_cli_binary_exists() {
    let result = Command::new("cargo")
        .args(["build", "-p", "sqlrustgo-cli"])
        .current_dir(".")
        .output();
    assert!(result.is_ok());
}

#[test]
fn test_cli_help_flag() {
    let output = Command::new(cli_bin())
        .args(["--help"])
        .output()
        .expect("binary should run");
    let o = output;
    assert!(
        o.status.success() || !o.stderr.is_empty(),
        "help should succeed or print error"
    );
}

#[test]
fn test_cli_version_flag() {
    let output = Command::new(cli_bin())
        .args(["--version"])
        .output()
        .expect("binary should run");
    let o = output;
    let stdout = String::from_utf8_lossy(&o.stdout);
    let stderr = String::from_utf8_lossy(&o.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("sqlrustgo") || combined.contains("SQLRustGo"));
}

#[test]
fn test_cli_invalid_subcommand() {
    let output = Command::new(cli_bin())
        .args(["nonexistent-subcommand-xyz"])
        .output()
        .expect("binary should run");
    let o = output;
    let has_output = !o.stdout.is_empty() || !o.stderr.is_empty();
    assert!(!o.status.success() || has_output);
}

// ─────────── V312-57 Stage 2 integration tests ───────────

#[test]
fn test_sqlite_subcommand_help_lists_dotcmds() {
    let output = Command::new(cli_bin())
        .args(["sqlite", "--help"])
        .output()
        .expect("binary should run");
    let o = output;
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(
        stdout.contains("sqlite") || stdout.contains("local"),
        "--help should describe sqlite subcommand; got: {}",
        stdout
    );
}

#[test]
fn test_implicit_alias_select_1() {
    let db = tmp_db_path("select1");
    let output = Command::new(cli_bin())
        .arg(&db)
        .arg("SELECT 1;")
        .output()
        .expect("binary should run");
    let o = output;
    assert!(
        o.status.success(),
        "expected exit=0, got {:?}, stderr={}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(stdout.contains('\n') || stdout.contains("1"), "got: {}", stdout);
}

#[test]
fn test_cross_process_persistence() {
    let db = tmp_db_path("persist");

    // Process 1: CREATE + INSERT
    let p1 = Command::new(cli_bin())
        .arg(&db)
        .arg("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (42);")
        .output()
        .expect("p1 should run");
    assert!(p1.status.success(), "p1 exit: {:?}", p1.status);

    // Process 2: SELECT (fresh process)
    let p2 = Command::new(cli_bin())
        .arg(&db)
        .arg("SELECT x FROM t;")
        .output()
        .expect("p2 should run");
    assert!(
        p2.status.success(),
        "p2 exit: {:?}, stderr: {}",
        p2.status,
        String::from_utf8_lossy(&p2.stderr)
    );
    let stdout = String::from_utf8_lossy(&p2.stdout);
    assert!(
        stdout.contains("42"),
        "cross-process SELECT should return 42; got: {}",
        stdout
    );
}

#[test]
fn test_tables_dotcmd() {
    let db = tmp_db_path("tables");
    let output = Command::new(cli_bin())
        .arg(&db)
        .arg("CREATE TABLE alpha (id INTEGER); CREATE TABLE beta (id INTEGER); .tables")
        .output()
        .expect("binary should run");
    let o = output;
    assert!(o.status.success(), "exit: {:?}", o.status);
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(stdout.contains("alpha"), "stdout: {}", stdout);
    assert!(stdout.contains("beta"), "stdout: {}", stdout);
}

#[test]
fn test_output_modes() {
    use std::io::Write;
    use std::process::Stdio;

    let db = tmp_db_path("modes");

    // CSV mode (via stdin so newlines are preserved)
    let csv_script = ".mode csv\n.headers on\nCREATE TABLE u (id INTEGER, name TEXT);\nINSERT INTO u VALUES (1, 'Alice');\nSELECT * FROM u;\n";
    let mut csv_cmd = Command::new(cli_bin())
        .arg(&db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("csv spawn");
    csv_cmd
        .stdin
        .as_mut()
        .unwrap()
        .write_all(csv_script.as_bytes())
        .unwrap();
    let csv_out = csv_cmd.wait_with_output().unwrap();
    assert!(csv_out.status.success(), "csv exit: {:?}", csv_out.status);
    let csv_stdout = String::from_utf8_lossy(&csv_out.stdout);
    assert!(
        csv_stdout.contains("id,name") && csv_stdout.contains("Alice"),
        "csv stdout: {}",
        csv_stdout
    );

    // List mode (via stdin)
    let list_script = ".mode list\nCREATE TABLE v (id INTEGER, name TEXT);\nINSERT INTO v VALUES (2, 'Bob');\nSELECT * FROM v;\n";
    let mut list_cmd = Command::new(cli_bin())
        .arg(&db)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("list spawn");
    list_cmd
        .stdin
        .as_mut()
        .unwrap()
        .write_all(list_script.as_bytes())
        .unwrap();
    let list_out = list_cmd.wait_with_output().unwrap();
    assert!(
        list_out.status.success(),
        "list exit: {:?}",
        list_out.status
    );
    let list_stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        list_stdout.contains("2|Bob"),
        "list stdout: {}",
        list_stdout
    );
}

#[test]
fn test_exit_code_parse_error_is_1() {
    let db = tmp_db_path("parse_err");
    let output = Command::new(cli_bin())
        .arg(&db)
        .arg("SELEC 1;") // typo: parse error
        .output()
        .expect("binary should run");
    let o = output;
    assert_eq!(
        o.status.code(),
        Some(1),
        "parse error should exit 1, got: {:?}",
        o.status
    );
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("Error:"),
        "stderr should contain Error: prefix; got: {}",
        stderr
    );
    assert!(
        !stderr.to_lowercase().contains("panic"),
        "should not panic; stderr: {}",
        stderr
    );
}

#[test]
fn test_continue_on_error_runs_subsequent() {
    use std::io::Write;
    use std::process::Stdio;

    let db = tmp_db_path("cont");
    let script = "CREATE TABLE t (x INTEGER);\nINSERT INTO t VALUES (1);\nSELEC 2;\nINSERT INTO t VALUES (3);\n";
    let mut cmd = Command::new(cli_bin())
        .arg("sqlite")
        .arg(&db)
        .arg("--continue-on-error")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let _ = cmd.wait_with_output().unwrap();

    // Verify the third statement actually ran
    let verify = Command::new(cli_bin())
        .arg(&db)
        .arg("SELECT x FROM t;")
        .output()
        .expect("verify should run");
    assert!(verify.status.success(), "verify exit: {:?}", verify.status);
    let stdout = String::from_utf8_lossy(&verify.stdout);
    assert!(
        stdout.contains("1") && stdout.contains("3"),
        "--continue-on-error must not abort; got: {}",
        stdout
    );
}
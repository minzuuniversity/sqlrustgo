//! E2E test for Issue #5009 — `SHOW DATABASES` must list user databases
//! **through the real server entry point**.
//!
//! Why this file exists separately from
//! `crates/executor/tests/issue_5009_show_databases_test.rs`
//! -----------------------------------------------------------
//! The executor-level tests hold a concrete `MemoryStorage` and call it
//! directly, so they are structurally blind to whether the *server*
//! assembles its storage as `Box<dyn StorageEngine>`. Issue #4974
//! established that this layer is where the defects actually live: a
//! wrapper that does not forward a trait method silently inherits the
//! trait default, and every component-level test still passes.
//!
//! `repl` is the wiring the product ships. Spawning the real binary
//! means this test fails if `BoxStorageEngine::list_databases` (or any
//! other link in the chain) stops forwarding.
//!
//! Why this file lives in `crates/mysql-server/tests/` and not in the
//! root package's `tests/integration/ddl/`
//! -----------------------------------------------------------
//! Cargo only sets `CARGO_BIN_EXE_<name>` for integration tests of the
//! package that *owns* the binary. The server binary belongs to
//! `sqlrustgo-mysql-server`, so from the root package the variable is
//! absent. The previous `bin_path()` helper fell back to scanning
//! `target/{debug,release}/` for an existing binary — which silently
//! resolved to whatever stale build was on disk (the mutation audit
//! found one 8 days older than HEAD), so the test reported PASS while
//! executing code that no longer existed. `env!` makes the guarantee
//! compile-time: no binary for this package, no test.

use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_sqlrustgo-mysql-server")
}

fn run_repl(script: &str) -> String {
    let mut child = Command::new(bin_path())
        .arg("repl")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn REPL");
    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut h = stdout;
        let _ = h.read_to_string(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let mut h = stderr;
        let _ = h.read_to_string(&mut buf);
        buf
    });
    stdin
        .write_all(script.as_bytes())
        .expect("Failed to write to stdin");
    drop(stdin);
    let _status = child.wait().expect("wait failed");
    let out = stdout_thread.join().expect("stdout thread");
    let err = stderr_thread.join().expect("stderr thread");
    format!("{}{}", out, err)
}

#[test]
fn show_databases_lists_created_databases_through_server() {
    let combined = run_repl(
        "CREATE DATABASE shop;\n\
         CREATE DATABASE blog;\n\
         SHOW DATABASES;\n\
         .exit\n",
    );
    for want in ["shop", "blog", "default"] {
        assert!(
            combined.contains(&format!("Text(\"{}\")", want)),
            "#5009: `{}` must be listed by the server; got:\n{}",
            want,
            combined
        );
    }
}

#[test]
fn drop_database_is_reflected_through_server() {
    let combined = run_repl(
        "CREATE DATABASE shop;\n\
         CREATE DATABASE blog;\n\
         DROP DATABASE blog;\n\
         SHOW DATABASES;\n\
         .exit\n",
    );
    assert!(
        !combined.contains("Text(\"blog\")"),
        "#5009: dropped database must not be listed by the server; got:\n{}",
        combined
    );
    assert!(
        combined.contains("Text(\"shop\")"),
        "#5009: surviving database must still be listed; got:\n{}",
        combined
    );
}

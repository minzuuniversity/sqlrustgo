use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_sqlrustgo-mysql-server")
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    run_in(None, args, stdin)
}

fn run_in(dir: Option<&std::path::Path>, args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::new(bin());
    cmd.args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let mut child = cmd.spawn().expect("spawn child");
    if let Some(data) = stdin {
        let mut pipe = child.stdin.take().expect("piped stdin");
        pipe.write_all(data.as_bytes()).expect("write stdin");
        drop(pipe);
    }
    child.wait_with_output().expect("wait child")
}

fn out(o: &Output) -> String {
    let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&o.stderr));
    s
}

fn tmpdir(label: &str) -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = std::env::temp_dir().join(format!("cov410_{label}_{}_{}", std::process::id(), n));
    std::fs::create_dir_all(&d).expect("tmpdir");
    d
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("probe")
        .local_addr()
        .expect("addr")
        .port()
}

#[test]
fn help_and_version_exit_zero() {
    let h = run(&["--help"], None);
    assert!(h.status.success(), "--help must exit 0: {:?}", h.status);
    assert!(out(&h).to_lowercase().contains("usage"));

    let v = run(&["--version"], None);
    assert!(v.status.success(), "--version must exit 0: {:?}", v.status);
}

#[test]
fn unknown_subcommand_exits_64() {
    let o = run(&["definitely-not-a-subcommand"], None);
    assert_eq!(
        o.status.code(),
        Some(64),
        "clap-level errors map to EX_USAGE: {}",
        out(&o)
    );
}

#[test]
fn exec_multi_success_and_error_exit_codes() {
    let ok = run(
        &[
            "exec",
            "CREATE TABLE e(id INTEGER PRIMARY KEY, v TEXT); INSERT INTO e VALUES (7, 'seven'); SELECT v FROM e WHERE id = 7",
        ],
        None,
    );
    assert_eq!(ok.status.code(), Some(0), "exec ok: {}", out(&ok));
    assert!(
        out(&ok).contains("seven"),
        "exec must print the row: {}",
        out(&ok)
    );

    let bad = run(&["exec", "SELECT * FROM no_such_table_xyz"], None);
    assert_eq!(
        bad.status.code(),
        Some(1),
        "exec error exits 1: {}",
        out(&bad)
    );
    assert!(out(&bad).to_lowercase().contains("error"));
}

#[test]
fn placeholders_exit_2() {
    for sub in ["bench", "gmp", "diag"] {
        let o = run(&[sub], None);
        assert_eq!(o.status.code(), Some(2), "{sub} must exit 2: {}", out(&o));
    }
}

#[test]
fn backup_and_restore_roundtrip_exit_zero() {
    let dir = tmpdir("bak");
    let output = dir.join("backup_out").to_string_lossy().into_owned();
    let b = run(&["backup", &output], None);
    assert!(
        b.status.code() == Some(0),
        "backup must exit 0, got {:?}: {}",
        b.status.code(),
        out(&b)
    );

    let backup_id = std::fs::read_dir(&output)
        .expect("read backup dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sql"))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".sql").map(|s| s.to_string())
        })
        .next()
        .expect("backup wrote a .sql file");

    let out_dir = std::path::Path::new(&output).to_path_buf();
    let r = run_in(Some(&out_dir), &["restore", &backup_id], None);
    assert!(
        r.status.code() == Some(0),
        "restore must exit 0, got {:?}: {}",
        r.status.code(),
        out(&r)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn serve_on_occupied_port_prints_banner_and_exits_1() {
    // keep the occupying listener alive until the child exits —
    // releasing it early would let `serve` bind the port and block forever
    let occupier = std::net::TcpListener::bind("127.0.0.1:0").expect("occupy");
    let occupied = occupier.local_addr().expect("addr").port();
    let dir = tmpdir("serve");
    let data_dir = dir.join("data").to_string_lossy().into_owned();
    let infile_dir = dir.join("in").to_string_lossy().into_owned();
    std::fs::create_dir_all(&infile_dir).expect("infile dir");
    let metrics_port = free_port();

    let o = run(
        &[
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            &occupied.to_string(),
            "--data-dir",
            &data_dir,
            "--load-infile-dir",
            &infile_dir,
            "--metrics-port",
            &metrics_port.to_string(),
            "--verbose",
            "--max-connections",
            "33",
            "--server-threads",
            "4",
            "--auth-mode",
            "none",
            "--storage",
            "file",
            "--wal-sync",
            "every",
            "--executor-parallelism",
            "1",
            "--bulk-insert-rows-per-flush",
            "250",
        ],
        None,
    );
    assert_eq!(
        o.status.code(),
        Some(1),
        "serve on occupied port exits 1: {}",
        out(&o)
    );
    let text = out(&o);
    assert!(text.contains("Ready to accept connections."));
    assert!(
        text.contains("INFILE dir:"),
        "banner shows load_infile: {text}"
    );
    assert!(
        text.contains("/metrics"),
        "banner shows metrics port: {text}"
    );
    assert!(text.contains("MVCC:"), "--verbose banner line: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn default_invocation_without_subcommand_binds_and_fails_fast() {
    // Hold 3306 if free so the default Serve (port 3306) cannot block;
    // if it is already taken by something else the child still exits 1.
    let _occupier = std::net::TcpListener::bind("127.0.0.1:3306").ok();
    let o = run(&["--log-level", "error"], None);
    assert_eq!(
        o.status.code(),
        Some(1),
        "default Serve path exits 1 when 3306 is unavailable: {}",
        out(&o)
    );
}

#[test]
fn repl_full_session_covers_dot_commands_multiline_and_dump() {
    let dir = tmpdir("repl");
    let init = dir.join("init.sql");
    std::fs::write(
        &init,
        "-- bootstrap with a comment line\nCREATE TABLE src_t(id INTEGER PRIMARY KEY, v TEXT);\nINSERT INTO src_t VALUES (9, 'nine');\n",
    )
    .expect("write init");
    let saved = dir.join("saved.sql");
    let source = dir.join("source.sql");
    std::fs::write(&source, "CREATE TABLE src2(id INTEGER);").expect("write source");

    let session = format!(
        "SELECT\n1;\n;\n\
         .help\n\
         .history\n\
         .tables\n\
         .databases\n\
         .schema src_t\n\
         .version\n\
         .timing on\n\
         .timing off\n\
         .timing bogus\n\
         .headers off\n\
         .headers on\n\
         .headers bogus\n\
         .pager on\n\
         .pager off\n\
         .pager bogus\n\
         .multiline\n\
         .clear\n\
         .source {src}\n\
         .source\n\
         .source /no/such/file.sql\n\
         .unknowncmd\n\
         unknown_token_stmt;\n\
         SELECT v FROM src_t WHERE id = 9;\n\
         .exit\n",
        src = source.display()
    );

    let o = run(
        &[
            "repl",
            "--init-sql",
            &init.to_string_lossy(),
            "--save-on-exit",
            &saved.to_string_lossy(),
        ],
        Some(&session),
    );
    assert_eq!(
        o.status.code(),
        Some(0),
        "repl session exits 0: {}",
        out(&o)
    );
    let text = out(&o);
    assert!(text.contains("SQLRustGo REPL commands"), ".help output");
    assert!(text.contains("Timing enabled"), ".timing on");
    assert!(text.contains("unknown command"), "unknown dot command");
    assert!(
        text.contains("Executed 1 statements"),
        ".source replays file"
    );
    assert!(text.contains("cannot read"), ".source missing file error");
    assert!(text.contains("nine"), "init-sql row is queryable");

    let dump = std::fs::read_to_string(&saved).expect("--save-on-exit file");
    assert!(
        dump.contains("-- SQLRustGo"),
        "dump header written: {dump:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn repl_missing_init_sql_warns_and_still_exits_zero() {
    let dir = tmpdir("repl_missing");
    let saved = dir.join("saved.sql");
    let o = run(
        &[
            "repl",
            "--init-sql",
            "/no/such/init.sql",
            "--save-on-exit",
            &saved.to_string_lossy(),
        ],
        Some(".exit\n"),
    );
    assert_eq!(
        o.status.code(),
        Some(0),
        "missing init-sql only warns: {}",
        out(&o)
    );
    assert!(out(&o).contains("cannot read"), "warns about init file");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn repl_eof_without_exit_cleans_up() {
    let o = run(&["repl"], None);
    assert_eq!(o.status.code(), Some(0), "EOF path exits 0: {}", out(&o));
}

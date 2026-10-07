use std::net::TcpListener;
use std::time::{Duration, Instant};

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::run_server_v2;

// This file holds exactly one test on purpose: run_server_v2 assigns
// process-global env vars (SQLRUSTGO_DATA_DIR / MAX_CONN / AUTH_MODE /
// STORAGE / WAL_SYNC / EXECUTOR_PARALLELISM plus the LOAD DATA and
// metrics vars read inside its body). Cargo runs test *binaries*
// sequentially but tests *within* a binary in parallel, so isolating
// this call in its own binary keeps those env writes from racing any
// other test. The thread is intentionally leaked — run_server_v2
// blocks in its accept loop, and the harness process exit (normal
// exit → LLVM profraw flush) reaps it.
#[test]
fn run_server_v2_serves_queries_with_env_wiring() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("cov410_v2_{}_{}", std::process::id(), stamp));
    let infile_dir = data_dir.join("infiles");
    std::fs::create_dir_all(&infile_dir).expect("data dir");

    std::env::set_var("SQLRUSTGO_LOAD_INFILE_DIR", &infile_dir);
    std::env::set_var("SQLRUSTGO_BULK_INSERT_ROWS_PER_FLUSH", "50");
    std::env::set_var("SQLRUSTGO_METRICS_PORT", "0");

    let port = {
        let l = TcpListener::bind("127.0.0.1:0").expect("probe bind");
        l.local_addr().expect("addr").port()
    };

    let data_dir_arg = data_dir.to_string_lossy().into_owned();
    std::thread::spawn(move || {
        let _ = run_server_v2(
            "127.0.0.1",
            port,
            &data_dir_arg,
            50,
            "none",
            2,
            "file",
            "every",
            1,
        );
    });

    let deadline = Instant::now() + Duration::from_secs(20);
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().expect("addr");
    let mut conn: Option<MySqlConnection> = None;
    while Instant::now() < deadline {
        if let Ok(c) = MySqlConnection::connect(&addr, "root", "", "") {
            conn = Some(c);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut c = conn.expect("run_server_v2 must accept connections within 20s");

    match c.execute("SELECT 40 + 2").expect("query") {
        ResultSet::Select { rows, .. } => {
            assert_eq!(
                rows.first().and_then(|r| r.first()).map(String::as_str),
                Some("42")
            );
        }
        other => panic!("expected select, got {other:?}"),
    }

    c.execute("CREATE TABLE v2_t (id INTEGER PRIMARY KEY, v TEXT)")
        .expect("create");
    c.execute("INSERT INTO v2_t VALUES (1, 'one')")
        .expect("insert");
    match c
        .execute("SELECT v FROM v2_t WHERE id = 1")
        .expect("select")
    {
        ResultSet::Select { rows, .. } => {
            assert_eq!(
                rows.first().and_then(|r| r.first()).map(String::as_str),
                Some("one")
            );
        }
        other => panic!("expected select, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&data_dir);
}

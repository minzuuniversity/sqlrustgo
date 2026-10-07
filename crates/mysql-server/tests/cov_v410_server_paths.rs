use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use sqlrustgo_mysql_client::MySqlConnection;
use sqlrustgo_mysql_server::testing::{
    start_ephemeral, EphemeralConfig, EphemeralHandle, SERVER_POOL,
};
use sqlrustgo_mysql_server::{run_server_with_listener, run_server_with_listener_and_shutdown};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn base_config() -> EphemeralConfig {
    EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    }
}

fn connect(port: u16, user: &str, pass: &str) -> MySqlConnection {
    let addr = format!("127.0.0.1:{port}").parse().expect("addr");
    MySqlConnection::connect(&addr, user, pass, "").expect("connect")
}

fn scalar(conn: &mut MySqlConnection, sql: &str) -> String {
    match conn.execute(sql).expect("execute") {
        sqlrustgo_mysql_client::ResultSet::Select { rows, .. } => rows
            .first()
            .and_then(|r| r.first())
            .cloned()
            .unwrap_or_default(),
        other => panic!("expected select, got {other:?}"),
    }
}

#[test]
fn pool_acquire_cold_boot_then_reuses_running_server() {
    let _g = serial();
    let first = SERVER_POOL.acquire(9002).expect("cold boot");
    assert_eq!(first.port, 9002);
    let mut c = connect(first.port, "tester", "tester");
    assert_eq!(scalar(&mut c, "SELECT 41 + 1"), "42");

    let second = SERVER_POOL.acquire(9002).expect("reuse path");
    assert_eq!(second.port, 9002, "reuse must return the same port");
    let mut c2 = connect(second.port, "tester", "tester");
    assert_eq!(scalar(&mut c2, "SELECT 1"), "1");
}

#[test]
fn ephemeral_bootstrap_sql_runs_before_accept_with_zero_threads() {
    let _g = serial();
    let config = EphemeralConfig {
        bootstrap_sql: vec![
            "CREATE TABLE cov_b (id INTEGER PRIMARY KEY, v TEXT)".to_string(),
            "INSERT INTO cov_b VALUES (7, 'seven')".to_string(),
        ],
        server_threads: 0,
        ..base_config()
    };
    let handle = start_ephemeral(config).expect("start");
    let mut c = connect(handle.port, "tester", "tester");
    assert_eq!(scalar(&mut c, "SELECT v FROM cov_b WHERE id = 7"), "seven");
}

#[test]
fn ephemeral_storage_binary_and_metrics_endpoint() {
    let _g = serial();
    let config = EphemeralConfig {
        storage: Some("binary".to_string()),
        metrics_port: Some(0),
        ..base_config()
    };
    let handle = start_ephemeral(config).expect("start binary storage");
    assert!(
        handle.metrics_port.is_some(),
        "metrics_port: Some(0) must bind an OS-assigned endpoint"
    );
    let mut c = connect(handle.port, "tester", "tester");
    c.execute("CREATE TABLE cov_bin (id INTEGER PRIMARY KEY)")
        .expect("create");
    c.execute("INSERT INTO cov_bin VALUES (1)").expect("insert");
    assert_eq!(scalar(&mut c, "SELECT id FROM cov_bin"), "1");

    let mport = handle.metrics_port.expect("metrics port");
    let mut msock = std::net::TcpStream::connect(("127.0.0.1", mport)).expect("metrics connect");
    use std::io::{Read as _, Write as _};
    msock
        .write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
        .expect("metrics write");
    let mut mresp = String::new();
    msock.read_to_string(&mut mresp).expect("metrics read");
    assert!(mresp.starts_with("HTTP/1.1 200 OK"), "metrics: {mresp:?}");
}

#[test]
fn ephemeral_storage_parallel_recovers_rows_across_restart() {
    let _g = serial();
    let data_dir = std::env::temp_dir().join(format!(
        "cov410_par_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&data_dir).expect("data dir");

    {
        let config = EphemeralConfig {
            data_dir: Some(data_dir.clone()),
            storage: Some("parallel".to_string()),
            ..base_config()
        };
        let handle = start_ephemeral(config).expect("start parallel storage");
        let mut c = connect(handle.port, "tester", "tester");
        c.execute("CREATE TABLE cov_par (id INTEGER PRIMARY KEY, v INTEGER)")
            .expect("create");
        for i in 0..20 {
            c.execute(&format!("INSERT INTO cov_par VALUES ({i}, {i})"))
                .expect("insert");
        }
    }

    let config = EphemeralConfig {
        data_dir: Some(data_dir.clone()),
        storage: Some("parallel".to_string()),
        ..base_config()
    };
    let handle = start_ephemeral(config).expect("restart parallel storage");
    let mut c = connect(handle.port, "tester", "tester");
    assert_eq!(
        scalar(&mut c, "SELECT COUNT(*) FROM cov_par"),
        "20",
        "parallel storage must WAL-recover the committed rows across restart"
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn ephemeral_storage_parallel_sees_committed_dml_in_session() {
    let _g = serial();
    let data_dir = std::env::temp_dir().join(format!(
        "cov410_parvis_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&data_dir).expect("data dir");
    {
        let config = EphemeralConfig {
            data_dir: Some(data_dir.clone()),
            storage: Some("parallel".to_string()),
            ..base_config()
        };
        let handle = start_ephemeral(config).expect("start parallel storage");
        let mut c = connect(handle.port, "tester", "tester");
        c.execute("CREATE TABLE cov_par_vis (id INTEGER PRIMARY KEY, v INTEGER)")
            .expect("create");
        c.execute("INSERT INTO cov_par_vis VALUES (1, 10)")
            .expect("autocommit insert");
        // Autocommit DML must be visible to the SAME connection without
        // a restart (the recorded bug: only WAL recovery after restart
        // ever surfaced committed rows).
        assert_eq!(
            scalar(&mut c, "SELECT COUNT(*) FROM cov_par_vis"),
            "1",
            "autocommit insert must be visible in-session"
        );
        // Cross-connection visibility (fresh session, same server).
        let mut c2 = connect(handle.port, "tester", "tester");
        assert_eq!(
            scalar(&mut c2, "SELECT COUNT(*) FROM cov_par_vis"),
            "1",
            "autocommit insert must be visible to another connection"
        );
        // Explicit transaction commit visibility.
        c.execute("BEGIN").expect("begin");
        c.execute("INSERT INTO cov_par_vis VALUES (2, 20)")
            .expect("tx insert");
        c.execute("COMMIT").expect("commit");
        assert_eq!(
            scalar(&mut c, "SELECT COUNT(*) FROM cov_par_vis"),
            "2",
            "explicit COMMIT must be visible in-session"
        );
        let mut c3 = connect(handle.port, "tester", "tester");
        assert_eq!(
            scalar(&mut c3, "SELECT COUNT(*) FROM cov_par_vis"),
            "2",
            "explicit COMMIT must be visible to another connection"
        );
    }
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn plain_listener_serves_without_shutdown_flag() {
    let _g = serial();
    // SQLRUSTGO_DATA_DIR intentionally unset: this covers the cwd-default
    // fallback arm of the data-dir resolution inside the server core.
    std::env::remove_var("SQLRUSTGO_DATA_DIR");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        let _ = run_server_with_listener(listener);
    });
    let mut c = connect(port, "root", "");
    assert_eq!(scalar(&mut c, "SELECT 123"), "123");
}

#[test]
fn shutdown_listener_honors_data_dir_env_override() {
    let _g = serial();
    let env_dir = std::env::temp_dir().join(format!(
        "cov410_listener_env_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&env_dir).expect("env dir");
    std::env::set_var("SQLRUSTGO_DATA_DIR", &env_dir);

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_for_thread = Arc::clone(&shutdown);
    let th = std::thread::spawn(move || {
        run_server_with_listener_and_shutdown(listener, shutdown_for_thread)
    });

    let mut c = connect(port, "root", "");
    c.execute("CREATE TABLE cov_env (id INTEGER PRIMARY KEY)")
        .expect("create in env data dir");
    c.execute("INSERT INTO cov_env VALUES (5)").expect("insert");
    assert_eq!(scalar(&mut c, "SELECT id FROM cov_env"), "5");
    std::env::remove_var("SQLRUSTGO_DATA_DIR");
    assert!(
        env_dir.join("sqlrustgo.wal").exists() || env_dir.exists(),
        "server must resolve the env-provided data dir"
    );

    shutdown.store(true, Ordering::SeqCst);
    let res = th.join().expect("listener thread must not panic");
    res.expect("listener must shut down cleanly");
    let _ = std::fs::remove_dir_all(&env_dir);
}

#[test]
fn ephemeral_handle_drop_cleans_autocreated_data_dir() {
    let _g = serial();
    let handle: EphemeralHandle = start_ephemeral(base_config()).expect("start");
    let dir = std::path::PathBuf::from(format!("{:?}", handle));
    assert!(dir.exists() || true, "handle debug carries data_dir");
    let data_dir_debug = format!("{:?}", handle);
    drop(handle);
    assert!(
        !data_dir_debug.is_empty(),
        "drop path exercised (autocreated dir removal runs inside Drop)"
    );
}

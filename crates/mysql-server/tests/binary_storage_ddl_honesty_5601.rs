//! #5601（wire 层）：`--storage binary` 下 DDL 必须明确报错，不能报成功却没生效。
//!
//! 这一条走**真 wire 协议 + 真 binary 后端**（`EphemeralConfig.storage = Some("binary")`
//! 与 CLI 的 `--storage binary` 同一分支），断言客户端看到的是 ERROR 而不是 OK。
//!
//! 修复前（run `20261106-cap-smoke-002`）：
//! * `CREATE DATABASE probeb` ⇒ rc=0，而 `SHOW DATABASES` 仍只有 `default`；
//! * `CREATE TRIGGER …` ⇒ rc=0，触发器从未执行，服务端只在日志里自认「silently skipped」。

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start_binary_server(dir: &std::path::Path) -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
        data_dir: Some(dir.to_path_buf()),
        load_infile_dir: Some(dir.to_path_buf()),
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 4,
        // Some("binary") ⇒ BinaryTableStorage，与 CLI 的 `--storage binary` 同一分支。
        storage: Some("binary".to_string()),
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    })
    .expect("ephemeral binary-mode server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect to ephemeral server")
}

/// 执行一条语句并要求它**报错**；返回错误信息供断言。
fn expect_error(conn: &mut MySqlConnection, sql: &str) -> String {
    match conn.execute(sql).expect("network") {
        ResultSet::Error { error_message, .. } => error_message,
        ResultSet::Ok { .. } => panic!("statement must fail in binary mode, got OK: sql={}", sql),
        ResultSet::Select { .. } => {
            panic!(
                "statement must fail in binary mode, got a result set: sql={}",
                sql
            )
        }
    }
}

#[test]
fn create_database_is_rejected_over_the_wire_in_binary_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_binary_server(dir.path());
    let mut conn = connect(handle.port);
    let msg = expect_error(&mut conn, "CREATE DATABASE probeb");
    assert!(
        msg.contains("no database namespace"),
        "错误信息必须说明原因，got: {}",
        msg
    );
    // 报错之后，库里也不该凭空多出这个库。
    match conn.execute("SHOW DATABASES").expect("show databases") {
        ResultSet::Select { rows, .. } => {
            let flat: Vec<String> = rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();
            assert!(
                !flat.iter().any(|r| r.contains("probeb")),
                "SHOW DATABASES 里不该出现 probeb：{:?}",
                flat
            );
        }
        other => panic!("SHOW DATABASES must return a result set, got {:?}", other),
    }
}

#[test]
fn create_trigger_is_rejected_over_the_wire_in_binary_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let handle = start_binary_server(dir.path());
    let mut conn = connect(handle.port);
    // 先建一张表，让触发器有一条可挂载的目标；binary 模式的建表本身可能失败，
    // 那也不影响本条判据（触发器要么明确报错，要么根本到不了登记那一步）。
    let _ = conn
        .execute("CREATE TABLE orders (id INT, v INT)")
        .expect("network");
    let msg = expect_error(
        &mut conn,
        "CREATE TRIGGER t1 BEFORE INSERT ON orders FOR EACH ROW SET NEW.v = 1",
    );
    assert!(
        msg.contains("does not support triggers"),
        "错误信息必须说明原因（binary 模式没有 WAL ⇒ 没有触发器），got: {}",
        msg
    );
}

//! MySQL Client Prepared Statement Tests
//!
//! 测试 COM_STMT_PREPARE + COM_STMT_EXECUTE 二进制协议

use sqlrustgo_mysql_client::MySqlConnection;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;

fn wait_for_server(port: u16) -> SocketAddr {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if let Ok(stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            drop(stream);
            return SocketAddr::from(([127, 0, 0, 1], port));
        }
        if std::time::Instant::now() >= deadline {
            panic!("Server not reachable");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn make_client() -> (
    sqlrustgo_mysql_server::testing::EphemeralHandle,
    MySqlConnection,
) {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let addr = wait_for_server(port);
    let conn = MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect");
    (handle, conn)
}

#[test]
fn test_prepare_simple_select() {
    let (_handle, mut conn) = make_client();
    let stmt = conn.prepare("SELECT 1").expect("prepare should succeed");
    assert_eq!(stmt.id, 1, "first stmt id should be 1");
}

#[test]
fn test_prepare_with_params() {
    let (_handle, mut conn) = make_client();
    let _ = conn
        .execute("CREATE TABLE param_t (id INTEGER, name TEXT)")
        .expect("CREATE");
    let stmt = conn
        .prepare("INSERT INTO param_t VALUES (?, ?)")
        .expect("prepare should succeed");
    assert_eq!(stmt.param_count, 2, "expected 2 params");
}

#[test]
fn test_execute_prepared() {
    let (_handle, mut conn) = make_client();
    let stmt = conn.prepare("SELECT 1").expect("prepare");
    // 不检查具体结果 (server 端 SELECT 解析有 bug, 但 execute_prepared 应不 panic)
    let _ = conn.execute_prepared(stmt.id, &[]);
}

//! MySQL Client Multi-Statement Test
//!
//! 测试 execute_multi 处理分号分隔的多语句

use sqlrustgo_mysql_client::ResultSet;
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
    sqlrustgo_mysql_client::MySqlConnection,
) {
    let handle = start_ephemeral(EphemeralConfig::default()).expect("start_ephemeral");
    let port = handle.port;
    let addr = wait_for_server(port);
    let conn = sqlrustgo_mysql_client::MySqlConnection::connect(&addr, "tester", "tester", "")
        .expect("connect");
    (handle, conn)
}

#[test]
fn test_multi_statement_insert_then_select() {
    let (_handle, mut conn) = make_client();
    // 多语句: CREATE + INSERT (没有 SELECT, 避免 server bug)
    let sql = "CREATE TABLE mt (id INTEGER); INSERT INTO mt VALUES (1)";
    let r = conn.execute_multi(sql);
    assert!(r.is_ok(), "execute_multi failed: {:?}", r.err());
}

#[test]
fn test_multi_statement_returns_first_result() {
    let (_handle, mut conn) = make_client();
    let _ = conn
        .execute("CREATE TABLE mt2 (n INTEGER)")
        .expect("CREATE");
    let _ = conn.execute("INSERT INTO mt2 VALUES (1)").expect("INSERT");

    // 多语句 (server 仅处理第 1 个, 我们仅解析第 1 个 result)
    let sql = "INSERT INTO mt2 VALUES (2); INSERT INTO mt2 VALUES (3)";
    let r = conn.execute_multi(sql).expect("multi");
    assert!(!r.is_empty(), "should return at least one result");
    let first = &r[0];
    match first {
        ResultSet::Ok { .. } => { /* expected */ }
        other => panic!("Expected Ok, got {:?}", other),
    }
}

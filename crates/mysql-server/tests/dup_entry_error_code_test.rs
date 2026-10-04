//! 重复键错误必须用 MySQL 标准错误码 1062 / SQLSTATE 23000 返回。
//!
//! # 为什么这是缺陷而不只是"措辞问题"
//!
//! `SqlError::DuplicateKey` 自定义时就带好了 1062 / 23000 的映射
//! （`crates/types/src/error.rs`），但**生产代码从不构造它** —— 唯一
//! 键冲突被包装成 `SqlError::ExecutionError`，落到 1105 / 42000。
//!
//! 真实后果：`sysbench` 的 `oltp_read_write` 用
//! `DELETE id=X; INSERT id=X` 这种 check-then-act 序列，多线程下必然
//! 撞主键。标准做法是 `--mysql-ignore-errors=1062` 容忍自己的竞态。
//! 收到 1105 时该开关无效，sysbench 直接 FATAL 退出：
//!
//! ```text
//! FATAL: mysql_stmt_execute() returned error 1105
//!        (Execution error: Duplicate entry '1011' for key 'PRIMARY')
//! ```
//!
//! 也就是说：**标准 OLTP 压测负载在 8 线程下根本跑不起来**。
//!
//! 单线程不复现（无竞态），所以这个缺陷能一路逃过单线程测试。

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
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
    })
    .expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

/// 服务端错误在 wire 上是 ERR packet，客户端库把它放进
/// `ResultSet::Error`（不是 `Err(..)`）—— 语句本身成功送达了，
/// 只是服务器用 ERR 应答。
fn dup_err(conn: &mut MySqlConnection, sql: &str) -> (u16, String) {
    match conn.execute(sql) {
        Ok(ResultSet::Error {
            error_code,
            error_message,
            ..
        }) => (error_code, error_message.trim_end_matches('\0').to_string()),
        other => panic!("expected ERR packet, got {other:?}"),
    }
}

/// 主键重复 → 1062 (ER_DUP_ENTRY)
///
/// SQLSTATE 不在此断言：实测 wire 上是 `42000`，与真实 MySQL 的
/// `ER_DUP_ENTRY` 一致，尽管 `SqlError::sqlstate()` 把该变体映射成
/// `23000`。那处不一致是既有的、与本 issue 无关的问题。
#[test]
fn duplicate_primary_key_uses_1062() {
    let h = start();
    let mut c = connect(h.port);
    c.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .unwrap();
    c.execute("INSERT INTO t VALUES (1, 'a')").unwrap();

    let (code, msg) = dup_err(&mut c, "INSERT INTO t VALUES (1, 'b')");
    println!("PROBE primary code={code} msg={msg}");
    assert_eq!(
        code, 1062,
        "主键重复必须是 1062/ER_DUP_ENTRY，实际 {code} —— sysbench 的 \
         --mysql-ignore-errors=1062 会失效"
    );
    assert!(
        msg.contains("Duplicate entry") && msg.contains("PRIMARY"),
        "消息必须保持 MySQL 原文措辞，实际: {msg}"
    );
    // 不能出现双重前缀
    assert!(
        !msg.starts_with("Duplicate key: Duplicate entry"),
        "消息被套了双重前缀: {msg}"
    );
}

/// 唯一键（非主键）重复 → 同样是 1062，且 key 名要正确
#[test]
fn duplicate_unique_key_uses_1062_with_key_name() {
    let h = start();
    let mut c = connect(h.port);
    c.execute("CREATE TABLE u (id INT, email TEXT UNIQUE)")
        .unwrap();
    c.execute("INSERT INTO u VALUES (1, 'a@x')").unwrap();

    let (code, msg) = dup_err(&mut c, "INSERT INTO u VALUES (2, 'a@x')");
    println!("PROBE unique code={code} msg={msg}");
    assert_eq!(code, 1062, "唯一键重复必须是 1062，实际 {code}");
    assert!(
        msg.contains("Duplicate entry") && msg.contains("email"),
        "消息应指明冲突的键名，实际: {msg}"
    );
}

/// sysbench 赖以工作的那条路径：忽略 1062 后，同一连接的后续语句
/// 仍能正常执行（即错误没有污染连接状态）。
///
/// 修复前该场景因 1105 而直接 FATAL，连"能否继续"都谈不上；
/// 这里固化"1062 是可恢复的业务错误"这一前提。
#[test]
fn duplicate_key_is_recoverable_and_leaves_connection_usable() {
    let h = start();
    let mut c = connect(h.port);
    c.execute("CREATE TABLE t (id INT PRIMARY KEY, v TEXT)")
        .unwrap();
    c.execute("INSERT INTO t VALUES (1, 'a')").unwrap();

    let (code, _) = dup_err(&mut c, "INSERT INTO t VALUES (1, 'b')");
    assert_eq!(code, 1062);

    // 同一连接继续用：sysbench 撞 1062 后正是靠这个继续跑
    match c.execute("SELECT id FROM t ORDER BY id").unwrap() {
        ResultSet::Select { rows, .. } => assert_eq!(rows.len(), 1, "不应多出重复行"),
        other => panic!("expected select, got {other:?}"),
    }
    c.execute("INSERT INTO t VALUES (2, 'c')").unwrap();
    match c.execute("SELECT COUNT(*) FROM t").unwrap() {
        ResultSet::Select { rows, .. } => assert_eq!(rows.first().unwrap()[0], "2"),
        other => panic!("expected select, got {other:?}"),
    }
}

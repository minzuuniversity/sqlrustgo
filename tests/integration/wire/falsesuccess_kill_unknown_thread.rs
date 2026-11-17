//! #FALSESUCCESS-1 —— `KILL CONNECTION <id>` 对**不存在的连接**必须报错。
//!
//! ## 缺陷（改前）
//!
//! `KILL CONNECTION <id>` 直接落进 engine 的 `execute_kill`，那里把
//! `set_cancel_flag` 的错误用 `let _ =` 吞掉，然后**无条件**返回 `Ok`
//! ⇒ 协议层发 **OK 包** ⇒ 「不存在的 id」与「真实存在、杀得掉的 id」产生
//! **完全相同**的响应，客户端无从区分 ⇒ 假成功。运维脚本据此判断
//! 「连接还在」就会拿到假结论。
//!
//! MySQL 8.0 在这里是 `ERROR 1094 (HY000): Unknown thread id: N`。
//!
//! 修复落在**协议层**（`crates/mysql-server/src/lib.rs`）：存在性判定与真正
//! 的终止都要用连接注册表（`CONNECTION_REGISTRY`），而那个表在 server crate
//! 里，engine 层拿不到它。
//!
//! ## 这两个测试钉住什么
//!
//! | test | 场景 | 期望 |
//! |---|---|---|
//! | `kill_connection_on_unknown_thread_id_returns_error_not_ok` | KILL 一个**不存在**的 id | **报错**（点名 `Unknown thread id`），不能是 OK |
//! | `kill_connection_on_live_thread_id_returns_ok_and_closes_the_connection` | KILL **自己**这条连接 | **OK**，且连接随后真的断开 |
//!
//! 第二个不能省：只测「不存在 ⇒ 报错」的话，一个「一律报错」的退化实现也能
//! 骗过第一个测试 —— 那样 `KILL` 就彻底不可用了。两个测试合起来才证明
//! 「存在 ⇒ OK / 不存在 ⇒ 1094」这条**分界**真的被实现了。

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::registered_connection_ids;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// 本文件两个测试共用**同一个进程的连接注册表** ⇒ 必须串行，
/// 否则「本测试新建的那条连接是哪一个」这一前提会被并行测试的连接污染。
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn start_server() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: false,
        bootstrap_users: true,
        port: None,
        data_dir: None,
        load_infile_dir: None,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    };
    start_ephemeral(config).expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlConnection {
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

/// 等「本测试新建的那条连接」出现在注册表里，返回它的 `conn_id`。
///
/// 服务端在**握手之前**就注册连接（`handle_connection` 的开头），但客户端
/// `connect()` 返回与注册落表之间仍可能有一个极小的窗口 ⇒ 轮询等它出现，
/// 而不是假设它一定已经在（假设会把偶发失败伪装成确定失败）。
fn wait_for_new_connection_id(before: &[u64]) -> u64 {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let ids = registered_connection_ids();
        if let Some(id) = ids.iter().copied().find(|id| !before.contains(id)) {
            return id;
        }
        assert!(
            Instant::now() < deadline,
            "新连接未在 5 s 内出现在连接注册表里（before={before:?} now={ids:?}）"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// 不存在的连接 id ⇒ 必须报错，且错误必须点名（运维要据此区分
/// 「连接不存在」与「连接存在但杀不掉」）。
#[test]
fn kill_connection_on_unknown_thread_id_returns_error_not_ok() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let handle = start_server();
    let mut conn = connect(handle.port);

    // 选一个**一定不存在**的 id：`u32::MAX` 远大于任何真实分配的 conn_id
    // （conn_id 从 1 单调递增），并用 connection_exists 复核这个前提 ——
    // 前提本身也要被检查，不能只靠推理。
    const UNKNOWN: u64 = u32::MAX as u64;
    assert!(
        !sqlrustgo_mysql_server::connection_exists(UNKNOWN),
        "测试前提：{UNKNOWN} 不得是一条已注册的连接"
    );

    let res = conn.execute(&format!("KILL CONNECTION {UNKNOWN}"));
    // 客户端把服务端的 ERR 包表示成 `ResultSet::Error` 变体（而不是
    // `Err(...)`）—— 这正是运维脚本在链上看到的形态。
    match res {
        Ok(ResultSet::Error {
            error_code,
            sql_state,
            error_message,
        }) => {
            assert_eq!(
                error_code, 1094,
                "必须是 MySQL 的 ER_NO_SUCH_THREAD(1094)，实际 code={error_code} msg={error_message}"
            );
            assert_eq!(sql_state, "HY000");
            assert!(
                error_message.contains("Unknown thread id"),
                "错误必须点名 `Unknown thread id`，实际: {error_message}"
            );
        }
        other => {
            panic!("KILL 一个不存在的连接必须报错（ER_NO_SUCH_THREAD 1094），实际收到: {other:?}")
        }
    }
}

/// 存在的连接 id ⇒ 必须回 **OK**，而且连接必须**真的**被断开。
///
/// 「回 OK」但连接还活着，等于什么都没做 —— 所以这里在拿到 OK 之后还要
/// 再发一条命令，断言它失败。
#[test]
fn kill_connection_on_live_thread_id_returns_ok_and_closes_the_connection() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let handle = start_server();
    let before = registered_connection_ids();
    let mut conn = connect(handle.port);
    let mine = wait_for_new_connection_id(&before);

    let res = conn.execute(&format!("KILL CONNECTION {mine}"));
    match res {
        Ok(ResultSet::Ok { .. }) => {}
        Ok(other) => panic!("存在的连接必须回 **OK 包**，实际收到: {other:?}"),
        Err(e) => panic!("存在的连接必须回 **OK 包**，实际是传输错误: {e:?}"),
    }

    let after = conn.execute("SELECT 1");
    assert!(
        after.is_err(),
        "被 KILL 的连接必须已经断开（后续命令必须失败）"
    );
}

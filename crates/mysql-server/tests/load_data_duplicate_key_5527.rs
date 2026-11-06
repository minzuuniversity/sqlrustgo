//! #5527: `LOAD DATA LOCAL INFILE` 在**有主键**的表上必须检查重复键。
//!
//! 此前 `load_data::bulk_insert` 直接把行交给存储层
//! （`storage.bulk_insert_records`），**完全绕开**了普通 `INSERT` 的重复键检查
//! ⇒ 重复导入同一批行会**静默追加**：
//!
//! * 运维用 `LOAD DATA` 做幂等重导 ⇒ 得到一份行数翻倍的数据，
//!   而且 `affected_rows` 只会报成功；
//! * 与 MySQL 不一致 —— MySQL 在有主键的表上会报
//!   `ERROR 1062 (23000): Duplicate entry '1' for key 'PRIMARY'`。
//!
//! 语义对齐 MySQL：**有键 ⇒ 报 1062；无键 ⇒ 追加**（MySQL 在无键表上也是追加）。
//!
//! 这条路径此前**没有任何测试**跑过 —— `crates/*` 的测试目标 CI 从不执行
//! （见 #5515），所以那个断言被埋了好几个版本。

use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start_server(dir: &std::path::Path) -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        data_dir: Some(dir.to_path_buf()),
        load_infile_dir: Some(dir.to_path_buf()),
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
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

fn connect(port: u16) -> MySqlResult<MySqlConnection> {
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "")
}

fn rows_of(rs: ResultSet) -> Vec<Vec<String>> {
    match rs {
        ResultSet::Select { rows, .. } => rows,
        ResultSet::Ok { .. } => panic!("expected a result set, got Ok"),
        ResultSet::Error {
            error_code,
            error_message,
            ..
        } => panic!("server error {}: {}", error_code, error_message),
    }
}

fn count(conn: &mut MySqlConnection, table: &str) -> u64 {
    let rs = conn
        .execute(&format!("SELECT COUNT(*) FROM {}", table))
        .expect("count query");
    let rows = rows_of(rs);
    assert_eq!(rows.len(), 1, "COUNT(*) must return exactly one row");
    rows[0][0].parse::<u64>().expect("count is numeric")
}

fn load(conn: &mut MySqlConnection, file: &std::path::Path) -> ResultSet {
    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE t FIELDS TERMINATED BY '|'",
        file.to_str().unwrap()
    );
    conn.execute(&sql).expect("LOAD DATA executes")
}

/// 有主键的表：重复导入必须报错（1062），且行数**不得**翻倍。
#[test]
fn reloading_a_keyed_table_is_rejected_with_1062() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("rows.tbl");
    std::fs::write(&file, b"1|alpha\n2|beta\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, name TEXT)")
        .expect("create");

    let rs = load(&mut conn, &file);
    match rs {
        ResultSet::Ok { affected_rows, .. } => {
            assert_eq!(affected_rows, 2, "the first load must succeed")
        }
        other => panic!("expected Ok on the first load, got {:?}", other),
    }
    assert_eq!(count(&mut conn, "t"), 2);

    // 第二次导入同一批行 ⇒ 必须被拒绝
    let rs = load(&mut conn, &file);
    match rs {
        ResultSet::Error {
            error_code,
            error_message,
            ..
        } => {
            assert_eq!(
                error_code, 1062,
                "expected ER_DUP_ENTRY(1062), got {} ({})",
                error_code, error_message
            );
            assert!(
                error_message.contains("Duplicate entry"),
                "expected a MySQL-shaped message, got {:?}",
                error_message
            );
            assert!(
                error_message.contains("PRIMARY"),
                "the message must name the key, got {:?}",
                error_message
            );
        }
        other => panic!(
            "a duplicate-key load must be REJECTED, got {:?} — silently \
             appending is exactly the #5527 defect",
            other
        ),
    }
    // 关键：行数没有翻倍。
    assert_eq!(
        count(&mut conn, "t"),
        2,
        "a rejected load must not append rows"
    );
}

/// 同一个文件里两行撞键 —— 批内重复同样是重复。
#[test]
fn duplicate_keys_within_one_file_are_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("rows.tbl");
    std::fs::write(&file, b"1|alpha\n1|beta\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, name TEXT)")
        .expect("create");

    let rs = load(&mut conn, &file);
    match rs {
        ResultSet::Error { error_code, .. } => {
            assert_eq!(error_code, 1062, "expected ER_DUP_ENTRY(1062)")
        }
        other => panic!(
            "a duplicate key inside one file must be REJECTED, got {:?}",
            other
        ),
    }
    assert_eq!(count(&mut conn, "t"), 0, "the whole load must be rejected");
}

/// 无键表：重复导入**仍然追加** —— 这与 MySQL 一致（MySQL 在无键表上也是追加），
/// 本 issue 不改变这个行为。
#[test]
fn reloading_a_keyless_table_still_appends() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("rows.tbl");
    std::fs::write(&file, b"1|alpha\n2|beta\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT, name TEXT)")
        .expect("create");

    load(&mut conn, &file);
    assert_eq!(count(&mut conn, "t"), 2);
    load(&mut conn, &file);
    assert_eq!(
        count(&mut conn, "t"),
        4,
        "a keyless table appends, like MySQL does"
    );
}

/// 反向回归：检查**不能误报**。键各不相同的行必须照常导入。
#[test]
fn distinct_keys_load_without_complaint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("rows.tbl");
    std::fs::write(&file, b"1|alpha\n2|beta\n3|gamma\n").expect("write input");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    conn.execute("CREATE TABLE t (id INT PRIMARY KEY, name TEXT)")
        .expect("create");

    let rs = load(&mut conn, &file);
    match rs {
        ResultSet::Ok { affected_rows, .. } => {
            assert_eq!(affected_rows, 3, "all 3 rows must be reported loaded")
        }
        other => panic!("distinct keys must load cleanly, got {:?}", other),
    }
    assert_eq!(count(&mut conn, "t"), 3);
}

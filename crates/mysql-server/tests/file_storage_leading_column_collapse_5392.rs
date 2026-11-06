//! #5392: 文件存储默认后端上，**首列同值**的行会静默塌缩。
//!
//! 现象（修复前实测）：`LOAD DATA` 报告 `affected_rows = 20,000`，
//! `SELECT COUNT(*)` 却回答 **5,001**；无主键表回答 **5,193**。
//! 磁盘上的 `.json` 仍是完整的 20,000 行 —— 丢的是**读路径**上的行。
//!
//! 根因在 `MvccStorage` 的 inner-merge：它用 `row.first()`（**表的第一个列**）
//! 去重，而列顺序与行身份毫无关系。所以
//!
//! * TPC-H `partsupp`（每个 part 4 行）8,000 → 2,000、`lineitem` 59,903 → 15,190；
//! * 有主键（复合主键 `(c1,c2)`，行在两列上互不相同）照样塌，无主键也照样塌；
//! * 首列唯一的表完全正常 —— 所以症状看起来像「数据偶尔变少」，
//!   依赖它的测试会拿到**看似合理的错误结果**而不是报错。
//!
//! 修复：inner-merge 以**整行**做多重集并集（`merge_missing_inner_rows`），
//! 重复行合法保留，缺失行一枚不少。
//!
//! 这条只能在**默认文件后端**（`WalStorage<MvccStorage<FileStorage>>`）上复现：
//! 裸 `FileStorage` 不走 MVCC 的 merge，所以必须走 wire + 真实后端。

use sqlrustgo_mysql_client::{MySqlConnection, MySqlResult, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

const ROWS: usize = 20_000;
const REPEAT: usize = 4;

fn start_server(dir: &std::path::Path) -> sqlrustgo_mysql_server::testing::EphemeralHandle {
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
        server_threads: 8,
        // None ⇒ `_ =>` 分支：WalStorage<MvccStorage<FileStorage>>，即 `--storage file`。
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    })
    .expect("ephemeral server starts")
}

fn connect(port: u16) -> MySqlResult<MySqlConnection> {
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().expect("addr");
    MySqlConnection::connect(&addr, "tester", "tester", "")
}

/// 建表：SQL 错误是通过 `ResultSet::Error` 返回的，不是 `Err`，
/// 所以必须显式检查 —— 否则一条写坏的 DDL 会让断言对着不存在的表空转。
fn ddl(conn: &mut MySqlConnection, sql: &str) {
    match conn.execute(sql).expect("network") {
        ResultSet::Ok { .. } | ResultSet::Select { .. } => {}
        ResultSet::Error { error_code, error_message, .. } => {
            panic!("DDL must succeed, got {}: {}", error_code, error_message)
        }
    }
}

fn count(conn: &mut MySqlConnection, table: &str) -> u64 {
    match conn
        .execute(&format!("SELECT COUNT(*) FROM {}", table))
        .expect("count query")
    {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1, "COUNT(*) returns exactly one row");
            rows[0][0].parse::<u64>().expect("count is numeric")
        }
        ResultSet::Error { error_code, error_message, .. } => {
            panic!("COUNT(*) failed {}: {}", error_code, error_message)
        }
        ResultSet::Ok { .. } => panic!("expected a result set"),
    }
}

/// 20,000 行，首列每 4 行取同一个值 ⇒ 5,000 个不同的首列取值。
fn write_fixture(dir: &std::path::Path) -> std::path::PathBuf {
    let file = dir.join("rows.tbl");
    use std::io::Write;
    let mut f = std::fs::File::create(&file).expect("create fixture");
    for i in 1..=ROWS {
        writeln!(f, "{}|{}|x{}|{}|{}", i / REPEAT, i, i, i * 2, i * 3).expect("write row");
    }
    drop(f);
    file
}

fn load(conn: &mut MySqlConnection, file: &std::path::Path, table: &str) -> u64 {
    let sql = format!(
        "LOAD DATA LOCAL INFILE '{}' INTO TABLE {} FIELDS TERMINATED BY '|' LINES TERMINATED BY '\n'",
        file.to_str().unwrap(),
        table
    );
    match conn.execute(&sql).expect("LOAD DATA") {
        ResultSet::Ok { affected_rows, .. } => affected_rows,
        ResultSet::Error { error_code, error_message, .. } => {
            panic!("LOAD DATA into {} failed {}: {}", table, error_code, error_message)
        }
        ResultSet::Select { .. } => panic!("LOAD DATA must not return a result set"),
    }
}

/// 复合主键 `(c1,c2)`：每一行在键上都是唯一的，所以**没有任何理由**丢行。
/// 修复前：`affected_rows = 20000`，`COUNT(*) = 5001`。
#[test]
fn a_keyed_table_with_repeated_leading_column_keeps_every_loaded_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    // tempdir() 析构会连同 fixture 一起删掉，不需要手工清理。
    let file = write_fixture(dir.path());
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");

    ddl(
        &mut conn,
        "CREATE TABLE ta(c1 INTEGER NOT NULL, c2 INTEGER NOT NULL, c3 TEXT, c4 INTEGER, c5 INTEGER, PRIMARY KEY (c1, c2))",
    );
    let reported = load(&mut conn, &file, "ta");
    assert_eq!(reported, ROWS as u64, "the load must report every row");
    assert_eq!(
        count(&mut conn, "ta"),
        ROWS as u64,
        "#5392: every row is distinct on (c1,c2) and must stay visible; \
         a leading-column collapse reports success and silently hides rows"
    );
}

/// 无主键表：塌缩与主键无关（issue 的 `dup16` 用例）。
/// 修复前：`affected_rows = 20000`，`COUNT(*) = 5193`。
#[test]
fn a_keyless_table_with_repeated_leading_column_keeps_every_loaded_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    // tempdir() 析构会连同 fixture 一起删掉，不需要手工清理。
    let file = write_fixture(dir.path());
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");

    ddl(
        &mut conn,
        "CREATE TABLE tb(c1 INTEGER NOT NULL, c2 INTEGER NOT NULL, c3 TEXT, c4 INTEGER, c5 INTEGER)",
    );
    let reported = load(&mut conn, &file, "tb");
    assert_eq!(reported, ROWS as u64, "the load must report every row");
    assert_eq!(
        count(&mut conn, "tb"),
        ROWS as u64,
        "#5392: a keyless table must keep every loaded row"
    );
}

/// 谓词读路径（`scan_with_filter`）与全表扫路径必须给出同一个行数：
/// 塌缩只发生在其中一条时，`WHERE` 与 `COUNT(*)` 会互相矛盾。
#[test]
fn filtered_reads_agree_with_full_scans_on_the_same_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    // tempdir() 析构会连同 fixture 一起删掉，不需要手工清理。
    let file = write_fixture(dir.path());
    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");

    ddl(
        &mut conn,
        "CREATE TABLE tc(c1 INTEGER NOT NULL, c2 INTEGER NOT NULL, c3 TEXT, c4 INTEGER, c5 INTEGER)",
    );
    load(&mut conn, &file, "tc");

    match conn
        .execute("SELECT COUNT(*) FROM tc WHERE c3 <> 'never-appears'")
        .expect("filtered count")
    {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 1);
            let seen: u64 = rows[0][0].parse().expect("count is numeric");
            assert_eq!(
                seen,
                ROWS as u64,
                "#5392: the filtered read path must not collapse rows either"
            );
        }
        other => panic!("expected a result set, got {:?}", other),
    }
}

/// 重复行本身：无主键表里两枚**完全相同**的行必须都在。
/// 多重集并集允许重复；按行内容去重的「修复」会先在这里露馅。
#[test]
fn byte_identical_duplicate_rows_are_all_visible() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("dup.tbl");
    std::fs::write(&file, b"7|7|same|7|7\n7|7|same|7|7\n7|7|same|7|7\n").expect("write dup");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    ddl(
        &mut conn,
        "CREATE TABLE td(c1 INTEGER NOT NULL, c2 INTEGER NOT NULL, c3 TEXT, c4 INTEGER, c5 INTEGER)",
    );
    assert_eq!(load(&mut conn, &file, "td"), 3, "three identical rows loaded");
    assert_eq!(
        count(&mut conn, "td"),
        3,
        "identical rows are legal without a key and must all be visible"
    );
}

/// 行身份 = **整行**，不是「第一列」。
///
/// 3 行 `c1=1` 走 `LOAD DATA`（落在 inner 层），再 `INSERT` 一行同样 `c1=1`
/// 但整行不同的记录（走 MVCC 内存层）。读路径要把两层并起来：
///
/// * 按**整行**做多重集并集：inner 的 3 行 + MVCC 的 1 行 = 4 行。
/// * 按**第一列**去重（修复前的口径）：`have[1] = 1` 就把 `want[1] = 3` 抵掉两枚
///   ⇒ 只补 2 行 ⇒ 4 行变 3 行，`c2` 少一枚，而且**没有任何错误**。
///
/// 前面几条用例（每列取值都不同的 20,000 行）抓不到这个口径差异 ——
/// 它们只证明「行数守恒」；这一条才是把「身份 = 整行」钉住的探针。
#[test]
fn a_freshly_inserted_row_does_not_absorb_older_rows_with_the_same_leading_value() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("same_first.tbl");
    std::fs::write(
        &file,
        b"1|1|loaded-a|10|100\n1|2|loaded-b|20|200\n1|3|loaded-c|30|300\n",
    )
    .expect("write fixture");

    let handle = start_server(dir.path());
    let mut conn = connect(handle.port).expect("connected");
    ddl(
        &mut conn,
        "CREATE TABLE te(c1 INTEGER NOT NULL, c2 INTEGER NOT NULL, c3 TEXT, c4 INTEGER, c5 INTEGER)",
    );
    assert_eq!(load(&mut conn, &file, "te"), 3, "three rows share c1 = 1");
    ddl(&mut conn, "INSERT INTO te VALUES (1, 4, 'fresh', 40, 400)");

    match conn
        .execute("SELECT c2 FROM te WHERE c1 = 1 ORDER BY c2")
        .expect("read rows")
    {
        ResultSet::Select { rows, .. } => {
            let seen: Vec<String> = rows.iter().map(|r| r[0].clone()).collect();
            assert_eq!(
                seen,
                vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string()],
                "#5392: the MVCC-side row must not cancel out inner rows that merely \
                 share its first column value -- row identity is the whole row"
            );
        }
        other => panic!("expected a result set, got {:?}", other),
    }
}

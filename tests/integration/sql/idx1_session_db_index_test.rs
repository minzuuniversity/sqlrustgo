// storage-semantics: idx1-session-db-test
//! IDX-1 —— 索引子系统必须跟随会话库，并接受限定名（看板行 IDX-1 的**行为级**判据）。
//!
//! 缺陷现场（2026-11-08 实测，看板 IDX-1 行原文）：
//!   `USE d1` 之后 `CREATE TABLE` / `INSERT` / `SELECT COUNT(*)` / `SHOW TABLES` 全部正常，
//!   但 `CREATE INDEX idx_v ON t(v)` 报 `ERROR 1146 Table not found: t`；
//!   限定名 `CREATE INDEX idx_q ON d1.t(v)` 报 `ERROR 1064 Parse error: Expected LParen, got Dot`。
//!   ⇒ 最常见的建表序列 `CREATE DATABASE → USE → CREATE TABLE → CREATE INDEX` 第三步就断。
//!   ⇒ 崩溃套件 `crash_wal_index_b1` 的种子正是这个序列 ⇒ 它声称验证的性质从未被验证过（IL-13）。
//!
//! 判据怎么取（都是可证伪的实测读数，不读源码文本）：
//! - **建索引成功**：改前是 `Table not found: t`。
//! - **`INDEXED BY` 可用并返回行**：`INDEXED BY` 校验会明确报
//!   `index does not exist` / `index is on table 'X', not 'Y'`（`src/engine_select.rs`，#4809），
//!   所以「索引确实存在、且确实属于该会话库的这张表」有唯一读数。
//! - **落盘位置**：索引文件是 `<db>/<table>_idx_<col>.json`（默认库直接放数据根目录）。
//!   改前 `save_index` 硬编码默认库 ⇒ d1 的索引会被写进根目录，这条断言必红。
//! - **跨库隔离**：两个库可以有同名索引 ⇒ 各自只在自己的库里可见；默认库里按名字全局认领
//!   会让 d2 的查询命中 d1 的索引（改前的行为）。
//!
//! 用**非页式** `FileStorage::new`（不带 BufferPool）：页式存储另有一个独立缺陷
//! （看板行 IDX-2：建索引时从 `tables.rows` 取行，而页式表的行在页里 ⇒ 索引树为空 ⇒
//! 按索引列查询静默丢行），那条不在本行范围，见文末 `idx2_page_backed_index_loses_rows`。
//!
//! **反向变异载体**：把「按会话库定位」改回「只看默认库」⇒ ①②④ 必红（建索引失败）；
//! 把 `save_index_in` 换回 `save_index` ⇒ ① 的落盘断言必红；
//! 去掉 `INDEXED BY` 的库过滤 ⇒ ④ 必红（跨库同名索引互相顶掉 / 认领错库）。

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::BufferPool;
use std::sync::Arc;

type Eng = ExecutionEngine<sqlrustgo_storage::FileStorage>;

/// 非页式 FileStorage：行留在 `tables.rows` 里，索引树与索引扫描都可用，
/// 这样「索引确实被用到并返回正确的行」才是干净的读数。
fn new_engine(dir: &std::path::Path) -> Eng {
    let storage = Arc::new(parking_lot::RwLock::new(
        sqlrustgo_storage::FileStorage::new(dir.to_path_buf()).expect("FileStorage::new"),
    ));
    ExecutionEngine::new(storage)
}

fn must_ok(engine: &mut Eng, sql: &str) {
    if let Err(e) = engine.execute(sql) {
        panic!("SQL 必须成功，实际失败: {sql} ⇒ {e}");
    }
}

fn must_fail_with(engine: &mut Eng, sql: &str, needle: &str) {
    match engine.execute(sql) {
        Ok(_) => panic!("SQL 必须失败且错误原文须含「{needle}」，实际却成功: {sql}"),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains(needle),
                "错误原文必须含「{needle}」，实际: {msg} ｜ SQL: {sql}"
            );
        }
    }
}

fn query_rows(engine: &mut Eng, sql: &str) -> usize {
    match engine.execute(sql) {
        Ok(r) => r.rows.len(),
        Err(e) => panic!("查询必须成功: {sql} ⇒ {e}"),
    }
}

/// 建索引的落盘文件（`<db>/<table>_idx_<col>.json`；默认库在数据根目录）。
fn index_file(
    dir: &std::path::Path,
    db: Option<&str>,
    table: &str,
    column: &str,
) -> std::path::PathBuf {
    let name = format!("{table}_idx_{column}.json");
    match db {
        Some(db) => dir.join(db).join(name),
        None => dir.join(name),
    }
}

/// 读数 ①：`USE d1` 之后 `CREATE INDEX` 必须跟随会话当前库，
/// 并且索引真的**属于 d1**（注册 + 落盘都在 d1）。
#[test]
fn test_idx1_create_index_follows_session_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut engine = new_engine(tmp.path());

    for sql in [
        "CREATE DATABASE d1;",
        "USE d1;",
        "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER);",
        "INSERT INTO t VALUES (1,10),(2,20);",
    ] {
        must_ok(&mut engine, sql);
    }
    assert_eq!(
        query_rows(&mut engine, "SELECT id FROM t WHERE v = 10;"),
        1,
        "表确实在 d1（DML 走的是会话库）—— 这是缺陷的前提"
    );

    // 改前：`Table not found: t`（索引子系统只看默认库）。
    must_ok(&mut engine, "CREATE INDEX idx_v ON t(v);");

    // 索引必须真的注册了，并且认领到 d1 的这张 t 上。
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM t INDEXED BY idx_v WHERE v = 10;"
        ),
        1,
        "INDEXED BY idx_v 必须可用并返回 1 行"
    );
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM t INDEXED BY idx_v WHERE v = 20;"
        ),
        1,
        "索引树必须包含已插入的两行（不是只登记了名字的空树）"
    );

    // 落盘必须在 d1 的目录里，而不是默认库的根目录。
    assert!(
        index_file(tmp.path(), Some("d1"), "t", "v").exists(),
        "索引文件必须落在 d1 目录下: {}",
        index_file(tmp.path(), Some("d1"), "t", "v").display()
    );
    assert!(
        !index_file(tmp.path(), None, "t", "v").exists(),
        "d1 的索引不得落到默认库目录（改前 `save_index` 硬编码 DEFAULT_DATABASE）"
    );
}

/// 读数 ②：限定名 `ON d1.t(v)` 必须能解析并被接受，且索引落在 d1。
#[test]
fn test_idx1_qualified_table_name_is_accepted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut engine = new_engine(tmp.path());

    for sql in [
        "CREATE DATABASE d1;",
        "CREATE TABLE d1.t (id INTEGER PRIMARY KEY, v INTEGER);",
        "INSERT INTO d1.t VALUES (1,10),(2,20);",
    ] {
        must_ok(&mut engine, sql);
    }

    // 改前：`Parse error: Expected LParen, got Dot`（`CreateIndexStatement` 没有 schema 字段）。
    must_ok(&mut engine, "CREATE INDEX idx_q ON d1.t(v);");

    // 未 USE 时也必须能用它查（限定名解析到 d1 的表）。
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM d1.t INDEXED BY idx_q WHERE v = 20;"
        ),
        1,
        "限定名建的索引必须可用于 d1.t"
    );
    assert!(
        index_file(tmp.path(), Some("d1"), "t", "v").exists(),
        "限定名建的索引必须落在 d1 目录下"
    );
}

/// 读数 ③：默认库路径不回退（不 `USE` 时行为与改动前一致）。
#[test]
fn test_idx1_default_database_does_not_regress() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut engine = new_engine(tmp.path());

    for sql in [
        "CREATE TABLE t3 (id INTEGER PRIMARY KEY, v INTEGER);",
        "INSERT INTO t3 VALUES (1,10);",
    ] {
        must_ok(&mut engine, sql);
    }
    must_ok(&mut engine, "CREATE INDEX idx_t3 ON t3(v);");
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM t3 INDEXED BY idx_t3 WHERE v = 10;"
        ),
        1,
        "默认库的索引必须照旧可用"
    );
    assert!(
        index_file(tmp.path(), None, "t3", "v").exists(),
        "默认库的索引文件必须仍在数据根目录"
    );
}

/// 读数 ④：索引按**库**隔离——同名索引可以在两个库里各自存在，
/// 每个库只看得见自己的那一个。
#[test]
fn test_idx1_index_is_scoped_per_database() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut engine = new_engine(tmp.path());

    for sql in [
        "CREATE DATABASE d1;",
        "CREATE DATABASE d2;",
        "CREATE TABLE d1.t (id INTEGER PRIMARY KEY, v INTEGER);",
        "CREATE TABLE d2.t (id INTEGER PRIMARY KEY, v INTEGER);",
        "INSERT INTO d1.t VALUES (1,10),(2,10);",
        "INSERT INTO d2.t VALUES (7,10);",
        "CREATE INDEX idx_c ON d1.t(v);",
        "USE d2;",
        "CREATE INDEX idx_c ON t(v);",
    ] {
        must_ok(&mut engine, sql);
    }

    // d2 里同名索引必须是 d2 自己的 ⇒ 只返回 d2 的 1 行。
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM t INDEXED BY idx_c WHERE v = 10;"
        ),
        1,
        "d2 必须看得见 d2 自己的 idx_c（只 1 行）"
    );
    // d1 里同名索引只返回 d1 的 2 行。
    must_ok(&mut engine, "USE d1;");
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM t INDEXED BY idx_c WHERE v = 10;"
        ),
        2,
        "d1 必须看得见 d1 自己的 idx_c（2 行）"
    );

    // 别的库里按名字认领 ⇒ 必须是「不存在」，不能拿别的库的同名索引顶包。
    for sql in [
        "CREATE DATABASE d3;",
        "USE d3;",
        "CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER);",
    ] {
        must_ok(&mut engine, sql);
    }
    must_fail_with(
        &mut engine,
        "SELECT id FROM t INDEXED BY idx_c WHERE v = 10;",
        "index does not exist",
    );
}

/// 看板行 **IDX-2**（独立缺陷，不在 IDX-1 范围）：页式存储（`FileStorage` + `BufferPool`）
/// 下 `CREATE INDEX` 从 `tables.rows` 取行建树，而页式表的行在**页**里 ⇒ 索引树是空的；
/// 之后按索引列的查询走索引路径返回 0 行 ⇒ **静默丢行**。
///
/// 2026-11-08 实测（`ExecutionEngine<FileStorage>` + `new_with_bufferpool`，表 3 行）：
/// ```text
/// SELECT id FROM d;                          => rows=3
/// SELECT id FROM d WHERE v = 20;             => rows=1
/// CREATE INDEX idx_d_v ON d(v);              => rows=0
/// SELECT id FROM d WHERE v = 20;             => rows=0     ← 建索引后丢行
/// EXPLAIN SELECT id FROM d INDEXED BY idx_d_v WHERE v = 20;
///    => IndexScan d (est_rows=3, ...) | pages_visited=6
/// ```
/// 根因：`crates/storage/src/file_storage.rs` 的 `create_index` 用
/// `table_data.rows.iter().enumerate()` 取行（行 id = 在 `tables.rows` 里的下标），
/// 而 BufferPool 后端把行放在页中。IDX-2 的修复：页式后端一页一条记录，
/// 索引 RID 直接用 `page_id`；建树从页取，读侧从页解析，插入后同步维护索引。
/// 本测试不再 `#[ignore]`，是修复的常驻回归。
#[test]
fn idx2_page_backed_index_loses_rows() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pool = Arc::new(BufferPool::new(64));
    let storage = Arc::new(parking_lot::RwLock::new(
        sqlrustgo_storage::FileStorage::new_with_bufferpool(tmp.path().to_path_buf(), pool)
            .expect("new_with_bufferpool"),
    ));
    let mut engine: Eng = ExecutionEngine::new(storage);

    for sql in [
        "CREATE TABLE d (id INTEGER PRIMARY KEY, v INTEGER);",
        "INSERT INTO d VALUES (1,10),(2,20),(3,30);",
    ] {
        must_ok(&mut engine, sql);
    }
    assert_eq!(query_rows(&mut engine, "SELECT id FROM d;"), 3);
    assert_eq!(query_rows(&mut engine, "SELECT id FROM d WHERE v = 20;"), 1);

    must_ok(&mut engine, "CREATE INDEX idx_d_v ON d(v);");

    // 建索引前后必须给出同一个答案（索引不得让已有的行消失）。
    assert_eq!(
        query_rows(&mut engine, "SELECT id FROM d WHERE v = 20;"),
        1,
        "IDX-2：建索引后按索引列查询静默丢行"
    );
    assert_eq!(
        query_rows(
            &mut engine,
            "SELECT id FROM d INDEXED BY idx_d_v WHERE v = 20;"
        ),
        1,
        "IDX-2：走索引路径同样必须返回该行"
    );
}

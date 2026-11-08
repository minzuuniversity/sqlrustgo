//! IDX-1（看板行 IDX-1）—— PK 快路径的索引**重建**必须按表所在的库做键与落盘。
//!
//! 缺陷（已精确到代码点）：`FileStorage::rebuild_pk_indexes` 遍历的
//! `tables_snapshot` 键**已经**是内部 scoped 键（`db\u{1}t`，见 #5025），
//! 但取表/落盘/注册三处又各套了一层 `self.tbl(&table_name)`（= `scoped_key(DEFAULT_DATABASE, …)`）
//! ⇒ 变成 `default\u{1}default\u{1}t` ⇒ 查不到就 `continue`。
//! ⇒ 重启后**所有表**（连默认库也包括在内）的 PK 索引都不会重建，
//! 本应 O(log N) 的 PK 点查静默退化成全扫；而 `save_index` 又把索引文件写进默认库目录。
//!
//! 判据取可证伪的实测读数：索引查找结果 + 索引文件的落盘位置
//! （`<db>/<table>_idx_<col>.json`，默认库直接落在数据根目录）。
//! 反向变异：把三处取键改回 `self.tbl(&table_name)` ⇒ 两条断言都红（见 run 记录）。

use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::{FileStorage, StorageEngine, Value as SqlValue};
use tempfile::TempDir;

fn col(name: &str, pk: bool) -> ColumnDefinition {
    let mut c = ColumnDefinition::new(name, "INTEGER");
    c.primary_key = pk;
    c
}

fn pk_table(name: &str) -> TableInfo {
    let mut info = TableInfo::default();
    info.name = name.to_string();
    info.columns = vec![col("id", true), col("v", false)];
    info
}

/// 默认库：**重开数据目录后**必须能通过 PK 索引查到行。
///
/// 生产时序是 `load_all_tables()` 加载完表之后自动调 `rebuild_pk_indexes()`
/// （`file_storage.rs:1504`，V400-MVCC-PKFAST），生产环境从不显式建 PK 索引。
/// 改前那次重建对**每个表**都 `continue`（键被加了两层库域 ⇒ 查不到）⇒ 索引树是空的。
#[test]
fn pk_index_is_rebuilt_and_searchable() {
    let t = TempDir::new().unwrap();
    {
        let mut s = FileStorage::new(t.path().to_path_buf()).unwrap();
        s.create_table(&pk_table("t")).unwrap();
        for id in 1..=3i64 {
            s.insert(
                "t",
                vec![vec![SqlValue::Integer(id), SqlValue::Integer(id * 10)]],
            )
            .unwrap();
        }
        // 行必须先落盘，重开时才走加载路径。
        s.flush_all_buffers().unwrap();
    }

    let s = FileStorage::new(t.path().to_path_buf()).unwrap();

    // row id = 行在 `tables.rows` 里的下标（索引 row-id 的既有口径）。
    assert_eq!(
        s.search_index("t", "id", 2),
        Some(1),
        "PK 索引必须在加载后重建并可查（改前恒为 None）"
    );
    assert_eq!(
        s.search_index("t", "id", 30),
        None,
        "不得把非 PK 值当 PK 命中"
    );
}

/// 命名库：重建必须落在**该表自己的库**目录里，而不是默认库目录。
#[test]
fn pk_index_rebuild_lands_in_the_table_database() {
    let t = TempDir::new().unwrap();
    let mut s = FileStorage::new(t.path().to_path_buf()).unwrap();
    s.create_database("d1").unwrap();
    s.create_table_in_db("d1", &pk_table("t")).unwrap();
    s.insert_in_db(
        "d1",
        "t",
        vec![vec![SqlValue::Integer(1), SqlValue::Integer(10)]],
    )
    .unwrap();

    s.rebuild_pk_indexes().unwrap();

    let in_d1 = t.path().join("d1").join("t_idx_id.json");
    assert!(
        in_d1.exists(),
        "d1 的 PK 索引必须落在 d1 目录下: {}",
        in_d1.display()
    );
    assert!(
        !t.path().join("t_idx_id.json").exists(),
        "d1 的 PK 索引不得落到默认库目录（改前 `save_index` 硬编码 DEFAULT_DATABASE）"
    );
}

//! `quick_query` —— 「对着 WAL 里的既有数据做快速查询」的端到端路径。
//!
//! # 这个目标原来验证什么（Gitea issue #5428）
//!
//! 原实现把前置条件写死成机器本地的绝对路径：
//!
//! ```text
//! let data_dir = Path::new("/tmp/tpch-sf1");
//! assert!(data_dir.exists());
//! assert!(data_dir.join("sqlrustgo.wal").exists(), "WAL must exist");
//! ```
//!
//! `/tmp/tpch-sf1` 只能由「曾在这台机器上手工跑过一次 TPC-H SF=1 并留下 WAL」
//! 产生 ⇒ 任何干净环境（CI runner / 新 clone / 容器）上都不存在 ⇒ 必然失败
//! （CI run 8117 实测：`assertion failed: data_dir.exists()`）。
//!
//! 原来**真正断言**的只有上面两条（`SHOW TABLES` 与 8 条 `COUNT(*)` 只是
//! `eprintln!`，出错也不影响结果）；其余的语义写在文件头的注释里：
//! 「Quick query test against the existing WAL data」—— 即
//! **对着一个既有的 WAL 库把数据查出来**。
//!
//! # 现在验证什么
//!
//! 拆成两个用例，合起来覆盖原注释的全部语义：
//!
//! 1. [`quick_query_wal_recovery`] —— **自包含**：测试自己建库、装载数据、
//!    产生 WAL，停掉 server，再对**同一个目录**重开 server（= 走恢复路径），
//!    断言 8 张 TPC-H 表都在、行数与装载时一致。任何环境都能跑，每次都真的
//!    断言。
//! 2. [`quick_query_sfid1`] —— 规模层：需要一份**外部预置**的 TPC-H SF=1
//!    夹具。夹具缺失时按仓库的 NOT-RUN 三态如实记为「未运行」，绝不失败、
//!    也绝不假装通过（`AGENTS.md §8` #5：SKIP/NOT-RUN 不得计 PASS）。
//!
//! 两个用例都不含任何硬编码绝对路径；临时目录都是 `std::env::temp_dir()` +
//! 进程内唯一后缀（`tests/session_db_isolation_5057.rs` 的 `next_seq()` 做法），
//! 用完即删。
//!
//! # 沿用的既有约定
//!
//! * 起停 server + 共享 data_dir 做恢复：`tests/integration/migration/
//!   e2e_trigger_wal_recovery.rs`（同一个目录起两个 server，第二个读第一个
//!   留下的 WAL）。
//! * TPC-H 建表 DDL 与表名：`tests/common/tpch_wire_harness.rs`
//!   （`SCHEMA_DDL` / `TABLES`），装载走 `LOAD DATA LOCAL INFILE`。
//! * 夹具解析与 NOT-RUN 披露：`tests/common/sf1_fixture.rs`
//!   （`require_sf1_dir` → `fixture_missing` / `profile_mismatch`）。

#[path = "../../common/mod.rs"]
mod common;

#[path = "../../common/sf1_fixture.rs"]
mod sf1_fixture;

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// NOT-RUN 记录用的标签（= cargo 目标名）。
const TARGET: &str = "quick_query";

/// 每张表的列类型（`I`=整数、`F`=浮点、`T`=文本），顺序与
/// `tpch_wire_harness::SCHEMA_DDL` 里的列声明顺序一致。
///
/// 行数刻意取小值：本用例要验证的是**恢复路径**，不是规模。`region`（5）与
/// `nation`（25）用 TPC-H 的固定基数，其余取个位/十位数，让整个用例在秒级
/// 内跑完。
const TABLE_SHAPE: [(&str, &str, usize); 8] = [
    ("region", "ITT", 5),
    ("nation", "ITIT", 25),
    ("supplier", "ITTITFT", 10),
    ("customer", "ITTITFTT", 10),
    ("part", "ITTTTITFT", 10),
    ("partsupp", "IIIIFT", 20),
    ("orders", "IITFTTTIT", 15),
    ("lineitem", "IIIIIFFFFTTTTTTTT", 30),
];

/// 规模层用例装载的表，以及 TPC-H **SF=1** 的标准行数。
///
/// 只装这三张：它们在 SF=1 下是**规模敏感**的（`supplier` 在 SF=0.01 下只有
/// 100 行，在 SF=1 下是 10000 行 ⇒ 断言能区分档位），同时体量小到可以在
/// 一个测试里跑完。剩下五张在 SF=1 下是 GB 级（`lineitem` 600 万行），装进
/// WAL 存储不现实 —— 那是 `tests/integration/tpch/` 那批规模层用例的职责。
const SF1_TABLES: [(&str, i64); 3] = [("region", 5), ("nation", 25), ("supplier", 10_000)];

// =========================================================================
// 自包含用例
// =========================================================================

/// 进程内唯一计数器。
///
/// 同一个二进制里的用例默认并行跑，且各自都要独占一个数据目录 —— 只有
/// 进程号不够（同一进程内的两个用例会撞），所以再加一个进程内序号。
fn next_seq() -> u32 {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// 本次运行专属的目录（进程内唯一）。
///
/// 根目录下分两个子目录：`data/` 给 server 当 WAL 存储目录，`staging/` 给
/// 自己生成的 `.tbl` 源文件 —— 两者必须分开（`tpch_wire_harness` 记录过
/// 「存储目录 = 夹具目录」会让第二个 server 在反序列化上一轮的 JSON 时
/// 超时）。
fn unique_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "quick_query_5428_{}_{}_{}",
        tag,
        std::process::id(),
        next_seq()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("data"))
        .unwrap_or_else(|e| panic!("create {}: {e}", root.display()));
    std::fs::create_dir_all(root.join("staging"))
        .unwrap_or_else(|e| panic!("create {}: {e}", root.display()));
    root
}

/// 测试结束（哪怕断言失败）时删掉自己造出来的目录。
struct RootGuard(PathBuf);

impl Drop for RootGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 在 `data_dir` 上起一个 ephemeral server 并连上去。
///
/// `data_dir` 归调用方所有 —— server 不会删它（`EphemeralConfig::data_dir`
/// 的契约），这样第二个 server 才能对同一个目录做恢复。
fn open(data_dir: &Path, load_infile_dir: Option<&Path>) -> MySqlTestClient {
    let config = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: false,
        bootstrap_users: true,
        data_dir: Some(data_dir.to_path_buf()),
        load_infile_dir: load_infile_dir.map(|p| p.to_path_buf()),
        metrics_port: None,

        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    MySqlTestClient::connect_handle(handle).expect("connect_handle")
}

/// 生成一份 TPC-H 形状的小数据集（`<dir>/<table>.tbl`）。
///
/// 格式与 dbgen 的 `.tbl` 一致：字段用 `|` 分隔、行尾不带多余的 `|`。
/// 第一列逐行递增 ⇒ 唯一 ⇒ 既满足主键，也让按首列寻址的文件存储层
/// 不会把行折叠掉（见 `tpch_wire_harness::ROWID_COLUMN` 的说明）。
fn write_tbl_fixture(dir: &Path, table: &str, kinds: &str, rows: usize) {
    let mut out = String::new();
    for i in 0..rows {
        let fields: Vec<String> = kinds
            .chars()
            .enumerate()
            .map(|(col, kind)| match kind {
                'I' => (i + 1).to_string(),
                'F' => format!("{}.5", i + 1),
                'T' => format!("{table}-{col}-{i}"),
                other => panic!("unknown column kind {other:?} in `{table}`"),
            })
            .collect();
        out.push_str(&fields.join("|"));
        out.push('\n');
    }
    std::fs::write(dir.join(format!("{table}.tbl")), out)
        .unwrap_or_else(|e| panic!("write {table}.tbl: {e}"));
}

/// 自包含的「从既有 WAL 恢复」路径。
///
/// 原来这个用例的前置条件（一个带 WAL 的数据目录）来自机器本地，本用例自己
/// 造：建库 → 装载 → 停 server → 重开 → 断言。
#[test]
fn quick_query_wal_recovery() {
    let root = unique_root("recovery");
    let _guard = RootGuard(root.clone());
    let data_dir = root.join("data");
    let staging = root.join("staging");

    // ---- Phase 1：建库 + 装载（产生 WAL）----
    {
        let mut client = open(&data_dir, Some(&staging));
        for ddl in common::tpch_wire_harness::SCHEMA_DDL {
            client
                .exec(ddl)
                .unwrap_or_else(|e| panic!("exec DDL `{ddl}`: {e}"));
        }
        for (table, kinds, rows) in TABLE_SHAPE {
            write_tbl_fixture(&staging, table, kinds, rows);
            let path = staging.join(format!("{table}.tbl"));
            let loaded = client
                .load_local_infile(&path, table)
                .unwrap_or_else(|e| panic!("load_local_infile {}: {e}", path.display()));
            // 断言**引擎看得见**的行数，而不是 LOAD DATA 报告的 affected_rows
            // —— 前者才是「数据真的进了存储层」的证据（`tpch_wire_harness`
            // 用同一条判据挡住过存储层折叠行的缺陷）。
            let visible = client
                .query_one_i64(&format!("SELECT COUNT(*) FROM {table}"))
                .unwrap_or_else(|e| panic!("counting the loaded `{table}`: {e}"));
            eprintln!("  loaded {table}: {loaded} rows reported, {visible} visible");
            assert_eq!(
                visible, rows as i64,
                "the engine can only see {visible} of the {rows} rows of `{table}` — \
                 the storage backend is dropping rows (see issue #5392)"
            );
        }
        // client 被 drop ⇒ handle 被 drop ⇒ server 线程 join ⇒ 目录里留下
        // WAL 与行文件。
    }

    // ---- 前置条件：数据目录确实带 WAL（原实现断言的就是这件事）----
    let wal = data_dir.join("sqlrustgo.wal");
    assert!(
        wal.exists(),
        "the data dir must carry a WAL after the writes: {}",
        wal.display()
    );
    assert!(
        wal.metadata().map(|m| m.len() > 0).unwrap_or(false),
        "the WAL file must not be empty: {}",
        wal.display()
    );

    // ---- Phase 2：按恢复路径对同一个目录重开 ----
    let mut client = open(&data_dir, None);

    // 8 张 TPC-H 表都在。
    let rows = client.query_rows("SHOW TABLES").expect("SHOW TABLES");
    let names: Vec<String> = rows.iter().filter_map(|r| r.first().cloned()).collect();
    for table in common::tpch_wire_harness::TABLES {
        assert!(
            names.iter().any(|n| n == table),
            "table `{table}` is missing after recovery; visible tables: {names:?}"
        );
    }

    // 每张表的行数与装载时一致 —— 这就是「从既有 WAL 恢复」的可观测结果。
    for (table, _kinds, expected) in TABLE_SHAPE {
        let count = client
            .query_one_i64(&format!("SELECT COUNT(*) FROM {table}"))
            .unwrap_or_else(|e| panic!("counting `{table}`: {e}"));
        assert_eq!(
            count, expected as i64,
            "table `{table}` has {count} rows after recovery, expected {expected}"
        );
    }
}

// =========================================================================
// 规模层用例（需要外部预置的 TPC-H SF=1 夹具）
// =========================================================================

/// 规模层：对着**外部预置**的 TPC-H SF=1 数据做同样的恢复 + 查询。
///
/// 夹具缺失时按仓库的 NOT-RUN 三态披露如实记为「未运行」
/// （`reason::fixture_missing` / `reason::profile_mismatch`），**不失败**、
/// **也不假装通过**。这是 issue #5428 给的两条路之一（`TPCH_DATA_DIR` +
/// NOT-RUN 披露）。
#[test]
fn quick_query_sfid1() {
    // 夹具解析：`require_sf1_dir` 自己会披露 `fixture_missing` /
    // `profile_mismatch` 并返回 `None`。
    let Some(fixture_dir) = sf1_fixture::require_sf1_dir(TARGET) else {
        return;
    };

    // 档位核对：`sf1_fixture` 判定「SF=1」的依据是「8 个 `.tbl` 都在」，而 CI
    // 生成的夹具（`TPCH_DATA_DIR`，SF=0.01）同样满足这一条 ⇒ 光靠它会把
    // SF=0.01 的数据当成 SF=1 跑。manifest 是档位的**唯一**真相来源：它在
    // 的时候必须写明 1.0，不在的时候（dbgen 原生夹具不带 manifest）才回退
    // 到 `sf1_fixture` 的判定。
    let manifest = common::tpch_cli_harness::tpch_fixture::Manifest::load(&fixture_dir);
    if manifest.sf != 0.0 && manifest.sf != 1.0 {
        sf1_fixture::report_not_run(
            TARGET,
            sf1_fixture::REASON_PROFILE_MISMATCH,
            &format!(
                "the fixture at {} declares scale factor {}, but this test's expectations \
                 are calibrated against TPC-H SF=1",
                fixture_dir.display(),
                manifest.sf
            ),
        );
        return;
    }

    let root = unique_root("sf1");
    let _guard = RootGuard(root.clone());
    let data_dir = root.join("data");

    // ---- Phase 1：建库 + 装载 SF=1 夹具里的三张表 ----
    {
        let mut client = open(&data_dir, Some(&fixture_dir));
        for ddl in common::tpch_wire_harness::SCHEMA_DDL {
            client
                .exec(ddl)
                .unwrap_or_else(|e| panic!("exec DDL `{ddl}`: {e}"));
        }
        for (table, _expected) in SF1_TABLES {
            let path = fixture_dir.join(format!("{table}.tbl"));
            assert!(
                path.is_file(),
                "SF=1 fixture is missing {table}.tbl: {}",
                path.display()
            );
            client
                .load_local_infile(&path, table)
                .unwrap_or_else(|e| panic!("load_local_infile {}: {e}", path.display()));
        }
    }

    // ---- 前置条件：数据目录确实带 WAL ----
    let wal = data_dir.join("sqlrustgo.wal");
    assert!(
        wal.exists(),
        "the data dir must carry a WAL after the loads: {}",
        wal.display()
    );

    // ---- Phase 2：按恢复路径对同一个目录重开 ----
    let mut client = open(&data_dir, None);

    for (table, expected) in SF1_TABLES {
        let count = client
            .query_one_i64(&format!("SELECT COUNT(*) FROM {table}"))
            .unwrap_or_else(|e| panic!("counting `{table}`: {e}"));
        assert_eq!(
            count, expected,
            "table `{table}` has {count} rows after recovery, expected the TPC-H SF=1 \
             count {expected}"
        );
    }
}

//! TPC-H Q1-Q22 合规性测试
//!
//! 使用标准 TPC-H 数据对比 SQLRustGo 与 SQLite 的查询结果
//!
//! 运行方式:
//!   cargo test --test tpch_compliance_test        # SQLRustGo vs SQLite
//!   cargo test --test tpch_compliance_test -- --nocapture  # 显示详细输出
//!
//! **自包含（issue #5427）**：本测试原先依赖仓库里**根本不存在**的两样东西：
//! `data/tpch-tiny/*.tbl`（引擎侧的装载源）与 `data/tpch-tiny/tpch.db`
//! （SQLite 侧的预建数据库）。二者都不是 git 跟踪文件，也没有任何 CI 步骤生成它们
//! ⇒ **任何干净 checkout 上这个目标都必然失败**（CI run 8117 实测：
//! `unable to open database file: data/tpch-tiny/tpch.db`）。
//!
//! 现在两侧都从**运行期生成的 fixture** 取数：
//!   - SQLite 侧用独立预言机 `tests/common/sqlite_oracle.rs`
//!     （它自己解析 `.tbl`，**不经过引擎的装载器**
//!     ⇒ 装载器的缺陷不会同时污染预言机）；
//!   - 引擎侧从同一份 fixture 装载。
//!
//! ⇒ 期望值仍然只来自独立预言机（AC-4），比较逻辑复用标准的
//! `check_against_oracle`（含已知差异登记机制：登记过的差异会被披露而不是 panic）。
//! fixture 缺失时走三态披露的 NOT-RUN（单独计数，绝不计入 passed）。

#[path = "../common/sqlite_oracle.rs"]
mod sqlite_oracle;
#[path = "../common/tpch_fixture.rs"]
mod tpch_fixture;

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use sqlrustgo_storage::{ColumnDefinition, TableInfo};
use std::sync::Arc;

/// The target name used for the NOT-RUN disclosure.
const TARGET: &str = "tpch_compliance_test";

/// Build the independent SQLite oracle from the runtime fixture.
fn create_sqlite_oracle(fixture: &tpch_fixture::Fixture) -> sqlite_oracle::SqliteOracle {
    sqlite_oracle::SqliteOracle::open(fixture, &sqlite_oracle::TPC_H_ALL_TABLES)
        .unwrap_or_else(|e| panic!("build the SQLite oracle from the fixture: {e}"))
}

fn setup_sqlrustgo_engine(fixture: &tpch_fixture::Fixture) -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage.clone());

    {
        let mut storage = storage.write();

        storage
            .create_table(&TableInfo {
                name: "region".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "r_regionkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "r_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "r_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "nation".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "n_nationkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "n_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "n_regionkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "n_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "customer".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "c_custkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_address".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_nationkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_phone".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_acctbal".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_mktsegment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "c_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "supplier".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "s_suppkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "s_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "s_address".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "s_nationkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "s_phone".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "s_acctbal".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "s_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "part".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "p_partkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_name".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_mfgr".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_brand".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_type".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_size".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_container".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_retailprice".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "p_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "partsupp".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "ps_partkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "ps_suppkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "ps_availqty".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "ps_supplycost".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "ps_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "orders".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "o_orderkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_custkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_orderstatus".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_totalprice".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_orderdate".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_orderpriority".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_clerk".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_shippriority".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "o_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();
        storage
            .create_table(&TableInfo {
                name: "lineitem".to_string(),
                columns: vec![
                    ColumnDefinition {
                        name: "l_orderkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_partkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_suppkey".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_linenumber".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_quantity".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_extendedprice".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_discount".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_tax".to_string(),
                        data_type: "REAL".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_returnflag".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_linestatus".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_shipdate".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_commitdate".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_receiptdate".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_shipinstruct".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_shipmode".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                    ColumnDefinition {
                        name: "l_comment".to_string(),
                        data_type: "TEXT".to_string(),
                        nullable: false,
                        primary_key: false,
                        char_max_length: None,

                        collation: None,

                        default_value: None,
                        auto_increment: false,
                    },
                ],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                compression: None,
                collations: std::collections::HashMap::new(),
                partition_info: None,
                original_sql: String::new(),
                clustered: false,
            })
            .unwrap();

        let tables = [
            "region", "nation", "customer", "supplier", "part", "partsupp", "orders", "lineitem",
        ];

        for table_name in tables {
            let filepath = fixture.table_path(table_name);
            // A failed load must NOT be swallowed: an empty table would make
            // every assertion below run against **no data at all** and pass
            // vacuously. That is exactly the fake-green channel this test
            // used to have.
            let count = storage
                .bulk_load_tbl_file(table_name, &filepath.to_string_lossy())
                .unwrap_or_else(|e| {
                    panic!("load {} from {}: {e:?}", table_name, filepath.display())
                });
            println!("  Loaded {} rows into {}", count, table_name);
        }
    }

    engine
}

#[test]
fn test_tpch_q1_simple() {
    // The fixture is resolved at run time; when it is missing the shared
    // resolver reports the NOT-RUN (with its reason) and we simply stop.
    let state = tpch_fixture::FixtureState::resolve();
    let Some(fixture) = state.require(TARGET) else {
        return;
    };

    let oracle = create_sqlite_oracle(fixture);
    let mut engine = setup_sqlrustgo_engine(fixture);

    let sql = "SELECT l_returnflag, SUM(l_quantity) FROM lineitem WHERE l_shipdate <= '1995-12-01' GROUP BY l_returnflag";

    println!("Running Q1...");

    let oracle_rows = oracle
        .query(sql)
        .unwrap_or_else(|e| panic!("Q1 on the independent SQLite oracle: {e}"));
    println!("SQLite returned {} rows", oracle_rows.len());

    let result = engine
        .execute(sql)
        .unwrap_or_else(|e| panic!("Q1 on the engine: {e:?}"));
    println!("SQLRustGo returned {} rows", result.rows.len());
    for row in result.rows.iter().take(5) {
        println!("  {:?}", row);
    }

    // The expectation comes from the **independent** oracle, never from the
    // engine's own output (AC-4). `check_against_oracle` sorts both sides
    // (GROUP BY does not define an order) and consults the known-difference
    // registry before failing.
    let engine_rows = sqlite_oracle::engine_rows_to_strings(&result.rows);
    sqlite_oracle::check_against_oracle("tpch:q1", &engine_rows, &oracle_rows);
}

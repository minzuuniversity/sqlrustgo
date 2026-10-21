//! Regression test for V312-58 / Issue #4375: TPC-H Q2 5-way comma-join
//! with `LIMIT 20` (queries/q2.sql canonical form) returns 15,628 rows
//! instead of 20 because the comma-join fast path's hash-chain
//! optimisation fails to apply either base-table predicate pushdown
//! (`p_size = 15`, `p_type LIKE '%BRASS'`, `r_name = 'EUROPE'`) or the
//! post-join WHERE filter, leaving a fully-joined cross-product before
//! LIMIT trims it.
//!
//! SF=0.1 does NOT reproduce this bug — at SF=0.1 only 3 of the 73
//! qualifying parts have suppliers in EUROPEAN nations, so the result is
//! only 3 rows even with a fully un-pruned cross-product (LIMIT 20 caps
//! at 3). At SF=1 the qualified join yields 642 rows and LIMIT 20 caps
//! at 20. If pushdown is dropped, the cross-product grows to 15,628
//! rows (suppliers_in_EUROPE × partsupp) and LIMIT 20 still caps at 20
//! — so the symptom there is wrong contents, not wrong cardinality.
//!
//! Expected baseline (SQLite oracle, sf=1, runs ~12s):
//!   747 qualifying parts → 642 qualified join rows → top 20 by
//!   ORDER BY s_acctbal ASC, n_name, s_name, p_partkey → LIMIT 20 = 20
//!   rows.
//!
//! Run:
//!   cargo test --test q2_5way_comma_limit_regression --all-features \
//!     -- --ignored --nocapture q2_canonical_5way_comma_limit

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

// #5406 / #5428：SF=1 夹具目录此前硬编码成某台开发机家目录下的绝对路径。
// 那条路径只在一台机器上存在 ⇒ 别的机器上要么 panic（`unwrap`），
// 要么被「找不到就跳过」吞掉（假绿）。现在走共享解析器：
// `TPCH_SF1_DIR` → `TPCH_DATA_DIR` → 缺失（披露 NOT-RUN，绝不回退硬编码）。
#[path = "../../common/sf1_fixture.rs"]
mod sf1_fixture;

/// Cargo 测试目标名（用作 NOT-RUN 标签）。
const TARGET: &str = "q2_5way_comma_limit_regression";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
];

const TBL_FILES: &[&str] = &["region", "nation", "supplier", "part", "partsupp"];

/// 装载 SF=1 夹具；夹具缺失时**披露 NOT-RUN** 并返回 `None`。
fn setup() -> Option<ExecutionEngine<MemoryStorage>> {
    let data_dir = sf1_fixture::require_sf1_dir(TARGET)?;
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for s in SCHEMAS {
        engine.execute(s).unwrap();
    }
    {
        let mut st = storage.write();
        for t in TBL_FILES {
            let path = data_dir.join(format!("{t}.tbl"));
            // 装载失败必须**显式失败**：一个空的 partsupp 表会让下面
            // 「返回 20 行」的断言变成空洞的通过。
            st.bulk_load_tbl_file(t, path.to_str().expect("utf-8 fixture path"))
                .unwrap_or_else(|e| panic!("loading {t} from {} failed: {e}", path.display()));
        }
    }
    Some(engine)
}

const Q2_SQL: &str =
    "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment \
                       FROM part, supplier, partsupp, nation, region \
                       WHERE p_partkey = ps_partkey \
                         AND s_suppkey = ps_suppkey \
                         AND p_size = 15 \
                         AND p_type LIKE '%BRASS' \
                         AND s_nationkey = n_nationkey \
                         AND n_regionkey = r_regionkey \
                         AND r_name = 'EUROPE' \
                       ORDER BY s_acctbal ASC, n_name, s_name, p_partkey \
                       LIMIT 20";

#[test]
#[ignore] // Heavy: 200K part + 800K partsupp + 10K supplier at SF=1
fn q2_canonical_5way_comma_limit() {
    // 夹具缺失 ⇒ 已在 require_sf1_dir 里披露 NOT-RUN，直接返回。
    // 绝不 panic，也绝不停在「什么都没跑却算通过」。
    let Some(mut engine) = setup() else {
        return;
    };
    let r = engine
        .execute(Q2_SQL)
        .unwrap_or_else(|e| panic!("Q2 failed: {}", e));
    eprintln!("Q2 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().take(5).enumerate() {
        eprintln!("  row[{}] = {:?}", i, row);
    }
    // Build TSV dump of the engine output for sha256 comparison vs the
    // authoritative SQLite oracle (`queries/expected/q2_sf1_5way_comma_limit.tsv`,
    // sha256 = 0849d0252928b2452e62a5ed341cd66e614ce0c42b4ad5d45a7845be8798e458).
    // Both sides use the same column separator (|) and float format ({:.2} /
    // printf '%.2f'), so the two files are bit-exact for a correct engine.
    let mut out = String::new();
    for row in &r.rows {
        let parts: Vec<String> = row
            .iter()
            .map(|v| match v {
                sqlrustgo::Value::Float(f) => format!("{:.2}", f),
                sqlrustgo::Value::Integer(i) => i.to_string(),
                sqlrustgo::Value::Text(s) => s.clone(),
                sqlrustgo::Value::Null => "NULL".to_string(),
                sqlrustgo::Value::Boolean(b) => b.to_string(),
                _ => format!("{:?}", v),
            })
            .collect();
        out.push_str(&parts.join("|"));
        out.push('\n');
    }
    std::fs::write("/tmp/q2_engine_dump.tsv", &out).unwrap();
    eprintln!("dumped to /tmp/q2_engine_dump.tsv");

    assert_eq!(
        r.rows.len(),
        20,
        "Q2 LIMIT 20 must cap the result at 20 rows (got {})",
        r.rows.len()
    );

    // Bit-exact content assertion: the engine output must match the
    // authoritative SQLite oracle byte-for-byte. This is the real regression
    // gate for #4375 — it verifies that (a) the LIMIT cap applies, (b) the
    // sort key s_acctbal ASC is interpreted numerically (not lexically), and
    // (c) the comma-join pushdown doesn't drop or duplicate rows.
    let oracle_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("queries/expected/q2_sf1_5way_comma_limit.tsv");
    let oracle = std::fs::read(&oracle_path)
        .unwrap_or_else(|e| panic!("missing oracle fixture {}: {}", oracle_path.display(), e));
    assert_eq!(
        out.as_bytes(),
        oracle.as_slice(),
        "Q2 engine output does not match SQLite oracle (sha256 mismatch). \
         See /tmp/q2_engine_dump.tsv for actual output."
    );
}

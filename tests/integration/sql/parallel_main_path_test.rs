//! Parallel main path E2E tests
//!
//! v3.10.0 Issue #3767 (V310-13 / I-12): Verify that
//! `ExecutionEngine::execute_select` actually engages the parallel filter
//! path under the documented preconditions.
//!
//! Tests verify:
//! - CLI flag / env var propagation to `parallel_degree`
//! - The parallel path is **actually engaged** (not merely "the answer is right")
//! - Cell-level equivalence N=1 vs N=4
//! - FOR UPDATE / correlated subquery fall back to sequential
//! - TPC-H Q1-style aggregates match across parallelism levels
//!
//! #5413：这些测试此前**判别力为零**
//!
//! 原来的前提是：
//!
//! ```text
//! const ROWS: usize = 600_000; // > PARALLEL_MIN_ROWS (500K)
//! ```
//!
//! 两处都不成立：
//!
//! 1. **阈值过时**：`PARALLEL_MIN_ROWS` 在 v3.10.0 Issue #3792 已从 100K 提到
//!    **2_000_000**（"raised from 100K to 2M after benchmark showed no speedup
//!    at SF=0.1/1.0/10.0"），注释里的 "500K" 停留在旧值。600 000 < 2 000 000。
//! 2. **更致命**：CBO 默认开启（`cbo_enabled`，见 `engine_builder.rs:35`），
//!    而 `should_parallelize_query` 在 CBO 分支里**根本不看扫过的行数** —��
//!    它走 `UnifiedCostModel::should_parallelize`，用的是 cost model 的
//!    **统计信息**（表没有统计时默认 1000 行）和谓词的选择性估算。
//!    ⇒ 光把行数提到 2M 也不够。
//!
//! 结果：`test_execute_select_parallel_degree_4_path_engaged` 从来没有走进过
//! 并行路径，而它断言的 `rows >= 50_000` 在串行下**完全相同** ⇒ 它既不能
//! 证明并行路径被走到，也发现不了并行被破坏。
//!
//! 修法（两处，缺一不可）：
//!
//! * **可注入的阈值**：`ExecutionEngine::set_parallel_min_rows_override` ——
//!   它在 CBO 分支**之前**短路整个判定，所以默认配置下也真正生效；
//! * **可观测的「真的走了」信号**：`ExecutionEngine::parallel_filter_runs()`
//!   —— 并行与串行的结果行完全相同，光看输出无法区分。
//!
//! 于是下面每个断言都同时钉住两件事：**结果正确** + **路径确实是那条**。

use sqlrustgo::{ExecutionEngine, MemoryStorage, PARALLEL_MIN_ROWS};
use sqlrustgo_types::Value;
use std::sync::Arc;

/// 装载的行数。
///
/// 比原来的 600 000 小：现在并行路径由**注入的阈值**触发，不再需要靠行数
/// 硬撑到生产阈值 2M 行。`bulk_insert_records` 一次调用装完（#5379）。
const ROWS: usize = 120_000;

/// 注入的并行阈值 —— 远小于生产阈值，让测试能真正走进并行路径。
const TEST_PARALLEL_MIN_ROWS: usize = 1_000;

/// 谓词 `k < 100` 的过滤后行数：100 个不同的 k × 每 k 1200 行 = 120_000 行中的
/// 12_000 行。
const EXPECTED_FILTERED_ROWS: usize = ROWS / 10;

fn make_engine(parallel: usize) -> ExecutionEngine<MemoryStorage> {
    let mut engine = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    engine.set_parallel_degree(parallel);
    // #5413: 注入阈值 —— 没有它，下面所有「并行」断言都在串行路径上跑。
    engine.set_parallel_min_rows_override(TEST_PARALLEL_MIN_ROWS);
    engine
}

fn populate(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE t (k INTEGER, v INTEGER)")
        .unwrap();
    // The rows and the row count are unchanged — only the LOADING PATH is.
    //
    // Why (issue #5379): `populate()` is called several times across this file
    // and each call used to issue ROWS single-row `INSERT` statements through
    // `engine.execute` — millions of full parse → plan → execute cycles on
    // the CI runner. `bulk_insert_records` inserts the same rows in ONE
    // storage call (O(N)), using the engine's public API.
    //
    // The table has no PRIMARY KEY, so this is not the O(N^2) duplicate-key
    // path (see #5412) — it is simply many statements that do not need to be
    // statements.
    let records: Vec<sqlrustgo_storage::Record> = (0..ROWS)
        .map(|i| vec![Value::Integer(i as i64 % 1000), Value::Integer(i as i64)])
        .collect();
    let inserted = engine.bulk_insert_records("t", records).unwrap();
    assert_eq!(inserted as usize, ROWS, "all rows must be inserted");
}

#[test]
fn test_execute_select_parallel_degree_4_path_engaged() {
    let mut engine = make_engine(4);
    populate(&mut engine);
    assert_eq!(
        engine.parallel_filter_runs(),
        0,
        "parallel filter must not have run before the query"
    );

    let result = engine.execute("SELECT k, v FROM t WHERE k < 100").unwrap();

    // (1) 结果正确
    let rows = result.rows.len();
    assert_eq!(
        rows, EXPECTED_FILTERED_ROWS,
        "expected {EXPECTED_FILTERED_ROWS} filtered rows, got {rows}"
    );
    // (2) **路径确实是并行那条** —— 这是 #5413 之前缺失的判别力。
    assert_eq!(
        engine.parallel_filter_runs(),
        1,
        "the parallel filter path must have been engaged exactly once"
    );
}

#[test]
fn test_execute_select_parallel_degree_1_path_sequential() {
    let mut engine = make_engine(1);
    populate(&mut engine);

    let result = engine.execute("SELECT k, v FROM t WHERE k < 100").unwrap();
    let rows = result.rows.len();
    assert_eq!(
        rows, EXPECTED_FILTERED_ROWS,
        "expected {EXPECTED_FILTERED_ROWS} filtered rows, got {rows}"
    );
    // degree=1 ⇒ 并行路径不得被走到。
    assert_eq!(
        engine.parallel_filter_runs(),
        0,
        "degree=1 must not engage the parallel filter path"
    );
}

#[test]
fn test_execute_select_results_match_parallel_1_vs_4() {
    // Sequential (N=1)
    let mut seq = make_engine(1);
    populate(&mut seq);
    let seq_r = seq.execute("SELECT k, v FROM t WHERE k < 100").unwrap();

    // Parallel (N=4)
    let mut par = make_engine(4);
    populate(&mut par);
    let par_r = par.execute("SELECT k, v FROM t WHERE k < 100").unwrap();

    // Row count and multiset should be identical
    assert_eq!(seq_r.rows.len(), par_r.rows.len());
    let mut seq_sorted = seq_r.rows.clone();
    let mut par_sorted = par_r.rows.clone();
    seq_sorted.sort();
    par_sorted.sort();
    assert_eq!(seq_sorted, par_sorted);

    // #5413: 若两边都没走并行路径，这个「一致性」断言是空的 ——
    // 它只能证明「串行 == 串行」。必须钉住：一真一假。
    assert_eq!(
        seq.parallel_filter_runs(),
        0,
        "N=1 engine must not have engaged the parallel path"
    );
    assert_eq!(
        par.parallel_filter_runs(),
        1,
        "N=4 engine must have engaged the parallel path"
    );
}

#[test]
fn test_for_update_disables_parallel_path() {
    let mut par = make_engine(4);
    populate(&mut par);

    // FOR UPDATE must disable parallel path (safety gate)
    let result = par
        .execute("SELECT k, v FROM t WHERE k < 100 FOR UPDATE")
        .unwrap();
    let rows = result.rows.len();
    assert_eq!(
        rows, EXPECTED_FILTERED_ROWS,
        "FOR UPDATE filter should still work"
    );
    // #5413: 这个测试的名字声称「FOR UPDATE 退回串行」—— 前提成立时，
    // 退回与否必须**看得见**。
    assert_eq!(
        par.parallel_filter_runs(),
        0,
        "FOR UPDATE must disable the parallel filter path"
    );
}

#[test]
fn test_cli_flag_propagation_via_env() {
    let engine = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    let initial = engine.parallel_degree();
    std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "8");
    let engine2 = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    assert_eq!(
        engine2.parallel_degree(),
        8,
        "ctor after env-set should read 8"
    );
    std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
    // restore initial state
    std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", initial.to_string());
}

#[test]
fn test_set_parallel_degree_override() {
    let mut engine = make_engine(4);
    engine.set_parallel_degree(0);
    assert_eq!(engine.parallel_degree(), 1);
}

#[test]
fn test_tpch_q1_style_aggregate_correctness() {
    // TPC-H Q1-like: SELECT SUM(v) over a filtered scan.
    // Verify aggregate correctness across parallel degrees — and that the
    // parallel degrees really did take the parallel path.
    let mut check_results = vec![];
    for parallel in [1, 2, 4] {
        let mut engine = make_engine(parallel);
        populate(&mut engine);
        let result = engine
            .execute("SELECT SUM(v) FROM t WHERE k < 100")
            .unwrap();
        let sum = if let Value::Integer(n) = &result.rows[0][0] {
            *n
        } else {
            panic!("Expected Integer sum");
        };
        // #5413: 并行度数 > 1  ⇒  必须真的走了并行路径。
        let expected_runs = if parallel > 1 { 1 } else { 0 };
        assert_eq!(
            engine.parallel_filter_runs(),
            expected_runs,
            "parallel_degree={parallel} should engage the parallel path {expected_runs} time(s)"
        );
        check_results.push((parallel, sum));
    }

    // All SUMs should be identical
    let sum_1 = check_results[0].1;
    for (p, s) in &check_results {
        assert_eq!(
            *s, sum_1,
            "SUM with parallel_degree={p} should equal N=1 ({sum_1})"
        );
    }
}

/// #5413 的前提守卫。
///
/// 这个文件里的并行断言**全部**依赖注入的阈值。如果哪天有人把生产阈值降到
/// `ROWS` 以下，`TEST_PARALLEL_MIN_ROWS` 就不再是「真正起作用的那一个」，
/// 这些测试会安静地退化成「串行 == 串行」。这条不变量让那种退化立刻变红。
#[test]
fn test_injected_threshold_is_what_engages_the_parallel_path() {
    let mut engine = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    // 默认（无注入）⇒ 生效阈值就是生产常量。
    assert_eq!(engine.parallel_min_rows(), PARALLEL_MIN_ROWS);

    // The production threshold is **pinned** here on purpose: if the
    // production constant moves, this test must go red — the data loaded
    // below is sized against it.
    //
    // These two assertions compare constants, so clippy's
    // `assertions_on_constants` fires on them. The lint is allowed here
    // with a reason: the assertion *is* the check. Moving it to a
    // compile-time `const { assert!(..) }` was tried and rejected — a
    // compile-time panic may not carry a formatted message (E0015), and a
    // moved production constant should turn this **test** red, not stop
    // the target from compiling at all.
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(
            PARALLEL_MIN_ROWS == 2_000_000,
            "the production parallel threshold moved: expected 2_000_000, got \
             {PARALLEL_MIN_ROWS}"
        );

        // 本文件装载的行数**不足以**单靠行数触发并行路径 ——
        // 所以注入的阈值确实是那个起作用的开关。
        assert!(
            ROWS < PARALLEL_MIN_ROWS,
            "ROWS ({ROWS}) must stay below PARALLEL_MIN_ROWS ({PARALLEL_MIN_ROWS}), \
             otherwise the injected threshold would not be what engages the parallel path"
        );
        assert!(TEST_PARALLEL_MIN_ROWS < ROWS);
    }

    // 注入后生效阈值变成注入值。
    engine.set_parallel_min_rows_override(TEST_PARALLEL_MIN_ROWS);
    assert_eq!(engine.parallel_min_rows(), TEST_PARALLEL_MIN_ROWS);
    // 归零 ⇒ 回到生产阈值。
    engine.set_parallel_min_rows_override(0);
    assert_eq!(engine.parallel_min_rows(), PARALLEL_MIN_ROWS);
}

/// 没有注入阈值时，行数不足 ⇒ 并行路径**不得**被走到。
///
/// 这是上面那条「注入才生效」的对照实验：同一个查询、同样的行数，
/// 唯一的差别是有没有注入阈值。
#[test]
fn test_without_injected_threshold_parallel_path_is_not_engaged() {
    let mut engine = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    engine.set_parallel_degree(4);
    populate(&mut engine);

    let result = engine.execute("SELECT k, v FROM t WHERE k < 100").unwrap();
    assert_eq!(result.rows.len(), EXPECTED_FILTERED_ROWS);
    assert_eq!(
        engine.parallel_filter_runs(),
        0,
        "without the injected threshold the parallel path must not be engaged \
         (ROWS={ROWS} < PARALLEL_MIN_ROWS={PARALLEL_MIN_ROWS})"
    );
}

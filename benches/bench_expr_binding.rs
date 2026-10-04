//! F-08 基准：表达式求值的**列名绑定**成本
//!
//! 背景（`docs/releases/v4.1.0/PERFORMANCE_AUDIT_REPORT_2026-09-30.md` §F-08）：
//! 求值器没有独立的绑定/编译阶段，`Expression::Identifier` **每次求值**
//! 都按字符串重新解析列位置，走 `crates/executor/src/expr/mod.rs::find_column_index`。
//! 该函数最坏要做 4 次线性扫描，且多 join 分支为每列 `split('.').collect()`
//! 分配一个 `Vec<&str>`。
//!
//! 优化计划 §B1.3 把「完整绑定阶段」列为高风险改动，并明确要求
//! **先补一个覆盖 `WHERE <col> = <lit>` 多列扫描的基准**，否则无法证明收益、
//! 也无法排除回归。本文件就是那个基准。
//!
//! 三组测量：
//! 1. `expr_find_column_index` / `expr_find_column_index_multijoin`
//!    —— 直接压 `find_column_index`，隔离「列数 × 命中位置 × 列名形态」。
//! 2. `expr_eval_identifier` —— 热路径入口（每行每标识符调用一次）。
//! 3. `expr_where_eq_multicolumn` —— 端到端 `WHERE <col> = <lit>`，
//!    表宽 4/16/64 列，过滤列取第一列（线性扫描最优）与最后一列（最坏）。
//!
//! 运行：
//! ```bash
//! cargo bench --bench bench_expr_binding
//! # 只看某一组：
//! cargo bench --bench bench_expr_binding -- expr_where_eq
//! ```

#![allow(dead_code)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlrustgo::ExecutionEngine;
use sqlrustgo::MemoryStorage;
use sqlrustgo_executor::expr::{eval_identifier, find_column_index};
use sqlrustgo_storage::ColumnDefinition;
use sqlrustgo_types::Value;
use std::hint::black_box;

/// 列数与命中位置：`ncols` 决定线性扫描的长度，命中位置决定扫多远。
const COL_COUNTS: [usize; 3] = [4, 16, 64];

fn col(name: &str) -> ColumnDefinition {
    ColumnDefinition {
        name: name.to_string(),
        data_type: "INTEGER".to_string(),
        ..Default::default()
    }
}

/// 直接压 `find_column_index`：命中首/中/尾列，以及完全未命中（最坏，全程扫描）。
fn bench_find_column_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("expr_find_column_index");

    for ncols in COL_COUNTS {
        let cols: Vec<ColumnDefinition> = (0..ncols).map(|i| col(&format!("c{i}"))).collect();

        let probes: [(&str, String); 4] = [
            ("first", "c0".to_string()),
            ("middle", format!("c{}", ncols / 2)),
            ("last", format!("c{}", ncols - 1)),
            ("miss", "no_such_column".to_string()),
        ];

        for (label, name) in probes {
            group.bench_with_input(BenchmarkId::new(label, ncols), &name, |b, name| {
                b.iter(|| black_box(find_column_index(black_box(name), &cols)));
            });
        }
    }

    group.finish();
}

/// 多 join 形态：累积列名是 `a_join_b.tN.cM`，用户写 `tN.cM`。
/// 这条路径会走 `split('.').collect()`，是 F-08 点名的 `Vec<&str>` 分配。
fn bench_find_column_index_multijoin(c: &mut Criterion) {
    let mut group = c.benchmark_group("expr_find_column_index_multijoin");

    for ncols in COL_COUNTS {
        // 每列都是三段式累积名；查找 `t2.c3` 命中最后一列 -> 最坏扫描 + 最坏分配。
        let cols: Vec<ColumnDefinition> = (0..ncols)
            .map(|i| col(&format!("a_join_b.t{}.c{}", i % 4, i % 4)))
            .collect();
        let hit_last = format!("t{}.c{}", (ncols - 1) % 4, (ncols - 1) % 4);
        let miss = "t9.c9".to_string();

        group.bench_with_input(BenchmarkId::new("hit_last", ncols), &hit_last, |b, n| {
            b.iter(|| black_box(find_column_index(black_box(n), &cols)));
        });
        group.bench_with_input(BenchmarkId::new("miss", ncols), &miss, |b, n| {
            b.iter(|| black_box(find_column_index(black_box(n), &cols)));
        });
    }

    group.finish();
}

/// 热路径入口：`eval_identifier` 每行每标识符调用一次。
fn bench_eval_identifier(c: &mut Criterion) {
    let mut group = c.benchmark_group("expr_eval_identifier");

    for ncols in COL_COUNTS {
        let cols: Vec<ColumnDefinition> = (0..ncols).map(|i| col(&format!("c{i}"))).collect();
        let row: Vec<Value> = (0..ncols).map(|i| Value::Integer(i as i64)).collect();

        let probes: [(&str, String); 3] = [
            ("first", "c0".to_string()),
            ("last", format!("c{}", ncols - 1)),
            ("miss", "no_such_column".to_string()),
        ];

        for (label, name) in probes {
            group.bench_with_input(BenchmarkId::new(label, ncols), &name, |b, name| {
                b.iter(|| black_box(eval_identifier(black_box(name), &row, &cols)));
            });
        }
    }

    group.finish();
}

/// 建一张 `ncols` 列的窄表并灌 `rows` 行。列值 = `r * ncols + c`，全局唯一。
fn setup_wide_engine(ncols: usize, rows: usize) -> ExecutionEngine<MemoryStorage> {
    let mut engine = ExecutionEngine::with_memory();

    let ddl_cols: Vec<String> = (0..ncols).map(|i| format!("c{i} INTEGER")).collect();
    engine
        .execute(&format!("CREATE TABLE wide ({})", ddl_cols.join(", ")))
        .unwrap();

    for r in 0..rows {
        let vals: Vec<String> = (0..ncols).map(|i| (r * ncols + i).to_string()).collect();
        engine
            .execute(&format!("INSERT INTO wide VALUES ({})", vals.join(", ")))
            .unwrap();
    }

    engine
}

/// 端到端 `WHERE <col> = <lit>`：这就是优化计划 §B1.3 要求的那个基准。
///
/// 过滤列取第一列（线性扫描立即命中）与最后一列（扫满整行）——
/// 两者的差值就是「每行按字符串重解析列位置」的成本。
fn bench_where_eq_multicolumn(c: &mut Criterion) {
    let mut group = c.benchmark_group("expr_where_eq_multicolumn");
    // 端到端建表 + 灌数据较慢，缩小样本量以控制总时长。
    group.sample_size(20);

    for ncols in [4usize, 16, 64] {
        for rows in [1_000usize, 4_000] {
            let mut engine = setup_wide_engine(ncols, rows);

            for (label, target) in [("first_col", 0usize), ("last_col", ncols - 1)] {
                let sql = format!("SELECT COUNT(*) FROM wide WHERE c{} = 1", target);
                group.bench_with_input(
                    BenchmarkId::new(format!("{label}_c{ncols}_r{rows}"), ncols),
                    &sql,
                    |b, sql| {
                        b.iter(|| black_box(engine.execute(black_box(sql)).unwrap()));
                    },
                );
            }
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_find_column_index,
    bench_find_column_index_multijoin,
    bench_eval_identifier,
    bench_where_eq_multicolumn
);
criterion_main!(benches);

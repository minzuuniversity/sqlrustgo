//! GMP Retrieval v3 Benchmark
//!
//! Benchmarks for:
//! - 6.1 P99 latency target (< 200ms)
//! - 6.2 Recall measurement on GMP test corpus
//! - 6.3 Stress test for 100+ concurrent queries

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sqlrustgo_gmp_retrieval::{Bm25Search, RetrievalEngine, RetrievalSource, RrfFusion};
use std::time::Instant;

/// GMP test corpus - sample documents for recall measurement
fn gmp_test_corpus() -> Vec<String> {
    vec![
        "GMP偏差调查报告 偏差编号 DEV-2024-001".to_string(),
        "偏差描述 生产过程中温度超出规定范围".to_string(),
        "偏差原因 设备制冷系统故障导致".to_string(),
        "偏差处理 停止生产并进行偏差调查".to_string(),
        "纠正措施 维修设备并加强监控".to_string(),
        "预防措施 定期维护设备建立SOP".to_string(),
        "SOP生产操作规程 洁净区环境要求".to_string(),
        "SOP设备清洁 清洁验证要求".to_string(),
        "变更控制 设备变更申请流程".to_string(),
        "质量审核 批记录审核要点".to_string(),
        "偏差报告A 洁净室压差异常".to_string(),
        "偏差报告B 人员更衣流程偏差".to_string(),
        "偏差报告C 物料称量误差超标".to_string(),
        "偏差报告D 环境监测数据异常".to_string(),
        "偏差报告E 设备校验超期".to_string(),
    ]
}

/// BM25 benchmark - baseline retrieval performance
fn bench_bm25_search(c: &mut Criterion) {
    let corpus = gmp_test_corpus();
    let bm25 = Bm25Search::new(corpus);

    let queries = vec!["偏差报告", "GMP生产", "设备故障", "清洁验证", "SOP操作"];

    let mut group = c.benchmark_group("bm25_search");
    for query in queries {
        group.bench_with_input(BenchmarkId::from_parameter(query), &query, |b, q| {
            b.iter(|| {
                let _ = bm25.search(black_box(q), 10);
            });
        });
    }
    group.finish();
}

/// RRF fusion benchmark
fn bench_rrf_fusion(c: &mut Criterion) {
    let corpus = gmp_test_corpus();
    let bm25 = Bm25Search::new(corpus);

    // Get results from two different queries
    let results1 = bm25.search("偏差报告", 50);
    let results2 = bm25.search("GMP", 50);

    let mut group = c.benchmark_group("rrf_fusion");
    group.bench_function("fuse_two_sources", |b| {
        b.iter(|| {
            let mut fusion = RrfFusion::new(60);
            // Add results from first source
            for r in &results1 {
                fusion.add_result(r.doc_id, r.score);
            }
            // Add results from second source
            for r in &results2 {
                fusion.add_result(r.doc_id, r.score);
            }
            let _ = fusion.rank();
        });
    });
}

/// Latency benchmark - measure P99 target < 200ms
fn bench_latency_p99(c: &mut Criterion) {
    let corpus = gmp_test_corpus();
    let bm25 = Bm25Search::new(corpus);

    let mut latencies: Vec<u64> = Vec::with_capacity(1000);

    // Run 1000 iterations to get P99
    for _ in 0..1000 {
        let start = Instant::now();
        let _ = bm25.search("偏差报告", 10);
        let elapsed = start.elapsed().as_millis() as u64;
        latencies.push(elapsed);
    }

    // Calculate P99
    latencies.sort();
    let p99_index = ((latencies.len() as f64 * 0.99) as usize).min(latencies.len() - 1);
    let p99 = latencies.get(p99_index).unwrap_or(&200);

    let mut group = c.benchmark_group("latency_p99");
    group.bench_function("bm25_search", |b| {
        b.iter(|| {
            let start = Instant::now();
            let _ = bm25.search("偏差报告", 10);
            let _ = start.elapsed().as_millis();
        });
    });

    println!("P99 latency: {}ms (target: <200ms)", p99);
}

/// Concurrent query stress test - simulated 100 queries
/// Note: Using synchronous benchmark as criterion doesn't support native async
fn bench_concurrent_queries(c: &mut Criterion) {
    let corpus = gmp_test_corpus();
    let bm25 = Bm25Search::new(corpus);

    let mut group = c.benchmark_group("concurrent_stress");
    group.bench_function("100_sequential_queries", |b| {
        b.iter(|| {
            for i in 0..100 {
                let query = format!("偏差报告 {}", i % 5);
                let _ = bm25.search(&query, 10);
            }
        });
    });
}

/// Recall benchmark - measure on GMP test corpus
fn bench_recall(c: &mut Criterion) {
    let corpus = gmp_test_corpus();
    let bm25 = Bm25Search::new(corpus);

    // Ground truth: relevant doc IDs for "偏差报告"
    // Doc 0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 14 should match
    let relevant_docs: std::collections::HashSet<usize> = [0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 14]
        .iter()
        .cloned()
        .collect();

    let mut group = c.benchmark_group("recall_measurement");
    group.bench_function("bm25_on_gmp_corpus", |b| {
        b.iter(|| {
            let results = bm25.search(black_box("偏差报告"), 10);
            // doc_id is usize directly, not String
            let retrieved: std::collections::HashSet<usize> =
                results.iter().map(|r| r.doc_id).collect();

            // Calculate recall
            let true_positives = retrieved.intersection(&relevant_docs).count();
            let recall = true_positives as f64 / relevant_docs.len() as f64;
            println!("Recall: {:.2}%", recall * 100.0);
        });
    });
}

criterion_group!(
    benches,
    bench_bm25_search,
    bench_rrf_fusion,
    bench_latency_p99,
    bench_concurrent_queries,
    bench_recall
);
criterion_main!(benches);

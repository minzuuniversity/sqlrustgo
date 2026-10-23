//! #5384 —— `cosine_similarity_simd` 用「自己到自己的距离」当范长 ⇒ 相似度**恒为 0**
//!
//! ## 缺陷
//!
//! `crates/vector/src/simd_explicit.rs` 的 `cosine_similarity_simd` 曾经这样算范长：
//!
//! ```text
//! let norm_a = euclidean_distance_simd(a, a).sqrt();   // 恒为 0
//! let norm_b = euclidean_distance_simd(b, b).sqrt();   // 恒为 0
//! ```
//!
//! `euclidean_distance_simd(a, a)` 是**向量到自身的距离**，恒为 0 ⇒ 两个范长恒为 0
//! ⇒ 函数**恒返回 0.0**，永远走不到 `dot / (norm_a * norm_b)` 那一支。
//!
//! 连带后果：cosine 度量下的相关性排序**完全失效** ——
//! `FlatIndex::search`（`flat.rs`）、`HnswIndex`（`hnsw.rs`）、
//! `compute_similarity_simd` 与 `batch_cosine_distance_simd` 全部拿到同一个分数。
//!
//! ## 这个文件的三个 guard
//!
//! 1. **三分量**（issue 建议）：相同 / 正交 / 相反必须给出**互不相同**的分数 ——
//!    缺陷让它们全都是 0.0；
//! 2. **实现等价**：标量实现与 SIMD 实现必须给出一致的分数 —— 两条路径共用
//!    `metrics::l2_norm`，不许再漂移；
//! 3. **端到端排序**：`FlatIndex` 与 `HnswIndex` 在 cosine 下必须把**最相似**的
//!    排在最前 —— 这正是坏代码下失效的那件事。
//!
//! 判据里用的是「相同 / 正交 / 相反」这种**与数据无关**的不变量，而不是某个
//! 数据集上的具体数字：断言必须说明「这个性质」，而不是「这个值」。

use sqlrustgo_vector::metrics::{cosine_similarity, l2_norm};
use sqlrustgo_vector::parallel_knn::simd as parallel_simd;
use sqlrustgo_vector::simd_explicit::{
    batch_cosine_similarity_simd, cosine_distance_simd, cosine_similarity_simd,
};
use sqlrustgo_vector::{DistanceMetric, FlatIndex, HnswIndex, VectorIndex};

/// The tolerance for "the same number".
///
/// The SIMD path accumulates the dot product in a different order than the
/// scalar path, so the two agree to within rounding, not bit-for-bit.
const EPS: f32 = 1e-5;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < EPS
}

/// A few vector pairs with a known relationship, plus some arbitrary ones.
fn sample_pairs() -> Vec<(Vec<f32>, Vec<f32>)> {
    vec![
        (vec![1.0, 0.0, 0.0], vec![1.0, 0.0, 0.0]),  // identical
        (vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]),  // orthogonal
        (vec![1.0, 0.0, 0.0], vec![-1.0, 0.0, 0.0]), // opposite
        (vec![1.0, 2.0, 3.0], vec![3.0, 2.0, 1.0]),
        (
            vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9],
            vec![0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1],
        ),
        (vec![0.5, -0.5, 0.25, -0.25], vec![0.25, 0.25, -0.5, 0.5]),
    ]
}

#[test]
fn the_three_cases_must_give_three_different_scores() {
    let query = [1.0_f32, 0.0, 0.0];
    let same = [1.0_f32, 0.0, 0.0];
    let orthogonal = [0.0_f32, 1.0, 0.0];
    let opposite = [-1.0_f32, 0.0, 0.0];

    let s_same = cosine_similarity_simd(&query, &same);
    let s_orthogonal = cosine_similarity_simd(&query, &orthogonal);
    let s_opposite = cosine_similarity_simd(&query, &opposite);

    assert!(
        close(s_same, 1.0),
        "identical vectors must score 1.0, got {s_same}"
    );
    assert!(
        close(s_orthogonal, 0.0),
        "orthogonal vectors must score 0.0, got {s_orthogonal}"
    );
    assert!(
        close(s_opposite, -1.0),
        "opposite vectors must score -1.0, got {s_opposite}"
    );

    // The three scores must be pairwise different. The defect made all
    // three of them 0.0 — this is the assertion that catches it.
    assert!(s_same != s_orthogonal);
    assert!(s_orthogonal != s_opposite);
    assert!(s_same != s_opposite);
}

#[test]
fn the_scalar_path_and_the_simd_paths_agree() {
    for (a, b) in sample_pairs() {
        let scalar = cosine_similarity(&a, &b);
        let simd = cosine_similarity_simd(&a, &b);
        let parallel = parallel_simd::cosine_similarity(&a, &b);

        assert!(
            close(scalar, simd),
            "scalar {scalar} and simd {simd} disagree on {a:?} vs {b:?}"
        );
        assert!(
            close(scalar, parallel),
            "scalar {scalar} and parallel-knn {parallel} disagree on {a:?} vs {b:?}"
        );
    }
}

#[test]
fn the_distance_is_the_complement_of_the_similarity() {
    for (a, b) in sample_pairs() {
        let distance = cosine_distance_simd(&a, &b);
        let similarity = cosine_similarity_simd(&a, &b);
        assert!(
            close(distance, 1.0 - similarity),
            "distance {distance} is not the complement of similarity {similarity}"
        );
    }
}

#[test]
fn the_norm_is_the_euclidean_length_not_the_distance_to_itself() {
    let v = [3.0_f32, 4.0];
    // The length of a 3-4-5 triangle is 5.
    assert!(
        close(l2_norm(&v), 5.0),
        "l2_norm must be 5, got {}",
        l2_norm(&v)
    );
    assert!(
        l2_norm(&v) != 0.0,
        "the norm of a non-zero vector must not be 0 — that was the defect"
    );
}

#[test]
fn a_zero_vector_scores_zero_instead_of_dividing_by_zero() {
    let zero = [0.0_f32, 0.0, 0.0];
    let v = [1.0_f32, 2.0, 3.0];
    assert!(close(cosine_similarity_simd(&v, &zero), 0.0));
    assert!(close(cosine_similarity_simd(&zero, &v), 0.0));
}

#[test]
fn mismatched_lengths_score_zero_instead_of_reading_out_of_bounds() {
    let a = [1.0_f32, 0.0, 0.0];
    let b = [1.0_f32, 0.0];
    assert!(close(cosine_similarity_simd(&a, &b), 0.0));
    assert!(close(cosine_similarity_simd(&b, &a), 0.0));
}

#[test]
fn the_batch_helper_matches_the_scalar_one() {
    let query = vec![1.0_f32, 0.5, -0.25];
    let vectors = vec![
        vec![1.0_f32, 0.5, -0.25],
        vec![0.0_f32, 1.0, 0.0],
        vec![-1.0_f32, -0.5, 0.25],
    ];

    let batch = batch_cosine_similarity_simd(&query, &vectors);
    assert_eq!(batch.len(), vectors.len());
    for (score, v) in batch.iter().zip(vectors.iter()) {
        assert!(
            close(*score, cosine_similarity_simd(&query, v)),
            "batch {score} differs from the single-vector result"
        );
    }
}

#[test]
fn flat_index_ranks_the_most_similar_first() {
    let mut index = FlatIndex::new(DistanceMetric::Cosine);
    index.insert(1, &[1.0, 0.0, 0.0]).unwrap(); // identical to the query
    index.insert(2, &[0.9, 0.1, 0.0]).unwrap(); // close
    index.insert(3, &[0.0, 1.0, 0.0]).unwrap(); // orthogonal
    index.insert(4, &[-1.0, 0.0, 0.0]).unwrap(); // opposite
    index.build_index().unwrap();

    let results = index.search(&[1.0, 0.0, 0.0], 4).unwrap();

    let ids: Vec<u64> = results.iter().map(|e| e.id).collect();
    assert_eq!(
        ids,
        vec![1, 2, 3, 4],
        "the cosine ranking must go from most to least similar, got {ids:?}"
    );
    assert!(
        close(results[0].score, 1.0),
        "the identical vector must score 1.0, got {}",
        results[0].score
    );
    assert!(results[0].score > results[1].score);
    assert!(results[1].score > results[2].score);
    assert!(results[2].score > results[3].score);
}

#[test]
fn hnsw_index_ranks_the_most_similar_first() {
    let mut index = HnswIndex::new(DistanceMetric::Cosine);

    // The query itself, a couple of near neighbours, and a set of
    // distractors that are all roughly orthogonal to it.
    index.insert(1, &[1.0, 0.0, 0.0]).unwrap();
    index.insert(2, &[0.95, 0.05, 0.0]).unwrap();
    for (i, v) in [
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [-1.0, 0.0, 0.0],
        [0.7, -0.7, 0.0],
        [0.7, 0.0, -0.7],
    ]
    .iter()
    .enumerate()
    {
        index.insert(10 + i as u64, v).unwrap();
    }
    index.build_index().unwrap();

    let results = index.search(&[1.0, 0.0, 0.0], 3).unwrap();

    assert_eq!(
        results[0].id,
        1,
        "the identical vector must come first, got {:?}",
        results.iter().map(|e| e.id).collect::<Vec<_>>()
    );
    assert!(
        results[0].score > results[1].score,
        "the scores must be sorted from high to low: {:?}",
        results.iter().map(|e| e.score).collect::<Vec<_>>()
    );
    assert!(
        close(results[0].score, 1.0),
        "the identical vector must score 1.0, got {}",
        results[0].score
    );
}

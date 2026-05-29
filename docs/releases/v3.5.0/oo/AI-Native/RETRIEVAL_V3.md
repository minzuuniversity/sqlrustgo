# GMP Retrieval v3

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1363
> **Status**: Planning

## 一、功能描述

BM25 + Vector + Graph 三路融合检索，接入 GMP API。

## 二、架构

### 2.1 三路融合

```
Query → BM25 → Top-K1
     → Vector → Top-K2  
     → Graph → Top-K3
     
     ↓ RRF k=60
     
Merged Top-K
```

### 2.2 RRF (Reciprocal Rank Fusion)

```rust
pub fn rrfFusion(results: Vec<Vec<ScoredDoc>>, k: u64) -> Vec<ScoredDoc> {
    let mut scores: HashMap<DocId, f64> = HashMap::new();
    for result in results {
        for (rank, doc) in result.iter().enumerate() {
            let score = scores.entry(doc.id).or_insert(0.0);
            *score += 1.0 / (k + rank as u64 + 1);
        }
    }
    // Sort by score descending
}
```

## 三、API 接口

```
GET /api/v1/gmp/retrieve
POST /api/v1/gmp/retrieve
```

## 四、验收标准

- BM25 + Vector + Graph 三路融合
- RRF k=60
- 召回率 > 90%，P99 < 200ms

## 五、依赖

- BM25 索引 (v3.4.0)
- Vector 索引 (v3.4.0)  
- Graph Store (v3.4.0)

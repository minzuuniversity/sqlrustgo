# BGE-M3 Embedding

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1363
> **Status**: Planning

## 一、功能描述

bge-m3 embedding 文本向量化，支持多语言（中文/英文）和长文本。

## 二、模型规格

| 属性 | 值 |
|------|-----|
| 模型 | BAAI/bge-m3 |
| 参数量 | 567M |
| 维度 | 1024 |
| 上下文 | 8192 tokens |
| 语言 | 中/英/多语言 |

## 三、API 接口

```rust
pub struct EmbeddingRequest {
    pub texts: Vec<String>,
    pub model: String,  // "bge-m3"
}

pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub token_usage: usize,
}
```

## 四、Ollama 集成

```bash
# Pull model
ollama pull bge-m3

# API call
curl http://localhost:11434/api/embeddings -d '{
  "model": "bge-m3",
  "prompt": "批次 #12345 偏差调查报告"
}'
```

## 五、验收标准

- 向量化延迟 < 100ms
- 维度 1024
- 支持中文分词
